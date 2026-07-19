use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::time::timeout;

use super::types::Verdict;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmtpProbeOutcome {
    pub code: Option<u16>,
    pub message: String,
    pub error: Option<String>,
}

/// Classifies an RCPT TO response code per the SMTP status code ranges this
/// project cares about. Codes outside the documented ranges default to
/// Unknown rather than guessing, since a misclassification here directly
/// mislabels a real mailbox as valid/invalid.
pub fn classify_code(code: u16) -> Verdict {
    match code {
        250 | 251 => Verdict::Valid,
        550 | 551 | 553 => Verdict::Invalid,
        450 | 451 | 452 | 421 => Verdict::Unknown,
        _ => Verdict::Unknown,
    }
}

/// Same as `classify_code` but for when no response code was obtained at all
/// (connection failure/timeout) — always Unknown, since we can't distinguish
/// "mailbox doesn't exist" from "network/IP reputation problem".
pub fn classify_outcome(outcome: &SmtpProbeOutcome) -> Verdict {
    match outcome.code {
        Some(code) => classify_code(code),
        None => Verdict::Unknown,
    }
}

fn parse_reply_code(line: &str) -> Option<u16> {
    line.get(0..3)?.parse().ok()
}

/// True when `line` is a non-final line of a multi-line SMTP reply, e.g.
/// "250-PIPELINING" (hyphen after the code) as opposed to "250 PIPELINING".
fn is_continuation_line(line: &str) -> bool {
    line.as_bytes().get(3) == Some(&b'-')
}

async fn read_reply<R: tokio::io::AsyncBufRead + Unpin>(
    reader: &mut R,
) -> std::io::Result<(u16, String)> {
    let mut full_message = String::new();
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).await?;
        if line.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "connection closed while reading SMTP reply",
            ));
        }
        let trimmed = line.trim_end();
        full_message.push_str(trimmed);
        let continuation = is_continuation_line(trimmed);
        if !continuation {
            let code = parse_reply_code(trimmed).ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("malformed SMTP reply: {trimmed}"),
                )
            })?;
            return Ok((code, full_message));
        }
        full_message.push(' ');
    }
}

pub struct HandshakeParams<'a> {
    pub host: &'a str,
    pub port: u16,
    pub helo_domain: &'a str,
    pub mail_from: &'a str,
    pub rcpt_to: &'a str,
    pub timeout: Duration,
}

/// Performs the SMTP handshake up to and including RCPT TO, then QUITs.
/// Deliberately never sends DATA — this only probes mailbox existence, it
/// never delivers a message.
pub async fn probe_rcpt(params: HandshakeParams<'_>) -> SmtpProbeOutcome {
    match timeout(params.timeout, run_handshake(&params)).await {
        Ok(Ok(outcome)) => outcome,
        Ok(Err(e)) => {
            log::warn!(
                "SMTP connection failed for {}:{}: {e}",
                params.host,
                params.port
            );
            SmtpProbeOutcome {
                code: None,
                message: String::new(),
                error: Some(e.to_string()),
            }
        }
        Err(_) => {
            log::warn!(
                "SMTP handshake with {}:{} timed out after {:?}",
                params.host,
                params.port,
                params.timeout
            );
            SmtpProbeOutcome {
                code: None,
                message: String::new(),
                error: Some("SMTP handshake timed out".to_string()),
            }
        }
    }
}

