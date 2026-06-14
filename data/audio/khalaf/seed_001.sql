-- Seed: Surah Al-Fatiha audio — Abdulrasheed Soufi, Rewayat Khalaf An Hamzah
--
-- Source: https://server16.mp3quran.net/download/soufi/Rewayat-Khalaf-A-n-Hamzah/001.mp3
-- Local:  data/audio/khalaf/001.mp3
-- Duration: 51,461 ms
--
-- Silence detection: ffmpeg silencedetect noise=-25dB:d=0.15
-- The recording opens with the Ta'awwudh (أعوذ بالله من الشيطان الرجيم)
-- followed by the 7 ayahs. Segment durations are consistent with word counts:
--   Ayah 3 (2 words) → shortest segment (3.4s) ✓
--   Ayah 7 (9 words) → longest segment (17.8s) ✓
--
-- recitation_id = 2 (khalaf, from the recitations table)
-- surah_id      = 1 (Al-Fatiha)

INSERT OR IGNORE INTO audio_files
    (recitation_id, surah_id, file_path, source_url, duration_ms)
VALUES
    (2, 1,
     'data/audio/khalaf/001.mp3',
     'https://server16.mp3quran.net/download/soufi/Rewayat-Khalaf-A-n-Hamzah/001.mp3',
     51461);

-- Capture the rowid of the row we just inserted (or already had).
-- SQLite allows a single-statement approach via last_insert_rowid() after
-- INSERT OR IGNORE; if the row already existed, last_insert_rowid() is 0,
-- so we use a sub-select fallback.

WITH af AS (
    SELECT id FROM audio_files WHERE recitation_id = 2 AND surah_id = 1
),
segs(ayah_number, start_ms, end_ms) AS (
    -- Timestamps are millisecond offsets within 001.mp3.
    -- Leading ta'awwudh occupies 170ms-4568ms and is not mapped to an ayah.
    SELECT 1,  5276,  9173  UNION ALL  -- 3.9s
    SELECT 2, 10015, 14226  UNION ALL  -- 4.2s
    SELECT 3, 14758, 18153  UNION ALL  -- 3.4s (shortest: 2 words)
    SELECT 4, 18750, 22234  UNION ALL  -- 3.5s
    SELECT 5, 22806, 27236  UNION ALL  -- 4.4s
    SELECT 6, 28209, 32066  UNION ALL  -- 3.9s
    SELECT 7, 32929, 50723             -- 17.8s (longest: 9 words)
)
INSERT OR IGNORE INTO audio_segments (audio_file_id, ayah_number, start_ms, end_ms)
SELECT af.id, segs.ayah_number, segs.start_ms, segs.end_ms
FROM af, segs;
