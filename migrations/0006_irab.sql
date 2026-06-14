-- Qaf — Per-word إعراب (Irab) at Ajurrumiyyah level
-- One row per word; UNIQUE on word_id enforces a single analysis per word.
--
-- Controlled vocabulary (validated in queries.rs):
--
--   word_type           : ism | fil | harf
--   case_marker         : marfu | mansub | majrur | majzum | mabni
--   case_sign           : damma | dammatan | fatha | fatahtan | kasra | kasratan
--                         | sukun | waw | alif | ya | nun_deletion | fatha_subst
--   grammatical_function: mubtada | khabar | fail | naib_fail | mafuul_bihi
--                         | mudaf | mudaf_ilayh | nat | hal | tamyiz | badal
--                         | atf | tawkid | zarf | mafuul_mutlaq | mafuul_fih
--                         | mafuul_lah | isim_kana | khabar_kana | isim_inna
--                         | khabar_inna | jar | majrur_bijar | munada
--                         | fil | harf (for particles/verbs themselves)

CREATE TABLE IF NOT EXISTS word_irab (
    id                   INTEGER PRIMARY KEY AUTOINCREMENT,
    -- FK to words; CASCADE so deleting a word removes its irab.
    word_id              INTEGER NOT NULL REFERENCES words(id) ON DELETE CASCADE,
    -- النوع: ism (اسم) | fil (فعل) | harf (حرف)
    word_type            TEXT    NOT NULL,
    -- الإعراب: marfu | mansub | majrur | majzum | mabni
    -- NULL for pure particles that have no case (مبني لا محل له من الإعراب).
    case_marker          TEXT,
    -- علامة الإعراب, e.g. damma, fatha, waw, ya, nun_deletion.
    -- NULL when case_marker is NULL or when the sign is contextually clear.
    case_sign            TEXT,
    -- الوظيفة النحوية (syntactic function), e.g. mubtada, fail, mafuul_bihi.
    grammatical_function TEXT,
    -- Optional sub-classification, e.g. fil_madhi | fil_mudari | fil_amr
    -- for verbs; ism_mawsul | damir_munfasil | damir_muttasil for nouns.
    subtype              TEXT,
    -- Full Arabic irab phrase, e.g.:
    -- "مبتدأ مرفوع وعلامة رفعه الضمة الظاهرة على آخره"
    -- "فعل ماضٍ مبني على الفتح"
    -- "حرف جر مبني على السكون لا محل له من الإعراب"
    note                 TEXT,
    -- Provenance: 'manual' (default) or 'quranic-corpus'.
    source               TEXT    NOT NULL DEFAULT 'manual',
    UNIQUE (word_id)
);

CREATE INDEX IF NOT EXISTS idx_word_irab_word_id
    ON word_irab (word_id);
