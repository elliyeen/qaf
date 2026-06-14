//! Linguistic reference models for Quranic morphological and rhetorical analysis.
//!
//! These types represent the building blocks of Arabic linguistic analysis:
//! roots, lemmata, parts of speech, morphological features, syntactic roles,
//! grammatical parsing (iʿrāb), and rhetorical devices (balāghah).
//!
//! # ID formats
//!
//! | Type              | Format                | Example                 |
//! |-------------------|-----------------------|-------------------------|
//! | [`Root`]          | `root:<arabic>`       | `root:كتب`              |
//! | [`Lemma`]         | `lemma:<arabic>`      | `lemma:كَتَبَ`          |
//! | [`PartOfSpeech`]  | `pos:<slug>`          | `pos:verb`              |
//! | [`MorphFeature`]  | `feat:<key>:<value>`  | `feat:gender:masculine` |
//! | [`SyntaxRole`]    | `syn:<slug>`          | `syn:fa3il`             |
//! | [`I3rabRole`]     | `i3rab:<slug>`        | `i3rab:marfu3`          |
//! | [`BalaghahFeature`] | `balagha:<slug>`    | `balagha:tashbih`       |

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::error::QafError;

// ── Root ──────────────────────────────────────────────────────────────────────

/// An Arabic root (جذر), typically composed of 2–4 consonants.
///
/// Arabic roots are the foundation of the lexical system.  Most words
/// derive from a trilateral (3-letter) root; some are quadrilateral.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Root {
    /// Root consonants as a single Arabic string, e.g. `"كتب"`.
    pub letters: String,
}

impl Root {
    /// Construct a [`Root`], validating that `letters` contains 2–4 characters.
    pub fn new(letters: impl Into<String>) -> Result<Self, QafError> {
        let letters = letters.into();
        let n = letters.chars().count();
        if n < 2 || n > 4 {
            return Err(QafError::InvalidInput(format!(
                "root must have 2–4 letters, got {n}: \"{letters}\""
            )));
        }
        Ok(Self { letters })
    }

    /// Stable ID — `root:كتب`.
    pub fn canonical_id(&self) -> String {
        format!("root:{}", self.letters)
    }
}

impl fmt::Display for Root {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.canonical_id())
    }
}

// ── Lemma ─────────────────────────────────────────────────────────────────────

/// A dictionary headword (مادة / مُعجَم).
///
/// The lemma is the canonical dictionary form of a word, distinct from
/// its inflected occurrences in the text.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Lemma {
    /// Arabic headword, e.g. `"كَتَبَ"`.
    pub text: String,
    /// The trilateral or quadrilateral root, if known.
    pub root: Option<String>,
}

impl Lemma {
    /// Construct a [`Lemma`], validating that `text` is non-empty.
    pub fn new(
        text: impl Into<String>,
        root: Option<impl Into<String>>,
    ) -> Result<Self, QafError> {
        let text = text.into();
        if text.is_empty() {
            return Err(QafError::InvalidInput("lemma text must not be empty".into()));
        }
        Ok(Self { text, root: root.map(Into::into) })
    }

    /// Stable ID — `lemma:كَتَبَ`.
    pub fn canonical_id(&self) -> String {
        format!("lemma:{}", self.text)
    }
}

impl fmt::Display for Lemma {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.canonical_id())
    }
}

// ── PartOfSpeech ──────────────────────────────────────────────────────────────

/// Part of speech (قسم الكلام) in the Arabic grammatical tradition.
///
/// Classical Arabic grammar divides words into three primary categories:
/// verb (فعل), noun (اسم), and particle (حرف).  This enum extends the
/// classical taxonomy with commonly distinguished sub-categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PartOfSpeech {
    /// فعل — action or occurrence, conjugated for tense and person.
    Verb,
    /// اسم — person, place, thing, quality, or abstraction.
    Noun,
    /// حرف — grammatical particle; does not accept inflection independently.
    Particle,
    /// ضمير — pronoun, attached or detached.
    Pronoun,
    /// صفة / نعت — adjective or qualifying attribute.
    Adjective,
    /// ظرف — adverb of time or place.
    Adverb,
}

