-- Qaf — Hadith cross-references
-- Links a Quran ayah to a hadith that explains, corroborates, or contextualises it.
-- Every row must satisfy the CLAUDE.md citation rules:
--   collection + hadith_number + grade (+ optional grader) must all be present.

CREATE TABLE IF NOT EXISTS hadith_cross_references (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    surah          INTEGER NOT NULL,
    ayah           INTEGER NOT NULL,
    -- Full collection name, e.g. "Ṣaḥīḥ al-Bukhārī", "Ṣaḥīḥ Muslim".
    collection     TEXT    NOT NULL,
    -- Number within the collection; TEXT because some editions use suffixes like "3432a".
    hadith_number  TEXT    NOT NULL,
    -- Authenticity grade: ṣaḥīḥ | ḥasan | ḍa'īf | mawḍū' (required).
    grade          TEXT    NOT NULL,
    -- Scholar who assigned the grade, e.g. "al-Albānī", "Ibn Ḥajar".
    grader         TEXT,
    -- Human-readable citation string supplied by the inserter,
    -- e.g. "Ṣaḥīḥ al-Bukhārī 3" or "Sunan Abī Dāwūd 4840 (ṣaḥīḥ)".
    -- Named `reference` (not `ref`) so callers never need r#ref.
    reference      TEXT    NOT NULL,
    -- How the hadith relates to the ayah: explains | corroborates | restricts | abrogates | contextualises
    relation       TEXT,
    -- Optional scholarly note on the connection.
    note           TEXT,
    UNIQUE (surah, ayah, collection, hadith_number)
);

CREATE INDEX IF NOT EXISTS idx_hadith_xref_ayah
    ON hadith_cross_references (surah, ayah);

CREATE INDEX IF NOT EXISTS idx_hadith_xref_collection
    ON hadith_cross_references (collection);
