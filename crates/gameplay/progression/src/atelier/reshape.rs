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

/// Where a reshape's `needed` fragments come from: carried containers first; the Supply
/// Stash pays the remainder only while it is open at a depot (owner decision WU26-10).
/// Returns the (carried, stash) split, or `None` when the reachable fragments fall short.
pub fn fragment_sources(
    needed: u32,
    carried: u32,
    stash: u32,
    at_depot: bool,
) -> Option<(u32, u32)> {
    let from_carried = needed.min(carried);
    let rest = needed - from_carried;
    if rest == 0 {
        return Some((from_carried, 0));
    }
    (at_depot && stash >= rest).then_some((from_carried, rest))
}

/// The modifiers position `position` may draw for a gem holding `mods`, in catalogue order.
/// A redraw never returns the modifier the position holds now (owner decision WU26-10), and,
/// as in the reveal draw, the two basic positions never hold the same modifier. `None` means
/// the gem has no such position. Every position pool holds at least two modifiers (content
/// invariant, tested below), so the list is never empty for valid content; the durable draw
/// treats an empty list as invalid content, not as a player refusal.
pub fn choices(family: &serde_json::Value, mods: &[u16], position: u8) -> Option<Vec<u16>> {
    let index = usize::from(position.checked_sub(1)?);
    let current = *mods.get(index)?;
    let other_basic = match index {
        0 => mods.get(1).copied(),
        1 => mods.first().copied(),
        _ => None,
    };
    let list = family[*POSITION_POOLS.get(index)?].as_array()?;
    let mut out = Vec::with_capacity(list.len());
    for entry in list {
        let id = u16::try_from(entry.as_u64()?).ok()?;
        if id != current && Some(id) != other_basic {
            out.push(id);
        }
    }
    Some(out)
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

    #[test]
    fn reshape_fragments_come_from_carried_first_and_the_stash_only_at_a_depot() {
        // Away from a depot: carried containers only.
        assert_eq!(fragment_sources(2, 2, 9, false), Some((2, 0)));
        assert_eq!(fragment_sources(2, 1, 9, false), None);
        assert_eq!(fragment_sources(1, 0, 9, false), None);
        // At a depot with the Supply Stash open: carried first, then the stash.
        assert_eq!(fragment_sources(2, 2, 9, true), Some((2, 0)));
        assert_eq!(fragment_sources(2, 1, 9, true), Some((1, 1)));
        assert_eq!(fragment_sources(3, 0, 3, true), Some((0, 3)));
        assert_eq!(fragment_sources(3, 1, 1, true), None);
    }

    fn pool(family: &serde_json::Value, list: &str) -> Result<Vec<u16>, Error> {
        let mut out = Vec::new();
        for entry in family[list].as_array().ok_or("pool")? {
            out.push(u16::try_from(entry.as_u64().ok_or("mod id")?)?);
        }
        Ok(out)
    }

    #[test]
    fn every_reshape_position_pool_has_another_modifier() -> Result<(), Error> {
        let data = WheelGemData::embedded()?;
        let vocations = data.gems()["vocations"].as_object().ok_or("vocations")?;
        assert!(!vocations.is_empty());
        for (vocation, family) in vocations {
            let pools = POSITION_POOLS
                .iter()
                .map(|list| pool(family, list))
                .collect::<Result<Vec<_>, _>>()?;
            for (list, entries) in POSITION_POOLS.iter().zip(&pools) {
                assert!(entries.len() >= 2, "{vocation} {list}");
            }
            // Whatever the gem holds, every position still has a modifier to draw.
            for &first in &pools[0] {
                for &second in pools[1].iter().filter(|id| **id != first) {
                    for &supreme in &pools[2] {
                        let mods = [first, second, supreme];
                        for position in 1..=3 {
                            let drawn = choices(family, &mods, position).ok_or("position")?;
                            assert!(!drawn.is_empty(), "{vocation} {mods:?} {position}");
                        }
                    }
                }
            }
        }
        Ok(())
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

        // The second basic position never draws the first position's modifier, and no
        // position redraws the modifier it holds now.
        let basic = choices(family, &mods, 2).ok_or("choices")?;
        assert!(!basic.contains(&mods[0]) && !basic.contains(&mods[1]));
        assert_eq!(
            basic.len() + usize::from(second.contains(&mods[0])) + 1,
            second.len()
        );
        let other_supreme: Vec<u16> = supreme
            .iter()
            .copied()
            .filter(|id| *id != mods[2])
            .collect();
        assert_eq!(choices(family, &mods, 3), Some(other_supreme));
        let fresh_first = *choices(family, &mods, 1)
            .ok_or("first choices")?
            .first()
            .ok_or("another first modifier")?;
        assert!(fresh_first != mods[0] && fresh_first != mods[1]);
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
        // Redrawing the modifier a position already holds is refused too.
        assert_eq!(
            state.commit(
                0,
                AtelierAction::Reshape {
                    id,
                    position: 3,
                    mod_id: mods[2],
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
                    mod_id: fresh_first,
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
                    mod_id: fresh_first,
                },
            ),
            Err(AtelierRefusal::Locked)
        );
        Ok(())
    }
}
