-- Qaf — tadabbur schema
-- Contemplation layer: reflections on ayahs, thematic tags, and cross-references.
-- No user auth required; author/source are free-text attribution fields.

-- ─── Reflections ────────────────────────────────────────────────────────────
-- A textual reflection on a single ayah.
-- body is plain text or Markdown.
-- lang is an ISO 639-1 code ('ar', 'en', 'ur', …).
CREATE TABLE IF NOT EXISTS reflections (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    surah      INTEGER NOT NULL,
    ayah       INTEGER NOT NULL,
    body       TEXT    NOT NULL,
    author     TEXT,                           -- e.g. "Ibn Kathīr", "al-Ṭabarī"
    source     TEXT,                           -- book title, hadith ref, etc.
    lang       TEXT    NOT NULL DEFAULT 'en',
    created_at TEXT    NOT NULL DEFAULT (datetime('now'))
    -- Note: no FK to words(surah,ayah) — that pair is not a unique key in words.
    -- Ayah coordinates are validated at the application layer.
);

CREATE INDEX IF NOT EXISTS idx_reflections_ayah
    ON reflections (surah, ayah);

CREATE INDEX IF NOT EXISTS idx_reflections_author
    ON reflections (author);

-- ─── Themes ─────────────────────────────────────────────────────────────────
-- A named subject-matter category (e.g. "tawḥīd", "mercy", "prayer").
CREATE TABLE IF NOT EXISTS themes (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    name_ar     TEXT NOT NULL,                 -- Arabic label, e.g. "التوحيد"
    name_en     TEXT NOT NULL,                 -- English label, e.g. "Divine Oneness"
    description TEXT,
    UNIQUE (name_ar),
    UNIQUE (name_en)
);

-- ─── Ayah–Theme mapping ──────────────────────────────────────────────────────
-- Many-to-many: an ayah can carry multiple themes; a theme spans many ayahs.
CREATE TABLE IF NOT EXISTS ayah_themes (
    surah    INTEGER NOT NULL,
    ayah     INTEGER NOT NULL,
    theme_id INTEGER NOT NULL REFERENCES themes (id) ON DELETE CASCADE,
    note     TEXT,                             -- why this ayah exemplifies the theme
    PRIMARY KEY (surah, ayah, theme_id)
);

CREATE INDEX IF NOT EXISTS idx_ayah_themes_theme
    ON ayah_themes (theme_id);

-- ─── Cross-references ────────────────────────────────────────────────────────
-- A directed semantic link between two ayahs.
-- relation is one of: elaborates | contrasts | repeats | explains | fulfills
CREATE TABLE IF NOT EXISTS cross_references (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    from_surah INTEGER NOT NULL,
    from_ayah  INTEGER NOT NULL,
    to_surah   INTEGER NOT NULL,
    to_ayah    INTEGER NOT NULL,
    relation   TEXT    NOT NULL,               -- controlled vocabulary above
    note       TEXT,                           -- scholarly note on the connection
    UNIQUE (from_surah, from_ayah, to_surah, to_ayah, relation)
);

CREATE INDEX IF NOT EXISTS idx_xref_from
    ON cross_references (from_surah, from_ayah);

CREATE INDEX IF NOT EXISTS idx_xref_to
    ON cross_references (to_surah, to_ayah);
