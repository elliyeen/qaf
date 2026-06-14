//! seed-structure — populate juz, surahs, and ayahs from the existing words table.
//!
//! Run after `quran-import` has loaded words/morphology.
//!
//! Usage:
//!   seed-structure
//!   seed-structure --db sqlite:/path/to/qaf.db
//!   seed-structure --reset   # truncate juz/surahs/ayahs before seeding

use anyhow::{Context, Result};
use clap::Parser;
use quran_db::{connect, run_migrations};
use sqlx::SqlitePool;
use tracing::info;

// ─── Canonical surah metadata ─────────────────────────────────────────────────
// (id, name_ar, name_en, name_en_meaning, revelation_type, ayah_count)

const SURAHS: &[(i32, &str, &str, &str, &str, i32)] = &[
    (1,   "الفاتحة",      "Al-Fatihah",      "The Opening",                   "makki",  7),
    (2,   "البقرة",       "Al-Baqarah",      "The Cow",                       "madani", 286),
    (3,   "آل عمران",     "Ali 'Imran",      "Family of Imran",               "madani", 200),
    (4,   "النساء",       "An-Nisa",         "The Women",                     "madani", 176),
    (5,   "المائدة",      "Al-Ma'idah",      "The Table Spread",              "madani", 120),
    (6,   "الأنعام",      "Al-An'am",        "The Cattle",                    "makki",  165),
    (7,   "الأعراف",      "Al-A'raf",        "The Heights",                   "makki",  206),
    (8,   "الأنفال",      "Al-Anfal",        "The Spoils of War",             "madani", 75),
    (9,   "التوبة",       "At-Tawbah",       "The Repentance",                "madani", 129),
    (10,  "يونس",         "Yunus",           "Jonah",                         "makki",  109),
    (11,  "هود",          "Hud",             "Hud",                           "makki",  123),
    (12,  "يوسف",         "Yusuf",           "Joseph",                        "makki",  111),
    (13,  "الرعد",        "Ar-Ra'd",         "The Thunder",                   "madani", 43),
    (14,  "إبراهيم",      "Ibrahim",         "Abraham",                       "makki",  52),
    (15,  "الحجر",        "Al-Hijr",         "The Rocky Tract",               "makki",  99),
    (16,  "النحل",        "An-Nahl",         "The Bee",                       "makki",  128),
    (17,  "الإسراء",      "Al-Isra",         "The Night Journey",             "makki",  111),
    (18,  "الكهف",        "Al-Kahf",         "The Cave",                      "makki",  110),
    (19,  "مريم",         "Maryam",          "Mary",                          "makki",  98),
    (20,  "طه",           "Ta-Ha",           "Ta-Ha",                         "makki",  135),
    (21,  "الأنبياء",     "Al-Anbiya",       "The Prophets",                  "makki",  112),
    (22,  "الحج",         "Al-Hajj",         "The Pilgrimage",                "madani", 78),
    (23,  "المؤمنون",     "Al-Mu'minun",     "The Believers",                 "makki",  118),
    (24,  "النور",        "An-Nur",          "The Light",                     "madani", 64),
    (25,  "الفرقان",      "Al-Furqan",       "The Criterion",                 "makki",  77),
    (26,  "الشعراء",      "Ash-Shu'ara",     "The Poets",                     "makki",  227),
    (27,  "النمل",        "An-Naml",         "The Ant",                       "makki",  93),
    (28,  "القصص",        "Al-Qasas",        "The Stories",                   "makki",  88),
    (29,  "العنكبوت",     "Al-'Ankabut",     "The Spider",                    "makki",  69),
    (30,  "الروم",        "Ar-Rum",          "The Romans",                    "makki",  60),
    (31,  "لقمان",        "Luqman",          "Luqman",                        "makki",  34),
    (32,  "السجدة",       "As-Sajdah",       "The Prostration",               "makki",  30),
    (33,  "الأحزاب",      "Al-Ahzab",        "The Combined Forces",           "madani", 73),
    (34,  "سبأ",          "Saba",            "Sheba",                         "makki",  54),
    (35,  "فاطر",         "Fatir",           "Originator",                    "makki",  45),
    (36,  "يس",           "Ya-Sin",          "Ya Sin",                        "makki",  83),
    (37,  "الصافات",      "As-Saffat",       "Those Who Set the Ranks",       "makki",  182),
    (38,  "ص",            "Sad",             "The Letter Sad",                "makki",  88),
    (39,  "الزمر",        "Az-Zumar",        "The Troops",                    "makki",  75),
    (40,  "غافر",         "Ghafir",          "The Forgiver",                  "makki",  85),
    (41,  "فصلت",         "Fussilat",        "Explained in Detail",           "makki",  54),
    (42,  "الشورى",       "Ash-Shuraa",      "The Consultation",              "makki",  53),
    (43,  "الزخرف",       "Az-Zukhruf",      "The Ornaments of Gold",         "makki",  89),
    (44,  "الدخان",       "Ad-Dukhan",       "The Smoke",                     "makki",  59),
    (45,  "الجاثية",      "Al-Jathiyah",     "The Crouching",                 "makki",  37),
    (46,  "الأحقاف",      "Al-Ahqaf",        "The Wind-Curved Sandhills",     "makki",  35),
    (47,  "محمد",         "Muhammad",        "Muhammad",                      "madani", 38),
    (48,  "الفتح",        "Al-Fath",         "The Victory",                   "madani", 29),
    (49,  "الحجرات",      "Al-Hujurat",      "The Rooms",                     "madani", 18),
    (50,  "ق",            "Qaf",             "The Letter Qaf",                "makki",  45),
    (51,  "الذاريات",     "Adh-Dhariyat",    "The Winnowing Winds",           "makki",  60),
    (52,  "الطور",        "At-Tur",          "The Mount",                     "makki",  49),
    (53,  "النجم",        "An-Najm",         "The Star",                      "makki",  62),
    (54,  "القمر",        "Al-Qamar",        "The Moon",                      "makki",  55),
    (55,  "الرحمن",       "Ar-Rahman",       "The Most Merciful",             "madani", 78),
    (56,  "الواقعة",      "Al-Waqi'ah",      "The Inevitable",                "makki",  96),
    (57,  "الحديد",       "Al-Hadid",        "The Iron",                      "madani", 29),
    (58,  "المجادلة",     "Al-Mujadila",     "The Pleading Woman",            "madani", 22),
    (59,  "الحشر",        "Al-Hashr",        "The Exile",                     "madani", 24),
    (60,  "الممتحنة",     "Al-Mumtahanah",   "She That is to be Examined",    "madani", 13),
    (61,  "الصف",         "As-Saf",          "The Ranks",                     "madani", 14),
    (62,  "الجمعة",       "Al-Jumu'ah",      "The Friday Congregation",       "madani", 11),
    (63,  "المنافقون",    "Al-Munafiqun",    "The Hypocrites",                "madani", 11),
    (64,  "التغابن",      "At-Taghabun",     "The Mutual Disillusion",        "madani", 18),
    (65,  "الطلاق",       "At-Talaq",        "The Divorce",                   "madani", 12),
    (66,  "التحريم",      "At-Tahrim",       "The Prohibition",               "madani", 12),
    (67,  "الملك",        "Al-Mulk",         "The Sovereignty",               "makki",  30),
    (68,  "القلم",        "Al-Qalam",        "The Pen",                       "makki",  52),
    (69,  "الحاقة",       "Al-Haqqah",       "The Reality",                   "makki",  52),
    (70,  "المعارج",      "Al-Ma'arij",      "The Ascending Stairways",       "makki",  44),
    (71,  "نوح",          "Nuh",             "Noah",                          "makki",  28),
    (72,  "الجن",         "Al-Jinn",         "The Jinn",                      "makki",  28),
    (73,  "المزمل",       "Al-Muzzammil",    "The Enshrouded One",            "makki",  20),
    (74,  "المدثر",       "Al-Muddaththir",  "The Cloaked One",               "makki",  56),
    (75,  "القيامة",      "Al-Qiyamah",      "The Resurrection",              "makki",  40),
    (76,  "الإنسان",      "Al-Insan",        "The Human",                     "madani", 31),
    (77,  "المرسلات",     "Al-Mursalat",     "The Emissaries",                "makki",  50),
    (78,  "النبأ",        "An-Naba",         "The Announcement",              "makki",  40),
    (79,  "النازعات",     "An-Nazi'at",      "Those Who Drag Forth",          "makki",  46),
    (80,  "عبس",          "'Abasa",          "He Frowned",                    "makki",  42),
    (81,  "التكوير",      "At-Takwir",       "The Overthrowing",              "makki",  29),
    (82,  "الانفطار",     "Al-Infitar",      "The Cleaving",                  "makki",  19),
    (83,  "المطففين",     "Al-Mutaffifin",   "The Defrauding",                "makki",  36),
    (84,  "الانشقاق",     "Al-Inshiqaq",     "The Sundering",                 "makki",  25),
    (85,  "البروج",       "Al-Buruj",        "The Mansions of the Stars",     "makki",  22),
    (86,  "الطارق",       "At-Tariq",        "The Nightcomer",                "makki",  17),
    (87,  "الأعلى",       "Al-A'la",         "The Most High",                 "makki",  19),
    (88,  "الغاشية",      "Al-Ghashiyah",    "The Overwhelming",              "makki",  26),
    (89,  "الفجر",        "Al-Fajr",         "The Dawn",                      "makki",  30),
    (90,  "البلد",        "Al-Balad",        "The City",                      "makki",  20),
    (91,  "الشمس",        "Ash-Shams",       "The Sun",                       "makki",  15),
    (92,  "الليل",        "Al-Layl",         "The Night",                     "makki",  21),
    (93,  "الضحى",        "Ad-Duha",         "The Morning Hours",             "makki",  11),
    (94,  "الشرح",        "Ash-Sharh",       "The Relief",                    "makki",  8),
    (95,  "التين",        "At-Tin",          "The Fig",                       "makki",  8),
    (96,  "العلق",        "Al-'Alaq",        "The Clot",                      "makki",  19),
    (97,  "القدر",        "Al-Qadr",         "The Power",                     "makki",  5),
    (98,  "البينة",       "Al-Bayyinah",     "The Clear Proof",               "madani", 8),
    (99,  "الزلزلة",      "Az-Zalzalah",     "The Earthquake",                "madani", 8),
    (100, "العاديات",     "Al-'Adiyat",      "The Courser",                   "makki",  11),
    (101, "القارعة",      "Al-Qari'ah",      "The Calamity",                  "makki",  11),
    (102, "التكاثر",      "At-Takathur",     "The Rivalry in World Increase", "makki",  8),
    (103, "العصر",        "Al-'Asr",         "The Declining Day",             "makki",  3),
    (104, "الهمزة",       "Al-Humazah",      "The Traducer",                  "makki",  9),
    (105, "الفيل",        "Al-Fil",          "The Elephant",                  "makki",  5),
    (106, "قريش",         "Quraysh",         "Quraysh",                       "makki",  4),
    (107, "الماعون",      "Al-Ma'un",        "The Small Kindnesses",          "makki",  7),
    (108, "الكوثر",       "Al-Kawthar",      "The Abundance",                 "makki",  3),
    (109, "الكافرون",     "Al-Kafirun",      "The Disbelievers",              "makki",  6),
    (110, "النصر",        "An-Nasr",         "The Divine Support",            "madani", 3),
    (111, "المسد",        "Al-Masad",        "The Palm Fiber",                "makki",  5),
    (112, "الإخلاص",      "Al-Ikhlas",       "The Sincerity",                 "makki",  4),
    (113, "الفلق",        "Al-Falaq",        "The Daybreak",                  "makki",  5),
    (114, "الناس",        "An-Nas",          "Mankind",                       "makki",  6),
];

