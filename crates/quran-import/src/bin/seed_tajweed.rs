//! seed-tajweed — programmatic tajweed span annotation.
//!
//! Reads every row in `recitation_texts`, runs a character-level Arabic rule
//! engine over the text, and inserts matching spans into `tajweed_spans`.
//! All inserts use `INSERT OR IGNORE`, so re-running is safe.
//!
//! ## Usage
//!
//! ```bash
//! # Annotate all recitations (hafs + khalaf if both have texts)
//! seed-tajweed
//!
//! # Annotate a single recitation only
//! seed-tajweed --recitation hafs
//! seed-tajweed --recitation khalaf
//!
//! # Wipe existing spans for the target recitation(s) then re-annotate
//! seed-tajweed --reset
//!
//! # Custom DB path
//! seed-tajweed --db sqlite:/path/to/qaf.db
//! ```
//!
//! ## Detection coverage
//!
//! ### Fully automated (Unicode-derived):
//! - `qalqalah`           — {ق ط ب ج د} + sukūn
//! - `ghunnah`            — {ن م} + shadda
//! - `madd_tabii`         — superscript alef (U+0670); bare alef after fatha;
//!                          waw+sukun after damma; ya+sukun after kasra
//! - `idgham_ghunnah`     — nun-sakina/tanwin before {ي ن م و}
//! - `idgham_bila_ghunnah`— nun-sakina/tanwin before {ل ر}
//! - `iqlab`              — nun-sakina/tanwin before {ب}
//! - `ikhfa`              — nun-sakina/tanwin before remaining 15 letters
//! - `izhar`              — nun-sakina/tanwin before throat letters {ء ه ع ح غ خ}
//! - `idgham_shafawi`     — meem-sakina before meem
//! - `ikhfa_shafawi`      — meem-sakina before ba
//! - `izhar_shafawi`      — meem-sakina before all other letters
//!
//! ### Hardcoded positions (rule too context-specific for Unicode detection):
//! - `sakt` (Hafs, 4 positions): الكهف 18:1, يس 36:52, القيامة 75:27, المطففين 83:14
//!
//! ### Not yet implemented (require cross-ayah or MSS-specific knowledge):
//! - `madd_muttasil`, `madd_munfasil`, `madd_lazim`, `madd_arid`, `madd_lin`, `madd_badal`
//! - `imalah`, `tashil`, `naql`, `ishmam` (Khalaf-specific; require Tanzil Khalaf XML)

// ─── Detection engine ─────────────────────────────────────────────────────────

mod detect {
    // ── Unicode constants ────────────────────────────────────────────────────

    // Diacritics
    pub const FATHATAN: char = '\u{064B}'; // ً  tanwin nasr
    pub const DAMMATAN: char = '\u{064C}'; // ٌ  tanwin damm
    pub const KASRATAN: char = '\u{064D}'; // ٍ  tanwin kasr
    pub const FATHA:    char = '\u{064E}'; // َ  fatha
    pub const DAMMA:    char = '\u{064F}'; // ُ  damma
    pub const KASRA:    char = '\u{0650}'; // ِ  kasra
    pub const SHADDA:   char = '\u{0651}'; // ّ  shadda
    pub const SUKUN:    char = '\u{0652}'; // ْ  sukun
    pub const _MADDAH:  char = '\u{0653}'; // ٓ  maddah above (used in آ)
    pub const SUPERALEF:char = '\u{0670}'; // ٰ  superscript alef (madd tabii marker)

