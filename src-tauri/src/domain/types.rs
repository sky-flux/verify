use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum Verdict {
    Valid,
    Invalid,
    RiskyCatchAll,
    Unknown,
}

impl Verdict {
    pub fn as_str(&self) -> &'static str {
        match self {
            Verdict::Valid => "Valid",
            Verdict::Invalid => "Invalid",
            Verdict::RiskyCatchAll => "RiskyCatchAll",
            Verdict::Unknown => "Unknown",
        }
    }
}

impl std::str::FromStr for Verdict {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Valid" => Ok(Verdict::Valid),
            "Invalid" => Ok(Verdict::Invalid),
            "RiskyCatchAll" => Ok(Verdict::RiskyCatchAll),
            "Unknown" => Ok(Verdict::Unknown),
            other => Err(format!("unknown verdict: {other}")),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct VerifyResult {
    pub id: String,
    pub email: String,
    pub syntax_valid: bool,
    pub mx_found: bool,
    pub mx_records: Vec<String>,
    pub catch_all: Option<bool>,
    pub smtp_code: Option<u16>,
    pub smtp_message: String,
    pub error: Option<String>,
    pub verdict: Verdict,
    pub checked_at: String,
    pub duration_ms: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct BatchSummary {
    pub total: u32,
    pub valid: u32,
    pub invalid: u32,
    pub unknown: u32,
    pub risky_catch_all: u32,
}

impl BatchSummary {
    pub fn from_results(results: &[VerifyResult]) -> Self {
        let mut summary = BatchSummary {
            total: results.len() as u32,
            ..Default::default()
        };
        for r in results {
            match r.verdict {
                Verdict::Valid => summary.valid += 1,
                Verdict::Invalid => summary.invalid += 1,
                Verdict::Unknown => summary.unknown += 1,
                Verdict::RiskyCatchAll => summary.risky_catch_all += 1,
            }
        }
        summary
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DashboardStats {
    pub total_verified_all_time: u64,
    pub overall_valid_rate: f32,
    pub catch_all_domain_count: u64,
    pub verified_today: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct NetworkHealth {
    pub port25_reachable: bool,
    pub checked_at: String,
    pub detail: Option<String>,
}

/// Which nameservers to query for MX/A lookups. Some ISP/router DNS
/// resolvers hijack NXDOMAIN responses (redirecting nonexistent domains to
/// an ad/captive-portal IP instead of returning "not found"), which makes a
/// genuinely nonexistent domain look like it has a valid A record and skews
/// verification toward false positives. Defaulting to a public resolver
/// avoids that; `System` is offered for users on a corporate VPN/internal
/// DNS that needs to be respected.
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DnsResolver {
    System,
    Cloudflare,
    Google,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub helo_domain: String,
    pub smtp_timeout_seconds: u32,
    pub domain_cooldown_seconds: u32,
    pub max_concurrent_domains: u32,
    pub dns_resolver: DnsResolver,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            helo_domain: "sky-flux-verify.local".to_string(),
            smtp_timeout_seconds: 10,
            domain_cooldown_seconds: 3,
            max_concurrent_domains: 5,
            dns_resolver: DnsResolver::Cloudflare,
        }
    }
}

/// Validates settings values before they're persisted. Bounds are chosen to
/// keep the app from accidentally hammering a target mail server: an
/// unbounded concurrency or a near-zero cooldown is what gets a source IP
/// blocklisted, so we enforce sane ranges here rather than trusting the UI.
/// Field identifiers here are camelCase to match the frontend `Settings`
/// type's JSON keys directly (see `#[serde(rename_all = "camelCase")]` on
/// `Settings`) — the frontend keys its per-field error display off this
/// string with no separate snake_case-to-camelCase mapping table to keep in
/// sync.
pub fn validate_settings(settings: &Settings) -> Result<(), (String, String)> {
    if settings.helo_domain.trim().is_empty() {
        return Err(("heloDomain".to_string(), "不能为空".to_string()));
    }
    if !(1..=120).contains(&settings.smtp_timeout_seconds) {
        return Err((
            "smtpTimeoutSeconds".to_string(),
            "超时时间必须在 1 到 120 秒之间".to_string(),
        ));
    }
    if !(1..=60).contains(&settings.domain_cooldown_seconds) {
        return Err((
            "domainCooldownSeconds".to_string(),
            "冷却间隔必须在 1 到 60 秒之间".to_string(),
        ));
    }
    if !(1..=20).contains(&settings.max_concurrent_domains) {
        return Err((
            "maxConcurrentDomains".to_string(),
            "并发域名数必须在 1 到 20 之间".to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_result(verdict: Verdict) -> VerifyResult {
        VerifyResult {
            id: "id".into(),
            email: "a@b.com".into(),
            syntax_valid: true,
            mx_found: true,
            mx_records: vec![],
            catch_all: None,
            smtp_code: Some(250),
            smtp_message: "OK".into(),
            error: None,
            verdict,
            checked_at: "2026-01-01T00:00:00Z".into(),
            duration_ms: 10,
        }
    }

    #[test]
    fn batch_summary_counts_each_verdict() {
        let results = vec![
            make_result(Verdict::Valid),
            make_result(Verdict::Valid),
            make_result(Verdict::Invalid),
            make_result(Verdict::Unknown),
            make_result(Verdict::RiskyCatchAll),
        ];
        let summary = BatchSummary::from_results(&results);
        assert_eq!(summary.total, 5);
        assert_eq!(summary.valid, 2);
        assert_eq!(summary.invalid, 1);
        assert_eq!(summary.unknown, 1);
        assert_eq!(summary.risky_catch_all, 1);
    }

    #[test]
    fn batch_summary_empty_results() {
        let summary = BatchSummary::from_results(&[]);
        assert_eq!(summary.total, 0);
        assert_eq!(summary.valid, 0);
    }

    #[test]
    fn verdict_round_trips_through_string() {
        for v in [
            Verdict::Valid,
            Verdict::Invalid,
            Verdict::Unknown,
            Verdict::RiskyCatchAll,
        ] {
            let s = v.as_str();
            let parsed: Verdict = s.parse().unwrap();
            assert_eq!(parsed, v);
        }
    }

    #[test]
    fn verdict_from_str_rejects_unknown_value() {
        let parsed = "NotAVerdict".parse::<Verdict>();
        assert!(parsed.is_err());
    }

    #[test]
    fn settings_default_values_are_sane() {
        let settings = Settings::default();
        assert!(settings.smtp_timeout_seconds > 0);
        assert!(settings.max_concurrent_domains > 0);
        assert!(settings.domain_cooldown_seconds > 0);
    }

    #[test]
    fn validate_settings_accepts_defaults() {
        assert!(validate_settings(&Settings::default()).is_ok());
    }

    #[test]
    fn validate_settings_rejects_empty_helo_domain() {
        let settings = Settings {
            helo_domain: "  ".to_string(),
            ..Settings::default()
        };
        let err = validate_settings(&settings).unwrap_err();
        assert_eq!(err.0, "heloDomain");
    }

    #[test]
    fn validate_settings_rejects_zero_timeout() {
        let settings = Settings {
            smtp_timeout_seconds: 0,
            ..Settings::default()
        };
        let err = validate_settings(&settings).unwrap_err();
        assert_eq!(err.0, "smtpTimeoutSeconds");
    }

    #[test]
    fn validate_settings_rejects_concurrency_above_twenty() {
        let settings = Settings {
            max_concurrent_domains: 21,
            ..Settings::default()
        };
        let err = validate_settings(&settings).unwrap_err();
        assert_eq!(err.0, "maxConcurrentDomains");
    }

    #[test]
    fn validate_settings_rejects_cooldown_above_sixty() {
        let settings = Settings {
            domain_cooldown_seconds: 61,
            ..Settings::default()
        };
        let err = validate_settings(&settings).unwrap_err();
        assert_eq!(err.0, "domainCooldownSeconds");
    }
}
