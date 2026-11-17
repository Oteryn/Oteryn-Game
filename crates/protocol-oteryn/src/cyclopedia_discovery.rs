//! MAPDISC: the Cyclopedia map discovery data of the admitted Character
//! (`cyclopedia-discovery-v1.md`). Crystal 201189f8 `protocolgame.cpp:4093-4116`
//! (`sendCyclopediaMapDiscoveryData`, 0xDD sub 1) sends, per main area with subareas, its
//! status and progress, then the discovered and the still discoverable subareas. Oteryn keeps
//! that order with the client 15.30 `map_area` source ids as `u32` and appends the Measuring
//! Tibia quest progress the same discoveries feed. Little-endian, fixed layout:
//!
//! `u16 n {u32 area, u8 status, u8 progress}`, `u16 n {u32 subarea}`, `u16 n {u32 subarea}`,
//! `u16 discovered subareas, u16 completed areas, u16 main areas, u8 stage`.
pub const CAPABILITY_CYCLOPEDIA_DISCOVERY_V1: u32 = 527;
pub const COMMAND_TYPE_CYCLOPEDIA_DISCOVERY_QUERY: u32 = 6262;
/// `oteryn_world_bundle::subarea::MAX_AREAS`: main areas and subareas share this bound.
pub const MAX_AREAS: usize = 1024;
/// Crystal `discovery_system.lua:520-521`: a main area with every subarea discovered.
pub const STATUS_COMPLETE: u8 = 3;
/// Crystal `discovery_system.lua:521`: any other main area that has subareas.
pub const STATUS_DISCOVERING: u8 = 1;
/// Crystal `discovery_system.lua:528-563` milestones: one area, half and all areas.
pub const MAX_STAGE: u8 = 3;
pub const MAX_REPLY_BYTES: usize = 2 + 2 + 2 + MAX_AREAS * 6 + 7;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MainArea {
    pub id: u32,
    pub status: u8,
    pub progress: u8,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct QuestProgress {
    pub discovered_subareas: u16,
    pub completed_areas: u16,
    pub main_areas: u16,
    pub stage: u8,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Discovery {
    pub mains: Vec<MainArea>,
    pub discovered: Vec<u32>,
    pub discoverable: Vec<u32>,
    pub quest: QuestProgress,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiscoveryWireError;

pub fn decode_query(payload: &[u8]) -> Result<(), DiscoveryWireError> {
    payload.is_empty().then_some(()).ok_or(DiscoveryWireError)
}

fn ascending(ids: impl Iterator<Item = u32>) -> bool {
    let mut last = 0;
    ids.into_iter().all(|id| {
        let ok = id > last;
        last = id;
        ok
    })
}

fn valid(value: &Discovery) -> bool {
    let quest = value.quest;
    value.mains.len() + value.discovered.len() + value.discoverable.len() <= MAX_AREAS
        && ascending(value.mains.iter().map(|m| m.id))
        && ascending(value.discovered.iter().copied())
        && ascending(value.discoverable.iter().copied())
        && !value
            .discovered
            .iter()
            .any(|id| value.discoverable.binary_search(id).is_ok())
        && value.mains.iter().all(|m| {
            m.progress <= 100
                && (m.status == STATUS_COMPLETE) == (m.progress == 100)
                && matches!(m.status, STATUS_COMPLETE | STATUS_DISCOVERING)
        })
        && quest.stage <= MAX_STAGE
        && quest.completed_areas <= quest.main_areas
        && usize::from(quest.main_areas) <= MAX_AREAS
        && usize::from(quest.discovered_subareas) <= MAX_AREAS
}

pub fn encode_reply(value: &Discovery) -> Result<Vec<u8>, DiscoveryWireError> {
    if !valid(value) {
        return Err(DiscoveryWireError);
    }
    let count = |n: usize| u16::try_from(n).map_err(|_| DiscoveryWireError);
    let mut bytes = Vec::with_capacity(
        13 + value.mains.len() * 6 + (value.discovered.len() + value.discoverable.len()) * 4,
    );
    bytes.extend_from_slice(&count(value.mains.len())?.to_le_bytes());
    for main in &value.mains {
        bytes.extend_from_slice(&main.id.to_le_bytes());
        bytes.push(main.status);
        bytes.push(main.progress);
    }
    for list in [&value.discovered, &value.discoverable] {
        bytes.extend_from_slice(&count(list.len())?.to_le_bytes());
        for id in list {
            bytes.extend_from_slice(&id.to_le_bytes());
        }
    }
    let quest = value.quest;
    bytes.extend_from_slice(&quest.discovered_subareas.to_le_bytes());
    bytes.extend_from_slice(&quest.completed_areas.to_le_bytes());
    bytes.extend_from_slice(&quest.main_areas.to_le_bytes());
    bytes.push(quest.stage);
    Ok(bytes)
}

struct Reader<'a>(&'a [u8]);
impl Reader<'_> {
    fn take<const N: usize>(&mut self) -> Result<[u8; N], DiscoveryWireError> {
        let (head, rest) = self.0.split_first_chunk::<N>().ok_or(DiscoveryWireError)?;
        self.0 = rest;
        Ok(*head)
    }
    fn u16(&mut self) -> Result<u16, DiscoveryWireError> {
        self.take().map(u16::from_le_bytes)
    }
    fn u32(&mut self) -> Result<u32, DiscoveryWireError> {
        self.take().map(u32::from_le_bytes)
    }
    fn count(&mut self, width: usize) -> Result<usize, DiscoveryWireError> {
        let count = usize::from(self.u16()?);
        if count > MAX_AREAS || count * width > self.0.len() {
            return Err(DiscoveryWireError);
        }
        Ok(count)
    }
    fn ids(&mut self) -> Result<Vec<u32>, DiscoveryWireError> {
        let count = self.count(4)?;
        (0..count).map(|_| self.u32()).collect()
    }
}

pub fn decode_reply(bytes: &[u8]) -> Result<Discovery, DiscoveryWireError> {
    if bytes.len() > MAX_REPLY_BYTES {
        return Err(DiscoveryWireError);
    }
    let mut reader = Reader(bytes);
    let count = reader.count(6)?;
    let mains = (0..count)
        .map(|_| {
            let id = reader.u32()?;
            let [status, progress] = reader.take()?;
            Ok(MainArea {
                id,
                status,
                progress,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let discovered = reader.ids()?;
    let discoverable = reader.ids()?;
    let quest = QuestProgress {
        discovered_subareas: reader.u16()?,
        completed_areas: reader.u16()?,
        main_areas: reader.u16()?,
        stage: reader.take::<1>()?[0],
    };
    let value = Discovery {
        mains,
        discovered,
        discoverable,
        quest,
    };
    if !reader.0.is_empty() || !valid(&value) {
        return Err(DiscoveryWireError);
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Discovery {
        Discovery {
            mains: vec![
                MainArea {
                    id: 1,
                    status: STATUS_COMPLETE,
                    progress: 100,
                },
                MainArea {
                    id: 4,
                    status: STATUS_DISCOVERING,
                    progress: 50,
                },
            ],
            discovered: vec![2, 3, 5],
            discoverable: vec![6],
            quest: QuestProgress {
                discovered_subareas: 3,
                completed_areas: 1,
                main_areas: 2,
                stage: 2,
            },
        }
    }

    #[test]
    fn mapdisc_reply_round_trips_and_every_truncation_fails() -> Result<(), DiscoveryWireError> {
        let value = sample();
        let bytes = encode_reply(&value)?;
        assert_eq!(bytes.len(), 2 + 12 + 2 + 12 + 2 + 4 + 7);
        assert_eq!(decode_reply(&bytes), Ok(value));
        for length in 0..bytes.len() {
            assert_eq!(decode_reply(&bytes[..length]), Err(DiscoveryWireError));
        }
        let mut longer = bytes.clone();
        longer.push(0);
        assert_eq!(decode_reply(&longer), Err(DiscoveryWireError));
        assert_eq!(decode_query(&[]), Ok(()));
        assert_eq!(decode_query(&[0]), Err(DiscoveryWireError));
        Ok(())
    }

    #[test]
    fn mapdisc_reply_refuses_unordered_overlapping_or_inconsistent_values() {
        let mut overlap = sample();
        overlap.discoverable = vec![3];
        let mut unordered = sample();
        unordered.discovered = vec![3, 2];
        let mut status = sample();
        status.mains[1].status = STATUS_COMPLETE;
        let mut stage = sample();
        stage.quest.stage = 4;
        let mut areas = sample();
        areas.quest.completed_areas = 3;
        let mut oversized = sample();
        oversized.discoverable = (10..10 + MAX_AREAS as u32).collect();
        for value in [overlap, unordered, status, stage, areas, oversized] {
            assert_eq!(encode_reply(&value), Err(DiscoveryWireError));
        }
        let full = Discovery {
            discoverable: (1..=MAX_AREAS as u32).collect(),
            ..Discovery::default()
        };
        assert!(encode_reply(&full).is_ok_and(|bytes| bytes.len() <= MAX_REPLY_BYTES));
    }
}
