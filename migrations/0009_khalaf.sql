-- Qaf — Recitation variants and tajweed annotation schema
--
-- Adds four tables that together let the application store and render
-- the text and colour-coded tajweed rules for any qira'ah / riwayah,
-- with Khalaf an Hamzah (رواية خلف عن حمزة) as the primary target.
--
-- Design principles:
--   • recitations   — a named catalogue entry per riwayah (slug-keyed).
--   • recitation_texts — one row per (recitation, surah, ayah); the text
--       field is the full ayah string exactly as it appears in that riwayah.
--   • tajweed_spans — character-level rule annotations; they reference
--       recitation_texts by id so cascade deletes work cleanly.
--   • tajweed_rule_colors — normalised colour map: one (recitation, rule) →
--       color_hex row rather than repeating the colour in every span.
--       The renderer does a single pre-load of this table; spans carry
--       only the rule name.
--
-- Tajweed rule vocabulary (tajweed_spans.rule CHECK):
--
--   Standard rules (apply to Hafs and all riwayat):
--     ghunnah             — nasalization (غنة), 2 counts
--     idgham_ghunnah      — assimilation with ghunnah (ي ن م و)
--     idgham_bila_ghunnah — assimilation without ghunnah (ل ر)
--     idgham_shafawi      — labial assimilation (م before ب)
--     ikhfa               — concealment (إخفاء), ن before 15 letters
--     ikhfa_shafawi       — labial concealment (م before ب)
--     iqlab               — conversion (ن → م before ب)
--     izhar               — clear pronunciation (إظهار حلقي)
--     izhar_shafawi       — labial clear pronunciation (م before غير ب م)
--     madd_tabii          — natural prolongation, 2 counts
--     madd_muttasil       — connected obligatory prolongation, 4–5 counts
--     madd_munfasil       — separated permissible prolongation, 2–5 counts
--     madd_lazim          — necessary prolongation, 6 counts
--     madd_arid           — prolongation before a stop, 2–6 counts
--     madd_lin            — soft-letter prolongation before a stop
--     madd_badal          — substitution prolongation (hamzah replaced by madd)
--     qalqalah            — echo/bounce (ق ط ب ج د)
--
--   Khalaf-specific (and shared with some other riwayat):
--     sakt                — slight breath-less pause (سكت); Khalaf has it in
--                           four places; Hafs has it in four other positions
--     imalah              — vowel tilting / imālah kubrā (إمالة كبرى)
--     tashil              — softening of hamzah (تسهيل)
--     naql                — transfer of hamzah vowel to preceding letter (نقل)
--     ishmam              — lip-pursing on paused damma (إشمام)

