-- Qaf — Word token system (QAF-2.1)
--
-- WordToken is a logical concept that maps 1:1 to the existing `words` table.
-- The canonical `tok:SSS:AAA:PPP` reference string is computed at query time
-- via printf — no stored column is needed.
--
-- WordSegment represents a sub-word morphological unit (prefix, stem, suffix…).
-- Each segment belongs to exactly one token (word) and has its own POS + features.
-- The canonical `seg:SSS:AAA:PPP:SS` reference string is stored explicitly so
-- callers can perform direct lookups without a JOIN.
--
-- ID formats:
--   token_ref   = "tok:060:012:023"        (surah, ayah, position — zero-padded 3 digits)
--   segment_ref = "seg:060:012:023:02"     (surah, ayah, position, segment_index — 2 digits)

CREATE TABLE IF NOT EXISTS word_segments (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    -- FK to the parent token; CASCADE so deleting a word removes all its segments.
    word_id       INTEGER NOT NULL REFERENCES words(id) ON DELETE CASCADE,
    -- 1-based ordinal within the token (prefix=1, stem=2, suffix=3…).
    segment_index INTEGER NOT NULL,
    -- Arabic text of this segment.
    arabic        TEXT    NOT NULL,
    -- Part-of-speech tag, e.g. "PREP", "DET", "N", "V", "PN".
    pos           TEXT    NOT NULL,
    -- JSON object of morphological features,
    -- e.g. {"case":"gen","number":"sg","gender":"m"}.
    features      TEXT    NOT NULL DEFAULT '{}',
    -- Canonical reference, e.g. "seg:001:001:001:01".
    -- Stored explicitly for fast lookup without a JOIN.
    segment_ref   TEXT    NOT NULL,
    UNIQUE (word_id, segment_index),
    UNIQUE (segment_ref)
);

CREATE INDEX IF NOT EXISTS idx_word_segments_word ON word_segments (word_id);
