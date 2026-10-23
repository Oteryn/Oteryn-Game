#![allow(clippy::expect_used)]

use super::*;

#[test]
fn fuzz_decode_parameter_spell_cast_intent() {
    crate::decoder_fuzz_support::codec(
        "decode_parameter_spell_cast_intent",
        |rng| ParameterSpellCastIntent {
            intent: crate::actor_spell::SpellCastIntent {
                spell: std::num::NonZeroU32::new(rng.u64() as u32)
                    .unwrap_or(std::num::NonZeroU32::MIN),
                target: crate::actor_spell::SpellTarget::None,
                aim_at_target: rng.bool(),
            },
            parameter: rng.bool().then(|| "é".repeat(rng.below(65) as usize)),
        },
        decode_parameter_spell_cast_intent,
        |v| encode_parameter_spell_cast_intent(v).expect("parameter"),
    );
}

#[test]
fn fuzz_decode_parameter_spell_cast_result() {
    crate::decoder_fuzz_support::codec(
        "decode_parameter_spell_cast_result",
        |rng| ParameterSpellCastResult {
            disposition: crate::actor_spell::SpellCastDisposition::Cast,
            feedback: rng.bool().then(|| PrivateSpellFeedback {
                text: "é".repeat((rng.below(20) + 1) as usize),
                effect: PrivateSpellEffect::MagicBlue,
            }),
            editor: rng.bool().then(|| SpellHouseEditor {
                editor_id: crate::decoder_fuzz_support::uuid(rng),
                house_key: "oteryn:content.house.test".into(),
                list: HouseEditorList::Guest,
                ownership_revision: std::num::NonZeroU64::MIN,
                acl_revision: rng.u64(),
                text: "Al\nBob".into(),
            }),
        },
        decode_parameter_spell_cast_result,
        |v| encode_parameter_spell_cast_result(v).expect("result"),
    );
}
