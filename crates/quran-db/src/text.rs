/// Arabic text normalization utilities.
///
/// Arabic text in the Quran is commonly stored with full vowelization (tashkeel).
/// For search purposes we need a bare form that strips diacritics and normalizes
/// alif variants so users can type ا and match أ, إ, آ, ٱ.

/// Remove Arabic diacritics and normalize alif variants from `s`.
///
/// Stripped codepoint ranges:
/// - U+0610–U+061A  Arabic signs (sallos, etc.)
/// - U+064B–U+065F  Harakat: fathatan, dammatan, kasratan, fatha, damma,
///                  kasra, shadda, sukun, and extended diacritics
/// - U+0670         Arabic letter superscript alef (used as vowel mark)
/// - U+06D6–U+06DC  Quranic annotation signs
/// - U+06DF–U+06E4  More annotation signs
/// - U+06E7–U+06E8  Rounded high stop, pause sign
/// - U+06EA–U+06ED  Empty centre, dotted circles, tone marks
///
/// Alif normalization (all → U+0627 ا):
/// - U+0622 آ  alef with madda above
/// - U+0623 أ  alef with hamza above
/// - U+0625 إ  alef with hamza below
/// - U+0671 ٱ  alef wasla (Quranic)
pub fn strip_diacritics(s: &str) -> String {
    s.chars()
        .filter(|&c| !is_arabic_diacritic(c))
        .map(normalize_alif)
        .collect()
}

#[inline]
fn is_arabic_diacritic(c: char) -> bool {
    matches!(c,
        '\u{0610}'..='\u{061A}'   // arabic sign sallallahou alayhe wasallam etc.
        | '\u{064B}'..='\u{065F}' // harakat + extended diacritics
        | '\u{0670}'              // arabic letter superscript alef
        | '\u{06D6}'..='\u{06DC}'
        | '\u{06DF}'..='\u{06E4}'
        | '\u{06E7}'..='\u{06E8}'
        | '\u{06EA}'..='\u{06ED}'
    )
}

#[inline]
fn normalize_alif(c: char) -> char {
    match c {
        '\u{0622}' // آ  alef with madda above
        | '\u{0623}' // أ  alef with hamza above
        | '\u{0625}' // إ  alef with hamza below
        | '\u{0671}' // ٱ  alef wasla
        => '\u{0627}', // → ا
        _ => c,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_harakat() {
        // رَبّ → رب
        assert_eq!(strip_diacritics("رَبّ"), "رب");
    }

    #[test]
    fn strips_shadda_and_sukun() {
        // بِسْمِ → بسم
        assert_eq!(strip_diacritics("بِسْمِ"), "بسم");
    }

    #[test]
    fn strips_tanwin() {
        // كِتَابٌ → كتاب
        assert_eq!(strip_diacritics("كِتَابٌ"), "كتاب");
    }

    #[test]
    fn normalizes_hamza_alif() {
        // أَحْمَد → احمد (hamza+fatha → bare alif, then sukun stripped)
        assert_eq!(strip_diacritics("أَحْمَد"), "احمد");
    }

    #[test]
    fn normalizes_alef_wasla() {
        // ٱللَّهِ → الله  (wasla→alif, shadda+kasra stripped)
        assert_eq!(strip_diacritics("ٱللَّهِ"), "الله");
    }

    #[test]
    fn plain_arabic_unchanged() {
        assert_eq!(strip_diacritics("رحم"), "رحم");
    }

    #[test]
    fn latin_and_empty_unchanged() {
        assert_eq!(strip_diacritics(""), "");
        assert_eq!(strip_diacritics("hello"), "hello");
    }
}
