use crate::domain::types::VerifyResult;
use crate::error::AppError;

/// Extracts email addresses from imported CSV/TXT file content. Takes the
/// first field of every row (works for both a single-column TXT-as-CSV file
/// and a CSV with an "email" column first), skipping a header row if its
/// first field isn't itself a plausible address (no '@').
pub fn parse_email_list(content: &str) -> Result<Vec<String>, AppError> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .from_reader(content.as_bytes());

    let mut emails = Vec::new();
    for (index, record) in reader.records().enumerate() {
        let record = record?;
        let Some(first_field) = record.get(0) else {
            continue;
        };
        let candidate = first_field.trim();
        if candidate.is_empty() {
            continue;
        }
        if index == 0 && !candidate.contains('@') {
            // Looks like a header row (e.g. "email"), not an address.
            continue;
        }
        emails.push(candidate.to_string());
    }
    Ok(emails)
}

/// Serializes verification results to CSV text for export, one row per
/// result with a fixed column order matching the UI's results table.
pub fn build_csv_export(results: &[VerifyResult]) -> Result<String, AppError> {
    let mut writer = csv::WriterBuilder::new().from_writer(vec![]);
    writer.write_record([
        "email",
        "verdict",
        "smtp_code",
        "catch_all",
        "duration_ms",
        "smtp_message",
        "checked_at",
    ])?;
    for result in results {
        writer.write_record([
            result.email.as_str(),
            result.verdict.as_str(),
            &result.smtp_code.map(|c| c.to_string()).unwrap_or_default(),
            &result.catch_all.map(|b| b.to_string()).unwrap_or_default(),
            &result.duration_ms.to_string(),
            result.smtp_message.as_str(),
            result.checked_at.as_str(),
        ])?;
    }
    let bytes = writer
        .into_inner()
        .map_err(|e| AppError::Csv(e.to_string()))?;
    String::from_utf8(bytes).map_err(|e| AppError::Csv(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::types::Verdict;

    fn make_result(email: &str) -> VerifyResult {
        VerifyResult {
            id: "id".into(),
            email: email.into(),
            syntax_valid: true,
            mx_found: true,
            mx_records: vec![],
            catch_all: Some(false),
            smtp_code: Some(250),
            smtp_message: "OK".into(),
            error: None,
            verdict: Verdict::Valid,
            checked_at: "2026-01-01T00:00:00Z".into(),
            duration_ms: 42,
        }
    }

    #[test]
    fn parse_email_list_reads_one_email_per_line() {
        let content = "a@example.com\nb@example.com\n";
        let result = parse_email_list(content).unwrap();
        assert_eq!(result, vec!["a@example.com", "b@example.com"]);
    }

    #[test]
    fn parse_email_list_skips_a_header_row() {
        let content = "email\na@example.com\nb@example.com\n";
        let result = parse_email_list(content).unwrap();
        assert_eq!(result, vec!["a@example.com", "b@example.com"]);
    }

    #[test]
    fn parse_email_list_skips_blank_lines() {
        let content = "a@example.com\n\nb@example.com\n";
        let result = parse_email_list(content).unwrap();
        assert_eq!(result, vec!["a@example.com", "b@example.com"]);
    }

    #[test]
    fn parse_email_list_reads_first_column_of_a_multi_column_csv() {
        let content = "email,note\na@example.com,vip\nb@example.com,\n";
        let result = parse_email_list(content).unwrap();
        assert_eq!(result, vec!["a@example.com", "b@example.com"]);
    }

    #[test]
    fn parse_email_list_on_empty_content_returns_empty_vec() {
        let result = parse_email_list("").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn build_csv_export_includes_header_and_one_row_per_result() {
        let results = vec![make_result("a@example.com"), make_result("b@example.com")];
        let csv_text = build_csv_export(&results).unwrap();
        let lines: Vec<&str> = csv_text.lines().collect();
        assert_eq!(lines.len(), 3);
        assert!(lines[0].contains("email"));
        assert!(lines[1].contains("a@example.com"));
        assert!(lines[2].contains("b@example.com"));
    }

    #[test]
    fn build_csv_export_on_empty_results_still_has_header() {
        let csv_text = build_csv_export(&[]).unwrap();
        let lines: Vec<&str> = csv_text.lines().collect();
        assert_eq!(lines.len(), 1);
    }
}
