//! Bestiary view typed payload (CHARM-5; **proposed**, not registered).
//!
//! Wire proposal: `docs/contracts/protocol-oteryn/CHARM5_BESTIARY_CHARM_WIRE_PROPOSAL_V1.md` §3.1,
//! following CHARM-0 §4.1 (`docs/architecture/reviews/
//! OTERYN_GAME_CHARM0_BESTIARY_CHARM_PROGRESSION_DECISION_PACKET_2026-09-29.md`). The state
//! domain and payload-type IDs below are proposals for the protocol owner. They are not in
//! `PROTOCOL_OTERYN_V1_REGISTRY.json`, and no session or server sends or accepts them until that
//! owner registers them.
//!
//! One `BestiaryViewV1` serves the snapshot (the full set of the character's counted races) and
//! the delta (an upsert of the listed races only), as `actor_spell` serves both with
//! `ActorVitalsV1`. The unlocked stage is not on the wire: it is derived from the kill count and
//! the thresholds, never stored (CHARM-0 §4.1), by [`BestiaryRaceProgress::unlocked_stage`].
//!
//! Decoding is strict, and encoding refuses the same values as a server fault before any byte is
//! emitted: races out of ascending order or repeated, a race index of zero or above
//! [`MAX_BESTIARY_RACE`], more than [`MAX_BESTIARY_VIEW_ENTRIES`] entries, thresholds that are not
//! strictly increasing from 1 or exceed [`MAX_BESTIARY_KILL_THRESHOLD`], and a kill count of zero
//! or above the final threshold (counters saturate there, CHARM-0 §4.1) all fail closed.

use std::num::NonZeroU32;

pub use crate::charm_wire::CyclopediaWireError;
use crate::charm_wire::{
    WireResult, push_message_field, push_varint_field, read_bytes, read_uint32_fields, read_varint,
};

/// Proposed state domain `CHARACTER_BESTIARY` (proposal §2).
pub const STATE_DOMAIN_CHARACTER_BESTIARY: u32 = 4;
pub const DELTA_TYPE_CHARACTER_BESTIARY_V1: u32 = 1;
pub const SNAPSHOT_TYPE_CHARACTER_BESTIARY_V1: u32 = 1;

/// Proposed `CHARM5-RL-01`: Bestiary races in one view. The captured client staticdata lists 833
/// Bestiary races (CHARM-0 §2); 1024 leaves room without making the bound open-ended.
pub const MAX_BESTIARY_VIEW_ENTRIES: usize = 1024;
/// A race index is 1-based into a race list of at most [`MAX_BESTIARY_VIEW_ENTRIES`] races.
pub const MAX_BESTIARY_RACE: u32 = 1024;
/// Proposed `CHARM5-RL-02`: the largest kill threshold (the captured definitions top out at 5000).
pub const MAX_BESTIARY_KILL_THRESHOLD: u32 = 100_000;
/// Stages are the thresholds reached: 0 (none) to 3 (entry complete).
pub const BESTIARY_COMPLETE_STAGE: u8 = 3;
/// Worst case per entry: race 1 + 2, kill count 1 + 3, three thresholds 3 x (1 + 3), plus the
/// entry's own tag and 1-byte length = 21 bytes; 1024 entries = 21 504 bytes.
pub const MAX_BESTIARY_VIEW_BYTES: usize = 21_504;

/// `BestiaryRaceProgressV1`: one race of the character's Bestiary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BestiaryRaceProgress {
    /// 1-based index into the Bestiary race list of the loaded content generation (proposal §2).
    pub race: NonZeroU32,
    /// Kills credited so far, saturated at the final threshold.
    pub kill_count: u32,
    /// The race's three kill thresholds from its Creature definition, strictly increasing.
    pub kill_thresholds: [u32; 3],
}

impl BestiaryRaceProgress {
    /// The derived unlocked stage: how many thresholds the kill count has reached (0..=3).
    #[must_use]
    pub fn unlocked_stage(&self) -> u8 {
        self.kill_thresholds.iter().fold(0, |stage, threshold| {
            stage + u8::from(self.kill_count >= *threshold)
        })
    }

    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.unlocked_stage() == BESTIARY_COMPLETE_STAGE
    }

    /// The next threshold to reach, or `None` once the entry is complete.
    #[must_use]
    pub fn next_threshold(&self) -> Option<u32> {
        self.kill_thresholds
            .iter()
            .copied()
            .find(|threshold| self.kill_count < *threshold)
    }

    fn check(&self) -> WireResult<()> {
        if self.race.get() > MAX_BESTIARY_RACE
            || self.kill_thresholds[2] > MAX_BESTIARY_KILL_THRESHOLD
        {
            return Err(CyclopediaWireError::LimitExceeded);
        }
        let [first, second, last] = self.kill_thresholds;
        if first == 0 || first >= second || second >= last {
            return Err(CyclopediaWireError::Malformed);
        }
        if self.kill_count == 0 || self.kill_count > last {
            return Err(CyclopediaWireError::Malformed);
        }
        Ok(())
    }
}

