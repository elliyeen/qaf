-- Add lemma_bare for diacritic-insensitive lemma search.
-- Populated at insert time by quran-db after stripping Arabic harakat.
-- Not exposed on the Word struct — used only by search_words(field="lemma").

ALTER TABLE words ADD COLUMN lemma_bare TEXT NOT NULL DEFAULT '';

CREATE INDEX IF NOT EXISTS idx_words_lemma_bare ON words (lemma_bare);
