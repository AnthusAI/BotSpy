//! The vector-search half of the query surface: vec0 k-NN over the
//! message-part embeddings written at ingest. Sessions rank by best
//! cosine similarity (the shipped embedder L2-normalizes, so cosine
//! similarity is `1 - distance` under the vec0 cosine metric); within a
//! session, parts follow their distance order. The engine stays
//! invisible: callers hand over a query vector and get located, scored
//! hits.

use crate::store::text::SearchHit;
use crate::store::StoreError;
use rusqlite::params;
use rusqlite::types::Value;
use rusqlite::Connection;
use std::collections::{HashMap, HashSet};

/// The semantic half of a search: the sessions that embed near the query,
/// best first with their best cosine similarity, plus the located hits.
pub(crate) struct SemanticMatches {
    /// Sessions in best-similarity order (ties broken by session id).
    pub ranking: Vec<(String, f64)>,
    /// The matched parts, session rank order first, then distance order
    /// within a session. Every hit carries its session's similarity.
    pub hits: Vec<SearchHit>,
}

/// Cap on the neighbors one semantic search pulls out of vec0: a k-NN
/// query is brute force anyway, so this bounds only the join size.
const MAX_NEIGHBORS: usize = 4096;

/// The cosine similarity a stored part must reach to count as a semantic
/// match. For the shipped L2-normalized all-MiniLM-L6-v2 embeddings,
/// unrelated texts land near zero (observed spread roughly -0.15 to
/// +0.15) while topically related pairs start around 0.4, so 0.3
/// separates signal from noise; anything weaker is not a match rather
/// than a low-ranked one.
pub(crate) const SEMANTIC_FLOOR: f64 = 0.3;

/// k-NN over the stored embeddings. `filter_conjuncts`/`filter_values`
/// restrict which sessions may appear (compiled from a [`crate::store::
/// query::SessionFilter`]); sessions without vectors simply never match.
pub(crate) fn semantic_matches(
    conn: &Connection,
    query_vector: &[f32],
    filter_conjuncts: &str,
    filter_values: &[Value],
    max_sessions: Option<usize>,
) -> Result<SemanticMatches, StoreError> {
    let bytes: Vec<u8> = query_vector.iter().flat_map(|f| f.to_le_bytes()).collect();
    let neighbors: Vec<(i64, f64)> = conn
        .prepare_cached(
            "SELECT rowid, distance FROM message_vec \
             WHERE embedding MATCH ?1 AND k = ?2 ORDER BY distance ASC",
        )
        .map_err(sqlite_error("vector k-nn query"))?
        .query_map(params![bytes, MAX_NEIGHBORS as i64], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, f64>(1)?))
        })
        .map_err(sqlite_error("vector k-nn query"))?
        .map(|row| row.map_err(sqlite_error("read vector neighbor")))
        .collect::<Result<Vec<_>, _>>()?;
    if neighbors.is_empty() {
        return Ok(SemanticMatches {
            ranking: Vec::new(),
            hits: Vec::new(),
        });
    }
    let coords = part_coords(conn, &neighbors)?;
    let allowed = allowed_sessions(conn, filter_conjuncts, filter_values)?;
    // Sessions in best-first order: a session's similarity is its nearest
    // part's (neighbors arrive distance-ordered). Ties break by id.
    let mut order: Vec<String> = Vec::new();
    let mut similarity: HashMap<String, f64> = HashMap::new();
    let mut by_session: HashMap<String, Vec<(usize, usize, String)>> = HashMap::new();
    for (rowid, distance) in &neighbors {
        let Some((session_id, message_ordinal, part_ordinal, text)) = coords.get(rowid) else {
            continue;
        };
        if 1.0 - distance < SEMANTIC_FLOOR {
            continue;
        }
        if let Some(allowed) = &allowed {
            if !allowed.contains(session_id) {
                continue;
            }
        }
        if !similarity.contains_key(session_id) {
            order.push(session_id.clone());
            similarity.insert(session_id.clone(), (1.0 - distance).clamp(-1.0, 1.0));
        }
        by_session.entry(session_id.clone()).or_default().push((
            *message_ordinal,
            *part_ordinal,
            text.clone(),
        ));
    }
    let ranking: Vec<(String, f64)> = order
        .iter()
        .map(|id| (id.clone(), similarity[id]))
        .collect();
    let sessions: Vec<&String> = match max_sessions {
        Some(k) => order.iter().take(k).collect(),
        None => order.iter().collect(),
    };
    let mut hits = Vec::new();
    for session_id in sessions {
        let score = similarity[session_id];
        for (message_ordinal, part_ordinal, text) in &by_session[session_id] {
            hits.push(SearchHit {
                session_id: session_id.clone(),
                message_ordinal: *message_ordinal,
                part_ordinal: *part_ordinal,
                text: text.clone(),
                score,
            });
        }
    }
    Ok(SemanticMatches { ranking, hits })
}

