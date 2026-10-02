//! The text-search half of the query surface: FTS5 over the message text
//! projections written at ingest. The caller's query text is compiled into
//! an FTS5 MATCH expression — each word a quoted term, punctuation and
//! operators dropped — so nothing a caller types can inject FTS syntax,
//! and a query that matches nothing is an empty result, never an error.

use crate::store::StoreError;
use rusqlite::params;
use rusqlite::types::Value;
use rusqlite::Connection;

/// One matched message part: where the match lives and the text that
/// matched. Hits are ordered best session first, then by message and part
/// ordinal within a session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchHit {
    /// The session the match belongs to.
    pub session_id: String,
    /// The message the matched part is part of.
    pub message_ordinal: usize,
    /// The matched part's ordinal within its message.
    pub part_ordinal: usize,
    /// The text that matched.
    pub text: String,
}

/// Compile a caller's query into an FTS5 MATCH expression: each
/// alphanumeric run becomes a quoted term (the unicode61 tokenizer folds
/// case), terms joined with the implicit AND. Punctuation and FTS
/// operators are dropped, so `"login!"` searches for `login`.
pub(crate) fn fts_match_expression(query: &str) -> String {
    query
        .split(|character: char| !(character.is_alphanumeric() || character == '_'))
        .filter(|term| !term.is_empty())
        .map(|term| format!("\"{term}\""))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Search the store's message texts. Sessions rank by matching-message
/// count (more first), then best (lowest) bm25 rank, then id; within a
/// session, hits follow message then part ordinal.
pub(crate) fn search(
    conn: &Connection,
    query: &str,
    filter_conjuncts: &str,
    filter_values: &[Value],
) -> Result<Vec<SearchHit>, StoreError> {
    let expression = fts_match_expression(query);
    if expression.is_empty() {
        return Ok(Vec::new());
    }
    let mut values: Vec<Value> = Vec::with_capacity(1 + filter_values.len());
    values.push(Value::from(expression.clone()));
    values.extend(filter_values.iter().cloned());
    // FTS5 auxiliary functions (rank/bm25) are only legal inside a bare
    // full-text query, so the ranking aggregation wraps one in a subquery.
    let ranked_sessions_sql = format!(
        "SELECT fts.session_id, COUNT(*) AS matches, MIN(fts.rank) AS best \
         FROM (SELECT session_id, rank FROM message_fts WHERE message_fts MATCH ?1) AS fts \
         JOIN sessions ON sessions.id = fts.session_id \
         WHERE 1 = 1{filter_conjuncts} \
         GROUP BY fts.session_id \
         ORDER BY matches DESC, best ASC, fts.session_id ASC"
    );
    let session_ids: Vec<String> = conn
        .prepare_cached(&ranked_sessions_sql)
        .map_err(sqlite_error("rank sessions for text search"))?
        .query_map(rusqlite::params_from_iter(&values), |row| row.get(0))
        .map_err(sqlite_error("rank sessions for text search"))?
        .map(|row| row.map_err(sqlite_error("read ranked session id")))
        .collect::<Result<Vec<_>, _>>()?;
    let mut hits = Vec::new();
    for session_id in session_ids {
        let rows = conn
            .prepare_cached(
                "SELECT message_ordinal, part_ordinal, text FROM message_fts \
                 WHERE message_fts MATCH ?1 AND session_id = ?2 \
                 ORDER BY message_ordinal ASC, part_ordinal ASC",
            )
            .map_err(sqlite_error("fetch text search hits"))?
            .query_map(params![expression, session_id], |row| {
                Ok(SearchHit {
                    session_id: session_id.clone(),
                    message_ordinal: row.get::<_, i64>(0)? as usize,
                    part_ordinal: row.get::<_, i64>(1)? as usize,
                    text: row.get(2)?,
                })
            })
            .map_err(sqlite_error("fetch text search hits"))?
            .map(|row| row.map_err(sqlite_error("read text search hit")))
            .collect::<Result<Vec<_>, _>>()?;
        hits.extend(rows);
    }
    Ok(hits)
}

fn sqlite_error(context: &'static str) -> impl FnOnce(rusqlite::Error) -> StoreError {
    move |source| StoreError::Sqlite {
        context: context.to_string(),
        source,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_text_becomes_quoted_terms() {
        assert_eq!(fts_match_expression("login"), "\"login\"".to_string());
        assert_eq!(
            fts_match_expression("Fix the LOGIN bug!"),
            "\"Fix\" \"the\" \"LOGIN\" \"bug\"".to_string()
        );
        assert_eq!(fts_match_expression(""), "");
        assert_eq!(fts_match_expression("!!! ..."), "");
    }
}
