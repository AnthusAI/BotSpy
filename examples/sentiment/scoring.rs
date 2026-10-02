//! Local sentiment scoring for the sentiment-analysis example consumer.
//! The default backend is a VADER-style valence lexicon, entirely in-process:
//! message text never leaves the machine, and the module imports nothing
//! beyond `botspy` and the standard library.
//!
//! The optional GoEmotions transformer path is deliberately NOT wired here:
//! it is planned behind an optional `botspy[metrics]` extra (see the example
//! use cases epic) and would ship as an opt-in backend, never in the core
//! dependency graph. This module is the seam for it.

use botspy::{KnownPart, Part, Session};

/// VADER-style scoring thresholds (compound in [-1, 1]).
const POSITIVE_THRESHOLD: f64 = 0.05;
const NEGATIVE_THRESHOLD: f64 = -0.05;
/// VADER's normalization constant.
const COMPOUND_ALPHA: f64 = 15.0;
/// Caps emphasis: an all-caps valence word scores this much stronger.
const CAPS_INCR: f64 = 0.733;
/// Negation flips a valence word by this factor within a short window.
const NEGATION_SCALAR: f64 = -0.74;
/// Intensifier boost per preceding adverb ("very", "extremely", ...).
const INTENSIFIER_INCR: f64 = 0.293;

/// A trimmed VADER-style valence lexicon: the clearly polar words a coding
/// session actually uses. Words without sentiment ("fine", "works") are
/// deliberately absent so they stay neutral.
const LEXICON: &[(&str, f64)] = &[
    ("love", 3.2),
    ("great", 3.1),
    ("awesome", 3.4),
    ("perfect", 3.0),
    ("excellent", 3.1),
    ("happy", 2.7),
    ("glad", 2.4),
    ("nice", 1.8),
    ("thanks", 2.4),
    ("thank", 2.2),
    ("fixed", 1.6),
    ("improved", 1.5),
    ("helpful", 1.7),
    ("clean", 1.2),
    ("terrible", -2.1),
    ("awful", -2.5),
    ("hate", -2.7),
    ("broken", -1.6),
    ("broke", -1.5),
    ("worst", -3.1),
    ("annoying", -1.9),
    ("fails", -1.5),
    ("failing", -1.5),
    ("failed", -1.5),
    ("failure", -1.7),
    ("crash", -1.4),
    ("crashes", -1.4),
    ("crashed", -1.4),
    ("regression", -1.5),
    ("regressions", -1.5),
    ("stuck", -1.2),
    ("wrong", -1.3),
    ("bad", -1.5),
];

const NEGATIONS: &[&str] = &[
    "not", "no", "never", "cannot", "cant", "can't", "won't", "wont", "dont", "don't", "isnt",
    "isn't", "wasnt", "wasn't", "arent", "aren't",
];

const INTENSIFIERS: &[&str] = &["very", "really", "extremely", "highly", "absolutely"];

/// How one message text lands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sentiment {
    Positive,
    Negative,
    Neutral,
}

/// Per-session aggregate counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SentimentReport {
    /// Messages that scored positive.
    pub positive: u64,
    /// Messages that scored negative.
    pub negative: u64,
    /// Messages that scored neutral.
    pub neutral: u64,
    /// Sessions that went into the report.
    pub sessions: u64,
}

/// The spoken text of every message part in a session. Thinking and tool
/// parts are not conversation, so they never score.
pub fn message_texts(session: &Session) -> Vec<String> {
    session
        .messages
        .iter()
        .flat_map(|message| message.parts.iter())
        .filter_map(|part| match part {
            Part::Known(KnownPart::Text { text, .. }) => Some(text.clone()),
            _ => None,
        })
        .collect()
}

/// VADER-style compound score for one message text, in [-1, 1]. Pure and
/// local: lexicon lookup and arithmetic only.
pub fn compound_score(text: &str) -> f64 {
    let mut valences = Vec::new();
    for (index, word) in tokenize(text).iter().enumerate() {
        let Some(mut valence) = lexicon_valence(word) else {
            continue;
        };
        if word.chars().all(|c| c.is_ascii_uppercase()) && word.len() > 1 {
            valence += valence.signum() * CAPS_INCR;
        }
        let window: Vec<String> = tokenize(text)
            .iter()
            .take(index)
            .rev()
            .take(3)
            .cloned()
            .collect();
        for previous in &window {
            if INTENSIFIERS.contains(&previous.as_str()) {
                valence += valence.signum() * INTENSIFIER_INCR;
            }
            if NEGATIONS.contains(&previous.as_str()) {
                valence *= NEGATION_SCALAR;
            }
        }
        valences.push(valence);
    }
    let sum: f64 = valences.iter().sum();
    if sum == 0.0 {
        return 0.0;
    }
    sum / (sum * sum + COMPOUND_ALPHA).sqrt()
}

/// Classify one message text: positive, negative, or neutral.
pub fn classify(text: &str) -> Sentiment {
    let compound = compound_score(text);
    if compound >= POSITIVE_THRESHOLD {
        Sentiment::Positive
    } else if compound <= NEGATIVE_THRESHOLD {
        Sentiment::Negative
    } else {
        Sentiment::Neutral
    }
}

/// Score one session by its message texts.
pub fn score_session(session: &Session) -> SentimentReport {
    let mut report = SentimentReport::default();
    for text in message_texts(session) {
        match classify(&text) {
            Sentiment::Positive => report.positive += 1,
            Sentiment::Negative => report.negative += 1,
            Sentiment::Neutral => report.neutral += 1,
        }
    }
    report.sessions = 1;
    report
}

/// Aggregate a whole unified history into one report. Used by the example
/// binary; the BDD include only exercises the per-session path.
#[allow(dead_code)]
pub fn score_history(sessions: &[Session]) -> SentimentReport {
    let mut report = SentimentReport {
        sessions: sessions.len() as u64,
        ..SentimentReport::default()
    };
    for session in sessions {
        let session_report = score_session(session);
        report.positive += session_report.positive;
        report.negative += session_report.negative;
        report.neutral += session_report.neutral;
    }
    report
}

fn tokenize(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_ascii_alphanumeric() && c != '\'')
        .filter(|word| !word.is_empty())
        .map(|word| word.to_lowercase())
        .collect()
}

fn lexicon_valence(word: &str) -> Option<f64> {
    LEXICON
        .iter()
        .find(|(entry, _)| *entry == word)
        .map(|(_, valence)| *valence)
}