    // Base letters
    pub const HAMZA:     char = '\u{0621}'; // ء
    pub const ALEF_M:    char = '\u{0622}'; // آ alef with madda (hamzah form)
    pub const ALEF_HA:   char = '\u{0623}'; // أ alef with hamzah above
    pub const WAW_H:     char = '\u{0624}'; // ؤ waw with hamzah
    pub const ALEF_HB:   char = '\u{0625}'; // إ alef with hamzah below
    pub const YA_H:      char = '\u{0626}'; // ئ ya with hamzah
    pub const ALEF:      char = '\u{0627}'; // ا
    pub const BA:        char = '\u{0628}'; // ب
    pub const JIM:       char = '\u{062C}'; // ج
    pub const HAH:       char = '\u{062D}'; // ح (throat letter)
    pub const KHA:       char = '\u{062E}'; // خ (throat letter)
    pub const DAL:       char = '\u{062F}'; // د
    pub const RA:        char = '\u{0631}'; // ر
    pub const TA_H:      char = '\u{0637}'; // ط
    pub const AIN:       char = '\u{0639}'; // ع (throat letter)
    pub const GHAIN:     char = '\u{063A}'; // غ (throat letter)
    pub const QAF:       char = '\u{0642}'; // ق
    pub const LAM:       char = '\u{0644}'; // ل
    pub const MIM:       char = '\u{0645}'; // م
    pub const NUN:       char = '\u{0646}'; // ن
    pub const HEH:       char = '\u{0647}'; // ه (throat letter)
    pub const WAW:       char = '\u{0648}'; // و
    pub const YA:        char = '\u{064A}'; // ي
    pub const _ALEF_WA:  char = '\u{0671}'; // ٱ alef wasla (base letter, not diacritic)

    // ── Character classification helpers ────────────────────────────────────

    /// Returns true for Arabic combining marks / diacritics (not base letters).
    /// U+0671 (alef wasla) is a BASE letter, not a diacritic, so excluded.
    pub fn is_combining(c: char) -> bool {
        matches!(c, '\u{064B}'..='\u{065F}' | '\u{0670}')
    }

    /// Returns true for Arabic base consonants and alef wasla.
    pub fn is_arabic_base(c: char) -> bool {
        matches!(c, '\u{0621}'..='\u{063A}' | '\u{0641}'..='\u{064A}' | '\u{0671}')
    }

    /// True for tanwin marks (double harakat indicating nunation).
    pub fn is_tanwin(c: char) -> bool {
        matches!(c, FATHATAN | DAMMATAN | KASRATAN)
    }

    /// True for qalqalah letters: ق ط ب ج د
    pub fn is_qalqalah(c: char) -> bool {
        matches!(c, QAF | TA_H | BA | JIM | DAL)
    }

    /// True for izhar (throat) letters: ء ه ع ح غ خ
    pub fn is_throat(c: char) -> bool {
        matches!(c, HAMZA | ALEF_M | ALEF_HA | WAW_H | ALEF_HB | YA_H | HEH | AIN | HAH | GHAIN | KHA)
    }

    // ── Grapheme model ───────────────────────────────────────────────────────

    /// A base Arabic letter together with its immediately following combining marks.
    /// `pos` is the 0-based Unicode character index into the source text.
    #[derive(Debug)]
    pub struct Grapheme {
        pub pos: usize,
        pub base: char,
        /// (char_index, char) for each combining mark following this base letter.
        pub marks: Vec<(usize, char)>,
        /// True when the next character in the original text after all marks is
        /// a space (U+0020) or end-of-text — i.e. this letter ends a word.
        /// Used to detect implicit sākina (vowel-less word-final consonants that
        /// the mushaf writes without an explicit sukūn diacritic, e.g. هُم بِـ).
        pub word_final: bool,
    }

    impl Grapheme {
        /// Returns true if this grapheme carries the given combining mark.
        pub fn has_mark(&self, m: char) -> bool {
            self.marks.iter().any(|(_, c)| *c == m)
        }

        /// Character index of a specific mark, if present.
        pub fn mark_pos(&self, m: char) -> Option<usize> {
            self.marks.iter().find(|(_, c)| *c == m).map(|(p, _)| *p)
        }

        /// `(start_index, length)` spanning from the base letter through (and
        /// including) the given mark.  Falls back to length 1 if mark absent.
        pub fn span_through(&self, m: char) -> (usize, usize) {
            match self.mark_pos(m) {
                Some(mp) => (self.pos, mp - self.pos + 1),
                None => (self.pos, 1),
            }
        }

        /// The vowel carried on this grapheme, if any (fatha/damma/kasra).
        pub fn vowel(&self) -> Option<char> {
            self.marks
                .iter()
                .find(|(_, c)| matches!(*c, FATHA | DAMMA | KASRA))
                .map(|(_, c)| *c)
        }

