#![allow(clippy::expect_used)]

use super::*;

#[test]
fn fuzz_decode() {
    crate::decoder_fuzz_support::codec(
        "decode",
        |rng| Snapshot {
            entity: EntityRef {
                identity: [1; 16],
                generation: rng.u64().max(1),
            },
            content: [1; 32],
            map: [2; 32],
            frame: [3; 32],
            x: rng.u64() as i32,
            y: rng.u64() as i32,
            floor: rng.u64() as i16,
            owner_time_us: rng.u64(),
            player_revision: rng.u64().max(1),
            invisible: rng.bool(),
            condition_light: rng.bool().then(|| Light {
                level: rng.u64() as u8,
                color: rng.u64() as u8,
            }),
            temporary_appearance: match rng.below(3) {
                0 => None,
                1 => Some(TemporaryAppearance::Outfit {
                    key: "outfit".into(),
                    colours: [0, 1, 255, u16::MAX],
                    addons: rng.below(4) as u8,
                    mount: rng.bool().then(|| "mount".into()),
                }),
                _ => Some(TemporaryAppearance::Item {
                    key: "item".into(),
                    revision: "rev".into(),
                    artifact: [1; 32],
                }),
            },
        },
        decode,
        |v| encode(v).expect("conditions"),
    );
}
