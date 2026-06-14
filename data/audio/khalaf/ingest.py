#!/usr/bin/env python3
"""
ingest.py — Download all 114 Soufi / Khalaf An Hamzah surahs and extract
ayah-level timestamps via ffmpeg silence detection into qaf.db.

Usage (from project root):
    python3 data/audio/khalaf/ingest.py

Silence detection strategy:
  - Primary threshold: -25dB, 0.15s minimum silence
  - Fallback thresholds tried in order if segment count doesn't match
  - Segment → ayah mapping:
      diff == 0  →  exact match (ta'awwudh likely merged with bismillah)
      diff == 1  →  strip first segment (ta'awwudh prefix)
      diff == 2  →  strip first and last (ta'awwudh prefix + ameen/dua suffix)
  - Anything else is written to review.log for manual inspection
"""

import json
import sqlite3
import subprocess
import sys
import time
from pathlib import Path

# ── Config ────────────────────────────────────────────────────────────────────
PROJECT_ROOT  = Path(__file__).resolve().parents[3]
AUDIO_DIR     = PROJECT_ROOT / "data" / "audio" / "khalaf"
DB_PATH       = PROJECT_ROOT / "qaf.db"
BASE_URL      = "https://server16.mp3quran.net/download/soufi/Rewayat-Khalaf-A-n-Hamzah"
RECITATION_ID = 2       # khalaf row in recitations table
DOWNLOAD_DELAY = 1.5    # seconds between downloads

# Thresholds tried in order: (noise_dB, min_silence_seconds)
THRESHOLDS = [
    ("-25dB", "0.15"),
    ("-30dB", "0.20"),
    ("-20dB", "0.20"),
    ("-35dB", "0.25"),
    ("-25dB", "0.30"),
]


# ── ffmpeg helpers ────────────────────────────────────────────────────────────

def probe_duration_ms(path: Path) -> int:
    r = subprocess.run(
        ["ffprobe", "-v", "quiet", "-print_format", "json", "-show_format", str(path)],
        capture_output=True, text=True, check=True,
    )
    return int(float(json.loads(r.stdout)["format"]["duration"]) * 1000)


def detect_segments(path: Path, duration_ms: int, noise: str, min_sil: str) -> list[tuple[int, int]]:
    """Return (start_ms, end_ms) content segments for the given silence params."""
    r = subprocess.run(
        ["ffmpeg", "-i", str(path),
         "-af", f"silencedetect=noise={noise}:d={min_sil}",
         "-f", "null", "-"],
        capture_output=True, text=True,
    )
    silences: list[tuple[int, int]] = []
    pending: int | None = None
    for line in r.stderr.splitlines():
        if "silence_start:" in line:
            pending = int(float(line.split("silence_start:")[1].strip()) * 1000)
        elif "silence_end:" in line and pending is not None:
            t = int(float(line.split("silence_end:")[1].split("|")[0].strip()) * 1000)
            silences.append((pending, t))
            pending = None
    if pending is not None:
        silences.append((pending, duration_ms))

    segments: list[tuple[int, int]] = []
    cursor = 0
    for s_start, s_end in silences:
        if s_start - cursor > 100:          # >100 ms of actual audio
            segments.append((cursor, s_start))
        cursor = s_end
    if duration_ms - cursor > 100:
        segments.append((cursor, duration_ms))
    return segments


# ── Segment → ayah mapping ────────────────────────────────────────────────────

def map_to_ayahs(
    segments: list[tuple[int, int]], ayah_count: int
) -> tuple[list[tuple[int, int, int]], str]:
    """
    Returns ([(ayah_num, start_ms, end_ms), ...], status).
    status: 'exact' | '+1' | '+2' | 'MISMATCH(n/a)'
    """
    diff = len(segments) - ayah_count
    if diff == 0:
        return [(i + 1, s, e) for i, (s, e) in enumerate(segments)], "exact"
    if diff == 1:
        return [(i + 1, s, e) for i, (s, e) in enumerate(segments[1:])], "+1"
    if diff == 2:
        return [(i + 1, s, e) for i, (s, e) in enumerate(segments[1:-1])], "+2"
    return [], f"MISMATCH({len(segments)}/{ayah_count})"