// ─── Canonical juz names ──────────────────────────────────────────────────────

const JUZ_NAMES: &[&str] = &[
    "الجزء الأول",    "الجزء الثاني",   "الجزء الثالث",  "الجزء الرابع",
    "الجزء الخامس",   "الجزء السادس",   "الجزء السابع",  "الجزء الثامن",
    "الجزء التاسع",   "الجزء العاشر",   "الجزء الحادي عشر", "الجزء الثاني عشر",
    "الجزء الثالث عشر", "الجزء الرابع عشر", "الجزء الخامس عشر", "الجزء السادس عشر",
    "الجزء السابع عشر", "الجزء الثامن عشر", "الجزء التاسع عشر", "الجزء العشرون",
    "الجزء الحادي والعشرون", "الجزء الثاني والعشرون", "الجزء الثالث والعشرون",
    "الجزء الرابع والعشرون", "الجزء الخامس والعشرون", "الجزء السادس والعشرون",
    "الجزء السابع والعشرون", "الجزء الثامن والعشرون", "الجزء التاسع والعشرون",
    "الجزء الثلاثون",
];

// ─── CLI ──────────────────────────────────────────────────────────────────────

#[derive(Parser, Debug)]
#[command(
    name = "seed-structure",
    about = "Populate juz, surahs, and ayahs from the existing words table"
)]
struct Args {
    /// SQLite database URL. Defaults to DATABASE_URL env var or sqlite:qaf.db.
    #[arg(short, long, value_name = "URL")]
    db: Option<String>,

