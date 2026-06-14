//! Canonical structural models for the Qur'an.
//!
//! These four types — [`Surah`], [`Ayah`], [`Page`], [`Juz`] — represent the
//! primary divisions of the muṣḥaf.  Each carries a deterministic
//! `canonical_id()` that can be used as a stable key across storage layers.
//!
//! # ID formats
//!
//! | Type   | Format            | Example       |
//! |--------|-------------------|---------------|
//! | Surah  | `surah:NNN`       | `surah:060`   |
//! | Ayah   | `ayah:NNN:NNN`    | `ayah:060:012`|
//! | Page   | `page:NNN`        | `page:550`    |
//! | Juz    | `juz:NNN`         | `juz:028`     |

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::error::QafError;

// ── Surah ─────────────────────────────────────────────────────────────────────

/// One of the 114 chapters of the Qur'an.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Surah {
    /// Chapter number, 1–114.
    pub number: u8,
    /// Arabic name, e.g. `"الفاتحة"`.
    pub name_ar: String,
    /// English transliteration, e.g. `"Al-Fatihah"`.
    pub name_en: String,
    /// Total number of ayahs in this chapter.
    pub ayah_count: u16,
    /// Revelation period: `"Meccan"` or `"Medinan"`.
    pub revelation: String,
}

impl Surah {
    /// Construct a [`Surah`], validating that `number` is in `1..=114`.
    pub fn new(
        number: u8,
        name_ar: impl Into<String>,
        name_en: impl Into<String>,
        ayah_count: u16,
        revelation: impl Into<String>,
    ) -> Result<Self, QafError> {
        if number == 0 || number > 114 {
            return Err(QafError::InvalidInput(format!(
                "surah number must be 1–114, got {number}"
            )));
        }
        if ayah_count == 0 {
            return Err(QafError::InvalidInput(
                "ayah_count must be ≥ 1".into(),
            ));
        }
        Ok(Self {
            number,
            name_ar: name_ar.into(),
            name_en: name_en.into(),
            ayah_count,
            revelation: revelation.into(),
        })
    }

    /// Stable, zero-padded ID — `surah:060`.
    pub fn canonical_id(&self) -> String {
        format!("surah:{:03}", self.number)
    }
}

impl fmt::Display for Surah {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.canonical_id())
    }
}

// ── Ayah ──────────────────────────────────────────────────────────────────────

/// A single verse within a surah.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Ayah {
    /// Surah number, 1–114.
    pub surah: u8,
    /// Ayah number within the surah (1-based).
    pub number: u16,
    /// Muṣḥaf page number, 1–604.
    pub page: u16,
    /// Juzʾ number, 1–30.
    pub juz: u8,
}

impl Ayah {
    /// Construct an [`Ayah`], validating all fields.
    pub fn new(surah: u8, number: u16, page: u16, juz: u8) -> Result<Self, QafError> {
        if surah == 0 || surah > 114 {
            return Err(QafError::InvalidInput(format!(
                "surah number must be 1–114, got {surah}"
            )));
        }
        if number == 0 {
            return Err(QafError::InvalidInput("ayah number must be ≥ 1".into()));
        }
        if page == 0 || page > 604 {
            return Err(QafError::InvalidInput(format!(
                "page must be 1–604, got {page}"
            )));
        }
        if juz == 0 || juz > 30 {
            return Err(QafError::InvalidInput(format!(
                "juz must be 1–30, got {juz}"
            )));
        }
        Ok(Self { surah, number, page, juz })
    }

    /// Stable, zero-padded ID — `ayah:060:012`.
    pub fn canonical_id(&self) -> String {
        format!("ayah:{:03}:{:03}", self.surah, self.number)
    }
}

impl fmt::Display for Ayah {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.canonical_id())
    }
}

// ── Page ──────────────────────────────────────────────────────────────────────

/// A single page of the standard 604-page muṣḥaf.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Page {
    /// Page number, 1–604.
    pub number: u16,
}

impl Page {
    /// Construct a [`Page`], validating that `number` is in `1..=604`.
    pub fn new(number: u16) -> Result<Self, QafError> {
        if number == 0 || number > 604 {
            return Err(QafError::InvalidInput(format!(
                "page number must be 1–604, got {number}"
            )));
        }
        Ok(Self { number })
    }

