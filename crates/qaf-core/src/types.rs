use serde::{Deserialize, Serialize};

/// Identifies a word by its position in the Quran.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WordRef {
    pub surah: u16,
    pub ayah: u16,
    pub position: u16,
}

/// Identifies an ayah by surah and ayah number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AyahRef {
    pub surah: u16,
    pub ayah: u16,
}
