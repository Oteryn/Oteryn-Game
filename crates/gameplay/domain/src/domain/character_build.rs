//! Pure Character build bounds, formula inputs and progress arithmetic.

/// Highest magic level or skill level stored (the 0030 CHECK).
pub const MAX_BUILD_LEVEL: u16 = 1000;
/// Lowest skill level, and the seed of every skill (SKILLS-0 §3.1).
pub const MIN_SKILL_LEVEL: u16 = 10;
/// Most `mana_spent` or tries stored: 2^63 - 1 (SKILLS-0 §3.1).
pub const MAX_BUILD_PROGRESS: u64 = i64::MAX.unsigned_abs();
/// The vocation of a Character that has not chosen one (A13 §4.1).
pub const NO_VOCATION: &str = "none";
/// The seven skills in the column order of 0030.
pub const SKILLS: [&str; 7] = [
    "fist",
    "club",
    "sword",
    "axe",
    "distance",
    "shielding",
    "fishing",
];

/// The content formula table (SKILLS-0 §3.5) the writer checks a change against: `req(L)` per
/// vocation and family, and the digest of the table revision, which the receipt binds as its
/// policy digest.
pub trait BuildFormula: Send + Sync {
    /// Progress needed to go from `level - 1` to `level` in `family` (0 is magic level, 1..=7 the
    /// skills in the order of [`SKILLS`]) under `vocation`. `None` means unreachable.
    fn required(&self, vocation: &str, family: usize, level: u16) -> Option<u64>;
    fn digest(&self) -> [u8; 32];
    /// The content revision this table belongs to. The writer refuses a table whose revision is
    /// not the Character's stored content revision.
    fn content_revision(&self) -> &str;
}

/// Supplied build values and reconstruction, independent of the storage adapter.
pub trait DeathBuildState: Sized {
    fn vocation(&self) -> &str;
    fn magic(&self) -> (u16, u64);
    fn skills(&self) -> [(u16, u64); 7];
    fn from_death_families(
        vocation: &str,
        magic: (u16, u64),
        skills: [(u16, u64); 7],
    ) -> Option<Self>;
}

/// `req(L)`, the progress needed to go from level `L - 1` to `L`: `base x multiplier^(L - 11)`
/// in f64, truncated to an integer, as Canary computes it (SKILLS-0 §2, §3.1). `None` for a level
/// outside 11..=1000, a multiplier that is not finite and positive, or a requirement above
/// [`MAX_BUILD_PROGRESS`]: that level is unreachable.
#[must_use]
pub fn skill_tries_required(base: u16, multiplier: f64, level: u16) -> Option<u64> {
    if !(MIN_SKILL_LEVEL + 1..=MAX_BUILD_LEVEL).contains(&level)
        || !multiplier.is_finite()
        || multiplier <= 0.0
    {
        return None;
    }
    let required = f64::from(base) * multiplier.powf(f64::from(level - (MIN_SKILL_LEVEL + 1)));
    // Below 2^63 every f64 truncates into 0..=2^63 - 1024.
    (required.is_finite() && required < 9_223_372_036_854_775_808.0).then_some(required as u64)
}

/// A family's cumulative progress from its `floor` level: `req` of every level from `floor + 1`
/// to `level`, plus `progress`. Summed in checked 128-bit arithmetic, saturating at
/// [`MAX_BUILD_PROGRESS`]; an unreachable level on the way also saturates.
#[must_use]
pub fn cumulative_progress(
    level: u16,
    progress: u64,
    floor: u16,
    req: impl Fn(u16) -> Option<u64>,
) -> u64 {
    let total = (floor.saturating_add(1)..=level).try_fold(u128::from(progress), |total, level| {
        total.checked_add(u128::from(req(level)?))
    });
    total.map_or(MAX_BUILD_PROGRESS, |total| {
        u64::try_from(total.min(u128::from(MAX_BUILD_PROGRESS))).unwrap_or(MAX_BUILD_PROGRESS)
    })
}

/// Re-level cumulative progress from (`floor`, 0) (SKILLS-0 §3.3, §3.6): advance while the next
/// level is reachable and paid for, up to [`MAX_BUILD_LEVEL`]; the rest is progress.
#[must_use]
pub fn relevel(cumulative: u64, floor: u16, req: impl Fn(u16) -> Option<u64>) -> (u16, u64) {
    let (mut level, mut rest) = (floor, cumulative.min(MAX_BUILD_PROGRESS));
    while level < MAX_BUILD_LEVEL {
        match req(level + 1) {
            Some(required) if rest >= required => {
                rest -= required;
                level += 1;
            }
            _ => break,
        }
    }
    (level, rest)
}
