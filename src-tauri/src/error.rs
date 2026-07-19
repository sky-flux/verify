#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("DNS查询失败: {0}")]
    DnsLookup(String),
    #[error("SMTP连接失败: {0}")]
    SmtpConnection(String),
    #[error("数据库错误: {0}")]
    Database(String),
    #[error("文件读写错误: {0}")]
    Io(String),
    #[error("CSV解析/导出失败: {0}")]
    Csv(String),
    #[error("设置项无效 ({field}): {message}")]
    InvalidSetting { field: String, message: String },
    #[error("批量任务未在运行")]
    NoBatchRunning,
}

/// Serializes as `{ message, field }` rather than a plain string so the
/// frontend can show a validation error under the specific form field it
/// belongs to (see `InvalidSetting`) instead of one opaque global message.
/// `field` is `null` for every other variant, which the frontend treats as
/// "show this as a general error" — see shared/lib/tauri.ts's AppCommandError.
impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("AppError", 2)?;
        state.serialize_field("message", &self.to_string())?;
        let field = match self {
            AppError::InvalidSetting { field, .. } => Some(field.as_str()),
            _ => None,
        };
        state.serialize_field("field", &field)?;
        state.end()
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Io(e.to_string())
    }
}

impl From<csv::Error> for AppError {
    fn from(e: csv::Error) -> Self {
        AppError::Csv(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dns_lookup_error_message_includes_the_detail() {
        let err = AppError::DnsLookup("no MX records".to_string());
        assert_eq!(err.to_string(), "DNS查询失败: no MX records");
    }

    #[test]
    fn invalid_setting_error_names_the_offending_field() {
        let err = AppError::InvalidSetting {
            field: "max_concurrent_domains".to_string(),
            message: "must be between 1 and 20".to_string(),
        };
        assert!(err.to_string().contains("max_concurrent_domains"));
        assert!(err.to_string().contains("must be between 1 and 20"));
    }

    #[test]
    fn non_field_errors_serialize_with_a_null_field() {
        let err = AppError::NoBatchRunning;
        let json = serde_json::to_string(&err).unwrap();
        assert_eq!(json, r#"{"message":"批量任务未在运行","field":null}"#);
    }

    #[test]
    fn invalid_setting_serializes_with_its_field_name() {
        let err = AppError::InvalidSetting {
            field: "maxConcurrentDomains".to_string(),
            message: "并发域名数必须在 1 到 20 之间".to_string(),
        };
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains(r#""field":"maxConcurrentDomains""#));
    }

    #[test]
    fn io_error_converts_via_from() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "missing file");
        let app_err: AppError = io_err.into();
        assert!(matches!(app_err, AppError::Io(_)));
    }
}