impl PartOfSpeech {
    /// Short ASCII slug used in the canonical ID.
    pub fn slug(self) -> &'static str {
        match self {
            Self::Verb      => "verb",
            Self::Noun      => "noun",
            Self::Particle  => "particle",
            Self::Pronoun   => "pronoun",
            Self::Adjective => "adjective",
            Self::Adverb    => "adverb",
        }
    }

    /// Stable ID — `pos:verb`.
    pub fn canonical_id(self) -> String {
        format!("pos:{}", self.slug())
    }
}

impl fmt::Display for PartOfSpeech {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.canonical_id())
    }
}

// ── MorphFeature ──────────────────────────────────────────────────────────────

/// A single morphological feature expressed as a key–value pair.
///
/// Examples: `gender → masculine`, `number → plural`, `case → genitive`,
/// `person → third`, `tense → perfect`, `voice → passive`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MorphFeature {
    /// Feature name, e.g. `"gender"`.
    pub key: String,
    /// Feature value, e.g. `"masculine"`.
    pub value: String,
}

impl MorphFeature {
    /// Construct a [`MorphFeature`], validating that neither field is empty.
    pub fn new(
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Result<Self, QafError> {
        let key = key.into();
        let value = value.into();
        if key.is_empty() {
            return Err(QafError::InvalidInput(
                "morph feature key must not be empty".into(),
            ));
        }
        if value.is_empty() {
            return Err(QafError::InvalidInput(
                "morph feature value must not be empty".into(),
            ));
        }
        Ok(Self { key, value })
    }

    /// Stable ID — `feat:gender:masculine`.
    pub fn canonical_id(&self) -> String {
        format!("feat:{}:{}", self.key, self.value)
    }
}

impl fmt::Display for MorphFeature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.canonical_id())
    }
}

// ── SyntaxRole ────────────────────────────────────────────────────────────────

/// Syntactic role (الوظيفة النحوية) of a word within its clause.
///
/// Drawn from classical Arabic naḥw (grammar).  Each variant corresponds
/// to a named function that a word can play in a sentence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyntaxRole {
    /// مبتدأ — subject of a nominal sentence.
    Mubtada,
    /// خبر — predicate of a nominal sentence.
    Khabar,
    /// فاعل — agent / subject of a verbal sentence.
    Fa3il,
    /// نائب الفاعل — subject of a passive verb.
    NaibFa3il,
    /// مفعول به — direct object.
    Maf3ul,
    /// حال — circumstantial qualifier (adverbial of manner/state).
    Hal,
    /// تمييز — specification / tamyīz.
    Tamyiz,
    /// بدل — appositive / substitute.
    Badal,
    /// نعت — adjectival modifier.
    Na3t,
    /// توكيد — corroborative emphasis.
    Tawkid,
    /// عطف — coordinating conjunction member.
    Atf,
}

impl SyntaxRole {
    /// Short ASCII slug used in the canonical ID.
    pub fn slug(self) -> &'static str {
        match self {
            Self::Mubtada   => "mubtada",
            Self::Khabar    => "khabar",
            Self::Fa3il     => "fa3il",
            Self::NaibFa3il => "naib_fa3il",
            Self::Maf3ul    => "maf3ul",
            Self::Hal       => "hal",
            Self::Tamyiz    => "tamyiz",
            Self::Badal     => "badal",
            Self::Na3t      => "na3t",
            Self::Tawkid    => "tawkid",
            Self::Atf       => "atf",
        }
    }

    /// Stable ID — `syn:fa3il`.
    pub fn canonical_id(self) -> String {
        format!("syn:{}", self.slug())
    }
}

impl fmt::Display for SyntaxRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.canonical_id())
    }
}

// ── I3rabRole ─────────────────────────────────────────────────────────────────

/// Iʿrāb role (إعراب) — grammatical case / parsing assignment.
///
/// Iʿrāb describes the inflectional ending of an Arabic word as determined
/// by its syntactic function.  It is the central concept of classical naḥw.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum I3rabRole {
    /// مرفوع — nominative; subject, predicate, and verbal agent.
    Marfu3,
    /// منصوب — accusative; object, ḥāl, tamyīz, and others.
    Mansub,
    /// مجرور — genitive; object of preposition, second term of iḍāfah.
    Majrur,
    /// مجزوم — jussive; applies to verbs in conditional/jussive contexts.
    Majzum,
    /// مبني — indeclinable; ending is fixed regardless of syntactic position.
    Mabni,
}

