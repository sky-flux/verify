use hickory_resolver::config::{ResolverConfig, CLOUDFLARE, GOOGLE};
use hickory_resolver::net::runtime::TokioRuntimeProvider;
use hickory_resolver::proto::rr::RData;
use hickory_resolver::TokioResolver;

use super::types::DnsResolver;

/// Minimal, dependency-free syntax check per RFC 5321/5322's practical
/// subset: exactly one '@', a non-empty local part with no whitespace, and a
/// domain part containing at least one '.' with no leading/trailing dot.
/// This intentionally does not attempt full RFC 5322 grammar (quoted
/// strings, comments, etc.) — those are vanishingly rare in real-world
/// addresses and not worth the complexity for a validity pre-check ahead of
/// the authoritative SMTP probe.
pub fn is_syntax_valid(email: &str) -> bool {
    let email = email.trim();
    if email.is_empty() || email.contains(char::is_whitespace) {
        return false;
    }
    let mut parts = email.splitn(2, '@');
    let local = match parts.next() {
        Some(l) if !l.is_empty() => l,
        _ => return false,
    };
    let domain = match parts.next() {
        Some(d) if !d.is_empty() => d,
        _ => return false,
    };
    if local.contains('@') || domain.contains('@') {
        return false;
    }
    if domain.starts_with('.') || domain.ends_with('.') || !domain.contains('.') {
        return false;
    }
    if domain.contains("..") {
        return false;
    }
    true
}

