//! Position-scoped source entrances for in-Channel solo boss rooms.
use super::super::{ComposedFreshAdmission, SecureIdentifiers};
use crate::durability::{
    activity_handoff::ActivityRefusal, boss_encounter::channel_admission::ChannelBossEntry,
    spell_owner_commit::SpellLanePermit,
};
use crate::foundation::{
    ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementLocalPosition, MovementPositionSnapshot,
};
use oteryn_gameplay_encounter_rules::boss_room::BossRoomPolicy;
use std::time::Duration;

const COOLDOWN_MS: u64 = 10 * 60 * 60 * 1000;
const FIGHT_MS: u64 = 10 * 60 * 1000;

type Cell = (i32, i32, i16);
struct Room {
    key: &'static str,
    name: &'static str,
    minimum_level: u32,
    appearance: u16,
    trigger: Cell,
    landing: Cell,
    spawn: Cell,
    first: Cell,
    last: Cell,
    exit: Cell,
}

impl Room {
    fn source_present(&self, map: &crate::map::boot::BundleMap) -> bool {
        let (x, y, floor) = self.trigger;
        let (Ok(x), Ok(y), Ok(floor)) = (u16::try_from(x), u16::try_from(y), i8::try_from(-floor))
        else {
            return false;
        };
        let at = crate::map::overlay::TilePos { x, y, floor };
        let Some(tile) = map.overlay.base().tile(x, y, floor) else {
            return false;
        };
        tile.ids()
            .iter()
            .zip(tile.depths())
            .filter(|(_, depth)| **depth == 0)
            .enumerate()
            .any(|(ordinal, (id, _))| {
                u8::try_from(ordinal)
                    .ok()
                    .and_then(|ordinal| crate::map::view::placement_key(at, ordinal))
                    .is_some_and(|key| {
                        map.facts
                            .quest_callback_native_object(key, *id, self.appearance)
                    })
            })
    }

    fn contains(&self, at: MovementLocalPosition) -> bool {
        let (x, y, floor) = self.first;
        at.floor == floor && (x..=self.last.0).contains(&at.x) && (y..=self.last.1).contains(&at.y)
    }
}

