//! Serializable domain models shared with the React layer. Field names match
//! the TypeScript definitions in `src/lib/types.ts`.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Student {
    pub id: i64,
    pub display_name: String,
    pub external_ref: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Assignment {
    pub id: i64,
    pub title: String,
    pub subject: String,
    pub education_level: String,
    pub custom_instructions: String,
    pub rubric: String,
    pub max_score: f64,
    pub created_at: String,
}

/// Input payload for creating an assignment (no id / timestamp yet).
#[derive(Debug, Clone, Deserialize)]
pub struct NewAssignment {
    pub title: String,
    pub subject: String,
    pub education_level: String,
    #[serde(default)]
    pub custom_instructions: String,
    #[serde(default)]
    pub rubric: String,
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
    pub original_path: String,
    pub mime_type: Option<String>,
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

/// The exact structured-output schema the grading engine must return.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GradeResult {
    pub inline_corrections: Vec<InlineCorrection>,
    pub summary: GradeSummary,
}
