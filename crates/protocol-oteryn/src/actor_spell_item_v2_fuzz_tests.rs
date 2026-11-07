#![allow(clippy::expect_used)]

use super::*;

#[test]
fn fuzz_decode_item_spell_cast_intent() {
    crate::decoder_fuzz_support::codec(
        "decode_item_spell_cast_intent",
        |rng| ItemSpellCastIntent {
            intent: crate::actor_spell::SpellCastIntent {
                spell: std::num::NonZeroU32::new(rng.u64() as u32)
                    .unwrap_or(std::num::NonZeroU32::MIN),
                target: crate::actor_spell::SpellTarget::None,
                aim_at_target: rng.bool(),
            },
            rune_item_instance: crate::decoder_fuzz_support::uuid(rng),
            expected_state_revision: rng.u64().max(1),
            target_item: rng.bool().then(|| TargetItemInstance {
                item_instance: crate::decoder_fuzz_support::uuid(rng),
                expected_state_revision: rng.u64().max(1),
            }),
        },
        decode_item_spell_cast_intent,
        |v| encode_item_spell_cast_intent(v).expect("item spell"),
    );
}