// Crystal 201189f8 quests/grimvale/actions_portal_minis_grimvale.lua:1-65
// and actions_portal_minis_ancient_feud.lua:1-57. Ingress and egress cells already
// exist in the source map. Existing authored callbacks own the manual exits.
const ROOMS: &[Room] = &[
    Room {
        key: "black_vixen",
        name: "Black Vixen",
        minimum_level: 1,
        appearance: 25049,
        trigger: (33442, 32051, 9),
        landing: (33448, 32038, 9),
        spawn: (33450, 32034, 9),
        first: (33442, 32027, 9),
        last: (33456, 32041, 9),
        exit: (33442, 32052, 9),
    },
    Room {
        key: "bloodback",
        name: "Bloodback",
        minimum_level: 1,
        appearance: 25049,
        trigger: (33167, 31977, 8),
        landing: (33182, 32012, 8),
        spawn: (33184, 32016, 8),
        first: (33174, 32007, 8),
        last: (33191, 32020, 8),
        exit: (33167, 31978, 8),
    },
    Room {
        key: "darkfang",
        name: "Darkfang",
        minimum_level: 1,
        appearance: 25049,
        trigger: (33055, 31910, 9),
        landing: (33055, 31889, 9),
        spawn: (33062, 31890, 9),
        first: (33050, 31883, 9),
        last: (33066, 31896, 9),
        exit: (33055, 31911, 9),
    },
    Room {
        key: "katex_blood_tongue",
        name: "Katex Blood Tongue",
        minimum_level: 250,
        appearance: 34104,
        trigger: (33114, 32252, 12),
        landing: (33149, 32283, 12),
        spawn: (33152, 32289, 12),
        first: (33145, 32279, 12),
        last: (33159, 32293, 12),
        exit: (33115, 32252, 12),
    },
    Room {
        key: "shadowpelt",
        name: "Shadowpelt",
        minimum_level: 1,
        appearance: 25050,
        trigger: (33402, 32097, 9),
        landing: (33395, 32112, 9),
        spawn: (33384, 32114, 9),
        first: (33376, 32107, 9),
        last: (33396, 32119, 9),
        exit: (33403, 32097, 9),
    },
    Room {
        key: "sharpclaw",
        name: "Sharpclaw",
        minimum_level: 1,
        appearance: 25049,
        trigger: (33128, 31971, 9),
        landing: (33121, 31998, 9),
        spawn: (33120, 32002, 9),
        first: (33113, 31994, 9),
        last: (33126, 32007, 9),
        exit: (33128, 31972, 9),
    },
    Room {
        key: "srezz_yellow_eyes",
        name: "Srezz Yellow Eyes",
        minimum_level: 250,
        appearance: 34105,
        trigger: (33131, 32252, 12),
        landing: (33120, 32278, 12),
        spawn: (33122, 32285, 12),
        first: (33115, 32275, 12),
        last: (33127, 32290, 12),
        exit: (33130, 32252, 12),
    },
    Room {
        key: "utua_stone_sting",
        name: "Utua Stone Sting",
        minimum_level: 250,
        appearance: 34107,
        trigger: (33123, 32265, 12),
        landing: (33087, 32240, 12),
        spawn: (33087, 32245, 12),
        first: (33082, 32237, 12),
        last: (33091, 32252, 12),
        exit: (33123, 32264, 12),
    },
    Room {
        key: "yirkas_blue_scales",
        name: "Yirkas Blue Scales",
        minimum_level: 250,
        appearance: 34106,
        trigger: (33123, 32239, 12),
        landing: (33154, 32246, 12),
        spawn: (33154, 32252, 12),
        first: (33150, 32242, 12),
        last: (33164, 32260, 12),
        exit: (33123, 32240, 12),
    },
];

fn local((x, y, floor): Cell) -> MovementLocalPosition {
    MovementLocalPosition { x, y, floor }
}

