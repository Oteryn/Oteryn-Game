//! Mod Reshaping (Winter Update 2026 test server): one modifier of a revealed gem is redrawn
//! from the same position pool as the reveal draw. The gem keeps its identity, quality,
//! lock and vessel. Prices are the published per-position table (gold plus fragments).

use super::FragmentKind;

/// Gold, fragment kind and fragment count for positions 1, 2 and 3: 125,000 gold and one
/// lesser fragment, 1,000,000 gold and two lesser fragments, 5,000,000 gold and three
/// greater fragments (lesser `i46625` = basic, greater `i46626` = supreme).
pub const RESHAPE_COSTS: [(u64, FragmentKind, u32); 3] = [
    (125_000, FragmentKind::Basic, 1),
    (1_000_000, FragmentKind::Basic, 2),
    (5_000_000, FragmentKind::Supreme, 3),
];

/// The catalogue list each modifier position draws from, in reveal order.
pub const POSITION_POOLS: [&str; 3] = [
    "basic_mods_position_1",
    "basic_mods_position_2",
    "supreme_mods",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReshapeRefusal {
    OutsideTemple,
    NoPremium,
}

/// The cost of reshaping position 1, 2 or 3; `None` for any other position.
pub fn cost(position: u8) -> Option<(u64, FragmentKind, u32)> {
    RESHAPE_COSTS
        .get(usize::from(position.checked_sub(1)?))
        .copied()
}

/// The client refuses Mod Reshaping outside a temple and without Premium.
pub fn admitted(in_temple: bool, premium: bool) -> Result<(), ReshapeRefusal> {
    if !in_temple {
        Err(ReshapeRefusal::OutsideTemple)
    } else if !premium {
        Err(ReshapeRefusal::NoPremium)
    } else {
        Ok(())
    }
}

/// The modifiers position `position` may draw for a gem holding `mods`, in catalogue order.
/// As in the reveal draw, the two basic positions never hold the same modifier.
pub fn choices(family: &serde_json::Value, mods: &[u16], position: u8) -> Option<Vec<u16>> {
    let index = usize::from(position.checked_sub(1)?);
    if index >= mods.len() {
        return None;
    }
    let other_basic = match index {
        0 => mods.get(1).copied(),
        1 => mods.first().copied(),
        _ => None,
    };
    let list = family[*POSITION_POOLS.get(index)?].as_array()?;
    let mut out = Vec::with_capacity(list.len());
    for entry in list {
        let id = u16::try_from(entry.as_u64()?).ok()?;
        if Some(id) != other_basic {
            out.push(id);
        }
    }
    (!out.is_empty()).then_some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atelier::{AtelierAction, AtelierRefusal, AtelierState, Gem, GemQuality};
    use crate::wheel_gem_data::WheelGemData;

    type Error = Box<dyn std::error::Error>;

    #[test]
    fn reshape_position_costs_match_thread_559() {
        assert_eq!(cost(0), None);
        assert_eq!(cost(1), Some((125_000, FragmentKind::Basic, 1)));
        assert_eq!(cost(2), Some((1_000_000, FragmentKind::Basic, 2)));
        assert_eq!(cost(3), Some((5_000_000, FragmentKind::Supreme, 3)));
        assert_eq!(cost(4), None);
    }

    #[test]
    fn reshape_refused_outside_temple_and_without_premium() {
        assert_eq!(admitted(true, true), Ok(()));
        assert_eq!(admitted(false, true), Err(ReshapeRefusal::OutsideTemple));
        assert_eq!(admitted(true, false), Err(ReshapeRefusal::NoPremium));
        assert_eq!(admitted(false, false), Err(ReshapeRefusal::OutsideTemple));
    }

    fn pool(family: &serde_json::Value, list: &str) -> Result<Vec<u16>, Error> {
        let mut out = Vec::new();
        for entry in family[list].as_array().ok_or("pool")? {
            out.push(u16::try_from(entry.as_u64().ok_or("mod id")?)?);
        }
        Ok(out)
    }

    #[test]
    fn reshape_draws_from_the_position_pool_and_keeps_gem_identity() -> Result<(), Error> {
        let data = WheelGemData::embedded()?;
        let family = &data.gems()["vocations"]["knight"];
        let first = pool(family, POSITION_POOLS[0])?;
        let second = pool(family, POSITION_POOLS[1])?;
        let supreme = pool(family, POSITION_POOLS[2])?;
        let mods = vec![
            *first.first().ok_or("first")?,
            *second
                .iter()
                .find(|id| Some(*id) != first.first())
                .ok_or("second")?,
            *supreme.first().ok_or("supreme")?,
        ];
        let id = u128::from_be_bytes([7; 16]);
        let mut state = AtelierState {
            vocation: "knight".into(),
            gold: 10_000_000,
            ..AtelierState::default()
        };
        state.fragments.insert(FragmentKind::Basic, 3);
        state.fragments.insert(FragmentKind::Supreme, 3);
        state.gems.insert(
            id,
            Gem {
                id,
                quality: GemQuality::Greater,
                mods: mods.clone(),
                locked: false,
                vessel: true,
                slot: None,
                ruleset_revision: data.revision().into(),
            },
        );

        // The second basic position never draws the first position's modifier.
        let basic = choices(family, &mods, 2).ok_or("choices")?;
        assert!(!basic.contains(&mods[0]));
        assert_eq!(
            basic.len() + usize::from(second.contains(&mods[0])),
            second.len()
        );
        assert_eq!(choices(family, &mods, 3), Some(supreme.clone()));
        assert_eq!(choices(family, &mods, 4), None);
        assert_eq!(choices(family, &mods[..1], 2), None);

        // A modifier outside the position's pool is refused without a change.
        let before = state.clone();
        assert_eq!(
            state.commit(
                0,
                AtelierAction::Reshape {
                    id,
                    position: 2,
                    mod_id: mods[0],
                },
            ),
            Err(AtelierRefusal::InvalidDraw)
        );
        assert_eq!(state, before);

        let drawn = *supreme
            .iter()
            .find(|id| **id != mods[2])
            .ok_or("another supreme mod")?;
        state
            .commit(
                0,
                AtelierAction::Reshape {
                    id,
                    position: 3,
                    mod_id: drawn,
                },
            )
            .map_err(|refusal| format!("{refusal:?}"))?;
        let gem = state.gems.get(&id).ok_or("gem kept")?;
        assert_eq!(gem.quality, GemQuality::Greater);
        assert!(gem.vessel);
        assert_eq!(gem.mods, [mods[0], mods[1], drawn]);
        assert_eq!(state.gold, 5_000_000);
        assert_eq!(state.fragments.get(&FragmentKind::Supreme), Some(&0));
        assert_eq!(state.fragments.get(&FragmentKind::Basic), Some(&3));
        assert_eq!(state.revision, 1);

        // Gold and fragments are checked per position; a locked gem must be unlocked first.
        assert_eq!(
            state.clone().commit(
                1,
                AtelierAction::Reshape {
                    id,
                    position: 3,
                    mod_id: mods[2],
                },
            ),
            Err(AtelierRefusal::InsufficientFragments)
        );
        let mut broke = state.clone();
        broke.gold = 124_999;
        assert_eq!(
            broke.commit(
                1,
                AtelierAction::Reshape {
                    id,
                    position: 1,
                    mod_id: first[0],
                },
            ),
            Err(AtelierRefusal::InsufficientGold)
        );
        let mut poor = state.clone();
        poor.fragments.insert(FragmentKind::Basic, 1);
        assert_eq!(
            poor.commit(
                1,
                AtelierAction::Reshape {
                    id,
                    position: 2,
                    mod_id: basic[0],
                },
            ),
            Err(AtelierRefusal::InsufficientFragments)
        );
        let mut locked = state.clone();
        if let Some(gem) = locked.gems.get_mut(&id) {
            gem.locked = true;
        }
        assert_eq!(
            locked.commit(
                1,
                AtelierAction::Reshape {
                    id,
                    position: 1,
                    mod_id: first[0],
                },
            ),
            Err(AtelierRefusal::Locked)
        );
        Ok(())
    }
}