/// Every entry valid, at most [`MAX_BESTIARY_VIEW_ENTRIES`], races strictly ascending.
fn check_view(races: &[BestiaryRaceProgress]) -> WireResult<()> {
    if races.len() > MAX_BESTIARY_VIEW_ENTRIES {
        return Err(CyclopediaWireError::LimitExceeded);
    }
    for entry in races {
        entry.check()?;
    }
    if races.windows(2).any(|pair| pair[0].race >= pair[1].race) {
        return Err(CyclopediaWireError::Malformed);
    }
    Ok(())
}

/// Encodes the domain-4 snapshot or delta payload. A view outside the bounds is a server fault
/// and is refused before any byte is emitted.
pub fn encode_bestiary_view(races: &[BestiaryRaceProgress]) -> WireResult<Vec<u8>> {
    check_view(races)?;
    let mut output = Vec::with_capacity(races.len() * 21);
    let mut entry = Vec::with_capacity(19);
    for race in races {
        entry.clear();
        push_varint_field(&mut entry, 1, u64::from(race.race.get()));
        push_varint_field(&mut entry, 2, u64::from(race.kill_count));
        for (field, threshold) in (3..).zip(race.kill_thresholds) {
            push_varint_field(&mut entry, field, u64::from(threshold));
        }
        push_message_field(&mut output, 1, &entry);
    }
    Ok(output)
}