        /// True if this grapheme carries any tanwin mark.
        pub fn has_tanwin(&self) -> bool {
            self.marks.iter().any(|(_, c)| is_tanwin(*c))
        }

        /// True if this grapheme is sākin — either by explicit sukūn diacritic
        /// or by being a vowel-less word-final consonant (implicit sākina).
        ///
        /// Mushaf text often omits the sukūn on word-final consonants such as
        /// the meem in "هُم" or the nun in "مِن".  The implicit-sākina check
        /// catches those cases: no vowel, no shadda, no tanwin, and word-final.
        pub fn is_sakin(&self) -> bool {
            if self.has_mark(SUKUN) {
                return true;
            }
            // Implicit: word-final with no vowel-class mark at all.
            self.word_final
                && self.vowel().is_none()
                && !self.has_mark(SHADDA)
                && !self.has_tanwin()
        }

        /// True if this grapheme is a bare alef used only to carry fathatan
        /// (the "alef of tanwin nasr").  Such alefs are not real madd letters.
        pub fn is_tanwin_alef(&self, prev: Option<&Grapheme>) -> bool {
            self.base == ALEF
                && self.marks.is_empty()
                && prev.map(|p| p.has_tanwin()).unwrap_or(false)
        }
    }

    /// Build a grapheme list from a Unicode text string.
    /// Spaces and non-Arabic characters are skipped; they do not appear in the
    /// list but do not affect `pos` values (those are char indices into `text`).
    pub fn build_graphemes(text: &str) -> Vec<Grapheme> {
        let chars: Vec<char> = text.chars().collect();
        let n = chars.len();
        let mut gs: Vec<Grapheme> = Vec::with_capacity(n / 2);
        let mut i = 0;

        while i < n {
            let c = chars[i];

            if is_arabic_base(c) {
                let pos = i;
                let base = c;
                let mut marks = Vec::new();
                i += 1;
                while i < n && is_combining(chars[i]) {
                    marks.push((i, chars[i]));
                    i += 1;
                }
                // Check whether the immediately following character (after all
                // combining marks) is a space or end-of-text.
                let word_final = i >= n || chars[i] == '\u{0020}' || chars[i] == '\u{06D6}'
                    || chars[i] == '\u{06D7}' || chars[i] == '\u{06D8}'
                    || chars[i] == '\u{06DF}' || chars[i] == '\u{06E0}';
                gs.push(Grapheme { pos, base, marks, word_final });
            } else {
                // Space, newline, punctuation — skip (preserves char index)
                i += 1;
            }
        }

        gs
    }

    // ── Span output ─────────────────────────────────────────────────────────

    #[derive(Debug)]
    pub struct Span {
        pub start: usize,
        pub length: usize,
        pub rule: &'static str,
        pub note: Option<String>,
    }

    // ── Main detection function ──────────────────────────────────────────────

