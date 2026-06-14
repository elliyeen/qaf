-- Qaf — canonical Quranic structural schema (QAF-1.2)
-- Adds juz, pages, surahs, ayahs with FK relations.
--
-- Design notes:
--   • juz.id, pages.id, surahs.id ARE the canonical numbers — no surrogate key.
--   • ayahs.page_id and ayahs.juz_id are nullable; populated by the importer.
--   • text_uthmani is nullable; populated by the importer.
--   • A CHECK constraint on surahs.revelation_type enforces the controlled vocab.
--   • ON DELETE CASCADE on ayahs → surahs: deleting a surah removes all its ayahs.
--   • ON DELETE SET NULL on ayahs → pages/juz: page/juz metadata loss leaves ayahs intact.

-- ─── Juz ──────────────────────────────────────────────────────────────────────
-- 30 fixed divisions of the Quran.  id IS the juz number (1–30).
CREATE TABLE IF NOT EXISTS juz (
    id      INTEGER PRIMARY KEY,   -- 1–30
    name_ar TEXT    NOT NULL       -- e.g. "الجزء الأول"
);

-- ─── Pages ────────────────────────────────────────────────────────────────────
-- 604 pages of the standard Uthmani muṣḥaf (Madinah King Fahd Complex edition).
-- id IS the page number (1–604).
-- juz_id: the juz that this page belongs to (nullable; set by importer).
CREATE TABLE IF NOT EXISTS pages (
    id     INTEGER PRIMARY KEY,   -- 1–604
    juz_id INTEGER REFERENCES juz(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_pages_juz ON pages (juz_id);

-- ─── Surahs ───────────────────────────────────────────────────────────────────
-- 114 chapters of the Quran.  id IS the surah number (1–114).
CREATE TABLE IF NOT EXISTS surahs (
    id              INTEGER PRIMARY KEY,
    name_ar         TEXT    NOT NULL,   -- e.g. "الفاتحة"
    name_en         TEXT    NOT NULL,   -- e.g. "Al-Fatiha"
    name_en_meaning TEXT    NOT NULL,   -- e.g. "The Opening"
    revelation_type TEXT    NOT NULL    CHECK (revelation_type IN ('makki', 'madani')),
    ayah_count      INTEGER NOT NULL,
    UNIQUE (name_ar),
    UNIQUE (name_en)
);

-- ─── Ayahs ────────────────────────────────────────────────────────────────────
-- Individual verses.  Relations: ayah → surah (required), page, juz (optional).
-- text_uthmani, page_id, juz_id are nullable until populated by the importer.
CREATE TABLE IF NOT EXISTS ayahs (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    surah_id     INTEGER NOT NULL REFERENCES surahs(id) ON DELETE CASCADE,
    ayah_number  INTEGER NOT NULL,
    text_uthmani TEXT,
    page_id      INTEGER REFERENCES pages(id) ON DELETE SET NULL,
    juz_id       INTEGER REFERENCES juz(id)   ON DELETE SET NULL,
    UNIQUE (surah_id, ayah_number)
);

CREATE INDEX IF NOT EXISTS idx_ayahs_surah ON ayahs (surah_id);
CREATE INDEX IF NOT EXISTS idx_ayahs_page  ON ayahs (page_id);
CREATE INDEX IF NOT EXISTS idx_ayahs_juz   ON ayahs (juz_id);