impl I3rabRole {
    /// Short ASCII slug used in the canonical ID.
    pub fn slug(self) -> &'static str {
        match self {
            Self::Marfu3 => "marfu3",
            Self::Mansub => "mansub",
            Self::Majrur => "majrur",
            Self::Majzum => "majzum",
            Self::Mabni  => "mabni",
        }
    }

    /// Stable ID — `i3rab:marfu3`.
    pub fn canonical_id(self) -> String {
        format!("i3rab:{}", self.slug())
    }
}

impl fmt::Display for I3rabRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.canonical_id())
    }
}

// ── BalaghahFeature ───────────────────────────────────────────────────────────

/// Rhetorical / balāghah feature (بلاغة) of an ayah or passage.
///
/// Balāghah (the science of eloquence) classifies the rhetorical and
/// stylistic devices of the Qur'an.  This enum covers the most widely
/// identified categories in classical balāghah treatises.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BalaghahFeature {
    /// تشبيه — explicit simile ("like" / "as").
    Tashbih,
    /// استعارة — metaphor; implicit comparison without a particle of likeness.
    Isti3arah,
    /// كناية — allusion / metonymy; meaning conveyed indirectly.
    Kinayah,
    /// طباق — antithesis; juxtaposition of opposite terms.
    Tibaq,
    /// مقابلة — extended parallelism of multiple corresponding pairs.
    Muqabalah,
    /// جناس — paronomasia; words similar in sound but different in meaning.
    Jinas,
    /// التفات — shift in grammatical person or address.
    Iltifat,
    /// قصر — restriction; confining an attribute to a single subject.
    Qasr,
}

impl BalaghahFeature {
    /// Short ASCII slug used in the canonical ID.
    pub fn slug(self) -> &'static str {
        match self {
            Self::Tashbih   => "tashbih",
            Self::Isti3arah => "isti3arah",
            Self::Kinayah   => "kinayah",
            Self::Tibaq     => "tibaq",
            Self::Muqabalah => "muqabalah",
            Self::Jinas     => "jinas",
            Self::Iltifat   => "iltifat",
            Self::Qasr      => "qasr",
        }
    }

    /// Stable ID — `balagha:tashbih`.
    pub fn canonical_id(self) -> String {
        format!("balagha:{}", self.slug())
    }
}

