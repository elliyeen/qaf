-- Qaf — audio file and ayah-level timestamp schema
--
-- Two tables:
--   audio_files    — one row per (recitation, surah); points to the local MP3.
--   audio_segments — one row per ayah; stores start/end milliseconds within
--                    the parent MP3 so the UI can seek to any ayah instantly.
--
-- Both tables FK into the existing `recitations` and `surahs` tables so
-- there is no duplication of reciter or chapter metadata.

-- ─── Audio files ──────────────────────────────────────────────────────────────
-- `file_path` is relative to the project root, e.g. data/audio/khalaf/001.mp3
-- `source_url` preserves the original download URL for auditing / re-download.
-- `duration_ms` is filled in by the ingest script after probing the file.

CREATE TABLE IF NOT EXISTS audio_files (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    recitation_id INTEGER NOT NULL REFERENCES recitations(id) ON DELETE CASCADE,
    surah_id      INTEGER NOT NULL REFERENCES surahs(id)      ON DELETE CASCADE,
    file_path     TEXT    NOT NULL,
    source_url    TEXT,
    duration_ms   INTEGER,
    UNIQUE (recitation_id, surah_id)
);

CREATE INDEX IF NOT EXISTS idx_audio_files_surah ON audio_files (surah_id);

-- ─── Audio segments ───────────────────────────────────────────────────────────
-- Ayah-level timestamp boundaries within the parent MP3.
-- `start_ms` and `end_ms` are millisecond offsets from the start of the file.
-- For playback: seek to start_ms, play until end_ms, advance to next segment.

CREATE TABLE IF NOT EXISTS audio_segments (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    audio_file_id INTEGER NOT NULL REFERENCES audio_files(id) ON DELETE CASCADE,
    ayah_number   INTEGER NOT NULL,
    start_ms      INTEGER NOT NULL,
    end_ms        INTEGER NOT NULL,
    UNIQUE (audio_file_id, ayah_number)
);

CREATE INDEX IF NOT EXISTS idx_audio_segments_file ON audio_segments (audio_file_id);