/// The (message_ordinal, part_ordinal, text) of each neighbor rowid.
type PartCoords = HashMap<i64, (String, usize, usize, String)>;

fn part_coords(conn: &Connection, neighbors: &[(i64, f64)]) -> Result<PartCoords, StoreError> {
    let placeholders = neighbors
        .iter()
        .enumerate()
        .map(|(index, _)| format!("?{}", index + 1))
        .collect::<Vec<_>>()
        .join(", ");
    let values: Vec<Value> = neighbors
        .iter()
        .map(|(rowid, _)| Value::from(*rowid))
        .collect();
    let sql = format!(
        "SELECT v.vec_rowid, v.session_id, v.message_ordinal, v.part_ordinal, p.text \
         FROM message_vec_rows v \
         JOIN parts p ON p.session_id = v.session_id \
           AND p.message_ordinal = v.message_ordinal \
           AND p.part_ordinal = v.part_ordinal \
         WHERE v.vec_rowid IN ({placeholders})"
    );
    let mut coords = HashMap::new();
    conn.prepare_cached(&sql)
        .map_err(sqlite_error("read vector part coordinates"))?
        .query_map(rusqlite::params_from_iter(values), |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)? as usize,
                row.get::<_, i64>(3)? as usize,
                row.get::<_, String>(4)?,
            ))
        })
        .map_err(sqlite_error("read vector part coordinates"))?
        .map(|row| row.map_err(sqlite_error("read vector part coordinates")))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .for_each(|(rowid, session_id, message_ordinal, part_ordinal, text)| {
            coords.insert(rowid, (session_id, message_ordinal, part_ordinal, text));
        });
    Ok(coords)
}

/// The sessions a filter allows, or `None` for an unrestricted search —
/// the filter compiles into SQL over the sessions table exactly as it
/// does for iteration and text search.
fn allowed_sessions(
    conn: &Connection,
    filter_conjuncts: &str,
    filter_values: &[Value],
) -> Result<Option<HashSet<String>>, StoreError> {
    if filter_values.is_empty() {
        return Ok(None);
    }
    let sql = format!("SELECT id FROM sessions WHERE 1 = 1{filter_conjuncts}");
    let ids: HashSet<String> = conn
        .prepare_cached(&sql)
        .map_err(sqlite_error("filter sessions for vector search"))?
        .query_map(rusqlite::params_from_iter(filter_values), |row| row.get(0))
        .map_err(sqlite_error("filter sessions for vector search"))?
        .map(|row| row.map_err(sqlite_error("filter sessions for vector search")))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .collect();
    Ok(Some(ids))
}

fn sqlite_error(context: &'static str) -> impl FnOnce(rusqlite::Error) -> StoreError {
    move |source| StoreError::Sqlite {
        context: context.to_string(),
        source,
    }
}

/// Reciprocal-rank-fusion constant: the text path contributes
/// `1 / (RRF_K + rank)` for a session at rank `rank` (1-based).
pub(crate) const RRF_K: f64 = 60.0;