-- ─── Recitations ──────────────────────────────────────────────────────────────
-- A catalogue of named Quranic recitation variants (riwayat / qira'at).
-- `name` is a stable ASCII slug used as a foreign key target in importers
-- and API routes (e.g. GET /ayah/1/1/recitation/khalaf).

CREATE TABLE IF NOT EXISTS recitations (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    -- Stable ASCII slug, e.g. 'hafs', 'khalaf', 'warsh', 'qalun'.
    name        TEXT    NOT NULL,
    -- Full name of the transmitter (rawi), e.g. "خلف بن هشام البزار".
    rawi        TEXT    NOT NULL,
    -- Full name of the reciter (qari), e.g. "حمزة بن حبيب الزيات".
    qari        TEXT    NOT NULL,
    -- Optional scholarly notes or source reference.
    description TEXT,
    UNIQUE (name)
);

-- ─── Recitation texts ─────────────────────────────────────────────────────────
-- One ayah text per recitation variant.  The `text` field stores the ayah
-- exactly as it appears in the source data (e.g. Tanzil XML) — including the
-- orthographic differences specific to that riwayah.
--
-- `source` tracks provenance (e.g. 'tanzil.net', 'manual').
-- page_id / juz_id are left for a later structural import.

CREATE TABLE IF NOT EXISTS recitation_texts (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    recitation_id INTEGER NOT NULL REFERENCES recitations(id) ON DELETE CASCADE,
    -- FK to surahs.id (canonical surah number 1–114).
    surah_id      INTEGER NOT NULL REFERENCES surahs(id)      ON DELETE CASCADE,
    ayah_number   INTEGER NOT NULL,
    -- Full ayah text in this recitation's orthography (with tashkeel).
    text          TEXT    NOT NULL,
    -- Provenance label, e.g. 'tanzil.net', 'manual'.
    source        TEXT,
    UNIQUE (recitation_id, surah_id, ayah_number)
);

CREATE INDEX IF NOT EXISTS idx_recitation_texts_surah
    ON recitation_texts (surah_id, ayah_number);

CREATE INDEX IF NOT EXISTS idx_recitation_texts_recitation_surah
    ON recitation_texts (recitation_id, surah_id);

-- ─── Tajweed spans ────────────────────────────────────────────────────────────
-- Character-level tajweed rule annotations on a recitation text.
--
-- `start_index` and `length` are byte-agnostic Unicode character offsets
-- into `recitation_texts.text` (i.e. use Rust's `chars().nth()`, not byte
-- indexing, when slicing).
--
-- `note` is for scholarly annotation, e.g.
--   "Sakt here because the word ends with ساكن before another ساكن"
--
-- Spans for the same ayah may overlap (e.g. a madd letter that also carries
-- ghunnah).  No UNIQUE constraint is placed on (recitation_text_id, start_index)
-- for this reason.

CREATE TABLE IF NOT EXISTS tajweed_spans (
    id                 INTEGER PRIMARY KEY AUTOINCREMENT,
    -- FK to recitation_texts; CASCADE so deleting an ayah text removes its spans.
    recitation_text_id INTEGER NOT NULL REFERENCES recitation_texts(id) ON DELETE CASCADE,
    -- 0-based Unicode character offset into recitation_texts.text.
    start_index        INTEGER NOT NULL,
    -- Number of Unicode characters covered by this rule annotation.
    length             INTEGER NOT NULL CHECK (length > 0),
    -- Tajweed rule name (controlled vocabulary — see header comment).
    rule               TEXT    NOT NULL,
    -- Optional scholarly note on why this rule applies here.
    note               TEXT,
    CHECK (rule IN (
        'ghunnah',
        'idgham_ghunnah',
        'idgham_bila_ghunnah',
        'idgham_shafawi',
        'ikhfa',
        'ikhfa_shafawi',
        'iqlab',
        'izhar',
        'izhar_shafawi',
        'madd_tabii',
        'madd_muttasil',
        'madd_munfasil',
        'madd_lazim',
        'madd_arid',
        'madd_lin',
        'madd_badal',
        'qalqalah',
        'sakt',
        'imalah',
        'tashil',
        'naql',
        'ishmam'
    ))
);

CREATE INDEX IF NOT EXISTS idx_tajweed_spans_text
    ON tajweed_spans (recitation_text_id);

CREATE INDEX IF NOT EXISTS idx_tajweed_spans_rule
    ON tajweed_spans (rule);

-- ─── Tajweed rule colours ─────────────────────────────────────────────────────
-- Normalised default render colour per (recitation, rule).
-- The renderer pre-loads this table once; tajweed_spans rows carry only the
-- rule name — no colour repetition across potentially hundreds of thousands
-- of span rows.
--
-- A recitation may override the default colour for any rule.
-- Rows for rules that have no colour (e.g. izhar, izhar_shafawi) are simply
-- omitted — the renderer falls back to unstyled text for those.
--
-- Default colours below are seeded in seed-structure alongside the recitation
-- catalogue entries.

CREATE TABLE IF NOT EXISTS tajweed_rule_colors (
    recitation_id INTEGER NOT NULL REFERENCES recitations(id) ON DELETE CASCADE,
    -- Must be one of the rule values in tajweed_spans.rule.
    rule          TEXT    NOT NULL,
    -- CSS hex colour, e.g. '#06A94D'.
    color_hex     TEXT    NOT NULL,
    PRIMARY KEY (recitation_id, rule)
);
