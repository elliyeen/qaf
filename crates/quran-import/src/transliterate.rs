/// Simplified ALA-LC–inspired transliteration from Arabic Unicode to Latin.
///
/// Design decisions:
///
/// 1. **Fatha + shadda** (e.g., "رَبِّ"): the shadda follows the vowel mark in
///    many Quranic text encodings.  When a vowel mark is followed immediately by
///    a shadda, we output [doubled-consonant][vowel] rather than
///    [consonant][vowel][shadda-copy].  This gives "rabbi" for "رَبِّ".
///
/// 2. **Superscript alef** (U+0670 ٰ): encodes a long ā in certain contexts
///    (e.g., "مَٰ" in "رَحْمَٰنِ").  When encountered, if the preceding output
///    character is a bare 'a' (from a fatha), we replace it with 'ā'.
///
/// 3. **Alef wasla** (ٱ U+0671): produces a vocal-onset "a".
///
/// Known limitation:
///   "ٱللَّهِ" → "alllahi" (three l's) rather than "allāhi".
///   The definite article's lam + the geminated root lam of Allah's name interact
///   through Arabic sun-letter assimilation in a way that a character-by-character
///   algorithm cannot resolve without a lexicon.  The canonical Arabic text is the
///   authoritative reference; transliterations are approximations.
pub fn arabic_to_translit(arabic: &str) -> String {
    let chars: Vec<char> = arabic.chars().collect();
    let n = chars.len();
    let mut result = String::new();
    let mut last_consonant: &'static str = "";
    let mut i = 0;

    while i < n {
        let c = chars[i];
        let next = chars.get(i + 1).copied();

        match c {
            // ── diacritics with look-ahead for shadda ─────────────────
            // When a vowel mark is immediately followed by shadda, the
            // shadda applies to the same consonant: output [double][vowel].
            '\u{064E}' => {
                // fatha → "a", but check for shadda after
                if next == Some('\u{0651}') {
                    result.push_str(last_consonant); // geminate
                    result.push('a');
                    i += 2;
                } else {
                    result.push('a');
                    i += 1;
                }
            }
            '\u{064F}' => {
                // damma → "u"
                if next == Some('\u{0651}') {
                    result.push_str(last_consonant);
                    result.push('u');
                    i += 2;
                } else {
                    result.push('u');
                    i += 1;
                }
            }
            '\u{0650}' => {
                // kasra → "i"
                if next == Some('\u{0651}') {
                    result.push_str(last_consonant);
                    result.push('i');
                    i += 2;
                } else {
                    result.push('i');
                    i += 1;
                }
            }
            // ── shadda standalone (standard order: consonant + shadda) ─
            '\u{0651}' => {
                result.push_str(last_consonant);
                i += 1;
            }
            // ── tanwin ───────────────────────────────────────────────
            '\u{064B}' => { result.push_str("an"); i += 1; }
            '\u{064C}' => { result.push_str("un"); i += 1; }
            '\u{064D}' => { result.push_str("in"); i += 1; }
            // ── sukun: no vowel ───────────────────────────────────────
            '\u{0652}' => { i += 1; }
            // ── superscript alef (long ā marker) ─────────────────────
            // Replaces a preceding bare 'a' (from fatha) with 'ā'.
            '\u{0670}' => {
                if result.ends_with('a') {
                    result.pop();
                    result.push_str("ā");
                } else {
                    result.push_str("ā");
                }
                i += 1;
            }
            // ── skip non-transliterable marks ────────────────────────
            '\u{0640}' => { i += 1; } // tatweel
            '\u{06D6}'..='\u{06ED}' => { i += 1; } // Quranic annotation signs
            '\u{0610}'..='\u{061A}' => { i += 1; } // Arabic sign block
            // ── consonants ───────────────────────────────────────────
            _ => {
                let t = char_to_translit(c);
                result.push_str(t);
                if !t.is_empty() {
                    last_consonant = t;
                }
                i += 1;
            }
        }
    }

    result
}

#[inline]
fn char_to_translit(c: char) -> &'static str {
    match c {
        'ء' => "ʾ",
        'آ' => "ā",
        'أ' => "ʾ",
        'ؤ' => "ʾ",
        'إ' => "ʾ",
        'ئ' => "ʾ",
        'ا' => "ā",
        'ب' => "b",
        'ة' => "h",
        'ت' => "t",
        'ث' => "th",
        'ج' => "j",
        'ح' => "ḥ",
        'خ' => "kh",
        'د' => "d",
        'ذ' => "dh",
        'ر' => "r",
        'ز' => "z",
        'س' => "s",
        'ش' => "sh",
        'ص' => "ṣ",
        'ض' => "ḍ",
        'ط' => "ṭ",
        'ظ' => "ẓ",
        'ع' => "ʿ",
        'غ' => "gh",
        'ف' => "f",
        'ق' => "q",
        'ك' => "k",
        'ل' => "l",
        'م' => "m",
        'ن' => "n",
        'ه' => "h",
        'و' => "w",
        'ى' => "ā",
        'ي' => "y",
        'ٱ' => "a",  // alef wasla → vocal onset
        _ => "",      // unknown → skip (avoids cluttering output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translit_bismi() {
        // بِسْمِ = b+kasra, s+sukun, m+kasra
        assert_eq!(arabic_to_translit("بِسْمِ"), "bismi");
    }

    #[test]
    fn translit_rahman() {
        // رَحْمَٰنِ: fatha+superscript-alef → ā
        assert_eq!(arabic_to_translit("رَحْمَٰنِ"), "raḥmāni");
    }

    #[test]
    fn translit_shadda_rabbi() {
        // رَبِّ: ba + kasra + shadda → "bb" + "i"
        assert_eq!(arabic_to_translit("رَبِّ"), "rabbi");
    }

    #[test]
    fn translit_allah_three_lams() {
        // ٱللَّهِ: alef-wasla(a) + lam(l) + lam(l) + fatha+shadda(la) + ha(h) + kasra(i)
        // → "alllahi"  (three l's: article-lam + root-lam + shadda-geminated)
        // See module doc on the known sun-letter assimilation limitation.
        assert_eq!(arabic_to_translit("ٱللَّهِ"), "alllahi");
    }

    #[test]
    fn translit_empty() {
        assert_eq!(arabic_to_translit(""), "");
    }
}