    /// Detect tajweed spans in a single ayah text.
    ///
    /// `surah` and `ayah` are provided to allow hardcoded sakt positions.
    /// The recitation slug is used to gate recitation-specific rules.
    pub fn detect_spans(
        surah: i32,
        ayah: i32,
        text: &str,
        recitation: &str,
    ) -> Vec<Span> {
        let gs = build_graphemes(text);
        let mut spans: Vec<Span> = Vec::new();

        for (gi, g) in gs.iter().enumerate() {
            let prev = if gi > 0 { Some(&gs[gi - 1]) } else { None };
            let next = gs.get(gi + 1);

            // ── qalqalah ────────────────────────────────────────────────────
            if is_qalqalah(g.base) && g.has_mark(SUKUN) {
                let (start, length) = g.span_through(SUKUN);
                spans.push(Span { start, length, rule: "qalqalah", note: None });
            }

            // ── ghunnah (ن/م + shadda) ──────────────────────────────────────
            if matches!(g.base, NUN | MIM) && g.has_mark(SHADDA) {
                let (start, length) = g.span_through(SHADDA);
                spans.push(Span { start, length, rule: "ghunnah", note: None });
            }

            // ── madd_tabii via superscript alef (U+0670) ────────────────────
            // The superscript alef is a combining mark on the preceding base
            // letter; the span covers that letter + the superscript alef.
            if g.has_mark(SUPERALEF) {
                let (start, length) = g.span_through(SUPERALEF);
                spans.push(Span { start, length, rule: "madd_tabii", note: None });
            }

            // ── madd_tabii via bare alef after fatha ─────────────────────────
            // Bare ا following a letter with fatha is a natural prolongation,
            // UNLESS the alef is the tanwin-nasr carrier (fathatan on prev).
            if g.base == ALEF
                && g.marks.is_empty()
                && !g.is_tanwin_alef(prev)
                && prev.and_then(|p| p.vowel()) == Some(FATHA)
            {
                spans.push(Span { start: g.pos, length: 1, rule: "madd_tabii", note: None });
            }

            // ── madd_tabii via waw-sukun after damma ─────────────────────────
            if g.base == WAW
                && g.has_mark(SUKUN)
                && prev.and_then(|p| p.vowel()) == Some(DAMMA)
            {
                let (start, length) = g.span_through(SUKUN);
                spans.push(Span { start, length, rule: "madd_tabii", note: None });
            }

            // ── madd_tabii via ya-sukun after kasra ──────────────────────────
            if g.base == YA
                && g.has_mark(SUKUN)
                && prev.and_then(|p| p.vowel()) == Some(KASRA)
            {
                let (start, length) = g.span_through(SUKUN);
                spans.push(Span { start, length, rule: "madd_tabii", note: None });
            }

            // ── nun-sakinah / tanwin rules ───────────────────────────────────
            // The rule depends on the next base letter (may cross a word space).
            // If there is no next grapheme (end of ayah), the rules don't apply
            // at this level (they apply only at pause, handled by madd_arid).
            let nun_sakina  = g.base == NUN && g.is_sakin();
            let has_tanwin  = g.has_tanwin();

            if (nun_sakina || has_tanwin) && next.is_some() {
                // For tanwin, skip the tanwin-alef carrier if it immediately follows.
                let effective_next: Option<&Grapheme> = if has_tanwin {
                    // Check if the next grapheme is a bare alef (tanwin carrier)
                    match next {
                        Some(ng) if ng.is_tanwin_alef(Some(g)) => gs.get(gi + 2),
                        other => other,
                    }
                } else {
                    next
                };

                if let Some(ng) = effective_next {
                    let rule: &'static str = if is_throat(ng.base) {
                        "izhar"
                    } else {
                        match ng.base {
                            BA               => "iqlab",
                            YA | NUN | MIM | WAW => "idgham_ghunnah",
                            LAM | RA         => "idgham_bila_ghunnah",
                            _                => "ikhfa",
                        }
                    };

                    if nun_sakina {
                        let (start, length) = g.span_through(SUKUN);
                        spans.push(Span { start, length, rule, note: None });
                    } else {
                        // Tanwin: span starts at the tanwin diacritic itself
                        // (the last mark on the grapheme that is a tanwin).
                        if let Some((tp, _)) = g
                            .marks
                            .iter()
                            .rev()
                            .find(|(_, c)| is_tanwin(*c))
                        {
                            spans.push(Span {
                                start: *tp,
                                length: 1,
                                rule,
                                note: None,
                            });
                        }
                    }
                }
            }

            // ── meem-sakinah rules ───────────────────────────────────────────
            // Only applies when meem is sākin; separate from meem+shadda above.
            if g.base == MIM && g.is_sakin() {
                if let Some(ng) = next {
                    let rule: &'static str = match ng.base {
                        BA  => "ikhfa_shafawi",
                        MIM => "idgham_shafawi",
                        _   => "izhar_shafawi",
                    };
                    let (start, length) = g.span_through(SUKUN);
                    spans.push(Span { start, length, rule, note: None });
                }
            }
        }

        // ── sakt (hardcoded positions) ───────────────────────────────────────
        if recitation == "hafs" {
            detect_hafs_sakt(surah, ayah, text, &mut spans);
        }

        spans
    }

    // ── Hafs sakt ────────────────────────────────────────────────────────────

    /// Hafs has sakt (brief voiceless pause) in exactly four positions.
    /// We locate each by searching for a known word fragment within the ayah
    /// text and marking the last character of that fragment.
    ///
    /// Positions (all within-ayah or at ayah-end):
    ///   الكهف   18:1   — end of "عِوَجَا" (sakt between 18:1 and 18:2)
    ///   يس      36:52  — after "مَّرْقَدِنَا" before "هَٰذَا"
    ///   القيامة 75:27  — after "رَاقٍ"
    ///   المطففين 83:14 — after "رَانَ" before "عَلَىٰ"
    fn detect_hafs_sakt(surah: i32, ayah: i32, text: &str, spans: &mut Vec<Span>) {
        // Each entry: (surah, ayah, fragment to locate, note)
        // We mark the last char of `fragment` in `text`.
        // NOTE on diacritic ordering: the DB stores shadda *before* fatha
        // (م + U+0651 + U+064E), matching the Quranic Corpus source convention.
        // Patterns must use the same ordering or the char-by-char search fails.
        const POSITIONS: &[(i32, i32, &str, &str)] = &[
            (18,  1,  "عِوَجَا",     "سكت حفص — الكهف 18:1 (آخر الآية)"),
            // م(0645) ّ(0651) َ(064E) ر(0631) ْ(0652) ق(0642) َ(064E) د(062F) ِ(0650) ن(0646) َ(064E) ا(0627)
            (36, 52,  "م\u{0651}\u{064E}رْقَدِنَا", "سكت حفص — يس 36:52"),
            (75, 27,  "رَاقٍ",       "سكت حفص — القيامة 75:27"),
            (83, 14,  "رَانَ",       "سكت حفص — المطففين 83:14"),
        ];

        for &(s, a, fragment, note) in POSITIONS {
            if surah != s || ayah != a {
                continue;
            }

            // Find the fragment as a Unicode char-level substring.
            let chars: Vec<char> = text.chars().collect();
            let frag_chars: Vec<char> = fragment.chars().collect();
            let fl = frag_chars.len();

            'outer: for start_idx in 0..=chars.len().saturating_sub(fl) {
                for (k, fc) in frag_chars.iter().enumerate() {
                    if chars[start_idx + k] != *fc {
                        continue 'outer;
                    }
                }
                // Found at start_idx; mark the last char of the fragment.
                let last = start_idx + fl - 1;
                spans.push(Span {
                    start: last,
                    length: 1,
                    rule: "sakt",
                    note: Some(note.to_string()),
                });
                break;
            }
        }
    }
}