#[derive(Default)]
pub(in crate::gameplay_transport) struct ChannelBossRooms {
    deadlines: std::sync::Mutex<std::collections::BTreeMap<usize, tokio::time::Instant>>,
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn step_channel_boss_room(
        &self,
        permit: &SpellLanePermit,
        actor: ExactActorRef,
        session: GameSessionId,
        origin: MovementPositionSnapshot,
        entered: MovementPositionSnapshot,
    ) -> MovementPositionSnapshot {
        let Some((index, room)) = ROOMS
            .iter()
            .enumerate()
            .find(|(_, room)| local(room.trigger) == entered.position())
        else {
            return entered;
        };
        let Some(map) = self.bundle_map() else {
            return entered;
        };
        if !matches!(
            actor.runtime_scope(),
            crate::foundation::RuntimeScopeRefV1::Channel { .. }
        ) {
            return entered;
        }
        if !room.source_present(map) {
            return entered;
        }
        let blockers = crate::map::floor::BundleDoors(&map.facts);
        let failure = match self
            .admit_channel_boss_room(permit, actor, session, entered, index, room)
            .await
        {
            Ok(arrived) => return arrived,
            Err(ActivityRefusal::NoCapacity) => {
                format!("There's someone fighting with {}.", room.name)
            }
            Err(ActivityRefusal::Level) => format!(
                "All the players need to be level {} or higher.",
                room.minimum_level
            ),
            Err(ActivityRefusal::Cooldown) => {
                format!("You have to wait 10 hours to face {} again!", room.name)
            }
            Err(_) => "Sorry, not possible.".into(),
        };
        self.states.chat.push_quest_feedback(session, &failure);
        let mut runtime = self.runtime.lock().await;
        let Ok(proof) = crate::map::floor::prepare_quest_encounter_ejection(
            &runtime,
            self.movement_cells(),
            actor,
            session,
            entered,
            origin.position(),
            &blockers,
        ) else {
            return entered;
        };
        runtime.commit_source_step(proof).unwrap_or(entered)
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "binds the native StepIn to its source room"
    )]
    async fn admit_channel_boss_room(
        &self,
        permit: &SpellLanePermit,
        actor: ExactActorRef,
        session: GameSessionId,
        entered: MovementPositionSnapshot,
        index: usize,
        room: &Room,
    ) -> Result<MovementPositionSnapshot, ActivityRefusal> {
        let invalid = ActivityRefusal::ContentUnavailable;
        let stale = ActivityRefusal::AuthorityRejected;
        let map = self.bundle_map().ok_or(invalid)?;
        let blockers = crate::map::floor::BundleDoors(&map.facts);
        let fence = super::super::character_gameplay_fence::current(self, session, false)
            .await
            .ok_or(stale)?
            .item_fence();
        let creature = format!("oteryn:creature.{}", room.key);
        let boss = format!("oteryn:boss.{}", room.key);
        let spawn = local(room.spawn);
        let qualify =
            |runtime: &ChannelRuntimeV1, spells: &super::super::actor_spell::ChannelSpellStates| {
                if runtime.owner_fence().is_err()
                    || runtime.binding().runtime_scope() != fence.runtime_scope
                    || runtime.binding().scope_generation() != fence.scope_ownership_generation
                    || runtime.read_actor_position(actor) != Ok(entered)
                    || spells.is_dead(actor)
                    || !spells.spell_idle(runtime, actor, session)
                    || !runtime
                        .bound_attacker_lease(actor, session)
                        .is_ok_and(|lease| {
                            lease.character_id().as_bytes() == fence.character_id.as_bytes()
                                && lease.generation() == fence.character_lease_generation
                        })
                {
                    return Err(stale);
                }
                if !room.source_present(map) {
                    return Err(invalid);
                }
                let census = runtime.positioned_melee_census().map_err(|_| stale)?;
                if census
                    .iter()
                    .any(|(_, at, session)| session.is_some() && room.contains(at.position()))
                {
                    return Err(ActivityRefusal::NoCapacity);
                }
                let monsters: Vec<_> = census
                    .into_iter()
                    .filter_map(|(actor, at, session)| {
                        (session.is_none() && room.contains(at.position())).then_some(actor)
                    })
                    .collect();
                if !monsters.iter().all(|monster| {
                    runtime
                        .encounter_actor_index(*monster)
                        .is_ok_and(|slot| runtime.assert_encounter_slot_unreserved(slot).is_ok())
                }) {
                    return Err(invalid);
                }
                if self.movement_cells().index().lookup(
                    self.movement_cells().scope(),
                    crate::content::LogicalCell {
                        x: spawn.x,
                        y: spawn.y,
                        z: i32::from(spawn.floor),
                    },
                ) != Ok(crate::content::CollisionClass::Walkable)
                {
                    return Err(invalid);
                }
                let policy = runtime.companion_policy(&creature).map_err(|_| invalid)?;
                if policy.definition_key != creature {
                    return Err(invalid);
                }
                Ok((policy.maximum_health, monsters))
            };
        let prepare =
            |runtime: &ChannelRuntimeV1, spells: &super::super::actor_spell::ChannelSpellStates| {
                crate::map::floor::prepare_quest_callback_teleport(
                    runtime,
                    self.movement_cells(),
                    actor,
                    session,
                    entered,
                    local(room.landing),
                    super::super::pvp_live::pz_blocked(spells, actor, session, self.owner_now()),
                    &blockers,
                )
                .map_err(|_| invalid)
            };
        {
            let runtime = self.runtime.lock().await;
            let spells = self.states.spell_states.lock().await;
            qualify(&runtime, &spells)?;
            if !self.states.player_trades.physical(session, None) {
                return Err(stale);
            }
            prepare(&runtime, &spells)?;
        }
        let policy = BossRoomPolicy {
            minimum_players: 1,
            maximum_players: 1,
            minimum_level: room.minimum_level,
            cooldown_ms: COOLDOWN_MS,
            time_limit_ms: FIGHT_MS,
            lever_positions: [room.trigger].into(),
        };
        let occurrence = SecureIdentifiers.source_think_event_id().ok_or(stale)?;
        let spend = || {
            self.root.admit_channel_boss(
                self.character,
                self.holder,
                fence,
                permit,
                ChannelBossEntry {
                    boss_key: &boss,
                    policy: &policy,
                    trigger: room.trigger,
                    occurrence,
                    reset_epoch: self.reset_epoch,
                },
            )
        };
        let spend = match spend().await {
            Ok(outcome) => outcome?,
            Err(_) => spend().await.map_err(|_| stale)??,
        };
        let installed = async {
            let mut runtime = self.runtime.lock().await;
            let spells = self.states.spell_states.lock().await;
            let (maximum_health, monsters) = qualify(&runtime, &spells)?;
            let proof = prepare(&runtime, &spells)?;
            let mut deadlines = self
                .states
                .channel_boss_rooms
                .deadlines
                .lock()
                .map_err(|_| stale)?;
            let (positions, ()) = runtime
                .commit_quest_encounter_with_phase(
                    vec![proof],
                    &[(&creature, maximum_health, spawn)],
                    |runtime, _| {
                        for monster in monsters {
                            runtime.remove_encounter_actor(monster)?;
                        }
                        Ok(())
                    },
                    || true,
                )
                .map_err(|_| invalid)?;
            deadlines.insert(
                index,
                tokio::time::Instant::now() + Duration::from_millis(FIGHT_MS),
            );
            positions.into_iter().next().ok_or(invalid)
        }
        .await;
        if installed.is_err() {
            self.root
                .release_channel_boss_cooldown(self.character, self.holder, permit, spend)
                .await
                .map_err(|_| stale)?;
        }
        installed
    }

    pub(in crate::gameplay_transport) async fn channel_boss_timeout_pass(
        &self,
    ) -> Result<bool, crate::monster_owner_cycle::respawn_job::RespawnExit> {
        let Some(map) = self.bundle_map() else {
            return Ok(true);
        };
        let due = tokio::time::Instant::now();
        if self
            .states
            .channel_boss_rooms
            .deadlines
            .lock()
            .is_ok_and(|deadlines| deadlines.values().all(|deadline| *deadline > due))
        {
            return Ok(true);
        }
        let Some(_permit) = self.spell_lane_permit().await else {
            return Ok(false);
        };
        let mut runtime = self.runtime.lock().await;
        runtime
            .owner_fence()
            .map_err(|_| crate::monster_owner_cycle::respawn_job::RespawnExit::StaleOwner)?;
        let Ok(mut deadlines) = self.states.channel_boss_rooms.deadlines.lock() else {
            return Ok(false);
        };
        let now = tokio::time::Instant::now();
        let blockers = crate::map::floor::BundleDoors(&map.facts);
        let mut completed = true;
        deadlines.retain(|index, deadline| {
            if *deadline > now {
                return true;
            }
            let Some(room) = ROOMS.get(*index) else {
                return false;
            };
            let Ok(census) = runtime.positioned_melee_census() else {
                completed = false;
                return true;
            };
            let mut pending = false;
            for (actor, at, session) in census {
                if let Some(session) = session
                    && room.contains(at.position())
                {
                    let moved = crate::map::floor::prepare_quest_encounter_ejection(
                        &runtime,
                        self.movement_cells(),
                        actor,
                        session,
                        at,
                        local(room.exit),
                        &blockers,
                    )
                    .ok()
                    .and_then(|proof| runtime.commit_source_step(proof).ok());
                    pending |= moved.is_none();
                }
            }
            completed &= !pending;
            pending
        });
        Ok(completed)
    }
}