    /// Truncate juz, surahs, and ayahs before seeding.
    #[arg(long)]
    reset: bool,
}

// ─── Entry point ──────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "seed_structure=info,warn".into()),
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

    if args.reset {
        tracing::warn!("--reset: truncating ayahs, surahs, juz");
        sqlx::query("DELETE FROM ayahs").execute(&pool).await?;
        sqlx::query("DELETE FROM surahs").execute(&pool).await?;
        sqlx::query("DELETE FROM juz").execute(&pool).await?;
    }

    let juz_inserted = seed_juz(&pool).await?;
    let surahs_inserted = seed_surahs(&pool).await?;
    let ayahs_inserted = seed_ayahs(&pool).await?;

    info!(
        "done — juz: {} | surahs: {} | ayahs: {}",
        juz_inserted, surahs_inserted, ayahs_inserted
    );
    println!(
        "\n  Juz inserted    : {}\n  Surahs inserted : {}\n  Ayahs inserted  : {}\n",
        juz_inserted, surahs_inserted, ayahs_inserted
    );

    Ok(())
}

// ─── Seeders ─────────────────────────────────────────────────────────────────

async fn seed_juz(pool: &SqlitePool) -> Result<u64> {
    let mut inserted: u64 = 0;
    let mut tx = pool.begin().await?;

    for (i, name_ar) in JUZ_NAMES.iter().enumerate() {
        let id = (i + 1) as i32;
        let r = sqlx::query("INSERT OR IGNORE INTO juz (id, name_ar) VALUES (?, ?)")
            .bind(id)
            .bind(name_ar)
            .execute(&mut *tx)
            .await
            .with_context(|| format!("insert juz {}", id))?;
        inserted += r.rows_affected();
    }

    tx.commit().await?;
    Ok(inserted)
}