// ─── CLI ─────────────────────────────────────────────────────────────────────

use anyhow::{Context, Result};
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use quran_db::{connect, run_migrations};
use sqlx::SqlitePool;
use tracing::info;

#[derive(Parser, Debug)]
#[command(
    name = "seed-tajweed",
    about = "Detect and insert tajweed spans for recitation texts"
)]
struct Args {
    /// Which recitation(s) to annotate.
    /// Pass multiple times for multiple recitations (default: all in DB).
    #[arg(long = "recitation", value_name = "SLUG")]
    recitations: Vec<String>,

    /// SQLite database URL. Defaults to DATABASE_URL env var or sqlite:qaf.db.
    #[arg(long, value_name = "URL")]
    db: Option<String>,

    /// Delete all existing tajweed_spans for the target recitation(s) before
    /// re-annotating. Idempotent either way — use when re-running after rule
    /// changes.
    #[arg(long)]
    reset: bool,

    /// Suppress the progress bar.
    #[arg(long)]
    no_progress: bool,
}

// ─── Entry point ──────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "seed_tajweed=info,warn".into()),
        )
        .init();

    let args = Args::parse();

    let database_url = args
        .db
        .or_else(|| std::env::var("DATABASE_URL").ok())
        .unwrap_or_else(|| "sqlite:qaf.db".into());

    info!("connecting to {}", database_url);
    let pool = connect(&database_url).await?;
    run_migrations(&pool).await?;

    // Resolve which recitations to annotate.
    let recitation_slugs: Vec<(i64, String)> = if args.recitations.is_empty() {
        sqlx::query_as::<_, (i64, String)>(
            "SELECT id, name FROM recitations ORDER BY id",
        )
        .fetch_all(&pool)
        .await
        .context("fetch recitations")?
    } else {
        let mut out = Vec::new();
        for slug in &args.recitations {
            let row: Option<(i64,)> =
                sqlx::query_as("SELECT id FROM recitations WHERE name = ?")
                    .bind(slug)
                    .fetch_optional(&pool)
                    .await
                    .with_context(|| format!("lookup recitation '{}'", slug))?;
            match row {
                Some((id,)) => out.push((id, slug.clone())),
                None => anyhow::bail!("recitation '{}' not found in catalogue", slug),
            }
        }
        out
    };

    if recitation_slugs.is_empty() {
        anyhow::bail!("no recitations found in the database — run seed-recitations first");
    }

    for (rec_id, rec_name) in &recitation_slugs {
        if args.reset {
            tracing::warn!(
                "--reset: deleting existing tajweed_spans for recitation '{}'",
                rec_name
            );
            sqlx::query(
                "DELETE FROM tajweed_spans
                 WHERE recitation_text_id IN (
                     SELECT id FROM recitation_texts WHERE recitation_id = ?
                 )",
            )
            .bind(*rec_id)
            .execute(&pool)
            .await
            .with_context(|| format!("reset spans for '{}'", rec_name))?;
        }

        let inserted = annotate_recitation(&pool, *rec_id, rec_name, !args.no_progress).await?;
        println!("  {}: {} spans inserted", rec_name, inserted);
    }

    Ok(())
}