pub fn extract_domain(email: &str) -> Option<&str> {
    email.rsplit_once('@').map(|(_, domain)| domain)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MxLookupResult {
    pub mx_found: bool,
    pub hosts: Vec<String>,
}

fn build_resolver(choice: DnsResolver) -> Result<TokioResolver, ()> {
    match choice {
        // `builder_tokio()` reads the OS's actual configured nameservers
        // (/etc/resolv.conf on Unix), respecting the user's real DNS/VPN
        // setup. Falls back to Cloudflare if that's unreadable, rather than
        // silently returning not-found.
        DnsResolver::System => TokioResolver::builder_tokio()
            .and_then(|builder| builder.build())
            .or_else(|_| {
                TokioResolver::builder_with_config(
                    ResolverConfig::udp_and_tcp(&CLOUDFLARE),
                    TokioRuntimeProvider::default(),
                )
                .build()
            })
            .map_err(|_| ()),
        DnsResolver::Cloudflare => TokioResolver::builder_with_config(
            ResolverConfig::udp_and_tcp(&CLOUDFLARE),
            TokioRuntimeProvider::default(),
        )
        .build()
        .map_err(|_| ()),
        DnsResolver::Google => TokioResolver::builder_with_config(
            ResolverConfig::udp_and_tcp(&GOOGLE),
            TokioRuntimeProvider::default(),
        )
        .build()
        .map_err(|_| ()),
    }
}

/// Resolves mail exchangers for `domain`, sorted by MX preference
/// (lowest/highest-priority first). Falls back to the domain's own A/AAAA
/// record per RFC 5321 §5.1 when no MX records exist — some domains route
/// mail directly to their web server's address.
///
/// `resolver_choice` matters beyond just "which servers to ask": some ISP/
/// router DNS resolvers hijack NXDOMAIN responses for nonexistent domains
/// (redirecting to an ad/captive-portal IP instead of returning "not
/// found"), which would otherwise make every nonexistent domain look like
/// it has a valid A record. Defaulting to a public resolver (see
/// `Settings::default`) avoids that.
pub async fn lookup_mx(domain: &str, resolver_choice: DnsResolver) -> MxLookupResult {
    let not_found = MxLookupResult {
        mx_found: false,
        hosts: vec![],
    };

    let Ok(resolver) = build_resolver(resolver_choice) else {
        log::warn!(
            "could not build any DNS resolver (choice: {resolver_choice:?}) — skipping MX lookup for {domain}"
        );
        return not_found;
    };

    let mx_response = resolver.mx_lookup(domain).await;
    if let Err(e) = &mx_response {
        log::debug!("MX lookup failed for {domain}: {e} (falling back to A/AAAA)");
    }
    if let Ok(response) = mx_response {
        let mut records: Vec<(u16, String)> = response
            .answers()
            .iter()
            .filter_map(|record| match &record.data {
                RData::MX(mx) => Some((
                    mx.preference,
                    mx.exchange.to_utf8().trim_end_matches('.').to_string(),
                )),
                _ => None,
            })
            .collect();
        if !records.is_empty() {
            records.sort_by_key(|(preference, _)| *preference);
            return MxLookupResult {
                mx_found: true,
                hosts: records.into_iter().map(|(_, host)| host).collect(),
            };
        }
    }

    // No MX records — fall back to A record per RFC 5321.
    if resolver.lookup_ip(domain).await.is_ok() {
        return MxLookupResult {
            mx_found: true,
            hosts: vec![domain.to_string()],
        };
    }

    MxLookupResult {
        mx_found: false,
        hosts: vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_a_simple_valid_address() {
        assert!(is_syntax_valid("user@example.com"));
    }

    #[test]
    fn accepts_addresses_with_plus_and_dot_in_local_part() {
        assert!(is_syntax_valid("first.last+tag@example.co.uk"));
    }

    #[test]
    fn rejects_missing_at_sign() {
        assert!(!is_syntax_valid("userexample.com"));
    }

    #[test]
    fn rejects_multiple_at_signs() {
        assert!(!is_syntax_valid("user@@example.com"));
        assert!(!is_syntax_valid("us@er@example.com"));
    }

    #[test]
    fn rejects_empty_local_or_domain_part() {
        assert!(!is_syntax_valid("@example.com"));
        assert!(!is_syntax_valid("user@"));
    }

    #[test]
    fn rejects_domain_without_a_dot() {
        assert!(!is_syntax_valid("user@localhost"));
    }

    #[test]
    fn rejects_internal_whitespace_but_trims_surrounding_whitespace() {
        assert!(!is_syntax_valid("us er@example.com"));
        // Leading/trailing whitespace is trimmed, not rejected — pasted
        // addresses commonly carry a stray space or newline.
        assert!(is_syntax_valid(" user@example.com "));
    }

    #[test]
    fn rejects_leading_trailing_or_double_dot_in_domain() {
        assert!(!is_syntax_valid("user@.example.com"));
        assert!(!is_syntax_valid("user@example.com."));
        assert!(!is_syntax_valid("user@exa..mple.com"));
    }

    #[test]
    fn rejects_empty_string() {
        assert!(!is_syntax_valid(""));
        assert!(!is_syntax_valid("   "));
    }

    #[test]
    fn extract_domain_returns_part_after_last_at() {
        assert_eq!(extract_domain("user@example.com"), Some("example.com"));
        assert_eq!(extract_domain("no-at-sign"), None);
    }

    #[tokio::test]
    async fn lookup_mx_on_a_domain_with_no_dns_at_all_reports_not_found() {
        // A syntactically valid but non-existent TLD: no MX, no A record.
        // Deliberately uses Cloudflare rather than System — this is exactly
        // the case a hijacking ISP/router resolver would get wrong.
        let result = lookup_mx(
            "this-domain-should-not-exist-sky-flux-verify.invalid",
            DnsResolver::Cloudflare,
        )
        .await;
        assert!(!result.mx_found);
        assert!(result.hosts.is_empty());
    }

    #[tokio::test]
    async fn lookup_mx_resolves_a_real_domains_mx_records_via_cloudflare() {
        let result = lookup_mx("gmail.com", DnsResolver::Cloudflare).await;
        assert!(result.mx_found);
        assert!(!result.hosts.is_empty());
    }
}