async fn run_handshake(params: &HandshakeParams<'_>) -> std::io::Result<SmtpProbeOutcome> {
    let stream = TcpStream::connect((params.host, params.port)).await?;
    let (read_half, mut write_half) = stream.into_split();
    let mut reader = BufReader::new(read_half);

    // 220 welcome
    read_reply(&mut reader).await?;

    write_half
        .write_all(format!("EHLO {}\r\n", params.helo_domain).as_bytes())
        .await?;
    read_reply(&mut reader).await?;

    write_half
        .write_all(format!("MAIL FROM:<{}>\r\n", params.mail_from).as_bytes())
        .await?;
    read_reply(&mut reader).await?;

    write_half
        .write_all(format!("RCPT TO:<{}>\r\n", params.rcpt_to).as_bytes())
        .await?;
    let (code, message) = read_reply(&mut reader).await?;

    let _ = write_half.write_all(b"QUIT\r\n").await;

    Ok(SmtpProbeOutcome {
        code: Some(code),
        message,
        error: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::BufReader as TokioBufReader;

    #[test]
    fn classify_code_2xx_is_valid() {
        assert_eq!(classify_code(250), Verdict::Valid);
        assert_eq!(classify_code(251), Verdict::Valid);
    }

    #[test]
    fn classify_code_5xx_permanent_is_invalid() {
        assert_eq!(classify_code(550), Verdict::Invalid);
        assert_eq!(classify_code(551), Verdict::Invalid);
        assert_eq!(classify_code(553), Verdict::Invalid);
    }

    #[test]
    fn classify_code_4xx_temporary_is_unknown() {
        assert_eq!(classify_code(450), Verdict::Unknown);
        assert_eq!(classify_code(451), Verdict::Unknown);
        assert_eq!(classify_code(452), Verdict::Unknown);
        assert_eq!(classify_code(421), Verdict::Unknown);
    }

    #[test]
    fn classify_code_unexpected_code_defaults_unknown() {
        assert_eq!(classify_code(999), Verdict::Unknown);
    }

    #[test]
    fn classify_outcome_with_no_code_is_unknown() {
        let outcome = SmtpProbeOutcome {
            code: None,
            message: String::new(),
            error: Some("connection refused".into()),
        };
        assert_eq!(classify_outcome(&outcome), Verdict::Unknown);
    }

    #[test]
    fn classify_outcome_with_code_delegates_to_classify_code() {
        let outcome = SmtpProbeOutcome {
            code: Some(550),
            message: "5.1.1 No such user".into(),
            error: None,
        };
        assert_eq!(classify_outcome(&outcome), Verdict::Invalid);
    }

    #[test]
    fn parse_reply_code_reads_leading_three_digits() {
        assert_eq!(parse_reply_code("250 OK"), Some(250));
        assert_eq!(parse_reply_code("550-partial"), Some(550));
        assert_eq!(parse_reply_code("nope"), None);
    }

    #[test]
    fn is_continuation_line_detects_hyphen_after_code() {
        assert!(is_continuation_line("250-PIPELINING"));
        assert!(!is_continuation_line("250 PIPELINING"));
    }

    #[tokio::test]
    async fn read_reply_handles_single_line_reply() {
        let data = b"250 OK\r\n".to_vec();
        let mut reader = TokioBufReader::new(&data[..]);
        let (code, message) = read_reply(&mut reader).await.unwrap();
        assert_eq!(code, 250);
        assert_eq!(message, "250 OK");
    }

    #[tokio::test]
    async fn read_reply_handles_multiline_reply() {
        let data =
            b"250-example.com at your service\r\n250-PIPELINING\r\n250 8BITMIME\r\n".to_vec();
        let mut reader = TokioBufReader::new(&data[..]);
        let (code, message) = read_reply(&mut reader).await.unwrap();
        assert_eq!(code, 250);
        assert!(message.contains("PIPELINING"));
        assert!(message.contains("8BITMIME"));
    }

    #[tokio::test]
    async fn read_reply_errors_on_connection_closed_mid_reply() {
        let data: Vec<u8> = vec![];
        let mut reader = TokioBufReader::new(&data[..]);
        let result = read_reply(&mut reader).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn probe_rcpt_times_out_against_unroutable_host() {
        let params = HandshakeParams {
            host: "10.255.255.1", // TEST-NET-1 style unroutable address, should time out
            port: 25,
            helo_domain: "test.local",
            mail_from: "verify@test.local",
            rcpt_to: "someone@example.com",
            timeout: Duration::from_millis(300),
        };
        let outcome = probe_rcpt(params).await;
        assert!(outcome.code.is_none());
        assert!(outcome.error.is_some());
    }
}