// ─── Core annotation loop ─────────────────────────────────────────────────────

/// Load all recitation_texts for one recitation, run the detection engine on
/// each ayah text, and bulk-insert the resulting spans.
async fn annotate_recitation(
    pool: &SqlitePool,
    recitation_id: i64,
    recitation_name: &str,
    show_progress: bool,
) -> Result<u64> {
    // Fetch every (id, surah_id, ayah_number, text) for this recitation.
    let rows: Vec<(i64, i32, i32, String)> = sqlx::query_as(
        "SELECT id, surah_id, ayah_number, text
         FROM recitation_texts
         WHERE recitation_id = ?
         ORDER BY surah_id, ayah_number",
    )
    .bind(recitation_id)
    .fetch_all(pool)
    .await
    .with_context(|| format!("fetch recitation_texts for '{}'", recitation_name))?;

    info!(
        "{}: {} ayah texts loaded",
        recitation_name,
        rows.len()
    );

    let total = rows.len() as u64;
    let pb = if show_progress {
        let pb = ProgressBar::new(total);
        pb.set_style(
            ProgressStyle::with_template(
                "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg}",
            )
            .unwrap()
            .progress_chars("=>-"),
        );
        pb.set_message(format!("annotating {}", recitation_name));
        Some(pb)
    } else {
        None
    };

    let mut total_inserted: u64 = 0;
    let mut tx = pool.begin().await?;
    let mut batch_count: usize = 0;

    for (rt_id, surah_id, ayah_number, text) in &rows {
        let spans = detect::detect_spans(*surah_id, *ayah_number, text, recitation_name);

        for span in &spans {
            let r = sqlx::query(
                "INSERT OR IGNORE INTO tajweed_spans
                 (recitation_text_id, start_index, length, rule, note)
                 VALUES (?, ?, ?, ?, ?)",
            )
            .bind(rt_id)
            .bind(span.start as i64)
            .bind(span.length as i64)
            .bind(span.rule)
            .bind(span.note.as_deref())
            .execute(&mut *tx)
            .await
            .with_context(|| {
                format!(
                    "insert span ({}, {}) rule='{}' for rt_id={}",
                    span.start, span.length, span.rule, rt_id
                )
            })?;

            total_inserted += r.rows_affected();
        }

        batch_count += 1;
        if let Some(ref pb) = pb {
            pb.inc(1);
        }

        // Commit every 500 ayahs to keep transaction size manageable.
        if batch_count >= 500 {
            tx.commit().await?;
            tx = pool.begin().await?;
            batch_count = 0;
        }
    }

    tx.commit().await?;

    if let Some(pb) = pb {
        pb.finish_with_message("done");
    }

    Ok(total_inserted)
}
