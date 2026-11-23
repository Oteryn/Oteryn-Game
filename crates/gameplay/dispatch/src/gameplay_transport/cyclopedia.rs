//! Basic Cyclopedia on the real admitted connection, owner option A.
use super::guild_intent::GuildReply;
use super::{ComposedFreshAdmission, capabilities::OfferedCapability, connection::AdmittedSession};
use crate::content::{CollisionClass, LogicalCell};
use crate::domain::cyclopedia::{BasicArea, MapAction, Point};
use crate::durability::DurabilityError;
use crate::durability::character_cyclopedia::{CyclopediaPremiumGate, DiscoveryRoster, MapRequest};
use crate::durability::character_progression::CharacterProgressionError;
use crate::durability::item_fee_burn::FeeChangeFacts;
use crate::durability::item_mint_audit::Type2Transaction;
use crate::durability::npc_purchase::{NpcPurchaseGate, PurchaseService};
use crate::foundation::{CommandId, CommandRef};
use oteryn_protocol_oteryn::cyclopedia as wire;
use sha2::{Digest, Sha256};

#[path = "cyclopedia/atlas.rs"]
mod atlas;
#[path = "cyclopedia/content.rs"]
mod content;
#[path = "cyclopedia/overview.rs"]
mod overview;
#[path = "cyclopedia/respawn.rs"]
mod respawn;
#[path = "cyclopedia/speed.rs"]
mod speed;
#[path = "cyclopedia/step.rs"]
mod step;

// Crystal 201189f8 discovery_geometry.lua:7-34, discovery_system.lua:528-537.
// Gnome Deep Hub and Water are map roots but are outside the discovery roster.
const DISCOVERY_MAINS: [u32; 26] = [
    1, 2, 3, 4, 5, 7, 8, 9, 11, 14, 16, 17, 18, 19, 20, 22, 23, 24, 25, 27, 305, 329, 352, 355,
    434, 466,
];

struct PinnedDiscoveryRoster {
    digest: [u8; 32],
    subareas: DiscoveryRoster,
}

pub(super) fn discovery_roster(map: &crate::map::boot::BundleMap) -> DiscoveryRoster {
    static PINNED: std::sync::Mutex<Option<PinnedDiscoveryRoster>> = std::sync::Mutex::new(None);
    let base = map.overlay.base();
    let digest = base.digest();
    let mut pinned = PINNED
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(roster) = pinned.as_ref().filter(|roster| roster.digest == digest) {
        return roster.subareas.clone();
    }
    let subareas: DiscoveryRoster = BasicArea::embedded()
        .map_or_else(Vec::new, |sampling| {
            discoverable_subareas(&base.subareas, &sampling, |pos| map.facts.doors().at(pos))
        })
        .into();
    *pinned = Some(PinnedDiscoveryRoster {
        digest,
        subareas: subareas.clone(),
    });
    subareas
}

pub(super) fn discoverable_subareas(
    table: &oteryn_world_bundle::subarea::Table,
    sampling: &BasicArea,
    excluded: impl Fn(crate::map::overlay::TilePos) -> bool,
) -> Vec<(u32, u32)> {
    let excluded = &excluded;
    let mut runs = std::collections::BTreeMap::<u32, Vec<_>>::new();
    for run in &table.runs {
        runs.entry(run.subarea).or_default().push(run);
    }
    table
        .areas
        .iter()
        .filter_map(|area| {
            let main = area
                .parent
                .filter(|id| DISCOVERY_MAINS.binary_search(id).is_ok())?;
            // Native runs are admitted walkable membership. Follow the discovery sampler's
            // run order, cap and door exclusion, and require an entry tile that can roll POIs.
            let cells = || {
                runs.get(&area.id).into_iter().flatten().flat_map(|run| {
                    (run.begin..run.end).filter_map(move |x| {
                        let pos = crate::map::overlay::TilePos {
                            x,
                            y: run.y,
                            floor: run.floor,
                        };
                        (!excluded(pos)).then_some(Point {
                            x: i32::from(pos.x),
                            y: i32::from(pos.y),
                            floor: i16::from(pos.floor),
                        })
                    })
                })
            };
            let pool: Vec<_> = cells().take(sampling.pool_limit).collect();
            (pool.len() >= sampling.generated_count
                && cells().any(|at| {
                    pool.iter()
                        .filter(|point| !point.in_window(at))
                        .take(sampling.generated_count)
                        .count()
                        == sampling.generated_count
                }))
            .then_some((main, area.id))
        })
        .collect()
}

