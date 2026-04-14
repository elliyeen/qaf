/// Parser for the Quranic Arabic Corpus morphology file.
///
/// Source: https://github.com/mustafa0x/quran-morphology  (quran-morphology.txt)
/// Original: https://corpus.quran.com/download/ (v0.4)
///
/// File format — tab-separated, 4 columns:
///   LOCATION    ARABIC_SEGMENT    POS_CODE    FEATURES
///
/// LOCATION is `surah:ayah:word:segment`  (parenthesised form `(1:1:1:1)` also accepted).
/// ARABIC_SEGMENT is the Arabic Unicode text for this one morpheme (with diacritics).
/// FEATURES is pipe-separated: ROOT:xxx | LEM:xxx | NOM | SG | M | DEF | …
///
/// One Quranic word can span 1–3 segments (prefix + stem + suffix). All segments
/// for the same (surah, ayah, word) are concatenated to produce the full word.
use anyhow::{Context, Result};
use serde_json::json;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufRead, BufReader};

/// Everything the importer needs to know about one Quranic word.
#[derive(Debug)]
pub struct QacWord {
    pub surah: i32,
    pub ayah: i32,
    pub position: i32,
    /// Full Arabic text (concatenated segments, with diacritics as in the mushaf).
    pub arabic: String,
    /// Arabic three-letter root, e.g. "رحم".  None for particles with no root.
    pub root: Option<String>,
    /// Arabic lemma (canonical form), e.g. "رَحِيم".
    pub lemma: String,
    /// Normalised POS tag, e.g. "N", "V", "Prep", "Conj".
    pub pos: String,
    /// JSON object of grammatical features, e.g. {"case":"genitive","number":"singular"}.
    pub features: serde_json::Value,
}

// ── internal segment representation ──────────────────────────────────────────

#[derive(Debug)]
struct Segment {
    arabic: String,
    pos: String,
    root: Option<String>,
    lemma: Option<String>,
    flags: Vec<String>,
    is_affix: bool, // true for PREF / SUFF segments
}

// ── public entry point ────────────────────────────────────────────────────────

/// Parse `quran-morphology.txt` and return words sorted by canonical Quran order.
/// The returned `BTreeMap` key is `(surah, ayah, position)`.
pub fn parse(path: &str) -> Result<BTreeMap<(i32, i32, i32), QacWord>> {
    let file = File::open(path)
        .with_context(|| format!("cannot open QAC file: {}", path))?;
    let reader = BufReader::new(file);

    let mut raw: BTreeMap<(i32, i32, i32), Vec<Segment>> = BTreeMap::new();
    let mut skipped: u32 = 0;

    for (lineno, line) in reader.lines().enumerate() {
        let line = line?;
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        match parse_line(line) {
            Some(((surah, ayah, word), seg)) => {
                raw.entry((surah, ayah, word)).or_default().push(seg);
            }
            None => {
                tracing::warn!("skipping malformed line {}: {:?}", lineno + 1, line);
                skipped += 1;
            }
        }
    }

    if skipped > 0 {
        tracing::warn!("{} lines could not be parsed and were skipped", skipped);
    }

    // Merge segments into words.
    let mut words = BTreeMap::new();
    for ((surah, ayah, position), segs) in raw {
        words.insert((surah, ayah, position), merge_segments(surah, ayah, position, segs));
    }

    tracing::info!("parsed {} words from QAC file", words.len());
    Ok(words)
}

// ── line parser ───────────────────────────────────────────────────────────────

fn parse_line(line: &str) -> Option<((i32, i32, i32), Segment)> {
    let parts: Vec<&str> = line.splitn(4, '\t').collect();
    if parts.len() < 3 {
        return None;
    }

    // Location: "1:2:3:4" or "(1:2:3:4)"
    let loc = parts[0].trim_matches(|c| c == '(' || c == ')');
    let loc_parts: Vec<&str> = loc.split(':').collect();
    if loc_parts.len() < 3 {
        return None;
    }
    let surah: i32 = loc_parts[0].parse().ok()?;
    let ayah: i32 = loc_parts[1].parse().ok()?;
    let word: i32 = loc_parts[2].parse().ok()?;

    let arabic = parts[1].to_string();
    let pos_raw = parts[2];
    let features_str = parts.get(3).copied().unwrap_or("");

    // Features
    let mut root: Option<String> = None;
    let mut lemma: Option<String> = None;
    let mut flags: Vec<String> = Vec::new();
    let mut is_affix = false;

    for token in features_str.split('|') {
        let token = token.trim();
        if token.is_empty() {
            continue;
        }
        if let Some(r) = token.strip_prefix("ROOT:") {
            root = Some(r.to_string());
        } else if let Some(l) = token.strip_prefix("LEM:") {
            lemma = Some(l.to_string());
        } else if matches!(token, "PREF" | "PREFIX" | "SUFF" | "SUFFIX") {
            is_affix = true;
            flags.push(token.to_string());
        } else {
            flags.push(token.to_string());
        }
    }

    // Some affix segments declare POS=DET, which also marks them as affixes.
    if matches!(pos_raw, "DET") {
        is_affix = true;
    }

    Some((
        (surah, ayah, word),
        Segment {
            arabic,
            pos: pos_raw.to_string(),
            root,
            lemma,
            flags,
            is_affix,
        },
    ))
}