impl fmt::Display for BalaghahFeature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.canonical_id())
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Root ──────────────────────────────────────────────────────────────────

    #[test]
    fn root_canonical_id() {
        let r = Root::new("كتب").unwrap();
        assert_eq!(r.canonical_id(), "root:كتب");
        assert_eq!(r.to_string(), "root:كتب");
    }

    #[test]
    fn root_accepts_biliterals_and_quadriliterals() {
        assert!(Root::new("كن").is_ok());    // 2 letters — biliteral
        assert!(Root::new("كتب").is_ok());   // 3 letters — trilateral
        assert!(Root::new("دحرج").is_ok());  // 4 letters — quadrilateral
    }

    #[test]
    fn root_rejects_single_letter() {
        assert!(Root::new("ك").is_err());
    }

    #[test]
    fn root_rejects_five_letters() {
        assert!(Root::new("كتبهم").is_err());
    }

    // ── Lemma ─────────────────────────────────────────────────────────────────

    #[test]
    fn lemma_canonical_id() {
        let l = Lemma::new("كَتَبَ", Some("كتب")).unwrap();
        assert_eq!(l.canonical_id(), "lemma:كَتَبَ");
        assert_eq!(l.to_string(), "lemma:كَتَبَ");
    }

    #[test]
    fn lemma_without_root() {
        let l = Lemma::new("إِنَّ", None::<String>).unwrap();
        assert!(l.root.is_none());
    }

    #[test]
    fn lemma_rejects_empty_text() {
        assert!(Lemma::new("", None::<String>).is_err());
    }

    // ── PartOfSpeech ──────────────────────────────────────────────────────────

    #[test]
    fn pos_canonical_ids() {
        assert_eq!(PartOfSpeech::Verb.canonical_id(),      "pos:verb");
        assert_eq!(PartOfSpeech::Noun.canonical_id(),      "pos:noun");
        assert_eq!(PartOfSpeech::Particle.canonical_id(),  "pos:particle");
        assert_eq!(PartOfSpeech::Pronoun.canonical_id(),   "pos:pronoun");
        assert_eq!(PartOfSpeech::Adjective.canonical_id(), "pos:adjective");
        assert_eq!(PartOfSpeech::Adverb.canonical_id(),    "pos:adverb");
    }

    #[test]
    fn pos_display() {
        assert_eq!(PartOfSpeech::Verb.to_string(), "pos:verb");
    }

    // ── MorphFeature ──────────────────────────────────────────────────────────

    #[test]
    fn morph_feature_canonical_id() {
        let f = MorphFeature::new("gender", "masculine").unwrap();
        assert_eq!(f.canonical_id(), "feat:gender:masculine");
        assert_eq!(f.to_string(), "feat:gender:masculine");
    }

    #[test]
    fn morph_feature_rejects_empty_key() {
        assert!(MorphFeature::new("", "masculine").is_err());
    }

    #[test]
    fn morph_feature_rejects_empty_value() {
        assert!(MorphFeature::new("gender", "").is_err());
    }

    // ── SyntaxRole ────────────────────────────────────────────────────────────

    #[test]
    fn syntax_role_canonical_ids() {
        assert_eq!(SyntaxRole::Mubtada.canonical_id(),   "syn:mubtada");
        assert_eq!(SyntaxRole::Khabar.canonical_id(),    "syn:khabar");
        assert_eq!(SyntaxRole::Fa3il.canonical_id(),     "syn:fa3il");
        assert_eq!(SyntaxRole::NaibFa3il.canonical_id(), "syn:naib_fa3il");
        assert_eq!(SyntaxRole::Maf3ul.canonical_id(),    "syn:maf3ul");
        assert_eq!(SyntaxRole::Hal.canonical_id(),       "syn:hal");
        assert_eq!(SyntaxRole::Tamyiz.canonical_id(),    "syn:tamyiz");
        assert_eq!(SyntaxRole::Badal.canonical_id(),     "syn:badal");
        assert_eq!(SyntaxRole::Na3t.canonical_id(),      "syn:na3t");
        assert_eq!(SyntaxRole::Tawkid.canonical_id(),    "syn:tawkid");
        assert_eq!(SyntaxRole::Atf.canonical_id(),       "syn:atf");
    }

    // ── I3rabRole ─────────────────────────────────────────────────────────────

    #[test]
    fn i3rab_role_canonical_ids() {
        assert_eq!(I3rabRole::Marfu3.canonical_id(), "i3rab:marfu3");
        assert_eq!(I3rabRole::Mansub.canonical_id(), "i3rab:mansub");
        assert_eq!(I3rabRole::Majrur.canonical_id(), "i3rab:majrur");
        assert_eq!(I3rabRole::Majzum.canonical_id(), "i3rab:majzum");
        assert_eq!(I3rabRole::Mabni.canonical_id(),  "i3rab:mabni");
    }

    // ── BalaghahFeature ───────────────────────────────────────────────────────

    #[test]
    fn balagha_feature_canonical_ids() {
        assert_eq!(BalaghahFeature::Tashbih.canonical_id(),   "balagha:tashbih");
        assert_eq!(BalaghahFeature::Isti3arah.canonical_id(), "balagha:isti3arah");
        assert_eq!(BalaghahFeature::Kinayah.canonical_id(),   "balagha:kinayah");
        assert_eq!(BalaghahFeature::Tibaq.canonical_id(),     "balagha:tibaq");
        assert_eq!(BalaghahFeature::Muqabalah.canonical_id(), "balagha:muqabalah");
        assert_eq!(BalaghahFeature::Jinas.canonical_id(),     "balagha:jinas");
        assert_eq!(BalaghahFeature::Iltifat.canonical_id(),   "balagha:iltifat");
        assert_eq!(BalaghahFeature::Qasr.canonical_id(),      "balagha:qasr");
    }

    #[test]
    fn balagha_feature_display() {
        assert_eq!(BalaghahFeature::Tashbih.to_string(), "balagha:tashbih");
    }
}
