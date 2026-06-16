//! Google Vertex AI (Gemini 2.5 Pro) client.
//!
//! Uses the RAM-only ephemeral access token from `AppState.creds` (never read
//! from disk). All payloads handed here are already GDPR-scrubbed by `gdpr`.
//!
//! Two capabilities:
//!   * `extract_handwritten` — Multi-Pass Consensus OCR (spec §Phase 2).
//!   * `grade` — Structured Output grading engine (spec §Phase 4).

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::db::models::{Assignment, GradeResult};
use crate::error::{AppError, AppResult};
use crate::gdpr::ScrubbedPayload;
use crate::state::AppState;

pub struct GeminiClient<'a> {
    state: &'a AppState,
}

impl<'a> GeminiClient<'a> {
    pub fn new(state: &'a AppState) -> Self {
        GeminiClient { state }
    }

    /// Pull the ephemeral token from RAM. Errors if not activated / expired.
    fn token(&self) -> AppResult<String> {
        let guard = self.state.creds.read().expect("creds lock poisoned");
        match guard.as_ref() {
            Some(c) if c.expires_at > chrono::Utc::now() => Ok(c.access_token.clone()),
            _ => Err(AppError::NotActivated),
        }
    }

    fn endpoint(&self) -> String {
        // Generative Language API (AI Studio). The API key authenticates via the
        // `x-goog-api-key` header (set in `generate`), not the URL.
        format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent",
            model = self.state.config.model,
        )
    }

    /// Low-level single generateContent call returning the first candidate's
    /// concatenated text.
    async fn generate(
        &self,
        system: Option<&str>,
        parts: Vec<Part>,
        gen: GenerationConfig,
    ) -> AppResult<String> {
        let token = self.token()?;
        let body = GenerateRequest {
            system_instruction: system.map(|s| SystemInstruction {
                parts: vec![Part::text(s)],
            }),
            contents: vec![Content {
                role: "user".into(),
                parts,
            }],
            generation_config: gen,
        };

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()?;
        let resp = client
            .post(self.endpoint())
            .header("x-goog-api-key", token)
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let detail = resp.text().await.unwrap_or_default();
            return Err(AppError::Other(format!(
                "Vertex AI call failed ({status}): {detail}"
            )));
        }

        let parsed: GenerateResponse = resp.json().await?;
        let text = parsed
            .candidates
            .into_iter()
            .next()
            .and_then(|c| c.content)
            .map(|c| {
                c.parts
                    .into_iter()
                    .filter_map(|p| p.text)
                    .collect::<Vec<_>>()
                    .join("")
            })
            .unwrap_or_default();

        if text.trim().is_empty() {
            return Err(AppError::Other("Vertex AI returned an empty response".into()));
        }
        Ok(text)
    }

    // ── Multi-Pass Consensus OCR ──────────────────────────────────────────

    /// Run three transcription passes at slightly different temperatures, then
    /// a final consensus pass that merges them into one Markdown transcription.
    pub async fn extract_handwritten(&self, images: &[ScrubbedPayload]) -> AppResult<String> {
        if images.is_empty() {
            return Err(AppError::Other("no images to transcribe".into()));
        }
        let (a, b, c) = tokio::join!(
            self.transcribe(images, 0.0),
            self.transcribe(images, 0.05),
            self.transcribe(images, 0.1),
        );
        let variants = [a?, b?, c?];
        self.consensus(&variants).await
    }

    async fn transcribe(&self, images: &[ScrubbedPayload], temperature: f32) -> AppResult<String> {
        let mut parts = vec![Part::text(TRANSCRIBE_INSTRUCTION)];
        for img in images {
            parts.push(Part::inline(&img.mime_type, &img.base64()));
        }
        self.generate(
            Some(TRANSCRIBE_SYSTEM),
            parts,
            GenerationConfig::text(temperature),
        )
        .await
    }

    async fn consensus(&self, variants: &[String]) -> AppResult<String> {
        let mut user = String::from(
            "Here are three independent transcriptions of the same student work.\n\n",
        );
        for (i, v) in variants.iter().enumerate() {
            user.push_str(&format!("=== Transcription {} ===\n{}\n\n", i + 1, v));
        }
        self.generate(
            Some(CONSENSUS_SYSTEM),
            vec![Part::text(&user)],
            GenerationConfig::text(0.0),
        )
        .await
    }

    // ── Structured Output grading ─────────────────────────────────────────

    /// Grade verified student text against the rubric, forcing the exact JSON
    /// schema. Returns a validated `GradeResult`.
    pub async fn grade(
        &self,
        assignment: &Assignment,
        verified_text: &str,
    ) -> AppResult<GradeResult> {
        let system = build_persona(assignment);
        let user = format!(
            "{strategy}\n\nMAXIMUM SCORE: {max}\n\n\
             STUDENT WORK (verified transcription — do not re-interpret):\n{work}\n\n\
             For every issue, add an entry to `inline_corrections` whose \
             `location_reference` quotes the exact student text (or names the \
             Cell/Slide). `points_deducted` MUST be negative. In `summary`, \
             `total_points_deducted` is the sum of all deductions (<= 0) and \
             `final_score` = {max} + total_points_deducted, clamped to a minimum of 0.",
            strategy = grading_strategy(assignment),
            max = assignment.max_score,
            work = verified_text,
        );

        let raw = self
            .generate(
                Some(&system),
                vec![Part::text(&user)],
                GenerationConfig::json(0.0, grade_schema()),
            )
            .await?;

        let mut result: GradeResult = serde_json::from_str(&raw).map_err(|e| {
            AppError::Other(format!("grading response was not valid JSON: {e}; raw: {raw}"))
        })?;

        // Defensive normalization: recompute the score from deductions so the
        // UI total always reconciles, regardless of model arithmetic.
        let total: f64 = result
            .inline_corrections
            .iter()
            .map(|c| c.points_deducted)
            .sum();
        result.summary.total_points_deducted = total;
        result.summary.final_score = (assignment.max_score + total).max(0.0);
        Ok(result)
    }
}

