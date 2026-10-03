# The local store

How BotSpy keeps every agent's history in one queryable SQLite file.
The behavior is pinned by the executable specs under
`features/02_querying/` (embedded in the rustdoc's querying chapter);
this page is the operations-level summary.

## Store path and opening

A store is a single SQLite file. `Store::open(path)` opens (or creates,
parent directories included) the store at a caller-given path; the
default path derives from the environment the same way the CLI's does:
`$BOTSPY_HOME/.botspy/store.db`, falling back to `$HOME`. A file that
is not a store (or is corrupt) is a clean typed error, never a panic.

The store records its schema version in its own `meta` table and
migrates itself on open. Opening never blocks readers: WAL mode keeps a
reader querying on a second connection while an ingest transaction
commits.

## Schema (DDL v1 and v2)

v1 lays down the normalized shape the chapter-1 schema promises, with
projected filter columns plus lossless serde JSON columns:

- `sessions` — identity, agent, project, timestamps, message count,
  content hash, and the full session JSON.
- `messages` — per-session ordinal, role, timestamp, turn id, message
  JSON; cascade with their session.
- `parts` — message/part ordinals, kind, text; cascade with their
  message. This is the table search and iteration project over.
- `message_fts` — an FTS5 index over part text for ranked text search.
- `meta` — key/value settings (schema version, embedding model id).

v2 adds `message_vec_rows` — the mapping from sqlite-vec rowids back to
(session, message, part) coordinates. The vec0 virtual table itself is
created lazily on the first embed, because its dimension comes from the
configured embedder.

## Ingest and the refresh model

`ingest` mines every registered adapter into the store in one
transaction: discovery first, then each session opened from its adapter
and written as normalized rows. First report wins when two adapters
name the same session id.

`refresh` is the incremental pass: it compares each adapter's session
summaries (id, last activity, message count, content hash) against the
store and opens only sessions whose content actually moved; sessions
the source no longer reports are pruned. Unchanged sessions are never
re-opened from their adapters, and a second pass changes nothing.

## The embedder seam

Vectors come from one seam: the `Embedder` trait (model id, dimension,
`embed(texts) -> vectors`). The shipped implementation is
`MiniLMEmbedder` — quantized all-MiniLM-L6-v2 through tract (ONNX) on
CPU, wordpiece tokenization, mean pooling over the attention mask,
L2-normalized 384-dimension output.

The weights and tokenizer are not committed and are never downloaded at
runtime: the one-time setup step (`cargo run --example setup_models`)
fetches each asset exactly once, verifies it against a pinned SHA-256
digest, and caches it under the models dir (`BOTSPY_MODELS` or
`$HOME/.botspy/models`). The embedder loads from that cache or fails
with a clean typed error.

## The model-id policy

The first embed records the embedder's model id in `meta`
(`embedding_model`). Every later ingest checks it before doing any
session work — a store never mixes embedding models: a different model
id is refused, even when the ingest would change nothing. A store
opened without an embedder records no model, answers text search
normally, and answers semantic search with a clean empty result.

Semantic matches must clear a 0.3 cosine relevance floor: for the
shipped embeddings, unrelated texts land near zero while topically
related pairs start around 0.4, so below 0.3 a similarity is noise, not
a weak ranking. Hybrid search fuses the lexical and semantic halves
(reciprocal-rank fusion for text plus the best cosine similarity), so a
session that both word-matches and embeds near the query ranks first.