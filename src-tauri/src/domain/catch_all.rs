use std::time::Duration;
use uuid::Uuid;

use super::smtp::{classify_outcome, probe_rcpt, HandshakeParams};
use super::types::Verdict;

/// Builds a one-off, effectively-unguessable local part for catch-all
/// probing. Uses uuid v4 (not v7) deliberately: this address is never
/// persisted, so there's no benefit to v7's time-ordering, and v4's full
/// randomness is exactly what's needed to guarantee the mailbox has never
/// legitimately existed at the target domain.
pub fn random_probe_address(domain: &str) -> String {
    format!("skyfluxverify-probe-{}@{}", Uuid::new_v4(), domain)
}

/// Probes whether `domain` accepts RCPT TO for *any* address (catch-all),
/// which would make every other verdict for that domain untrustworthy.
pub async fn is_catch_all(
    domain: &str,
    host: &str,
    helo_domain: &str,
    mail_from: &str,
    timeout: Duration,
) -> bool {
    let probe_address = random_probe_address(domain);
    let outcome = probe_rcpt(HandshakeParams {
        host,
        port: 25,
        helo_domain,
        mail_from,
        rcpt_to: &probe_address,
        timeout,
    })
    .await;
    classify_outcome(&outcome) == Verdict::Valid
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_probe_address_is_scoped_to_the_given_domain() {
        let addr = random_probe_address("example.com");
        assert!(addr.ends_with("@example.com"));
    }

    #[test]
    fn random_probe_address_is_different_every_call() {
        let a = random_probe_address("example.com");
        let b = random_probe_address("example.com");
        assert_ne!(a, b);
    }

    #[test]
    fn random_probe_address_has_syntactically_valid_shape() {
        let addr = random_probe_address("example.com");
        assert_eq!(addr.matches('@').count(), 1);
        let local = addr.split('@').next().unwrap();
        assert!(!local.is_empty());
    }
}