/// Build the dynamic persona System Instruction (spec §Phase 1).
pub fn build_persona(a: &Assignment) -> String {
    let mut s = format!(
        "You are a professional {level} {subject} teacher. Grade the student's \
         work strictly and fairly.",
        level = a.education_level,
        subject = a.subject,
    );
    if !a.custom_persona.trim().is_empty() {
        s.push(' ');
        s.push_str(a.custom_persona.trim());
    }
    s
}

/// Polymorphic grading strategy (Strategy pattern): the grading instructions
/// adapt to whether the teacher supplied a rubric template, a freeform prompt,
/// both, or neither.
fn grading_strategy(a: &Assignment) -> String {
    let has_rubric = !a.rubric_template.trim().is_empty();
    let has_prompt = !a.grading_prompt.trim().is_empty();
    match (has_rubric, has_prompt) {
        (true, true) => format!(
            "Grade strictly against the itemized RUBRIC below, while following the \
             additional grading INSTRUCTIONS for style and leniency.\n\n\
             RUBRIC:\n{}\n\nINSTRUCTIONS:\n{}",
            a.rubric_template.trim(),
            a.grading_prompt.trim()
        ),
        (true, false) => format!(
            "Grade strictly against this itemized RUBRIC, allocating the listed \
             points.\n\nRUBRIC:\n{}",
            a.rubric_template.trim()
        ),
        (false, true) => format!(
            "Grade holistically according to these INSTRUCTIONS.\n\nINSTRUCTIONS:\n{}",
            a.grading_prompt.trim()
        ),
        (false, false) => {
            "Grade on correctness, completeness, and clarity (no explicit rubric provided)."
                .to_string()
        }
    }
}

// ── Prompts ───────────────────────────────────────────────────────────────────

const TRANSCRIBE_SYSTEM: &str =
    "You are a meticulous transcription engine for student exams. You never \
     grade, correct, or comment — you only transcribe.";

const TRANSCRIBE_INSTRUCTION: &str =
    "Transcribe the student's handwritten work from the image(s) into Markdown \
     EXACTLY as written. PRESERVE every spelling error, typo, grammatical \
     mistake, and incorrect math — do NOT fix anything. Format every \
     mathematical expression using LaTeX: inline math as $...$ and display math \
     as $$...$$. Output only the Markdown transcription, with no preamble.";

const CONSENSUS_SYSTEM: &str =
    "You merge multiple OCR transcriptions of the SAME student work into one \
     definitive Markdown transcription. Where versions disagree, pick the most \
     consistent and contextually plausible reading, but still PRESERVE the \
     student's original errors (spelling, grammar, wrong math). Keep all math in \
     LaTeX ($...$ / $$...$$). Output only the final Markdown, no commentary.";