pub(super) fn offered(base: &[OfferedCapability], available: bool) -> Vec<OfferedCapability> {
    let mut offers: Vec<_> = base
        .iter()
        .copied()
        .filter(|c| c.id != wire::CAPABILITY_CYCLOPEDIA_MAP_V1)
        .collect();
    if available {
        let at = offers.partition_point(|c| c.id < wire::CAPABILITY_CYCLOPEDIA_MAP_V1);
        offers.insert(
            at,
            OfferedCapability {
                id: wire::CAPABILITY_CYCLOPEDIA_MAP_V1,
                requires: &[],
            },
        );
    }
    offers
}
struct Premium<'a>(Option<&'a super::spell_premium_coordinator::SpellPremiumCoordinator>);
impl CyclopediaPremiumGate for Premium<'_> {
    async fn premium(
        &self,
        tx: &mut Type2Transaction<'_>,
        account: [u8; 16],
    ) -> Result<bool, DurabilityError> {
        match self.0 {
            Some(gate) => {
                gate.authorize(tx, account, &PurchaseService::Blessing(String::new()))
                    .await
            }
            None => Ok(false),
        }
    }
}
impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) fn cyclopedia_content(&self) -> Option<content::AdmittedArea> {
        content::admit(self)
    }
    pub(super) async fn cyclopedia_wire(
        &self,
        admitted: AdmittedSession,
        id: u64,
        kind: u32,
        payload: &[u8],
    ) -> GuildReply {
        if kind == wire::COMMAND_TYPE_CYCLOPEDIA_SPEED_QUERY {
            return self.cyclopedia_speed_wire(admitted, id, payload).await;
        }
        if kind == wire::COMMAND_TYPE_CYCLOPEDIA_QUERY && payload.first() == Some(&128) {
            return self.cyclopedia_atlas_wire(admitted, id, payload).await;
        }
        self.cyclopedia_basic_wire(admitted, id, kind, payload)
            .await
    }
    pub(super) async fn cyclopedia_basic_wire(
        &self,
        admitted: AdmittedSession,
        id: u64,
        kind: u32,
        payload: &[u8],
    ) -> GuildReply {
        if !admitted
            .continuity
            .selected_capabilities
            .contains(wire::CAPABILITY_CYCLOPEDIA_MAP_V1)
        {
            return GuildReply::Rejected;
        }
        let (Some(actor), Some(fence)) = (admitted.runtime_actor, admitted.item_fence) else {
            return GuildReply::Rejected;
        };
        if kind == wire::COMMAND_TYPE_CYCLOPEDIA_QUERY && payload.first() == Some(&255) {
            if CommandId::new(id).is_err() {
                return GuildReply::Rejected;
            }
            return atlas::dispatch(self, admitted, payload).await;
        }
        let Ok(intent) = wire::decode_request(kind, payload) else {
            return GuildReply::Rejected;
        };
        let Ok(id) = CommandId::new(id) else {
            return GuildReply::Rejected;
        };
        let trade_permit = if intent.operation == 4 {
            let Some(permit) = self.spell_lane_permit().await else {
                return GuildReply::Rejected;
            };
            if !self
                .states
                .player_trades
                .physical_service(admitted.game_session_id)
            {
                return GuildReply::Rejected;
            }
            Some(permit)
        } else {
            None
        };
        let Some(current) =
            super::character_gameplay_fence::current(self, admitted.game_session_id, false).await
        else {
            return GuildReply::Rejected;
        };
        if current.item_fence() != fence
            || fence.game_session_id != admitted.game_session_id
            || fence.connection_generation.get() != admitted.continuity.connection_generation
        {
            return GuildReply::Rejected;
        }
        let mut slot = self.revision_sequencer.acquire(fence.character_id).await;
        let mut runtime = self.runtime.lock().await;
        let binding = runtime.binding();
        if admitted.world_id != binding.world_id()
            || Some(admitted.channel_id) != binding.channel_id()
            || fence.runtime_scope != binding.runtime_scope()
            || fence.scope_ownership_generation != binding.scope_generation()
            || runtime.owner_fence().is_err()
            || !runtime
                .player_control_facts(actor, admitted.game_session_id)
                .is_ok_and(|f| f.control_loss.is_none())
        {
            return GuildReply::Rejected;
        }
        let Some(position) = super::bundle_map_position(&mut runtime, self.movement_cells(), actor)
        else {
            return GuildReply::Rejected;
        };
        let at = Point {
            x: position.at.x,
            y: position.at.y,
            floor: position.at.floor,
        };
        if intent.operation == 5 {
            return self
                .bundle_map()
                .and_then(|map| atlas::page(&map.overlay.base().subareas, intent.amount))
                .map(GuildReply::Payload)
                .unwrap_or(GuildReply::Rejected);
        }
        // Membership is resolved from the server-owned position, never the legacy wire alias.
        let Some(area) = content::admit_at(self, at) else {
            return GuildReply::Rejected;
        };
        let action = match intent.operation {
            0 => MapAction::Query,
            1 => MapAction::Start,
            2 => MapAction::Shuffle,
            3 => MapAction::Find,
            4 => MapAction::Donate(intent.amount),
            _ => return GuildReply::Rejected,
        };
        let derive =
            |tag: &[u8]| super::npc_trade_command::derived(tag, admitted.game_session_id, id.get());
        let fee_definitions = if matches!(action, MapAction::Donate(_)) {
            let Some((gold, platinum, crystal)) =
                super::npc_command::bank_coin_definitions(self, &runtime)
            else {
                return GuildReply::Rejected;
            };
            let Some(index) =
                super::item_ref_admission::pinned_index(&runtime, self.active_generation)
            else {
                return GuildReply::Rejected;
            };
            let Some(revision) = index.revision("oteryn:item.tibia.i2854") else {
                return GuildReply::Rejected;
            };
            let fallback = crate::durability::item_mint::TypedDefinitionRef {
                family: "Item".into(),
                production_key: "oteryn:item.tibia.i2854".into(),
                revision_ref: revision.into(),
            };
            Some((gold, platinum, crystal, fallback))
        } else {
            None
        };
        drop(runtime);
        let fee = if let Some((gold, platinum, crystal, fallback)) = fee_definitions {
            let backpack = match self
                .root
                .read_character_backpack(self.character, fence.character_id)
                .await
            {
                Ok(b) => b,
                Err(_) => return GuildReply::Unavailable,
            };
            let Some(native) = self
                .active_generation
                .and_then(crate::content::ActiveGeneration::native_gameplay)
            else {
                return GuildReply::Rejected;
            };
            let definition = backpack
                .as_ref()
                .map(|b| &b.backpack.definition)
                .unwrap_or(&fallback);
            let Ok(backpack) = super::item_ref_admission::generation_item_facts(native, definition)
            else {
                return GuildReply::Rejected;
            };
            Some(FeeChangeFacts {
                item_instance_ids: [
                    derive(b"cyclopedia:change:0"),
                    derive(b"cyclopedia:change:1"),
                ],
                gold,
                platinum,
                crystal,
                backpack,
            })
        } else {
            None
        };
        let mut hash = Sha256::new();
        hash.update(b"oteryn:cyclopedia-command:v1");
        hash.update(kind.to_be_bytes());
        hash.update(payload);
        let request = MapRequest {
            command: CommandRef::new(admitted.game_session_id, id),
            action,
            binding: hash.finalize().into(),
            occurrence: derive(b"cyclopedia:occurrence"),
            bundle: area.digest,
            area: area.area,
            legacy_area: area.legacy_area,
            subarea: area.subarea,
            subareas: area.subareas,
            milestone_subareas: discovery_roster(position.map),
            epoch: self.reset_epoch,
            pool: area.pool,
            at,
            fee,
            transaction: derive(b"cyclopedia:fee"),
            event: derive(b"cyclopedia:event"),
            now: super::npc_trade_command::unix_ms_now(),
        };
        {
            let mut runtime = self.runtime.lock().await;
            let binding = runtime.binding();
            if binding.runtime_scope() != fence.runtime_scope
                || binding.scope_generation() != fence.scope_ownership_generation
                || runtime.owner_fence().is_err()
                || !runtime
                    .player_control_facts(actor, admitted.game_session_id)
                    .is_ok_and(|facts| facts.control_loss.is_none())
                || super::bundle_map_position(&mut runtime, self.movement_cells(), actor)
                    .is_none_or(|position| {
                        position.at.x != at.x
                            || position.at.y != at.y
                            || position.at.floor != at.floor
                    })
            {
                return GuildReply::Rejected;
            }
        }
        let before_find = if action == MapAction::Find {
            match slot.cursor(self.root, self.character).await {
                Ok(revision) => Some(revision),
                Err(_) => return GuildReply::Unavailable,
            }
        } else {
            None
        };
        match slot
            .cyclopedia(
                self.root,
                self.character,
                self.holder,
                current,
                request,
                &Premium(self.premium_coordinator),
            )
            .await
        {
            Ok(bytes) => {
                let Ok(mut result) = wire::decode_retained_result(&bytes) else {
                    return GuildReply::Unavailable;
                };
                let discovered_now = before_find.is_some_and(|before| {
                    result.code == 0
                        && result.discovered
                        && result.found == 7
                        && result.revision > before.get()
                });
                for (_, _, floor) in &mut result.visible {
                    *floor = -*floor;
                }
                let Ok(bytes) = wire::encode_wire_result(&result) else {
                    return GuildReply::Unavailable;
                };
                if discovered_now {
                    self.states.tutorial_sessions.notice(
                        admitted.game_session_id,
                        oteryn_protocol_oteryn::tutorial_notices::TutorialNotice::StatusText(
                            "Subarea discovered! All seven points of interest found.".into(),
                        ),
                    );
                }
                drop(slot);
                // The donation is complete; respawn initialization acquires its own lane.
                drop(trade_permit);
                self.refresh_cyclopedia_speed(actor, admitted.game_session_id)
                    .await;
                if self.initialize_cyclopedia_respawn().await.is_err() {
                    return GuildReply::Unavailable;
                }
                GuildReply::Payload(bytes)
            }
            Err(
                CharacterProgressionError::AuthorityRejected
                | CharacterProgressionError::InvalidInput
                | CharacterProgressionError::ConflictingOccurrence,
            ) => GuildReply::Rejected,
            Err(_) => GuildReply::Unavailable,
        }
    }
}
macro_rules! authority_methods {
    () => {
        fn cyclopedia_available(&self) -> bool {
            false
        }
        fn cyclopedia_command(
            &self,
            _admitted: $crate::gameplay_transport::connection::AdmittedSession,
            _id: u64,
            _kind: u32,
            _payload: &[u8],
        ) -> impl std::future::Future<Output = $crate::gameplay_transport::guild_intent::GuildReply>
        {
            async { $crate::gameplay_transport::guild_intent::GuildReply::Rejected }
        }
    };
}
pub(super) use authority_methods;
macro_rules! composed_methods {
    () => {
        fn cyclopedia_available(&self) -> bool {
            self.cyclopedia_content().is_some()
        }
        async fn cyclopedia_command(
            &self,
            admitted: connection::AdmittedSession,
            id: u64,
            kind: u32,
            payload: &[u8],
        ) -> guild_intent::GuildReply {
            self.cyclopedia_wire(admitted, id, kind, payload).await
        }
    };
}
pub(super) use composed_methods;

