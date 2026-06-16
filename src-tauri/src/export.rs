//! File-system injection (spec §Phase 5). Builds a human-readable Markdown
//! correction document and writes it to the student's local folder as
//! `Correction_[Assignment_Name].md`.

use std::path::{Path, PathBuf};

use crate::db::models::{Assignment, GradeResult, Student};
use crate::error::AppResult;

/// Make a string safe to use as a file/dir name on all OSes.
pub fn sanitize(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();
    let trimmed = cleaned.trim().trim_matches('.').trim();
    if trimmed.is_empty() {
        "untitled".to_string()
    } else {
        trimmed.replace(' ', "_")
    }
}

/// Compose the correction Markdown: header, per-location inline comments with
/// point deductions, then a summary section (Builder pattern).
pub fn build_markdown(student: &Student, assignment: &Assignment, result: &GradeResult) -> String {
    let mut md = String::new();
    md.push_str(&format!("# Correction — {}\n\n", assignment.title));
    md.push_str(&format!("**Student:** {}\n\n", student.full_name()));
    md.push_str(&format!("**Subject:** {} · {}\n\n", assignment.subject, assignment.education_level));
    md.push_str(&format!(
        "**Score:** {} / {}\n\n",
        result.summary.final_score, assignment.max_score
    ));
    md.push_str("---\n\n## Inline corrections\n\n");
    if result.inline_corrections.is_empty() {
        md.push_str("_No deductions — full marks._\n\n");
    } else {
        for c in &result.inline_corrections {
            md.push_str(&format!(
                "- **{}** — {} _( {:+} pts )_\n",
                c.location_reference, c.correction_comment, c.points_deducted
            ));
        }
        md.push('\n');
    }
    md.push_str("---\n\n## Summary\n\n");
    md.push_str(&format!("{}\n\n", result.summary.general_feedback));
    md.push_str(&format!(
        "**Total deducted:** {:+} pts\n",
        result.summary.total_points_deducted
    ));
    md
}

/// Write the correction file into the student's folder. Returns its full path.
/// `students/[First_Last]/Correction_[Assignment_Title].md`
pub fn write_correction(
    students_root: &Path,
    student: &Student,
    assignment: &Assignment,
    result: &GradeResult,
) -> AppResult<PathBuf> {
    let folder = if student.local_folder_path.trim().is_empty() {
        students_root.join(sanitize(&student.full_name()))
    } else {
        PathBuf::from(&student.local_folder_path)
    };
    std::fs::create_dir_all(&folder)?;

    let file = folder.join(format!("Correction_{}.md", sanitize(&assignment.title)));
    std::fs::write(&file, build_markdown(student, assignment, result))?;
    Ok(file)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> (Student, Assignment, GradeResult) {
        let student = Student {
            id: 1,
            class_id: None,
            first_name: "Xavier".into(),
            last_name: "Bergeron".into(),
            local_folder_path: String::new(),
            teacher_notes: String::new(),
            created_at: String::new(),
        };
        let assignment = Assignment {
            id: 1,
            class_id: None,
            title: "Algebra Midterm".into(),
            subject: "Math".into(),
            education_level: "High School".into(),
            custom_persona: String::new(),
            rubric_template: String::new(),
            grading_prompt: String::new(),
            max_score: 10.0,
            created_at: String::new(),
        };
        let result = GradeResult {
            inline_corrections: vec![crate::db::models::InlineCorrection {
                id: 0,
                location_reference: "Q2".into(),
                correction_comment: "Used circumference, not area.".into(),
                points_deducted: -5.0,
            }],
            summary: crate::db::models::GradeSummary {
                general_feedback: "Good algebra; review circle formulas.".into(),
                total_points_deducted: -5.0,
                final_score: 5.0,
            },
        };
        (student, assignment, result)
    }

    #[test]
    fn sanitize_handles_unsafe_chars() {
        assert_eq!(sanitize("Algebra/Quiz: 1"), "Algebra_Quiz__1");
        assert_eq!(sanitize("  "), "untitled");
        assert_eq!(sanitize("Xavier Bergeron"), "Xavier_Bergeron");
    }

    #[test]
    fn markdown_contains_score_and_deduction() {
        let (s, a, r) = sample();
        let md = build_markdown(&s, &a, &r);
        assert!(md.contains("Score:** 5 / 10"));
        assert!(md.contains("Q2"));
        assert!(md.contains("-5 pts"));
    }
}
