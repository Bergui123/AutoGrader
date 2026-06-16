//! Serializable domain models shared with the React layer. Field names match
//! the TypeScript definitions in `src/lib/types.ts`.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Class {
    pub id: i64,
    pub name: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Student {
    pub id: i64,
    pub class_id: Option<i64>,
    pub first_name: String,
    pub last_name: String,
    pub local_folder_path: String,
    pub teacher_notes: String,
    pub created_at: String,
}

impl Student {
    pub fn full_name(&self) -> String {
        format!("{} {}", self.first_name, self.last_name).trim().to_string()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Assignment {
    pub id: i64,
    pub class_id: Option<i64>,
    pub title: String,
    pub subject: String,
    pub education_level: String,
    pub custom_persona: String,
    pub rubric_template: String,
    pub grading_prompt: String,
    pub max_score: f64,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NewAssignment {
    pub class_id: Option<i64>,
    pub title: String,
    pub subject: String,
    pub education_level: String,
    #[serde(default)]
    pub custom_persona: String,
    #[serde(default)]
    pub rubric_template: String,
    #[serde(default)]
    pub grading_prompt: String,
    #[serde(default = "default_max_score")]
    pub max_score: f64,
}

fn default_max_score() -> f64 {
    100.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Submission {
    pub id: i64,
    pub assignment_id: i64,
    pub student_id: Option<i64>,
    pub source_route: String,
    pub file_type: String,
    pub status: String,
    pub extracted_markdown: String,
    pub verified_markdown: Option<String>,
    pub evaluation_json: Option<String>,
    pub final_score: Option<f64>,
    pub local_output_path: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InlineCorrection {
    #[serde(default)]
    pub id: i64,
    pub location_reference: String,
    pub correction_comment: String,
    pub points_deducted: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GradeSummary {
    pub general_feedback: String,
    pub total_points_deducted: f64,
    pub final_score: f64,
}

/// The exact structured-output schema the grading engine must return (§Phase 4).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GradeResult {
    pub inline_corrections: Vec<InlineCorrection>,
    pub summary: GradeSummary,
}
