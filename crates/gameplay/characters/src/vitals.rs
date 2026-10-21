//! Vocation health, mana and soul maxima from the accepted SPELL-D5 formulas.

use serde_json::Value;

/// The accepted SPELL-D5 input, including its exact wiki and source revision provenance.
const VITALS: &str = include_str!(
    "../../../../tools/content-schema/spell-authoring/samples/vocation-vitals-candidate-2026-09-28.json"
);

/// Compute maxima within the bounds supplied by the protocol adapter.
pub fn maxima(
    vocation: &str,
    level: u32,
    max_vital_pool: u32,
    max_soul: u32,
) -> Option<(u32, u32, u32)> {
    let table: Value = serde_json::from_str(VITALS).ok()?;
    if table.get("schema")?.as_str()? != "OTERYN_VOCATION_VITALS_CANDIDATE/v1" || level == 0 {
        return None;
    }
    let profile = table.get("vocations")?.get(vocation)?;
    let formula = if u64::from(level) < profile.get("rookie_until_level")?.as_u64()? {
        "rookie_total"
    } else {
        "total"
    };
    let pool = |key: &str| {
        let curve = profile.get(key)?.get(formula)?;
        let factor = curve.get("level_factor")?.as_i64()?;
        let offset = curve.get("offset")?.as_i64()?;
        if factor < 0 {
            return None;
        }
        let value = factor.checked_mul(i64::from(level))?.checked_add(offset)?;
        let value = u32::try_from(value).ok()?;
        (value <= max_vital_pool).then_some(value)
    };
    // SPELL-D5 explicitly keeps the free-account maximum for V1. A promotion is not an
    // entitlement and cannot activate Premium or select the premium_account value.
    let soul = u32::try_from(
        profile
            .get("soul")?
            .get("max")?
            .get("free_account")?
            .as_u64()?,
    )
    .ok()?;
    if soul > max_soul {
        return None;
    }
    Some((pool("hitpoints")?, pool("mana")?, soul))
}

#[cfg(test)]
mod tests {
    fn maxima(vocation: &str, level: u32) -> Option<(u32, u32, u32)> {
        super::maxima(vocation, level, (1 << 28) - 1, 16_383)
    }

    #[test]
    fn accepted_maxima_keep_rookie_boundary_promotions_and_v1_soul() {
        for vocation in [
            "druid",
            "elder_druid",
            "sorcerer",
            "master_sorcerer",
            "knight",
            "elite_knight",
            "paladin",
            "royal_paladin",
            "monk",
            "exalted_monk",
            "none",
        ] {
            assert_eq!(maxima(vocation, 1), Some((150, 55, 100)));
            assert_eq!(maxima(vocation, 7), Some((180, 85, 100)));
            assert_eq!(maxima(vocation, 8), Some((185, 90, 100)));
        }
        assert_eq!(maxima("knight", 20), Some((365, 150, 100)));
        assert_eq!(maxima("elite_knight", 20), Some((365, 150, 100)));
        assert_eq!(maxima("paladin", 20), Some((305, 270, 100)));
        assert_eq!(maxima("royal_paladin", 20), Some((305, 270, 100)));
        assert_eq!(maxima("monk", 20), Some((305, 210, 100)));
        assert_eq!(maxima("exalted_monk", 20), Some((305, 210, 100)));
        assert_eq!(maxima("none", 20), Some((245, 150, 100)));
        assert_eq!(maxima("druid", 0), None);
        assert_eq!(maxima("druid", u32::MAX), None);
        assert_eq!(maxima("unknown", 20), None);
    }
}