    /// Stable, zero-padded ID — `page:550`.
    pub fn canonical_id(&self) -> String {
        format!("page:{:03}", self.number)
    }
}

impl fmt::Display for Page {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.canonical_id())
    }
}

// ── Juz ───────────────────────────────────────────────────────────────────────

/// One of the 30 equal portions of the Qur'an.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Juz {
    /// Juzʾ number, 1–30.
    pub number: u8,
}

impl Juz {
    /// Construct a [`Juz`], validating that `number` is in `1..=30`.
    pub fn new(number: u8) -> Result<Self, QafError> {
        if number == 0 || number > 30 {
            return Err(QafError::InvalidInput(format!(
                "juz number must be 1–30, got {number}"
            )));
        }
        Ok(Self { number })
    }

    /// Stable, zero-padded ID — `juz:028`.
    pub fn canonical_id(&self) -> String {
        format!("juz:{:03}", self.number)
    }
}

impl fmt::Display for Juz {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.canonical_id())
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Surah ─────────────────────────────────────────────────────────────────

    #[test]
    fn surah_canonical_id() {
        let s = Surah::new(60, "الممتحنة", "Al-Mumtahanah", 13, "Medinan").unwrap();
        assert_eq!(s.canonical_id(), "surah:060");
        assert_eq!(s.to_string(), "surah:060");
    }

    #[test]
    fn surah_boundary_values() {
        assert!(Surah::new(1, "الفاتحة", "Al-Fatihah", 7, "Meccan").is_ok());
        assert!(Surah::new(114, "الناس", "An-Nas", 6, "Meccan").is_ok());
    }

    #[test]
    fn surah_rejects_zero() {
        assert!(Surah::new(0, "", "", 1, "").is_err());
    }

    #[test]
    fn surah_rejects_out_of_range() {
        assert!(Surah::new(115, "", "", 1, "").is_err());
    }

    // ── Ayah ──────────────────────────────────────────────────────────────────

    #[test]
    fn ayah_canonical_id() {
        let a = Ayah::new(60, 12, 550, 28).unwrap();
        assert_eq!(a.canonical_id(), "ayah:060:012");
        assert_eq!(a.to_string(), "ayah:060:012");
    }

    #[test]
    fn ayah_stores_page_and_juz() {
        let a = Ayah::new(2, 255, 42, 3).unwrap();
        assert_eq!(a.page, 42);
        assert_eq!(a.juz, 3);
    }

    #[test]
    fn ayah_rejects_invalid_surah() {
        assert!(Ayah::new(0, 1, 1, 1).is_err());
        assert!(Ayah::new(115, 1, 1, 1).is_err());
    }

    #[test]
    fn ayah_rejects_invalid_page() {
        assert!(Ayah::new(1, 1, 0, 1).is_err());
        assert!(Ayah::new(1, 1, 605, 1).is_err());
    }

    #[test]
    fn ayah_rejects_invalid_juz() {
        assert!(Ayah::new(1, 1, 1, 0).is_err());
        assert!(Ayah::new(1, 1, 1, 31).is_err());
    }

    // ── Page ──────────────────────────────────────────────────────────────────

    #[test]
    fn page_canonical_id() {
        let p = Page::new(550).unwrap();
        assert_eq!(p.canonical_id(), "page:550");
        assert_eq!(p.to_string(), "page:550");
    }

    #[test]
    fn page_low_number_pads() {
        let p = Page::new(1).unwrap();
        assert_eq!(p.canonical_id(), "page:001");
    }

    #[test]
    fn page_boundary() {
        assert!(Page::new(1).is_ok());
        assert!(Page::new(604).is_ok());
    }

    #[test]
    fn page_rejects_out_of_range() {
        assert!(Page::new(0).is_err());
        assert!(Page::new(605).is_err());
    }

    // ── Juz ───────────────────────────────────────────────────────────────────

    #[test]
    fn juz_canonical_id() {
        let j = Juz::new(28).unwrap();
        assert_eq!(j.canonical_id(), "juz:028");
        assert_eq!(j.to_string(), "juz:028");
    }

    #[test]
    fn juz_boundary_values() {
        assert!(Juz::new(1).is_ok());
        assert!(Juz::new(30).is_ok());
    }

    #[test]
    fn juz_rejects_out_of_range() {
        assert!(Juz::new(0).is_err());
        assert!(Juz::new(31).is_err());
    }
}