async fn seed_surahs(pool: &SqlitePool) -> Result<u64> {
    let mut inserted: u64 = 0;
    let mut tx = pool.begin().await?;

    for &(id, name_ar, name_en, name_en_meaning, revelation_type, ayah_count) in SURAHS {
        let r = sqlx::query(
            "INSERT OR IGNORE INTO surahs
             (id, name_ar, name_en, name_en_meaning, revelation_type, ayah_count)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(name_ar)
        .bind(name_en)
        .bind(name_en_meaning)
        .bind(revelation_type)
        .bind(ayah_count)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("insert surah {}", id))?;
        inserted += r.rows_affected();
    }

    tx.commit().await?;
    Ok(inserted)
}

/// Derive all (surah, ayah) pairs from the words table.
///
/// For each pair, words are joined in position order with a space to produce
/// a working `text_uthmani`.  page_id and juz_id are left NULL — those require
/// the full Madinah Mushaf layout data which is imported separately.
async fn seed_ayahs(pool: &SqlitePool) -> Result<u64> {
    // Fetch all distinct (surah, ayah) pairs.
    let pairs: Vec<(i32, i32)> = sqlx::query_as(
        "SELECT DISTINCT surah, ayah FROM words ORDER BY surah, ayah",
    )
    .fetch_all(pool)
    .await
    .context("fetch distinct (surah, ayah) pairs")?;

    info!("{} distinct ayahs to seed", pairs.len());

    let mut inserted: u64 = 0;
    let mut tx = pool.begin().await?;

    for (surah, ayah) in pairs {
        // Concatenate words in position order → text_uthmani.
        let words: Vec<String> = sqlx::query_scalar(
            "SELECT arabic FROM words WHERE surah = ? AND ayah = ? ORDER BY position",
        )
        .bind(surah)
        .bind(ayah)
        .fetch_all(&mut *tx)
        .await
        .with_context(|| format!("fetch words for {}:{}", surah, ayah))?;

        let text_uthmani = words.join(" ");

        let r = sqlx::query(
            "INSERT OR IGNORE INTO ayahs (surah_id, ayah_number, text_uthmani)
             VALUES (?, ?, ?)",
        )
        .bind(surah)
        .bind(ayah)
        .bind(&text_uthmani)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("insert ayah {}:{}", surah, ayah))?;

        inserted += r.rows_affected();

        // Checkpoint every 500 ayahs.
        if inserted % 500 == 0 && inserted > 0 {
            tx.commit().await?;
            tx = pool.begin().await?;
        }
    }

    tx.commit().await?;
    Ok(inserted)
}