// ── segment merger ────────────────────────────────────────────────────────────

fn merge_segments(surah: i32, ayah: i32, position: i32, segs: Vec<Segment>) -> QacWord {
    // Full Arabic = concatenated segments in order.
    let arabic: String = segs.iter().map(|s| s.arabic.as_str()).collect();

    // Stem = first non-affix segment that has root or lemma; else last segment.
    let stem = segs
        .iter()
        .find(|s| !s.is_affix && (s.root.is_some() || s.lemma.is_some()))
        .or_else(|| segs.iter().find(|s| !s.is_affix))
        .or_else(|| segs.last())
        .unwrap(); // segs is never empty here

    let root = stem.root.clone();
    // Lemma: prefer explicit LEM from stem; fall back to bare Arabic.
    let lemma = stem
        .lemma
        .clone()
        .unwrap_or_else(|| quran_db::strip_diacritics(&arabic));

    let pos = normalize_pos(&stem.pos);
    let features = flags_to_json(&stem.flags);

    QacWord {
        surah,
        ayah,
        position,
        arabic,
        root,
        lemma,
        pos,
        features,
    }
}

// ── POS normalisation ─────────────────────────────────────────────────────────

fn normalize_pos(pos: &str) -> String {
    match pos {
        "N" => "N",
        "PN" => "PN",
        "ADJ" => "Adj",
        "V" | "IV" | "PV" | "CV" => "V",
        "IMPV" => "V",
        "P" => "Prep",
        "CONJ" => "Conj",
        "DET" => "Det",
        "PRON" | "DEM" | "REF" => "Pron",
        "NEG" | "ACC" | "VOC" | "INTG" | "PREV" | "REM" | "RSLT" | "EXPL" | "CERT" | "ANS"
        | "COND" | "FUT" | "INC" | "SUP" | "AMD" | "PRO" | "EXH" | "EMPH" | "INTERJ" => "Part",
        "REL" => "Rel",
        "T" | "LOC" => "N",
        "INL" => "INL",
        other => other,
    }
    .to_string()
}

// ── feature flags → JSON ──────────────────────────────────────────────────────

fn flags_to_json(flags: &[String]) -> serde_json::Value {
    let mut map = serde_json::Map::new();

    for flag in flags {
        match flag.as_str() {
            // Case
            "NOM" => { map.insert("case".into(), json!("nominative")); }
            "ACC" => { map.insert("case".into(), json!("accusative")); }
            "GEN" => { map.insert("case".into(), json!("genitive")); }
            // Gender + Number combined codes (common in QAC)
            "MS" => { map.insert("gender".into(), json!("masculine")); map.insert("number".into(), json!("singular")); }
            "FS" => { map.insert("gender".into(), json!("feminine")); map.insert("number".into(), json!("singular")); }
            "MD" => { map.insert("gender".into(), json!("masculine")); map.insert("number".into(), json!("dual")); }
            "FD" => { map.insert("gender".into(), json!("feminine")); map.insert("number".into(), json!("dual")); }
            "MP" => { map.insert("gender".into(), json!("masculine")); map.insert("number".into(), json!("plural")); }
            "FP" => { map.insert("gender".into(), json!("feminine")); map.insert("number".into(), json!("plural")); }
            // Gender alone (use or_insert so combined MS/FS codes above win)
            "M" => { map.entry("gender".to_string()).or_insert(json!("masculine")); }
            "F" => { map.entry("gender".to_string()).or_insert(json!("feminine")); }
            // Number alone
            "SG" | "S" => { map.entry("number".to_string()).or_insert(json!("singular")); }
            "DU" | "D" => { map.entry("number".to_string()).or_insert(json!("dual")); }
            "PL" | "P" => { map.entry("number".to_string()).or_insert(json!("plural")); }
            // State
            "DEF" => { map.insert("state".into(), json!("definite")); }
            "INDEF" => { map.insert("state".into(), json!("indefinite")); }
            // Voice
            "ACT" => { map.insert("voice".into(), json!("active")); }
            "PASS" => { map.insert("voice".into(), json!("passive")); }
            // Mood
            "IND" => { map.insert("mood".into(), json!("indicative")); }
            "SUBJ" => { map.insert("mood".into(), json!("subjunctive")); }
            "JUS" => { map.insert("mood".into(), json!("jussive")); }
            // Aspect / tense
            "PERF" => { map.insert("aspect".into(), json!("perfect")); }
            "IMPF" => { map.insert("aspect".into(), json!("imperfect")); }
            "IMPV" => { map.insert("aspect".into(), json!("imperative")); }
            // Person
            "1" => { map.insert("person".into(), json!(1)); }
            "2" => { map.insert("person".into(), json!(2)); }
            "3" => { map.insert("person".into(), json!(3)); }
            // Emphasis
            "EMPH" => { map.insert("emphasis".into(), json!(true)); }
            // Skip structural / POS-repeat markers
            _ => {}
        }
    }

    serde_json::Value::Object(map)
}

// ── unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_test_line(line: &str) -> Option<((i32, i32, i32), Segment)> {
        parse_line(line)
    }

    #[test]
    fn parses_prefix_segment() {
        let line = "1:1:1:1\tبِ\tP\tP|PREF|LEM:ب";
        let ((s, a, w), seg) = parse_test_line(line).unwrap();
        assert_eq!((s, a, w), (1, 1, 1));
        assert_eq!(seg.arabic, "بِ");
        assert_eq!(seg.pos, "P");
        assert!(seg.is_affix);
        assert_eq!(seg.lemma.as_deref(), Some("ب"));
    }

    #[test]
    fn parses_stem_segment_with_root() {
        let line = "1:1:1:2\tسْمِ\tN\tROOT:سمو|LEM:اسْم|M|GEN";
        let ((s, a, w), seg) = parse_test_line(line).unwrap();
        assert_eq!((s, a, w), (1, 1, 1));
        assert_eq!(seg.root.as_deref(), Some("سمو"));
        assert_eq!(seg.lemma.as_deref(), Some("اسْم"));
        assert!(!seg.is_affix);
    }

    #[test]
    fn merges_segments_for_bismi() {
        // (1:1:1) = بِ + سْمِ  →  بِسْمِ
        let segs = vec![
            Segment {
                arabic: "بِ".into(), pos: "P".into(),
                root: None, lemma: Some("ب".into()),
                flags: vec!["PREF".into()], is_affix: true,
            },
            Segment {
                arabic: "سْمِ".into(), pos: "N".into(),
                root: Some("سمو".into()), lemma: Some("اسْم".into()),
                flags: vec!["M".into(), "GEN".into()], is_affix: false,
            },
        ];
        let word = merge_segments(1, 1, 1, segs);
        assert_eq!(word.arabic, "بِسْمِ");
        assert_eq!(word.root.as_deref(), Some("سمو"));
        // The STEM segment's POS is "N" (noun اسم); the "bi" prefix is a
        // preposition but the word's POS is taken from the stem, not the prefix.
        assert_eq!(word.pos, "N");
    }

    #[test]
    fn parses_parenthesised_location() {
        let line = "(1:1:2:1)\tٱللَّهِ\tPN\tROOT:أله|LEM:اللَّه|GEN";
        let ((s, a, w), seg) = parse_test_line(line).unwrap();
        assert_eq!((s, a, w), (1, 1, 2));
        assert_eq!(seg.root.as_deref(), Some("أله"));
    }

    #[test]
    fn normalize_pos_roundtrip() {
        assert_eq!(normalize_pos("N"), "N");
        assert_eq!(normalize_pos("ADJ"), "Adj");
        assert_eq!(normalize_pos("P"), "Prep");
        assert_eq!(normalize_pos("IV"), "V");
        assert_eq!(normalize_pos("DET"), "Det");
    }

    #[test]
    fn features_json_genitive_singular_masculine() {
        let flags: Vec<String> = vec!["MS".into(), "GEN".into()];
        let j = flags_to_json(&flags);
        assert_eq!(j["case"], "genitive");
        assert_eq!(j["gender"], "masculine");
        assert_eq!(j["number"], "singular");
    }
}