/// Merge the text and semantic halves into one hybrid ranking: a
/// session's fused score is its reciprocal-rank text contribution plus
/// its best cosine similarity to the query. Pure rank fusion cannot
/// separate a lexical-only hit from a semantic-only one — the similarity
/// magnitude is what lets a strong semantic match outrank a weak lexical
/// one, as the hybrid spec pins. Sessions order by fused score (ties by
/// id); each session's text hits come first, then its semantic-only
/// parts, all carrying the session's fused score.
pub(crate) fn hybrid_merge(
    text_hits: Vec<SearchHit>,
    semantic: Option<SemanticMatches>,
) -> Vec<SearchHit> {
    let mut fused: HashMap<String, f64> = HashMap::new();
    let mut text_rank: HashMap<String, usize> = HashMap::new();
    let mut next_rank = 1usize;
    for hit in &text_hits {
        if !text_rank.contains_key(&hit.session_id) {
            text_rank.insert(hit.session_id.clone(), next_rank);
            *fused.entry(hit.session_id.clone()).or_insert(0.0) += 1.0 / (RRF_K + next_rank as f64);
            next_rank += 1;
        }
    }
    if let Some(semantic) = &semantic {
        for (session_id, similarity) in &semantic.ranking {
            *fused.entry(session_id.clone()).or_insert(0.0) += similarity;
        }
    }
    let mut order: Vec<String> = fused.keys().cloned().collect();
    order.sort_by(|a, b| {
        fused[b]
            .partial_cmp(&fused[a])
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.cmp(b))
    });
    let mut emitted: HashSet<(String, usize, usize)> = HashSet::new();
    let mut hits = Vec::new();
    for session_id in order {
        let score = fused[&session_id];
        for hit in text_hits
            .iter()
            .filter(|hit| hit.session_id == session_id)
            .cloned()
        {
            emitted.insert((
                hit.session_id.clone(),
                hit.message_ordinal,
                hit.part_ordinal,
            ));
            hits.push(SearchHit { score, ..hit });
        }
        if let Some(semantic) = &semantic {
            for hit in semantic
                .hits
                .iter()
                .filter(|hit| hit.session_id == session_id)
                .cloned()
            {
                let key = (
                    hit.session_id.clone(),
                    hit.message_ordinal,
                    hit.part_ordinal,
                );
                if emitted.insert(key) {
                    hits.push(SearchHit { score, ..hit });
                }
            }
        }
    }
    hits
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hit(session: &str, message: usize, part: usize, score: f64) -> SearchHit {
        SearchHit {
            session_id: session.to_string(),
            message_ordinal: message,
            part_ordinal: part,
            text: format!("{session}:{message}:{part}"),
            score,
        }
    }

    #[test]
    fn hybrid_fuses_text_rank_and_similarity() {
        // Text ranks: b(1), a(2). Semantic sims: a 0.9, c 0.5.
        let text = vec![hit("b", 0, 0, 0.0), hit("a", 1, 0, 0.0)];
        let semantic = SemanticMatches {
            ranking: vec![("a".to_string(), 0.9), ("c".to_string(), 0.5)],
            hits: vec![hit("a", 1, 0, 0.9), hit("c", 2, 0, 0.5)],
        };
        let merged = hybrid_merge(text, Some(semantic));
        let sessions: Vec<String> = {
            let mut seen = Vec::new();
            for merged_hit in &merged {
                if !seen.contains(&merged_hit.session_id) {
                    seen.push(merged_hit.session_id.clone());
                }
            }
            seen
        };
        // a: 1/62 + 0.9; b: 1/61; c: 0.5 — a first, then b and c.
        assert_eq!(sessions[0], "a");
        let scores: HashMap<String, f64> = merged
            .iter()
            .map(|merged_hit| (merged_hit.session_id.clone(), merged_hit.score))
            .collect();
        assert!((scores["a"] - (1.0 / 62.0 + 0.9)).abs() < 1e-12);
        assert!((scores["b"] - 1.0 / 61.0).abs() < 1e-12);
        assert!((scores["c"] - 0.5).abs() < 1e-12);
        // A part matched by both paths appears once, with the fused score.
        let a_hits: Vec<&SearchHit> = merged
            .iter()
            .filter(|merged_hit| merged_hit.session_id == "a")
            .collect();
        assert_eq!(a_hits.len(), 1);
        assert_eq!(a_hits[0].message_ordinal, 1);
    }

    #[test]
    fn hybrid_works_without_an_embedder() {
        let text = vec![hit("a", 0, 0, 0.0)];
        let merged = hybrid_merge(text, None);
        assert_eq!(merged.len(), 1);
        assert!((merged[0].score - 1.0 / 61.0).abs() < 1e-12);
    }
}
