use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{Row, SqlitePool};

use crate::domain::types::{Verdict, VerifyResult};
use crate::error::AppError;

pub const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS verify_results (
    id            TEXT PRIMARY KEY,
    email         TEXT NOT NULL,
    syntax_valid  INTEGER NOT NULL,
    mx_found      INTEGER NOT NULL,
    mx_records    TEXT NOT NULL,
    catch_all     INTEGER,
    smtp_code     INTEGER,
    smtp_message  TEXT,
    error         TEXT,
    verdict       TEXT NOT NULL,
    duration_ms   INTEGER NOT NULL,
    checked_at    TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_verify_results_email ON verify_results(email);
CREATE INDEX IF NOT EXISTS idx_verify_results_checked_at ON verify_results(checked_at);

CREATE TABLE IF NOT EXISTS catch_all_cache (
    id           TEXT PRIMARY KEY,
    domain       TEXT NOT NULL UNIQUE,
    is_catch_all INTEGER NOT NULL,
    checked_at   TEXT NOT NULL
);
"#;

pub async fn connect(database_url: &str) -> Result<SqlitePool, AppError> {
    let pool = SqlitePoolOptions::new()
        .connect(database_url)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;
    sqlx::raw_sql(SCHEMA)
        .execute(&pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;
    Ok(pool)
}

fn row_to_verify_result(row: &sqlx::sqlite::SqliteRow) -> Result<VerifyResult, AppError> {
    let mx_records_json: String = row.try_get("mx_records").map_err(db_err)?;
    let mx_records: Vec<String> = serde_json::from_str(&mx_records_json)
        .map_err(|e| AppError::Database(format!("corrupt mx_records JSON: {e}")))?;
    let verdict_str: String = row.try_get("verdict").map_err(db_err)?;
    let verdict: Verdict = verdict_str.parse().map_err(AppError::Database)?;

    let smtp_code_raw: Option<i64> = row.try_get("smtp_code").map_err(db_err)?;
    let smtp_code = smtp_code_raw
        .map(u16::try_from)
        .transpose()
        .map_err(|_| AppError::Database("smtp_code out of range for u16".to_string()))?;

    let duration_ms_raw: i64 = row.try_get("duration_ms").map_err(db_err)?;
    let duration_ms = u64::try_from(duration_ms_raw)
        .map_err(|_| AppError::Database("duration_ms out of range for u64".to_string()))?;

    Ok(VerifyResult {
        id: row.try_get("id").map_err(db_err)?,
        email: row.try_get("email").map_err(db_err)?,
        syntax_valid: row.try_get::<i64, _>("syntax_valid").map_err(db_err)? != 0,
        mx_found: row.try_get::<i64, _>("mx_found").map_err(db_err)? != 0,
        mx_records,
        catch_all: row
            .try_get::<Option<i64>, _>("catch_all")
            .map_err(db_err)?
            .map(|v| v != 0),
        smtp_code,
        smtp_message: row
            .try_get::<Option<String>, _>("smtp_message")
            .map_err(db_err)?
            .unwrap_or_default(),
        error: row.try_get("error").map_err(db_err)?,
        verdict,
        checked_at: row.try_get("checked_at").map_err(db_err)?,
        duration_ms,
    })
}

fn db_err(e: sqlx::Error) -> AppError {
    AppError::Database(e.to_string())
}

/// Inserts a new history row, or — if `result.id` matches an existing row
/// (a "重新验证" re-probe of an existing history entry reusing its id) —
/// updates that row in place instead of appending a duplicate. Per spec:
/// re-verifying from History must update the one row, not grow the table
/// with a second entry for the same address.
pub async fn insert_result(pool: &SqlitePool, result: &VerifyResult) -> Result<(), AppError> {
    let mx_records_json = serde_json::to_string(&result.mx_records).unwrap_or_default();
    sqlx::query(
        "INSERT INTO verify_results
         (id, email, syntax_valid, mx_found, mx_records, catch_all, smtp_code, smtp_message, error, verdict, duration_ms, checked_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(id) DO UPDATE SET
            email = excluded.email,
            syntax_valid = excluded.syntax_valid,
            mx_found = excluded.mx_found,
            mx_records = excluded.mx_records,
            catch_all = excluded.catch_all,
            smtp_code = excluded.smtp_code,
            smtp_message = excluded.smtp_message,
            error = excluded.error,
            verdict = excluded.verdict,
            duration_ms = excluded.duration_ms,
            checked_at = excluded.checked_at",
    )
    .bind(&result.id)
    .bind(&result.email)
    .bind(result.syntax_valid as i64)
    .bind(result.mx_found as i64)
    .bind(mx_records_json)
    .bind(result.catch_all.map(|b| b as i64))
    .bind(result.smtp_code.map(|c| c as i64))
    .bind(&result.smtp_message)
    .bind(&result.error)
    .bind(result.verdict.as_str())
    .bind(result.duration_ms as i64)
    .bind(&result.checked_at)
    .execute(pool)
    .await
    .map_err(db_err)?;
    Ok(())
}

pub struct HistoryQuery {
    pub domain_filter: Option<String>,
    pub email_search: Option<String>,
    pub limit: i64,
    pub offset: i64,
}

impl Default for HistoryQuery {
    fn default() -> Self {
        HistoryQuery {
            domain_filter: None,
            email_search: None,
            limit: 50,
            offset: 0,
        }
    }
}

/// Builds the shared `WHERE` clause (and its bind values, applied by the
/// caller) for both `fetch_history` and `count_history` — kept in one place
/// so the two queries can never drift out of sync on what counts as a
/// match.
fn history_where_clause(query: &HistoryQuery) -> String {
    let mut sql = String::from(" WHERE 1=1");
    if query.domain_filter.is_some() {
        sql.push_str(" AND email LIKE ?");
    }
    if query.email_search.is_some() {
        sql.push_str(" AND email LIKE ?");
    }
    sql
}

fn bind_history_filters<'a>(
    mut q: sqlx::query::Query<'a, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'a>>,
    query: &'a HistoryQuery,
) -> sqlx::query::Query<'a, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'a>> {
    if let Some(domain) = &query.domain_filter {
        q = q.bind(format!("%@{domain}"));
    }
    if let Some(search) = &query.email_search {
        q = q.bind(format!("%{search}%"));
    }
    q
}

pub async fn fetch_history(
    pool: &SqlitePool,
    query: &HistoryQuery,
) -> Result<Vec<VerifyResult>, AppError> {
    let sql = format!(
        "SELECT * FROM verify_results{} ORDER BY checked_at DESC LIMIT ? OFFSET ?",
        history_where_clause(query)
    );
    let q = bind_history_filters(sqlx::query(&sql), query)
        .bind(query.limit)
        .bind(query.offset);

    let rows = q.fetch_all(pool).await.map_err(db_err)?;
    rows.iter().map(row_to_verify_result).collect()
}

/// Total number of rows matching `query`'s filters, ignoring `limit`/
/// `offset` — lets the UI render "共 N 条" and correctly disable "下一页"
/// on the last page instead of guessing from the current page's row count.
pub async fn count_history(pool: &SqlitePool, query: &HistoryQuery) -> Result<i64, AppError> {
    let sql = format!(
        "SELECT COUNT(*) as c FROM verify_results{}",
        history_where_clause(query)
    );
    let row = bind_history_filters(sqlx::query(&sql), query)
        .fetch_one(pool)
        .await
        .map_err(db_err)?;
    row.try_get::<i64, _>("c").map_err(db_err)
}

pub async fn distinct_domains(pool: &SqlitePool) -> Result<Vec<String>, AppError> {
    let rows = sqlx::query("SELECT DISTINCT email FROM verify_results")
        .fetch_all(pool)
        .await
        .map_err(db_err)?;
    let mut domains: Vec<String> = rows
        .iter()
        .filter_map(|row| {
            let email: String = row.try_get("email").ok()?;
            email.rsplit_once('@').map(|(_, d)| d.to_string())
        })
        .collect();
    domains.sort();
    domains.dedup();
    Ok(domains)
}

/// How long a cached catch-all verdict stays trustworthy before a fresh
/// probe is required. A domain's catch-all status can change (mail
/// migration, config change) — without expiry a stale cache entry would
/// silently misclassify every future verification for that domain forever.
pub const CATCH_ALL_CACHE_TTL: chrono::Duration = chrono::Duration::days(7);

pub async fn is_domain_catch_all(
    pool: &SqlitePool,
    domain: &str,
) -> Result<Option<bool>, AppError> {
    let cutoff = (chrono::Utc::now() - CATCH_ALL_CACHE_TTL).to_rfc3339();
    let row =
        sqlx::query("SELECT is_catch_all FROM catch_all_cache WHERE domain = ? AND checked_at > ?")
            .bind(domain)
            .bind(cutoff)
            .fetch_optional(pool)
            .await
            .map_err(db_err)?;
    Ok(row.map(|r| r.try_get::<i64, _>("is_catch_all").unwrap_or(0) != 0))
}

pub async fn upsert_catch_all(
    pool: &SqlitePool,
    domain: &str,
    is_catch_all: bool,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO catch_all_cache (id, domain, is_catch_all, checked_at)
         VALUES (?, ?, ?, ?)
         ON CONFLICT(domain) DO UPDATE SET is_catch_all = excluded.is_catch_all, checked_at = excluded.checked_at",
    )
    .bind(uuid::Uuid::now_v7().to_string())
    .bind(domain)
    .bind(is_catch_all as i64)
    .bind(chrono::Utc::now().to_rfc3339())
    .execute(pool)
    .await
    .map_err(db_err)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_result(email: &str, verdict: Verdict) -> VerifyResult {
        VerifyResult {
            id: uuid::Uuid::now_v7().to_string(),
            email: email.into(),
            syntax_valid: true,
            mx_found: true,
            mx_records: vec!["mx1.example.com".into()],
            catch_all: Some(false),
            smtp_code: Some(250),
            smtp_message: "OK".into(),
            error: None,
            verdict,
            checked_at: chrono::Utc::now().to_rfc3339(),
            duration_ms: 5,
        }
    }

    async fn memory_pool() -> SqlitePool {
        connect("sqlite::memory:").await.unwrap()
    }

    #[tokio::test]
    async fn insert_and_fetch_round_trips_all_fields() {
        let pool = memory_pool().await;
        let result = make_result("a@example.com", Verdict::Valid);
        insert_result(&pool, &result).await.unwrap();

        let fetched = fetch_history(&pool, &HistoryQuery::default())
            .await
            .unwrap();
        assert_eq!(fetched.len(), 1);
        assert_eq!(fetched[0].email, "a@example.com");
        assert_eq!(fetched[0].verdict, Verdict::Valid);
        assert_eq!(fetched[0].mx_records, vec!["mx1.example.com".to_string()]);
    }

    #[tokio::test]
    async fn insert_result_with_an_existing_id_updates_in_place_instead_of_duplicating() {
        let pool = memory_pool().await;
        let mut result = make_result("a@example.com", Verdict::Invalid);
        insert_result(&pool, &result).await.unwrap();

        // A "重新验证" re-probe reuses the same id but gets a fresh verdict.
        result.verdict = Verdict::Valid;
        result.smtp_message = "250 OK".into();
        insert_result(&pool, &result).await.unwrap();

        let fetched = fetch_history(&pool, &HistoryQuery::default())
            .await
            .unwrap();
        assert_eq!(
            fetched.len(),
            1,
            "reverifying must update the row, not insert a duplicate"
        );
        assert_eq!(fetched[0].verdict, Verdict::Valid);
        assert_eq!(fetched[0].smtp_message, "250 OK");
    }

    #[tokio::test]
    async fn fetch_history_orders_most_recent_first() {
        let pool = memory_pool().await;
        let mut older = make_result("old@example.com", Verdict::Valid);
        older.checked_at = "2020-01-01T00:00:00Z".into();
        let mut newer = make_result("new@example.com", Verdict::Valid);
        newer.checked_at = "2026-01-01T00:00:00Z".into();
        insert_result(&pool, &older).await.unwrap();
        insert_result(&pool, &newer).await.unwrap();

        let fetched = fetch_history(&pool, &HistoryQuery::default())
            .await
            .unwrap();
        assert_eq!(fetched[0].email, "new@example.com");
        assert_eq!(fetched[1].email, "old@example.com");
    }

    #[tokio::test]
    async fn fetch_history_filters_by_domain() {
        let pool = memory_pool().await;
        insert_result(&pool, &make_result("a@x.com", Verdict::Valid))
            .await
            .unwrap();
        insert_result(&pool, &make_result("b@y.com", Verdict::Valid))
            .await
            .unwrap();

        let query = HistoryQuery {
            domain_filter: Some("x.com".to_string()),
            ..Default::default()
        };
        let fetched = fetch_history(&pool, &query).await.unwrap();
        assert_eq!(fetched.len(), 1);
        assert_eq!(fetched[0].email, "a@x.com");
    }

    #[tokio::test]
    async fn fetch_history_filters_by_email_search() {
        let pool = memory_pool().await;
        insert_result(&pool, &make_result("alice@x.com", Verdict::Valid))
            .await
            .unwrap();
        insert_result(&pool, &make_result("bob@x.com", Verdict::Valid))
            .await
            .unwrap();

        let query = HistoryQuery {
            email_search: Some("ali".to_string()),
            ..Default::default()
        };
        let fetched = fetch_history(&pool, &query).await.unwrap();
        assert_eq!(fetched.len(), 1);
        assert_eq!(fetched[0].email, "alice@x.com");
    }

    #[tokio::test]
    async fn count_history_matches_filters_independent_of_limit_offset() {
        let pool = memory_pool().await;
        for email in ["a@x.com", "b@x.com", "c@y.com"] {
            insert_result(&pool, &make_result(email, Verdict::Valid))
                .await
                .unwrap();
        }

        assert_eq!(
            count_history(&pool, &HistoryQuery::default())
                .await
                .unwrap(),
            3
        );

        let filtered = HistoryQuery {
            domain_filter: Some("x.com".to_string()),
            limit: 1,
            offset: 0,
            ..Default::default()
        };
        assert_eq!(
            count_history(&pool, &filtered).await.unwrap(),
            2,
            "count must ignore limit/offset and only reflect the filter"
        );
    }

    #[tokio::test]
    async fn distinct_domains_returns_sorted_unique_domains() {
        let pool = memory_pool().await;
        insert_result(&pool, &make_result("a@z.com", Verdict::Valid))
            .await
            .unwrap();
        insert_result(&pool, &make_result("b@a.com", Verdict::Valid))
            .await
            .unwrap();
        insert_result(&pool, &make_result("c@z.com", Verdict::Valid))
            .await
            .unwrap();

        let domains = distinct_domains(&pool).await.unwrap();
        assert_eq!(domains, vec!["a.com".to_string(), "z.com".to_string()]);
    }

    #[tokio::test]
    async fn fetch_history_errors_instead_of_silently_dropping_corrupt_mx_records() {
        let pool = memory_pool().await;
        insert_result(&pool, &make_result("a@x.com", Verdict::Valid))
            .await
            .unwrap();
        // Corrupt the stored JSON directly, simulating on-disk corruption.
        sqlx::query("UPDATE verify_results SET mx_records = 'not json' WHERE email = 'a@x.com'")
            .execute(&pool)
            .await
            .unwrap();

        let result = fetch_history(&pool, &HistoryQuery::default()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn catch_all_cache_upsert_then_read_round_trips() {
        let pool = memory_pool().await;
        assert_eq!(
            is_domain_catch_all(&pool, "example.com").await.unwrap(),
            None
        );

        upsert_catch_all(&pool, "example.com", true).await.unwrap();
        assert_eq!(
            is_domain_catch_all(&pool, "example.com").await.unwrap(),
            Some(true)
        );

        upsert_catch_all(&pool, "example.com", false).await.unwrap();
        assert_eq!(
            is_domain_catch_all(&pool, "example.com").await.unwrap(),
            Some(false)
        );
    }

    #[tokio::test]
    async fn is_domain_catch_all_ignores_entries_older_than_the_ttl() {
        let pool = memory_pool().await;
        let stale_checked_at =
            (chrono::Utc::now() - CATCH_ALL_CACHE_TTL - chrono::Duration::days(1)).to_rfc3339();
        sqlx::query(
            "INSERT INTO catch_all_cache (id, domain, is_catch_all, checked_at) VALUES (?, ?, ?, ?)",
        )
        .bind(uuid::Uuid::now_v7().to_string())
        .bind("stale.example.com")
        .bind(1i64)
        .bind(stale_checked_at)
        .execute(&pool)
        .await
        .unwrap();

        assert_eq!(
            is_domain_catch_all(&pool, "stale.example.com")
                .await
                .unwrap(),
            None,
            "an expired cache entry must be treated as a cache miss, forcing a fresh probe"
        );
    }
}
