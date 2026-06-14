-- Qaf — ayah-level translations
-- One row per (surah, ayah, translator, lang) triple.
-- Multiple translators and languages for the same ayah are allowed.

CREATE TABLE IF NOT EXISTS translations (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    surah       INTEGER NOT NULL,
    ayah        INTEGER NOT NULL,
    text        TEXT    NOT NULL,
    translator  TEXT,                           -- e.g. "Sahih International"
    lang        TEXT    NOT NULL DEFAULT 'en',  -- ISO 639-1
    source      TEXT,                           -- book title or URL
    created_at  TEXT    NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_translations_ayah
    ON translations (surah, ayah);

CREATE INDEX IF NOT EXISTS idx_translations_lang
    ON translations (surah, ayah, lang);