pub fn decode_bestiary_view(payload: &[u8]) -> WireResult<Vec<BestiaryRaceProgress>> {
    if payload.len() > MAX_BESTIARY_VIEW_BYTES {
        return Err(CyclopediaWireError::LimitExceeded);
    }
    let mut cursor = 0;
    let mut races = Vec::new();
    while cursor < payload.len() {
        if read_varint(payload, &mut cursor)? != 0x0a {
            return Err(CyclopediaWireError::Malformed);
        }
        if races.len() == MAX_BESTIARY_VIEW_ENTRIES {
            return Err(CyclopediaWireError::LimitExceeded);
        }
        let [race, kill_count, first, second, last] =
            read_uint32_fields::<5>(read_bytes(payload, &mut cursor)?)?;
        let race = NonZeroU32::new(race).ok_or(CyclopediaWireError::Malformed)?;
        races.push(BestiaryRaceProgress {
            race,
            kill_count,
            kill_thresholds: [first, second, last],
        });
    }
    check_view(&races)?;
    Ok(races)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    fn race(index: u32) -> NonZeroU32 {
        NonZeroU32::new(index).expect("non-zero race index")
    }

    fn progress(index: u32, kill_count: u32, kill_thresholds: [u32; 3]) -> BestiaryRaceProgress {
        BestiaryRaceProgress {
            race: race(index),
            kill_count,
            kill_thresholds,
        }
    }

    /// One `races` entry around hand-written inner bytes: 0a <len> <inner>.
    fn entry(inner: &[u8]) -> Vec<u8> {
        let mut bytes = vec![0x0a, u8::try_from(inner.len()).expect("short entry")];
        bytes.extend_from_slice(inner);
        bytes
    }

    /// Hand-computed canonical proto3 bytes (not produced by the encoder under test).
    fn fixtures() -> Vec<(Vec<BestiaryRaceProgress>, Vec<u8>)> {
        vec![
            // No counted race: the empty payload.
            (vec![], vec![]),
            // race 1, 7 kills, thresholds 5/10/25 (25 = 0x19): 08 01 10 07 18 05 20 0a 28 19.
            (
                vec![progress(1, 7, [5, 10, 25])],
                entry(&[0x08, 0x01, 0x10, 0x07, 0x18, 0x05, 0x20, 0x0a, 0x28, 0x19]),
            ),
            // race 2 complete at 1000 = E8 07 (50 = 32, 500 = F4 03), then race 300 = AC 02 with
            // one kill of 1/3/5.
            (
                vec![
                    progress(2, 1000, [50, 500, 1000]),
                    progress(300, 1, [1, 3, 5]),
                ],
                [
                    entry(&[
                        0x08, 0x02, 0x10, 0xe8, 0x07, 0x18, 0x32, 0x20, 0xf4, 0x03, 0x28, 0xe8,
                        0x07,
                    ]),
                    entry(&[
                        0x08, 0xac, 0x02, 0x10, 0x01, 0x18, 0x01, 0x20, 0x03, 0x28, 0x05,
                    ]),
                ]
                .concat(),
            ),
            // Per-entry worst case, 21 bytes: race 1024 = 80 08, 100000 = A0 8D 06,
            // 99998 = 9E 8D 06, 99999 = 9F 8D 06.
            (
                vec![progress(
                    MAX_BESTIARY_RACE,
                    MAX_BESTIARY_KILL_THRESHOLD,
                    [99_998, 99_999, MAX_BESTIARY_KILL_THRESHOLD],
                )],
                entry(&[
                    0x08, 0x80, 0x08, 0x10, 0xa0, 0x8d, 0x06, 0x18, 0x9e, 0x8d, 0x06, 0x20, 0x9f,
                    0x8d, 0x06, 0x28, 0xa0, 0x8d, 0x06,
                ]),
            ),
        ]
    }

    #[test]
    fn bestiary_view_matches_independent_fixtures() {
        for (value, bytes) in fixtures() {
            assert_eq!(encode_bestiary_view(&value), Ok(bytes.clone()), "{value:?}");
            assert!(bytes.len() <= MAX_BESTIARY_VIEW_BYTES);
            assert_eq!(decode_bestiary_view(&bytes), Ok(value), "{bytes:02x?}");
        }
        // Field order inside an entry is not significant.
        assert_eq!(
            decode_bestiary_view(&entry(&[
                0x28, 0x19, 0x20, 0x0a, 0x18, 0x05, 0x10, 0x07, 0x08, 0x01
            ])),
            Ok(vec![progress(1, 7, [5, 10, 25])])
        );
    }

    #[test]
    fn the_full_view_at_its_bounds_fits_the_byte_bound() {
        let races: Vec<_> = (1..=MAX_BESTIARY_RACE)
            .map(|index| {
                progress(
                    index,
                    MAX_BESTIARY_KILL_THRESHOLD,
                    [99_998, 99_999, MAX_BESTIARY_KILL_THRESHOLD],
                )
            })
            .collect();
        assert_eq!(races.len(), MAX_BESTIARY_VIEW_ENTRIES);
        let bytes = encode_bestiary_view(&races).expect("view at its bounds");
        // Races 1..=127 take one index byte less than the 21-byte worst case.
        assert_eq!(bytes.len(), MAX_BESTIARY_VIEW_BYTES - 127);
        assert_eq!(decode_bestiary_view(&bytes), Ok(races));
    }

    #[test]
    fn unlocked_stage_is_derived_from_kills_and_thresholds() {
        let thresholds = [5, 10, 25];
        for (kills, stage, next) in [
            (1, 0, Some(5)),
            (4, 0, Some(5)),
            (5, 1, Some(10)),
            (9, 1, Some(10)),
            (10, 2, Some(25)),
            (24, 2, Some(25)),
            (25, 3, None),
        ] {
            let entry = progress(1, kills, thresholds);
            assert_eq!(entry.unlocked_stage(), stage, "{kills}");
            assert_eq!(entry.next_threshold(), next, "{kills}");
            assert_eq!(entry.is_complete(), stage == BESTIARY_COMPLETE_STAGE);
        }
    }

    #[test]
    fn each_invariant_fails_closed_in_both_directions() {
        let valid = progress(1, 7, [5, 10, 25]);
        let valid_bytes = [0x08, 0x01, 0x10, 0x07, 0x18, 0x05, 0x20, 0x0a, 0x28, 0x19];
        // One value per invariant; the encoder refuses it and the decoder rejects its bytes.
        let cases: [(
            &str,
            Vec<BestiaryRaceProgress>,
            Vec<u8>,
            CyclopediaWireError,
        ); 7] = [
            (
                "race above its bound",
                vec![progress(MAX_BESTIARY_RACE + 1, 7, [5, 10, 25])],
                // 1025 = 81 08.
                entry(&[
                    0x08, 0x81, 0x08, 0x10, 0x07, 0x18, 0x05, 0x20, 0x0a, 0x28, 0x19,
                ]),
                CyclopediaWireError::LimitExceeded,
            ),
            (
                "races out of order",
                vec![progress(2, 7, [5, 10, 25]), valid],
                [
                    entry(&[0x08, 0x02, 0x10, 0x07, 0x18, 0x05, 0x20, 0x0a, 0x28, 0x19]),
                    entry(&valid_bytes),
                ]
                .concat(),
                CyclopediaWireError::Malformed,
            ),
            (
                "race repeated",
                vec![valid, valid],
                [entry(&valid_bytes), entry(&valid_bytes)].concat(),
                CyclopediaWireError::Malformed,
            ),
            (
                "kill count zero",
                vec![progress(1, 0, [5, 10, 25])],
                entry(&[0x08, 0x01, 0x18, 0x05, 0x20, 0x0a, 0x28, 0x19]),
                CyclopediaWireError::Malformed,
            ),
            (
                "kill count past the saturating final threshold",
                vec![progress(1, 26, [5, 10, 25])],
                entry(&[0x08, 0x01, 0x10, 0x1a, 0x18, 0x05, 0x20, 0x0a, 0x28, 0x19]),
                CyclopediaWireError::Malformed,
            ),
            (
                "thresholds not strictly increasing",
                vec![progress(1, 7, [5, 5, 25])],
                entry(&[0x08, 0x01, 0x10, 0x07, 0x18, 0x05, 0x20, 0x05, 0x28, 0x19]),
                CyclopediaWireError::Malformed,
            ),
            (
                "final threshold above its bound",
                vec![progress(1, 7, [5, 10, MAX_BESTIARY_KILL_THRESHOLD + 1])],
                // 100001 = A1 8D 06.
                entry(&[
                    0x08, 0x01, 0x10, 0x07, 0x18, 0x05, 0x20, 0x0a, 0x28, 0xa1, 0x8d, 0x06,
                ]),
                CyclopediaWireError::LimitExceeded,
            ),
        ];
        for (case, value, bytes, error) in cases {
            assert_eq!(encode_bestiary_view(&value), Err(error), "encode {case}");
            assert_eq!(decode_bestiary_view(&bytes), Err(error), "decode {case}");
        }
        // A first threshold of zero is omitted on the wire and still fails closed.
        assert_eq!(
            encode_bestiary_view(&[progress(1, 7, [0, 10, 25])]),
            Err(CyclopediaWireError::Malformed)
        );
        assert_eq!(
            decode_bestiary_view(&entry(&[0x08, 0x01, 0x10, 0x07, 0x20, 0x0a, 0x28, 0x19])),
            Err(CyclopediaWireError::Malformed)
        );
    }

    #[test]
    fn entry_count_and_byte_bounds_fail_closed() {
        let too_many: Vec<_> = (1..=MAX_BESTIARY_RACE + 1)
            .map(|index| progress(index, 7, [5, 10, 25]))
            .collect();
        assert_eq!(
            encode_bestiary_view(&too_many),
            Err(CyclopediaWireError::LimitExceeded)
        );
        // 1025 small entries fit the byte bound; the count alone refuses them.
        let bytes: Vec<u8> = (0..=MAX_BESTIARY_VIEW_ENTRIES)
            .flat_map(|_| entry(&[0x08, 0x01, 0x10, 0x07, 0x18, 0x05, 0x20, 0x0a, 0x28, 0x19]))
            .collect();
        assert!(bytes.len() <= MAX_BESTIARY_VIEW_BYTES);
        assert_eq!(
            decode_bestiary_view(&bytes),
            Err(CyclopediaWireError::LimitExceeded)
        );
        assert_eq!(
            decode_bestiary_view(&[0; MAX_BESTIARY_VIEW_BYTES + 1]),
            Err(CyclopediaWireError::LimitExceeded)
        );
    }

    #[test]
    fn malformed_encodings_fail_closed() {
        let malformed: &[(&str, &[u8])] = &[
            (
                "race absent",
                &[0x0a, 0x08, 0x10, 0x07, 0x18, 0x05, 0x20, 0x0a, 0x28, 0x19],
            ),
            (
                "race zero",
                &[
                    0x0a, 0x0a, 0x08, 0x00, 0x10, 0x07, 0x18, 0x05, 0x20, 0x0a, 0x28, 0x19,
                ],
            ),
            ("unknown top-level field", &[0x12, 0x00]),
            ("top-level varint", &[0x08, 0x01]),
            (
                "unknown entry field 6",
                &[
                    0x0a, 0x0c, 0x08, 0x01, 0x10, 0x07, 0x18, 0x05, 0x20, 0x0a, 0x28, 0x19, 0x30,
                    0x01,
                ],
            ),
            (
                "entry field repeated",
                &[
                    0x0a, 0x0c, 0x08, 0x01, 0x08, 0x01, 0x10, 0x07, 0x18, 0x05, 0x20, 0x0a, 0x28,
                    0x19,
                ],
            ),
            ("entry field 0", &[0x0a, 0x02, 0x00, 0x01]),
            ("entry wrong wire type", &[0x0a, 0x02, 0x0a, 0x00]),
            ("entry length past end", &[0x0a, 0x05, 0x08, 0x01]),
            ("truncated key", &[0x0a]),
            ("truncated varint", &[0x0a, 0x02, 0x08, 0x81]),
            (
                "kill count above u32",
                &[0x0a, 0x08, 0x08, 0x01, 0x10, 0x80, 0x80, 0x80, 0x80, 0x10],
            ),
        ];
        for (case, bytes) in malformed {
            assert_eq!(
                decode_bestiary_view(bytes),
                Err(CyclopediaWireError::Malformed),
                "{case}"
            );
        }
    }
}