#[cfg(test)]
mod milestone_tests {
    use super::*;
    use oteryn_world_bundle::subarea::{Area, Mask, Run, Table};

    #[test]
    fn discovery_milestone_roster_requires_playable_subareas()
    -> Result<(), Box<dyn std::error::Error>> {
        let areas = [
            (1, None),
            (3, None),
            (18, None),
            (51, Some(1)),
            (52, Some(1)),
            (53, Some(1)),
            (54, Some(3)),
            (55, Some(18)),
            (10, None),
            (441, Some(10)),
        ]
        .into_iter()
        .map(|(id, parent)| Area {
            id,
            key: format!("oteryn:content.area.fixture{id}"),
            name: format!("Area {id}"),
            parent,
            source_record_sha256: "a".repeat(64),
            town_id: None,
            hometown: false,
            temple: None,
        })
        .collect();
        let masks = [(51, 16), (53, 7), (54, 16), (55, 0), (441, 16)]
            .into_iter()
            .map(|(subarea, admitted_tiles)| Mask {
                subarea,
                admitted_tiles,
                source_tiles: 16,
                asset: String::new(),
                compressed_sha256: "a".repeat(64),
                bmp_sha256: "a".repeat(64),
                source_record_sha256: "a".repeat(64),
            })
            .collect();
        let runs = [(51, 16), (53, 7), (54, 16), (441, 16)]
            .into_iter()
            .enumerate()
            .map(|(y, (subarea, size))| Run {
                floor: -7,
                y: y as u16,
                begin: 0,
                end: size,
                subarea,
            })
            .collect();
        let table = Table {
            areas,
            masks,
            runs,
            ..Table::default()
        };
        let sampling = BasicArea::embedded().ok_or("discovery sampling catalogue")?;
        assert_eq!(
            discoverable_subareas(&table, &sampling, |_| false),
            [(1, 51), (3, 54)]
        );
        assert_eq!(
            discoverable_subareas(&table, &sampling, |pos| pos.x == 0 && pos.y == 0),
            [(3, 54)]
        );
        Ok(())
    }
}
