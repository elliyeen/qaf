-- Qaf — initial schema
-- Surah Al-Fatiha (1:1–7) is the canonical test fixture.

CREATE TABLE IF NOT EXISTS words (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    surah           INTEGER NOT NULL,
    ayah            INTEGER NOT NULL,
    position        INTEGER NOT NULL,
    arabic          TEXT    NOT NULL,
    transliteration TEXT    NOT NULL,
    root            TEXT,                   -- NULL for particles with no trilateral root
    lemma           TEXT    NOT NULL,
    UNIQUE (surah, ayah, position)
);

CREATE INDEX IF NOT EXISTS idx_words_root   ON words (root);
CREATE INDEX IF NOT EXISTS idx_words_surah  ON words (surah, ayah);

CREATE TABLE IF NOT EXISTS morphology (
    word_id  INTEGER PRIMARY KEY REFERENCES words(id) ON DELETE CASCADE,
    pos      TEXT NOT NULL,       -- N / V / Adj / Prep / Pron / Conj / Det / Intj
    features TEXT NOT NULL,       -- JSON object  {"case":"nominative","number":"singular",...}
    source   TEXT NOT NULL        -- e.g. "quranic-corpus"
);

CREATE TABLE IF NOT EXISTS ontology (
    root            TEXT PRIMARY KEY,
    semantic_domain TEXT    NOT NULL,
    derivatives     TEXT    NOT NULL,   -- JSON array of strings
    scholar_notes   TEXT                -- nullable
);