/// The forced response schema for grading (Vertex AI OpenAPI subset).
fn grade_schema() -> Value {
    json!({
        "type": "OBJECT",
        "properties": {
            "inline_corrections": {
                "type": "ARRAY",
                "items": {
                    "type": "OBJECT",
                    "properties": {
                        "location_reference": { "type": "STRING" },
                        "correction_comment": { "type": "STRING" },
                        "points_deducted": { "type": "NUMBER" }
                    },
                    "required": ["location_reference", "correction_comment", "points_deducted"]
                }
            },
            "summary": {
                "type": "OBJECT",
                "properties": {
                    "general_feedback": { "type": "STRING" },
                    "total_points_deducted": { "type": "NUMBER" },
                    "final_score": { "type": "NUMBER" }
                },
                "required": ["general_feedback", "total_points_deducted", "final_score"]
            }
        },
        "required": ["inline_corrections", "summary"]
    })
}

// ── Wire types for the REST call ───────────────────────────────────────────────

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GenerateRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    system_instruction: Option<SystemInstruction>,
    contents: Vec<Content>,
    generation_config: GenerationConfig,
}

#[derive(Serialize)]
struct SystemInstruction {
    parts: Vec<Part>,
}

#[derive(Serialize)]
struct Content {
    role: String,
    parts: Vec<Part>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Part {
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    inline_data: Option<InlineData>,
}

impl Part {
    fn text(s: &str) -> Self {
        Part {
            text: Some(s.to_string()),
            inline_data: None,
        }
    }
    fn inline(mime: &str, base64: &str) -> Self {
        Part {
            text: None,
            inline_data: Some(InlineData {
                mime_type: mime.to_string(),
                data: base64.to_string(),
            }),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct InlineData {
    mime_type: String,
    data: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GenerationConfig {
    temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_schema: Option<Value>,
}

impl GenerationConfig {
    fn text(temperature: f32) -> Self {
        GenerationConfig {
            temperature,
            response_mime_type: None,
            response_schema: None,
        }
    }
    fn json(temperature: f32, schema: Value) -> Self {
        GenerationConfig {
            temperature,
            response_mime_type: Some("application/json".into()),
            response_schema: Some(schema),
        }
    }
}

#[derive(Deserialize)]
struct GenerateResponse {
    #[serde(default)]
    candidates: Vec<Candidate>,
}

#[derive(Deserialize)]
struct Candidate {
    #[serde(default)]
    content: Option<RespContent>,
}

#[derive(Deserialize)]
struct RespContent {
    #[serde(default)]
    parts: Vec<RespPart>,
}

#[derive(Deserialize)]
struct RespPart {
    #[serde(default)]
    text: Option<String>,
}

#[cfg(test)]
mod live_tests {
    use super::*;
    use crate::config::AppConfig;
    use crate::db;
    use crate::db::models::Assignment;
    use crate::state::{AppState, EphemeralCreds};
    use chrono::{Duration, Utc};

    // Exercises the REAL Rust client against live Gemini, verifying that our
    // request structs serialize correctly and the response deserializes into
    // GradeResult. Ignored by default (needs network + AIGRADER_DEV_API_KEY).
    //   cargo test -- --ignored live_structured_grading --nocapture
    #[tokio::test]
    #[ignore = "hits the real Gemini API"]
    async fn live_structured_grading() {
        let config = AppConfig::from_env();
        let key = config
            .dev_api_key
            .clone()
            .expect("set AIGRADER_DEV_API_KEY in .env");
        let conn = db::open_in_memory().unwrap();
        let state = AppState::new(conn, config);
        state.set_creds(EphemeralCreds {
            access_token: key,
            expires_at: Utc::now() + Duration::days(1),
        });

        let assignment = Assignment {
            id: 1,
            class_id: None,
            title: "Quiz".into(),
            subject: "Mathematics".into(),
            education_level: "High School".into(),
            custom_persona: String::new(),
            rubric_template: "Q1 (5 pts): solve 2x+3=7 -> x=2. Q2 (5 pts): area r=3 -> 9*pi."
                .into(),
            grading_prompt: String::new(),
            max_score: 10.0,
            created_at: String::new(),
        };
        let work = "Q1: 2x=4, x=2. Q2: area = 2*pi*3 = 6pi.";

        let client = GeminiClient::new(&state);
        let result = client.grade(&assignment, work).await.expect("grade failed");

        println!(
            "LIVE GRADE -> final_score={}, deducted={}, corrections={}",
            result.summary.final_score,
            result.summary.total_points_deducted,
            result.inline_corrections.len()
        );
        for c in &result.inline_corrections {
            println!("  [{}] {} ({})", c.location_reference, c.correction_comment, c.points_deducted);
        }
        assert!(result.summary.final_score >= 0.0);
    }
}