def best_mapping(
    path: Path, duration_ms: int, ayah_count: int
) -> tuple[list[tuple[int, int, int]], str, str, str]:
    """
    Try each threshold in order; return the first one that yields a clean mapping.
    Returns (ayah_list, status, noise, min_sil).
    """
    results = []
    for noise, min_sil in THRESHOLDS:
        segs = detect_segments(path, duration_ms, noise, min_sil)
        mapped, status = map_to_ayahs(segs, ayah_count)
        if mapped:
            return mapped, status, noise, min_sil
        results.append((len(segs), status))

    # No threshold worked — return the closest attempt for the log
    segs = detect_segments(path, duration_ms, *THRESHOLDS[0])
    _, status = map_to_ayahs(segs, ayah_count)
    return [], status, THRESHOLDS[0][0], THRESHOLDS[0][1]


# ── Download ──────────────────────────────────────────────────────────────────

def download(surah_id: int, path: Path) -> bool:
    url = f"{BASE_URL}/{surah_id:03d}.mp3"
    r = subprocess.run(["curl", "-fsSL", "--retry", "3", "-o", str(path), url],
                       capture_output=True)
    return r.returncode == 0 and path.stat().st_size > 1000


# ── Main ──────────────────────────────────────────────────────────────────────

def main() -> None:
    AUDIO_DIR.mkdir(parents=True, exist_ok=True)
    review: list[str] = []

    with sqlite3.connect(str(DB_PATH)) as conn:
        surah_ayah_counts: dict[int, int] = {
            row[0]: row[1]
            for row in conn.execute("SELECT id, ayah_count FROM surahs ORDER BY id")
        }
        if not surah_ayah_counts:
            sys.exit("ERROR: surahs table is empty — run structure import first.")

        total = len(surah_ayah_counts)
        print(f"Processing {total} surahs → {AUDIO_DIR}\n")

        for surah_id in range(1, 115):
            ayah_count = surah_ayah_counts.get(surah_id)
            if ayah_count is None:
                print(f"[{surah_id:3}/114] SKIP — not in surahs table")
                continue

            mp3 = AUDIO_DIR / f"{surah_id:03d}.mp3"
            label = f"[{surah_id:3}/114]"

            # ── Download ──────────────────────────────────────────────────────
            if mp3.exists() and mp3.stat().st_size > 1000:
                print(f"{label} already present", end="  ")
            else:
                print(f"{label} downloading ...", end="  ", flush=True)
                if not download(surah_id, mp3):
                    print("DOWNLOAD FAILED")
                    review.append(f"surah {surah_id:3}: download failed")
                    continue
                print(f"OK ({mp3.stat().st_size // 1024} KB)", end="  ")
                time.sleep(DOWNLOAD_DELAY)

            # ── Probe duration ────────────────────────────────────────────────
            try:
                duration_ms = probe_duration_ms(mp3)
            except Exception as exc:
                print(f"FFPROBE ERROR: {exc}")
                review.append(f"surah {surah_id:3}: ffprobe failed")
                continue

            # ── Silence detection + mapping ───────────────────────────────────
            mapped, status, noise, min_sil = best_mapping(mp3, duration_ms, ayah_count)
            dur_s = duration_ms / 1000

            if not mapped:
                print(f"{dur_s:7.1f}s  {status}")
                review.append(f"surah {surah_id:3}: {status}  ({dur_s:.0f}s)  "
                               f"tried all thresholds")
                continue

            print(f"{dur_s:7.1f}s  {ayah_count} ayahs  [{status}]  "
                  f"noise={noise} d={min_sil}")

            # ── DB insert ─────────────────────────────────────────────────────
            conn.execute(
                "INSERT OR IGNORE INTO audio_files "
                "(recitation_id, surah_id, file_path, source_url, duration_ms) "
                "VALUES (?, ?, ?, ?, ?)",
                (RECITATION_ID, surah_id,
                 f"data/audio/khalaf/{surah_id:03d}.mp3",
                 f"{BASE_URL}/{surah_id:03d}.mp3",
                 duration_ms),
            )
            file_id = conn.execute(
                "SELECT id FROM audio_files WHERE recitation_id=? AND surah_id=?",
                (RECITATION_ID, surah_id),
            ).fetchone()[0]

            conn.executemany(
                "INSERT OR IGNORE INTO audio_segments "
                "(audio_file_id, ayah_number, start_ms, end_ms) VALUES (?,?,?,?)",
                [(file_id, ayah_num, s, e) for ayah_num, s, e in mapped],
            )
            conn.commit()

    # ── Summary ───────────────────────────────────────────────────────────────
    print("\n" + "─" * 60)
    if review:
        review_path = AUDIO_DIR / "review.log"
        review_path.write_text("\n".join(review) + "\n")
        print(f"{len(review)} surah(s) need review → {review_path}")
        for line in review:
            print(f"  {line}")
    else:
        print("All 114 surahs processed cleanly.")


if __name__ == "__main__":
    main()
