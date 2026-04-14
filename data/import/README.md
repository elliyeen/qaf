# Qaf Import — Data Sources

## Required: Quranic Arabic Corpus morphology file

The importer reads `quran-morphology.txt` from the
[mustafa0x/quran-morphology](https://github.com/mustafa0x/quran-morphology) mirror.

This is the Quranic Arabic Corpus v0.4 by Kais Dukes (University of Leeds, 2010).

### Download

```bash
# Option A — git clone (whole repo, ~2 MB)
git clone https://github.com/mustafa0x/quran-morphology.git data/import/quranic-corpus

# Option B — single file download
curl -Lo data/import/quran-morphology.txt \
  https://raw.githubusercontent.com/mustafa0x/quran-morphology/master/quran-morphology.txt
```

### File format

Tab-separated, ~128 k lines:

```
LOCATION            ARABIC_SEGMENT    POS    FEATURES
1:1:1:1             بِ                P      P|PREF|LEM:ب
1:1:1:2             سْمِ              N      ROOT:سمو|LEM:اسْم|M|GEN
1:1:2:1             ٱللَّهِ           N      PN|ROOT:أله|LEM:اللَّه|GEN
1:1:3:1             ٱل               DET    PREF|DET
1:1:3:2             رَّحْمَٰنِ        N      ROOT:رحم|LEM:رَحْمٰن|MS|GEN|ADJ
```

- **LOCATION** — `surah:ayah:word:segment` (parenthesised form also accepted)
- **ARABIC_SEGMENT** — Arabic Unicode text for this morpheme (with diacritics)
- **POS** — part-of-speech code: N V ADJ P CONJ DET PRON PN …
- **FEATURES** — pipe-separated: `ROOT:arabic` | `LEM:arabic` | case | gender | number | …

One Quranic *word* can span multiple segments (prefix + stem + suffix).
The importer concatenates all segment Arabic text for the same word coordinate
to reconstruct the full mushaf word.

### Coverage

| Stat | Value |
|------|-------|
| Lines | ~128,276 |
| Unique word positions | ~77,430 |
| Surahs | 114 |
| Ayahs | 6,236 |

---

## Running the import

```bash
# 1. Download the file (see above)

# 2. Apply migrations (creates qaf.db if absent)
sqlx migrate run --database-url sqlite:qaf.db

# 3. Import (~10–30 s on a laptop)
DATABASE_URL=sqlite:qaf.db cargo run -p quran-import -- \
  --qac data/import/quran-morphology.txt

# Re-run safely — all inserts are INSERT OR IGNORE (idempotent).
# Add --reset to clear and re-import from scratch.
```

### Verify

```bash
sqlite3 qaf.db "SELECT COUNT(*) FROM words;"         # ~77430
sqlite3 qaf.db "SELECT COUNT(*) FROM morphology;"    # ~77430
sqlite3 qaf.db "SELECT COUNT(*) FROM ontology;"      # ~2000 distinct roots
sqlite3 qaf.db "SELECT * FROM words WHERE surah=1 AND ayah=1 ORDER BY position;"
```

---

## Enriching the ontology table

After import, the `ontology` table has one stub row per root:
`(root, semantic_domain='', derivatives='[]', scholar_notes=NULL)`.

To add semantic domains and scholar notes, use the REST API or direct SQL:

```sql
UPDATE ontology
   SET semantic_domain = 'mercy-and-compassion',
       derivatives     = '["رَحْمَة","رَحِيم","رَحْمَان","تَرَاحُم"]',
       scholar_notes   = 'Ibn al-Qayyim distinguishes al-Raḥmān (universal mercy) ...'
 WHERE root = 'رحم';
```

---

## Attribution

**Quranic Arabic Corpus v0.4**
Kais Dukes, University of Leeds, 2010.
https://corpus.quran.com/

Please cite the original work if you publish results using this data.
