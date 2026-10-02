//! Source-qualified planning for house, locate, vertical movement and appearance spells.
//!
//! Facts are read-only snapshots, never commit authority. Applying a move, opening/saving a
//! house window or changing an outfit remains the respective fenced owner's responsibility.
use std::collections::BTreeMap;

use serde_json::Value;

use crate::domain::appearance::AppearanceSelection;

use super::locate::locate;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Position {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) z: i32,
}

impl Position {
    fn offset(self, x: i32, y: i32, z: i32) -> Result<Self, Error> {
        Ok(Self {
            x: self.x.checked_add(x).ok_or(Error::PositionOverflow)?,
            y: self.y.checked_add(y).ok_or(Error::PositionOverflow)?,
            z: self.z.checked_add(z).ok_or(Error::PositionOverflow)?,
        })
    }

    fn phrase(self, target: Self) -> Result<String, Error> {
        Ok(locate(
            self.x
                .checked_sub(target.x)
                .ok_or(Error::PositionOverflow)?,
            self.y
                .checked_sub(target.y)
                .ok_or(Error::PositionOverflow)?,
            self.z
                .checked_sub(target.z)
                .ok_or(Error::PositionOverflow)?,
        )
        .phrase())
    }
}

pub(crate) use crate::durability::spell_house_abi::{HouseAccess, HouseList};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LocatedPlayer {
    pub(crate) name: String,
    pub(crate) position: Position,
    pub(crate) staff_access: bool,
    pub(crate) exiva_permitted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Fiend {
    pub(crate) position: Position,
    pub(crate) live_fiendish: bool,
    pub(crate) bestiary_completed: bool,
    pub(crate) bestiary_required_kills: u32,
    pub(crate) reverts_at_seconds: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Tile {
    pub(crate) ground: Option<u32>,
    pub(crate) top_items: Vec<u32>,
    pub(crate) block_solid: bool,
    pub(crate) block_projectile: bool,
    pub(crate) immovable_block_solid: bool,
    pub(crate) immovable_block_item: bool,
    pub(crate) immovable_nonfield_block_item: bool,
    pub(crate) floor_change: bool,
    pub(crate) entry_permitted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum HouseMovementFacts {
    LocatePlayer {
        caster: Position,
        caster_staff: bool,
        /// Name resolver must have applied exact/unique-prefix matching and length bounds.
        target: Option<LocatedPlayer>,
    },
    FindFiend {
        caster: Position,
        /// Preserve supplied registry traversal order for equal-distance targets.
        fiends: Vec<Fiend>,
        now_seconds: i64,
    },
    HouseList {
        caster_house: Option<u64>,
        access: HouseAccess,
        front_door: Option<u32>,
        own_door: Option<u32>,
    },
    HouseKick {
        caster_house: Option<u64>,
        target_house: Option<u64>,
        target_is_caster: bool,
        caster_can_edit_guest: bool,
        caster_access_in_target_house: HouseAccess,
        target_access: HouseAccess,
        target_can_edit_houses: bool,
        caster_entry: Option<Position>,
        target_entry: Option<Position>,
    },
    VerticalMove {
        caster: Position,
        /// Facing is cardinal; no diagonal levitate admission.
        facing: (i32, i32),
        choice: String,
        tiles: BTreeMap<Position, Tile>,
    },
    Appearance {
        outfit: Option<AppearanceSelection<String>>,
        illusionable: bool,
        can_illusion_all: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum HouseMovementPlan {
    Message {
        text: String,
        effect: &'static str,
    },
    OpenHouseEditor {
        house: u64,
        list: HouseList,
        recheck_access_on_save: bool,
        require_matching_window_and_session: bool,
        evict_uninvited_on_save: bool,
    },
    Move {
        target_is_caster: bool,
        destination: Position,
        before_effect: &'static str,
        after_effect: &'static str,
        failed_move_counts_as_success: bool,
    },
    Appearance {
        outfit: AppearanceSelection<String>,
        duration_ms: u64,
        replace_existing: bool,
        effect: &'static str,
    },
    Refused {
        reason: &'static str,
        effect: Option<&'static str>,
        cast_succeeds: bool,
        start_cooldown: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Error {
    InvalidParameters(&'static str),
    WrongFacts,
    PositionOverflow,
    MissingDestination,
}

fn string<'a>(parameters: &'a Value, key: &'static str) -> Result<&'a str, Error> {
    parameters[key]
        .as_str()
        .ok_or(Error::InvalidParameters(key))
}

fn refusal(reason: &'static str) -> HouseMovementPlan {
    HouseMovementPlan::Refused {
        reason,
        effect: Some("poff"),
        cast_succeeds: false,
        start_cooldown: false,
    }
}

fn player_plan(
    caster: Position,
    caster_staff: bool,
    target: &Option<LocatedPlayer>,
) -> Result<HouseMovementPlan, Error> {
    let Some(target) = target
        .as_ref()
        .filter(|target| !target.staff_access || caster_staff)
    else {
        return Ok(HouseMovementPlan::Refused {
            reason: "player_with_this_name_is_not_online",
            effect: Some("poff"),
            cast_succeeds: false,
            start_cooldown: true,
        });
    };
    if !target.exiva_permitted {
        return Ok(refusal("exiva_protected"));
    }
    Ok(HouseMovementPlan::Message {
        text: format!("{} {}.", target.name, caster.phrase(target.position)?),
        effect: "magic_blue",
    })
}

fn fiend_plan(caster: Position, fiends: &[Fiend], now: i64) -> Result<HouseMovementPlan, Error> {
    let target = fiends
        .iter()
        .filter(|fiend| fiend.live_fiendish)
        .min_by_key(|fiend| {
            let position = fiend.position;
            [
                i64::from(caster.x) - i64::from(position.x),
                i64::from(caster.y) - i64::from(position.y),
                i64::from(caster.z) - i64::from(position.z),
            ]
            .into_iter()
            .map(i64::abs)
            .max()
            .unwrap_or(0)
        });
    let Some(target) = target else {
        return Ok(refusal(
            "At the moment there is no fiend with special loot roaming this world.",
        ));
    };
    let difficulty = if !target.bestiary_completed {
        "Unknown"
    } else {
        match target.bestiary_required_kills {
            5..=25 => "Harmless",
            0..=250 => "Trivial",
            251..=500 => "Easy",
            501..=1000 => "Medium",
            1001..=2500 => "Hard",
            2501..=5000 => "Challenging",
            _ => "Unknown",
        }
    };
    let mut text = format!(
        "The monster {}. Be prepared to find a creature of difficulty level \"{difficulty}\".",
        caster.phrase(target.position)?
    );
    let seconds = target
        .reverts_at_seconds
        .checked_sub(now)
        .ok_or(Error::PositionOverflow)?;
    if seconds < 900 {
        text.push_str(" This monster will stay fiendish for less than");
        if seconds >= 60 {
            text.push_str(&format!(" {} minutes and", seconds / 60));
        }
        let remainder = if seconds >= 60 { seconds % 60 } else { seconds };
        text.push_str(&format!(" {remainder} seconds."));
    }
    Ok(HouseMovementPlan::Message {
        text,
        effect: "magic_blue",
    })
}

fn list_plan(
    parameters: &Value,
    house: Option<u64>,
    access: HouseAccess,
    front: Option<u32>,
    own: Option<u32>,
) -> Result<HouseMovementPlan, Error> {
    let which = string(parameters, "list")?;
    if !matches!(which, "guest" | "subowner" | "door") {
        return Err(Error::InvalidParameters("list"));
    }
    let Some(house) = house else {
        return Ok(if which == "door" {
            refusal("not_possible")
        } else {
            HouseMovementPlan::Refused {
                reason: "no_house",
                effect: None,
                cast_succeeds: false,
                start_cooldown: false,
            }
        });
    };
    let door = front.or(own);
    let allowed = (access == HouseAccess::Owner
        || access == HouseAccess::Subowner && which == "guest")
        && (which != "door" || door.is_some());
    if !allowed {
        return Ok(HouseMovementPlan::Refused {
            reason: "not_possible",
            effect: Some("poff"),
            cast_succeeds: which != "door",
            start_cooldown: which != "door",
        });
    }
    let list = match which {
        "guest" => HouseList::Guest,
        "subowner" => HouseList::Subowner,
        _ => HouseList::Door(door.ok_or(Error::MissingDestination)?),
    };
    Ok(HouseMovementPlan::OpenHouseEditor {
        house,
        list,
        recheck_access_on_save: true,
        require_matching_window_and_session: true,
        evict_uninvited_on_save: which != "door",
    })
}

fn movement_plan(
    parameters: &Value,
    caster: Position,
    facing: (i32, i32),
    choice: &str,
    tiles: &BTreeMap<Position, Tile>,
) -> Result<HouseMovementPlan, Error> {
    let mode = string(parameters, "mode")?;
    let destination = match mode {
        "levitate" => {
            if !matches!(facing, (0, 1) | (0, -1) | (1, 0) | (-1, 0)) {
                return Err(Error::InvalidParameters("facing"));
            }
            let choice = choice.to_ascii_lowercase();
            if !matches!(choice.as_str(), "up" | "down")
                || choice == "up" && caster.z == 8
                || choice == "down" && caster.z == 7
            {
                return Ok(refusal("not_possible"));
            }
            let delta = if choice == "up" { -1 } else { 1 };
            let probe = if delta == -1 {
                caster.offset(0, 0, -1)?
            } else {
                caster.offset(facing.0, facing.1, 0)?
            };
            if tiles.get(&probe).is_some_and(|tile| {
                tile.ground.is_some()
                    || if delta == -1 {
                        tile.immovable_block_solid
                    } else {
                        tile.block_solid
                    }
            }) {
                return Ok(refusal("not_possible"));
            }
            let destination = caster.offset(facing.0, facing.1, delta)?;
            if !tiles.get(&destination).is_some_and(|tile| {
                tile.ground.is_some()
                    && !tile.immovable_block_solid
                    && !tile.floor_change
                    && tile.entry_permitted
            }) {
                return Ok(refusal("not_possible"));
            }
            destination
        }
        "rope_up" => {
            if parameters["failed_teleport_counts_as_success"].as_bool() != Some(true) {
                return Err(Error::InvalidParameters(
                    "failed_teleport_counts_as_success",
                ));
            }
            let ground = parameters["rope_ground_items"]
                .as_array()
                .ok_or(Error::InvalidParameters("rope_ground_items"))?;
            let top = parameters["rope_top_items"]
                .as_array()
                .ok_or(Error::InvalidParameters("rope_top_items"))?;
            for id in ground.iter().chain(top) {
                if id.as_u64().is_none() {
                    return Err(Error::InvalidParameters("rope_items"));
                }
            }
            let Some(origin) = tiles.get(&caster) else {
                return Ok(refusal("not_possible"));
            };
            let rope_spot = origin.ground.is_some_and(|id| {
                ground
                    .iter()
                    .any(|value| value.as_u64() == Some(u64::from(id)))
            }) || origin.top_items.iter().any(|id| {
                top.iter()
                    .any(|value| value.as_u64() == Some(u64::from(*id)))
            });
            if origin.ground.is_none() || !rope_spot {
                return Ok(refusal("not_possible"));
            }
            let mut destination = None;
            for (x, y) in [
                (0, 1),
                (0, -1),
                (1, 0),
                (-1, 0),
                (-1, 0),
                (-1, 1),
                (1, 1),
                (-1, -1),
                (1, -1),
            ] {
                let candidate = caster.offset(x, y, -1)?;
                if tiles.get(&candidate).is_some_and(|tile| {
                    tile.ground.is_some()
                        && !tile.block_solid
                        && !tile.block_projectile
                        && !tile.immovable_block_solid
                        && !tile.immovable_block_item
                        && !tile.immovable_nonfield_block_item
                }) {
                    destination = Some(candidate);
                    break;
                }
            }
            let fallback = caster.offset(0, 1, -1)?;
            let Some(destination) =
                destination.or_else(|| tiles.contains_key(&fallback).then_some(fallback))
            else {
                return Ok(refusal("not_enough_room"));
            };
            // The donor ignores teleportTo's result. The owner still checks current
            // entry permission at commit; a denied move must not refund this cast.
            destination
        }
        _ => return Err(Error::InvalidParameters("mode")),
    };
    Ok(HouseMovementPlan::Move {
        target_is_caster: true,
        destination,
        before_effect: if mode == "rope_up" { "poff" } else { "none" },
        after_effect: "teleport",
        failed_move_counts_as_success: mode == "rope_up",
    })
}

pub(crate) fn plan(
    parameters: &Value,
    facts: &HouseMovementFacts,
) -> Result<HouseMovementPlan, Error> {
    match facts {
        HouseMovementFacts::LocatePlayer {
            caster,
            caster_staff,
            target,
        } if string(parameters, "source")? == "online_player" => {
            player_plan(*caster, *caster_staff, target)
        }
        HouseMovementFacts::FindFiend {
            caster,
            fiends,
            now_seconds,
        } if string(parameters, "source")? == "nearest_fiendish" => {
            fiend_plan(*caster, fiends, *now_seconds)
        }
        HouseMovementFacts::HouseList {
            caster_house,
            access,
            front_door,
            own_door,
        } if string(parameters, "action")? == "edit_list" => {
            list_plan(parameters, *caster_house, *access, *front_door, *own_door)
        }
        HouseMovementFacts::HouseKick {
            caster_house,
            target_house,
            target_is_caster,
            caster_can_edit_guest,
            caster_access_in_target_house,
            target_access,
            target_can_edit_houses,
            caster_entry,
            target_entry,
        } if string(parameters, "action")? == "kick" => {
            let allowed = if *target_is_caster {
                caster_house.is_some()
            } else {
                caster_house.is_some()
                    && target_house.is_some()
                    && *caster_can_edit_guest
                    && caster_access_in_target_house >= target_access
                    && !target_can_edit_houses
            };
            if !allowed {
                return Ok(refusal("not_possible"));
            }
            let destination = if *target_is_caster {
                caster_entry
            } else {
                target_entry
            }
            .ok_or(Error::MissingDestination)?;
            Ok(HouseMovementPlan::Move {
                target_is_caster: *target_is_caster,
                destination,
                before_effect: "poff",
                after_effect: "teleport",
                failed_move_counts_as_success: true,
            })
        }
        HouseMovementFacts::VerticalMove {
            caster,
            facing,
            choice,
            tiles,
        } => movement_plan(parameters, *caster, *facing, choice, tiles),
        HouseMovementFacts::Appearance {
            outfit,
            illusionable,
            can_illusion_all,
        } if string(parameters, "source")? == "creature_name_parameter" => {
            let Some(outfit) = outfit else {
                return Ok(refusal("creature_does_not_exist"));
            };
            if !illusionable && !can_illusion_all {
                return Ok(refusal("not_possible"));
            }
            let duration_ms = parameters["duration_ms"]
                .as_u64()
                .filter(|value| *value > 0)
                .ok_or(Error::InvalidParameters("duration_ms"))?;
            Ok(HouseMovementPlan::Appearance {
                outfit: outfit.clone(),
                duration_ms,
                replace_existing: true,
                effect: "magic_red",
            })
        }
        _ => Err(Error::WrongFacts),
    }
}

#[cfg(test)]
mod tests {
    // Panicking assertions are confined to regression tests.
    #![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
    use serde_json::json;

    use super::*;

    fn position(x: i32, y: i32, z: i32) -> Position {
        Position { x, y, z }
    }

    fn ground() -> Tile {
        Tile {
            ground: Some(386),
            entry_permitted: true,
            ..Tile::default()
        }
    }

    #[test]
    fn player_location_uses_existing_rule_and_protects_staff() {
        let target = LocatedPlayer {
            name: "Example".into(),
            position: position(0, 251, 8),
            staff_access: false,
            exiva_permitted: true,
        };
        let parameters = json!({"source": "online_player"});
        let facts = HouseMovementFacts::LocatePlayer {
            caster: position(0, 0, 7),
            caster_staff: false,
            target: Some(target.clone()),
        };
        assert_eq!(
            plan(&parameters, &facts),
            Ok(HouseMovementPlan::Message {
                text: "Example is very far to the south.".into(),
                effect: "magic_blue"
            })
        );
        let facts = HouseMovementFacts::LocatePlayer {
            caster: position(0, 0, 7),
            caster_staff: false,
            target: Some(LocatedPlayer {
                staff_access: true,
                ..target
            }),
        };
        assert!(matches!(
            plan(&parameters, &facts),
            Ok(HouseMovementPlan::Refused {
                start_cooldown: true,
                cast_succeeds: false,
                ..
            })
        ));
    }

    #[test]
    fn fiend_uses_xyz_nearest_and_completed_bestiary() {
        let fiend = Fiend {
            position: position(3, 2, 0),
            live_fiendish: true,
            bestiary_completed: false,
            bestiary_required_kills: 25,
            reverts_at_seconds: 1090,
        };
        let facts = HouseMovementFacts::FindFiend {
            caster: position(0, 0, 0),
            fiends: vec![
                fiend,
                Fiend {
                    position: position(0, 0, 5),
                    live_fiendish: true,
                    bestiary_completed: true,
                    bestiary_required_kills: 25,
                    reverts_at_seconds: 5000,
                },
            ],
            now_seconds: 1000,
        };
        let HouseMovementPlan::Message { text, .. } =
            plan(&json!({"source": "nearest_fiendish"}), &facts).unwrap()
        else {
            panic!("message expected")
        };
        assert!(text.contains("standing next to you"));
        assert!(text.contains("\"Unknown\""));
        assert!(text.ends_with("1 minutes and 30 seconds."));
    }

    #[test]
    fn house_editor_denial_preserves_donor_success_boundary() {
        let facts = HouseMovementFacts::HouseList {
            caster_house: Some(1),
            access: HouseAccess::Subowner,
            front_door: None,
            own_door: Some(3),
        };
        assert!(matches!(
            plan(&json!({"action": "edit_list", "list": "guest"}), &facts),
            Ok(HouseMovementPlan::OpenHouseEditor {
                list: HouseList::Guest,
                evict_uninvited_on_save: true,
                ..
            })
        ));
        assert!(matches!(
            plan(&json!({"action": "edit_list", "list": "subowner"}), &facts),
            Ok(HouseMovementPlan::Refused {
                cast_succeeds: true,
                start_cooldown: true,
                ..
            })
        ));
        assert!(matches!(
            plan(&json!({"action": "edit_list", "list": "door"}), &facts),
            Ok(HouseMovementPlan::Refused {
                cast_succeeds: false,
                ..
            })
        ));
        let facts = HouseMovementFacts::HouseList {
            caster_house: Some(1),
            access: HouseAccess::Owner,
            front_door: None,
            own_door: Some(3),
        };
        assert!(matches!(
            plan(&json!({"action": "edit_list", "list": "door"}), &facts),
            Ok(HouseMovementPlan::OpenHouseEditor {
                list: HouseList::Door(3),
                ..
            })
        ));
    }

    #[test]
    fn cross_house_kick_obeys_target_house_rank() {
        let facts = HouseMovementFacts::HouseKick {
            caster_house: Some(1),
            target_house: Some(2),
            target_is_caster: false,
            caster_can_edit_guest: true,
            caster_access_in_target_house: HouseAccess::Owner,
            target_access: HouseAccess::Guest,
            target_can_edit_houses: false,
            caster_entry: Some(position(1, 1, 7)),
            target_entry: Some(position(5, 5, 7)),
        };
        assert!(matches!(
            plan(&json!({"action": "kick"}), &facts),
            Ok(HouseMovementPlan::Move {
                destination: Position { x: 5, y: 5, z: 7 },
                failed_move_counts_as_success: true,
                ..
            })
        ));
        let HouseMovementFacts::HouseKick {
            caster_house,
            target_house,
            target_is_caster,
            caster_can_edit_guest,
            target_access,
            target_can_edit_houses,
            caster_entry,
            target_entry,
            ..
        } = facts
        else {
            unreachable!()
        };
        let facts = HouseMovementFacts::HouseKick {
            caster_house,
            target_house,
            target_is_caster,
            caster_can_edit_guest,
            caster_access_in_target_house: HouseAccess::NotInvited,
            target_access,
            target_can_edit_houses,
            caster_entry,
            target_entry,
        };
        assert!(matches!(
            plan(&json!({"action": "kick"}), &facts),
            Ok(HouseMovementPlan::Refused { .. })
        ));
    }

    #[test]
    fn levitate_checks_surface_probe_and_current_entry_admission() {
        let caster = position(10, 10, 9);
        let destination = position(11, 10, 8);
        let mut tiles = BTreeMap::from([(destination, ground())]);
        let parameters = json!({"mode": "levitate"});
        let facts = HouseMovementFacts::VerticalMove {
            caster,
            facing: (1, 0),
            choice: "UP".into(),
            tiles: tiles.clone(),
        };
        assert!(matches!(
            plan(&parameters, &facts),
            Ok(HouseMovementPlan::Move {
                destination: Position { x: 11, y: 10, z: 8 },
                ..
            })
        ));
        tiles.get_mut(&destination).unwrap().entry_permitted = false;
        let facts = HouseMovementFacts::VerticalMove {
            caster,
            facing: (1, 0),
            choice: "up".into(),
            tiles,
        };
        assert!(matches!(
            plan(&parameters, &facts),
            Ok(HouseMovementPlan::Refused { .. })
        ));
        let facts = HouseMovementFacts::VerticalMove {
            caster: position(10, 10, 8),
            facing: (1, 0),
            choice: "up".into(),
            tiles: BTreeMap::new(),
        };
        assert!(matches!(
            plan(&parameters, &facts),
            Ok(HouseMovementPlan::Refused { .. })
        ));
    }

    #[test]
    fn rope_finds_north_then_uses_source_south_fallback() {
        let caster = position(10, 10, 9);
        let south = position(10, 11, 8);
        let north = position(10, 9, 8);
        let mut tiles = BTreeMap::from([
            (caster, ground()),
            (
                south,
                Tile {
                    block_solid: true,
                    ..ground()
                },
            ),
            (north, ground()),
        ]);
        let parameters = json!({"mode": "rope_up", "rope_ground_items": [386], "rope_top_items": [12935], "failed_teleport_counts_as_success": true});
        let facts = HouseMovementFacts::VerticalMove {
            caster,
            facing: (0, 1),
            choice: String::new(),
            tiles: tiles.clone(),
        };
        assert!(matches!(
            plan(&parameters, &facts),
            Ok(HouseMovementPlan::Move {
                destination: Position { x: 10, y: 9, z: 8 },
                ..
            })
        ));
        tiles.remove(&north);
        let facts = HouseMovementFacts::VerticalMove {
            caster,
            facing: (0, 1),
            choice: String::new(),
            tiles,
        };
        assert!(matches!(
            plan(&parameters, &facts),
            Ok(HouseMovementPlan::Move {
                destination: Position { x: 10, y: 11, z: 8 },
                ..
            })
        ));
    }

    #[test]
    fn appearance_retains_colours_and_checks_illusionability() {
        let outfit = AppearanceSelection {
            outfit_key: "creature:rat".to_owned(),
            colours: [1, 2, 3, 4],
            addons: 1,
            mount_key: None,
        };
        let parameters = json!({"source": "creature_name_parameter", "duration_ms": 180000});
        let denied = HouseMovementFacts::Appearance {
            outfit: Some(outfit.clone()),
            illusionable: false,
            can_illusion_all: false,
        };
        assert!(matches!(
            plan(&parameters, &denied),
            Ok(HouseMovementPlan::Refused { .. })
        ));
        let permitted = HouseMovementFacts::Appearance {
            outfit: Some(outfit.clone()),
            illusionable: false,
            can_illusion_all: true,
        };
        assert_eq!(
            plan(&parameters, &permitted),
            Ok(HouseMovementPlan::Appearance {
                outfit,
                duration_ms: 180000,
                replace_existing: true,
                effect: "magic_red"
            })
        );
    }

    #[test]
    fn malformed_parameters_and_coordinate_overflow_fail_closed() {
        let facts = HouseMovementFacts::LocatePlayer {
            caster: position(i32::MAX, 0, 7),
            caster_staff: true,
            target: Some(LocatedPlayer {
                name: "Target".into(),
                position: position(-1, 0, 7),
                staff_access: false,
                exiva_permitted: true,
            }),
        };
        assert_eq!(
            plan(&json!({}), &facts),
            Err(Error::InvalidParameters("source"))
        );
        assert_eq!(
            plan(&json!({"source": "online_player"}), &facts),
            Err(Error::PositionOverflow)
        );
    }

    #[test]
    fn all_nine_generated_profiles_plan_with_typed_owner_facts() {
        let document: Value = serde_json::from_str(include_str!(
            "../../../../tools/content-schema/spell-authoring/samples/native-spell-profiles.json"
        ))
        .unwrap();
        let mut checked = std::collections::BTreeSet::new();
        for profile in document["profiles"].as_array().unwrap() {
            let name = profile["name"].as_str().unwrap().to_ascii_lowercase();
            let caster = position(10, 10, 9);
            let facts = match name.as_str() {
                "find person" => HouseMovementFacts::LocatePlayer {
                    caster,
                    caster_staff: false,
                    target: Some(LocatedPlayer {
                        name: "Located".into(),
                        position: position(10, 261, 9),
                        staff_access: false,
                        exiva_permitted: true,
                    }),
                },
                "find fiend" => HouseMovementFacts::FindFiend {
                    caster,
                    now_seconds: 1000,
                    fiends: vec![Fiend {
                        position: position(13, 12, 9),
                        live_fiendish: true,
                        bestiary_completed: true,
                        bestiary_required_kills: 25,
                        reverts_at_seconds: 1200,
                    }],
                },
                "house guest list" | "house subowner list" | "house door list" => {
                    HouseMovementFacts::HouseList {
                        caster_house: Some(1),
                        access: HouseAccess::Owner,
                        front_door: Some(3),
                        own_door: None,
                    }
                }
                "house kick" => HouseMovementFacts::HouseKick {
                    caster_house: Some(1),
                    target_house: Some(1),
                    target_is_caster: true,
                    caster_can_edit_guest: false,
                    caster_access_in_target_house: HouseAccess::Guest,
                    target_access: HouseAccess::Guest,
                    target_can_edit_houses: false,
                    caster_entry: Some(position(15, 15, 9)),
                    target_entry: Some(position(15, 15, 9)),
                },
                "levitate" => HouseMovementFacts::VerticalMove {
                    caster,
                    facing: (1, 0),
                    choice: "up".into(),
                    tiles: BTreeMap::from([(position(11, 10, 8), ground())]),
                },
                "magic rope" => HouseMovementFacts::VerticalMove {
                    caster,
                    facing: (0, 1),
                    choice: String::new(),
                    tiles: BTreeMap::from([(caster, ground()), (position(10, 11, 8), ground())]),
                },
                "creature illusion" => HouseMovementFacts::Appearance {
                    outfit: Some(AppearanceSelection {
                        outfit_key: "creature:rat".into(),
                        colours: [0; 4],
                        addons: 0,
                        mount_key: None,
                    }),
                    illusionable: true,
                    can_illusion_all: false,
                },
                _ => continue,
            };
            let parameters = &profile["execution"]["native_behavior"]["parameters"];
            let result = plan(parameters, &facts).unwrap_or_else(|error| {
                panic!("{name} canonical recipe failed to plan: {error:?}")
            });
            assert!(
                !matches!(result, HouseMovementPlan::Refused { .. }),
                "{name}"
            );
            assert!(checked.insert(name));
        }
        assert_eq!(checked.len(), 9);
    }

    #[test]
    fn rope_denied_entry_does_not_refund_a_source_successful_cast() {
        let caster = position(10, 10, 9);
        let destination = position(10, 11, 8);
        let tiles = BTreeMap::from([
            (caster, ground()),
            (
                destination,
                Tile {
                    entry_permitted: false,
                    ..ground()
                },
            ),
        ]);
        let facts = HouseMovementFacts::VerticalMove {
            caster,
            facing: (0, 1),
            choice: String::new(),
            tiles,
        };
        let parameters = json!({"mode": "rope_up", "rope_ground_items": [386], "rope_top_items": [12935], "failed_teleport_counts_as_success": true});
        let result = plan(&parameters, &facts).unwrap();
        assert!(matches!(
            result,
            HouseMovementPlan::Move {
                failed_move_counts_as_success: true,
                ..
            }
        ));
        // The plan does not apply a move or authorize entry. Commit rechecks admission.
    }
}
