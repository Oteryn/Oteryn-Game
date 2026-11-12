//! Fixed Soulpit passages from Crystal 201189f871d23597b82caf88fd611454a2ddefb8.
//! Sources: data-global/scripts/actions/soulpit/soulpit_entrance.lua:1-57
//! and soulpit_arena_exit.lua:1-17. Native TilePos floors are -source_z.
use super::{ARENA_EXIT_POSITION, TilePos};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PassageTrigger {
    StepIn,
    Use,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SoulpitPassage {
    Lobby(TilePos),
    /// The shared Instance handoff owner performs the return; this is not a Channel teleport.
    ArenaExit,
}

pub fn soulpit_passage(trigger: PassageTrigger, position: TilePos) -> Option<SoulpitPassage> {
    match (trigger, position.x, position.y, position.floor) {
        (PassageTrigger::StepIn, 32349 | 32350, 31030, -3) => {
            Some(SoulpitPassage::Lobby(TilePos {
                x: 32374,
                y: 31171,
                floor: -8,
            }))
        }
        (PassageTrigger::StepIn, 32374, 31173, -8) => Some(SoulpitPassage::Lobby(TilePos {
            x: 32349,
            y: 31032,
            floor: -3,
        })),
        (PassageTrigger::Use, _, _, _) if position == ARENA_EXIT_POSITION => {
            Some(SoulpitPassage::ArenaExit)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_entrances_share_one_lobby_destination() {
        for x in [32349, 32350] {
            assert_eq!(
                soulpit_passage(
                    PassageTrigger::StepIn,
                    TilePos {
                        x,
                        y: 31030,
                        floor: -3
                    }
                ),
                Some(SoulpitPassage::Lobby(TilePos {
                    x: 32374,
                    y: 31171,
                    floor: -8
                }))
            );
        }
    }

    #[test]
    fn source_lobby_exit_returns_to_the_surface() {
        assert_eq!(
            soulpit_passage(
                PassageTrigger::StepIn,
                TilePos {
                    x: 32374,
                    y: 31173,
                    floor: -8
                }
            ),
            Some(SoulpitPassage::Lobby(TilePos {
                x: 32349,
                y: 31032,
                floor: -3
            }))
        );
    }

    #[test]
    fn arena_exit_requires_use_and_never_runs_from_step_ingress() {
        assert_eq!(
            soulpit_passage(PassageTrigger::Use, ARENA_EXIT_POSITION),
            Some(SoulpitPassage::ArenaExit)
        );
        assert_eq!(
            soulpit_passage(PassageTrigger::StepIn, ARENA_EXIT_POSITION),
            None
        );
    }

    #[test]
    fn passage_dispatch_rejects_other_trigger_position_and_floor() {
        for position in [
            TilePos {
                x: 32349,
                y: 31030,
                floor: -3,
            },
            TilePos {
                x: 32350,
                y: 31030,
                floor: -3,
            },
            TilePos {
                x: 32374,
                y: 31173,
                floor: -8,
            },
        ] {
            assert_eq!(soulpit_passage(PassageTrigger::Use, position), None);
            assert_eq!(
                soulpit_passage(
                    PassageTrigger::StepIn,
                    TilePos {
                        floor: position.floor + 1,
                        ..position
                    }
                ),
                None
            );
        }
        assert_eq!(
            soulpit_passage(
                PassageTrigger::StepIn,
                TilePos {
                    x: 32348,
                    y: 31030,
                    floor: -3
                }
            ),
            None
        );
        assert_eq!(
            soulpit_passage(
                PassageTrigger::Use,
                TilePos {
                    x: 32376,
                    ..ARENA_EXIT_POSITION
                }
            ),
            None
        );
    }
}
