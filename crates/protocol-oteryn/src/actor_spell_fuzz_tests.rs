#![allow(clippy::expect_used)]

use super::*;

#[test]
fn fuzz_decode_spell_cast_intent() {
    crate::decoder_fuzz_support::codec(
        "decode_spell_cast_intent",
        |rng| SpellCastIntent {
            spell: NonZeroU32::new(rng.u64() as u32).unwrap_or(NonZeroU32::MIN),
            target: match rng.below(3) {
                0 => SpellTarget::None,
                1 => SpellTarget::AttackTarget,
                _ => SpellTarget::Position(SpellTargetPosition {
                    x: rng.u64() as i32,
                    y: rng.u64() as i32,
                    floor: rng.u64() as i16,
                }),
            },
            aim_at_target: rng.bool(),
        },
        decode_spell_cast_intent,
        encode_spell_cast_intent,
    );
}

#[test]
fn fuzz_decode_spell_cast_result() {
    crate::decoder_fuzz_support::codec(
        "decode_spell_cast_result",
        |rng| {
            *rng.pick(&[
                SpellCastDisposition::Cast,
                SpellCastDisposition::CoolingDown,
                SpellCastDisposition::Rejected,
            ])
        },
        decode_spell_cast_result,
        |v| encode_spell_cast_result(*v),
    );
}

#[test]
fn fuzz_decode_spell_cast_result_with_detail() {
    crate::decoder_fuzz_support::codec(
        "decode_spell_cast_result_with_detail",
        |rng| {
            (
                SpellCastDisposition::Rejected,
                rng.bool().then_some(SpellCastRefusalDetail::Muted),
            )
        },
        decode_spell_cast_result_with_detail,
        |v| encode_spell_cast_result_with_detail(v.0, v.1).expect("detail"),
    );
}

#[test]
fn fuzz_decode_actor_vitals() {
    crate::decoder_fuzz_support::codec(
        "decode_actor_vitals",
        |rng| ActorVitals {
            health: rng.below(u64::from(MAX_VITAL_POOL) + 1) as u32,
            max_health: MAX_VITAL_POOL,
            mana: rng.below(u64::from(MAX_VITAL_POOL) + 1) as u32,
            max_mana: MAX_VITAL_POOL,
            soul: rng.below(u64::from(MAX_SOUL) + 1) as u32,
            harmony: rng.below(6) as u32,
            serene: rng.bool(),
        },
        decode_actor_vitals,
        |v| encode_actor_vitals(v).expect("vitals"),
    );
}
