//! Shared world MINT component; seeded mission acceptance does not qualify the quest.
use super::fixture::{Harness, debug, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn questpc5_nimmersatt_world_supply_mints_twenty_snowballs_with_one_quest_receipt() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-nimmersatt-world-items")
}
async fn count(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i2992' AND lifecycle=1").fetch_one(&h.pool).await?)
}
pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    supply(owner, admitted, h, true).await
}
async fn supply(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    seed_acceptance: bool,
) -> TestResult {
    use crate::durability::quest_state::{
        QuestCause, QuestTransitionOutcome, QuestTransitionRequest,
    };
    use crate::foundation::{CommandId, CommandRef};
    use oteryn_gameplay_quests::quest::{
        QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTransition,
    };
    let started = std::time::Instant::now();
    let callback = oteryn_gameplay_quests::quest::world_callbacks::load()?
        .into_iter()
        .find(|c| c.key == "oteryn:quest-callback/authored-nimmersatt-snowbank")
        .ok_or("missing Nimmersatt world MINT callback")?;
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    let mut selected = admitted
        .continuity
        .selected_capabilities
        .as_slice()
        .to_vec();
    selected.extend([3, 6, 25]);
    selected.sort_unstable();
    selected.dedup();
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered, &selected)
            .ok_or("capabilities")?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    owner
        .runtime
        .lock()
        .await
        .bind_attacker_lease(
            admitted.runtime_actor.ok_or("actor")?,
            admitted.game_session_id,
            crate::foundation::CharacterLease::new(
                crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                    .map_err(debug)?,
                fence.character_lease_generation,
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;
    if seed_acceptance {
        let original = owner.quest_catalogue.as_ref().ok_or("catalogue")?;
        let tracks = callback
            .storage
            .iter()
            .map(|s| original.track(&s.track).cloned().ok_or("track"))
            .collect::<Result<Vec<_>, _>>()?;
        let setup = "oteryn:quest-transition/fixture-nimmersatt-accept";
        let mut transitions = vec![
            original
                .transition(&callback.key)
                .cloned()
                .ok_or("world transition")?,
            QuestTransition {
                key: setup.into(),
                quest: callback.quest.clone(),
                effects: vec![QuestEffect {
                    track: "oteryn:quest-progress/nimmersatt/stage".into(),
                    from: QuestComparison::Eq(-1),
                    effect: QuestEffectKind::Set(0),
                }],
                completes: false,
                experience: None,
            },
        ];
        if let Some(wall) = original.transition("oteryn:quest-callback/authored-nimmersatt-wall") {
            transitions.push(wall.clone());
        }
        let catalogue = std::sync::Arc::new(
            QuestStateCatalogue::new(original.content_revision(), tracks, transitions)
                .map_err(debug)?,
        );
        owner.quest_catalogue = Some(std::sync::Arc::clone(&catalogue));
        let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
        let result = slot
            .commit_quest_transition(
                owner.root,
                owner.character,
                owner.holder,
                fence,
                QuestTransitionRequest {
                    transition_key: setup.into(),
                    cause: QuestCause::Command(CommandRef::new(
                        admitted.game_session_id,
                        CommandId::new(9000).map_err(debug)?,
                    )),
                },
                catalogue,
            )
            .await
            .map_err(debug)?;
        assert!(matches!(result, QuestTransitionOutcome::Committed(_)));
        drop(slot);
        owner
            .refresh_quest_session(admitted.game_session_id, false)
            .await;
    }
    super::quest_catalogue_chests::relocate_source_actor(
        owner,
        admitted,
        admitted.runtime_actor.ok_or("actor")?,
        Position {
            x: 32232,
            y: 31579,
            floor: -7,
        },
    )
    .await?;
    assert_eq!(count(h).await?, 0);
    for first in [true, false] {
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(2))
            .ok_or("clock")?;
        let map = owner.bundle_map().ok_or("map")?;
        let key = oteryn_world_bundle::bundle::placement_key(-7, 32232, 31578, 0).ok_or("bank")?;
        let mut placement = map.overlay.base().digest().to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        let target = oteryn_protocol_oteryn::world_object::WorldObjectTarget {
            placement,
            expected_revision: map.facts.pinned().object_revision(key),
        };
        let command = admitted.continuity.next_command_id;
        let reply = super::quest_world_callbacks::wire(
            owner,
            admitted,
            command,
            2,
            oteryn_protocol_oteryn::world_object::encode_use_intent(&target).map_err(debug)?,
        )
        .await?;
        let result =
            oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)?;
        if first {
            assert_eq!(
                result,
                oteryn_protocol_oteryn::world_object::UseDisposition::Committed
            );
        }
        assert_eq!(
            count(h).await?,
            20,
            "one bounded world reward and no second value grant"
        );
    }
    let sources:i64=sqlx::query_scalar("SELECT count(*) FROM game_quest_reward_sources WHERE transition_key=$1 AND plan #>> '{world_callback,key}'=$1").bind(&callback.key).fetch_one(&h.pool).await?;
    assert_eq!(sources, 1, "same-transaction world reward source evidence");
    eprintln!(
        "POSTGRES_CASE_COMPLETED nimmersatt_world_items duration={:?}",
        started.elapsed()
    );
    Ok(())
}

pub(super) fn world() -> TestResult<crate::map::boot::BundleWorld> {
    use oteryn_world_bundle::{bundle, sector};
    let (bytes, mut pins) = crate::map::boot::tests::bundle();
    let mut data = bundle::read(&bytes)?;
    data.manifest.palette = vec![bundle::PaletteEntry {
        key: "oteryn:terrain.tibia.i799".into(),
        family: bundle::Family::Terrain,
        id: 0,
        terrain: Some(bundle::Terrain {
            kind: bundle::TerrainKind::Ground,
            walkable: Some(true),
            ground_speed: Some(160),
        }),
    }];
    data.manifest.palette.push(bundle::PaletteEntry {
        key: "oteryn:terrain.tibia.i1441".into(),
        family: bundle::Family::Terrain,
        id: 1,
        terrain: Some(bundle::Terrain {
            kind: bundle::TerrainKind::Common,
            walkable: None,
            ground_speed: None,
        }),
    });
    let mut grouped = std::collections::BTreeMap::<(u16, u16), Vec<sector::Tile>>::new();
    for y in 31577..=31583 {
        for x in 32231..=32237 {
            grouped
                .entry((y / 32, x / 32))
                .or_default()
                .push(sector::Tile {
                    x,
                    y,
                    flags: 0,
                    house: 0,
                    zones: Vec::new(),
                    items: vec![sector::Item {
                        palette: 0,
                        depth: 0,
                        attrs: sector::Attrs::default(),
                    }],
                });
        }
    }
    let sectors = grouped
        .into_iter()
        .map(|((sy, sx), tiles)| bundle::Sector {
            floor: -7,
            sx,
            sy,
            tiles,
        })
        .collect::<Vec<_>>();
    data.manifest.world = bundle::Extent {
        min_x: 32231,
        min_y: 31577,
        max_x: 32238,
        max_y: 31584,
        floors: vec![-7],
    };
    let bytes = oteryn_world_bundle::writer::write(&data.manifest, &sectors, &Default::default())?;
    pins.digest = bundle::read(&bytes)?.digest;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    crate::map::boot::boot(
        &bytes,
        &crate::map::boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start: crate::map::overlay::TilePos {
                x: 32231,
                y: 31578,
                floor: -7,
            },
        },
        crate::foundation::WorldId::decode(&id(42)).map_err(debug)?,
        crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
        |_| None,
    )
    .map_err(debug)
}

#[test]
fn questpc5_nimmersatt_twenty_exact_snowball_uses_open_wall_only_on_committed_last_hit()
-> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-nimmersatt-wall")
}
async fn wall_appearance(owner: &ComposedFreshAdmission<'_, '_, '_>) -> TestResult<u16> {
    let map = owner.bundle_map().ok_or("map")?;
    let facts = map.facts.pinned();
    let tile = crate::map::view::compose_tile(
        &map.overlay,
        &facts,
        crate::map::overlay::TilePos {
            x: 32236,
            y: 31582,
            floor: -7,
        },
    )
    .map_err(debug)?
    .ok_or("wall tile")?;
    Ok(tile
        .entries
        .first()
        .ok_or("wall entry")?
        .facts
        .appearance_id)
}
async fn hits(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key='oteryn:quest-progress/nimmersatt/wall_hits'").bind(id(41).as_slice()).fetch_one(&h.pool).await?)
}
pub(super) async fn exercise_wall(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    wall(owner, admitted, h, true).await
}
async fn wall(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    seed_acceptance: bool,
) -> TestResult {
    use oteryn_protocol_oteryn::world_object::{
        MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget,
        encode_item_use_intent,
    };
    let started = std::time::Instant::now();
    assert!(
        oteryn_gameplay_quests::quest::world_callbacks::load()?
            .iter()
            .any(|c| c.key == "oteryn:quest-callback/authored-nimmersatt-wall"),
        "missing bounded counted-use wall callback"
    );
    supply(owner, admitted, h, seed_acceptance).await?;
    assert_eq!(hits(h).await?, 0);
    assert_eq!(wall_appearance(owner).await?, 1441);
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    let mut selected = admitted
        .continuity
        .selected_capabilities
        .as_slice()
        .to_vec();
    selected.extend([4, 6, 11, 15, 18]);
    selected.sort_unstable();
    selected.dedup();
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered, &selected)
            .ok_or("item use capabilities")?;
    super::quest_catalogue_chests::relocate_source_actor(
        owner,
        admitted,
        admitted.runtime_actor.ok_or("actor")?,
        Position {
            x: 32235,
            y: 31582,
            floor: -7,
        },
    )
    .await?;
    let source = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .and_then(|n| n.item_index())
        .and_then(|i| i.definition_ref("Item", "oteryn:item.tibia.i2992", "definition-r1"))
        .ok_or("snowball ref")?;
    let mut last = Vec::new();
    for hit in 1..=20 {
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(2))
            .ok_or("clock")?;
        let map = owner.bundle_map().ok_or("map")?;
        let key = oteryn_world_bundle::bundle::placement_key(-7, 32236, 31582, 0).ok_or("wall")?;
        let mut placement = map.overlay.base().digest().to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        let intent = UseIntent {
            target: UseTarget::Item(std::num::NonZeroU64::new(3).ok_or("handle")?),
            use_with: None,
            use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget {
                placement,
                expected_revision: map.facts.pinned().object_revision(key),
            })),
            use_with_position: None,
        };
        if hit == 1 {
            let Some(MapUseTarget::Object(target)) = intent.use_with_map_item.clone() else {
                return Err("wall target".into());
            };
            let plain = UseIntent {
                target: UseTarget::WorldObject(target),
                use_with: None,
                use_with_map_item: None,
                use_with_position: None,
            };
            let command = admitted.continuity.next_command_id;
            let (next, result, _) = super::item_craft::exchange(
                owner,
                *admitted,
                command,
                &encode_item_use_intent(&plain).map_err(debug)?,
                None,
            )
            .await?;
            *admitted = next;
            assert_ne!(
                result,
                UseDisposition::Committed,
                "plain wall click must not infer an input"
            );
            assert_eq!(hits(h).await?, 0);
            assert_eq!(count(h).await?, 20);
            assert_eq!(wall_appearance(owner).await?, 1441);
            let premium = owner.premium_coordinator.take();
            let command = admitted.continuity.next_command_id;
            let (next, result, _) = super::item_craft::exchange(
                owner,
                *admitted,
                command,
                &encode_item_use_intent(&intent).map_err(debug)?,
                Some(source),
            )
            .await?;
            *admitted = next;
            assert_ne!(
                result,
                UseDisposition::Committed,
                "current Premium is required for every value-bearing hit"
            );
            assert_eq!(hits(h).await?, 0);
            assert_eq!(count(h).await?, 20);
            owner.premium_coordinator = premium;
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(std::time::Duration::from_secs(2))
                .ok_or("clock")?;
        }
        let command = admitted.continuity.next_command_id;
        let (next, result, payload) = super::item_craft::exchange(
            owner,
            *admitted,
            command,
            &encode_item_use_intent(&intent).map_err(debug)?,
            Some(source),
        )
        .await?;
        *admitted = next;
        last = payload;
        assert_eq!(result, UseDisposition::Committed, "exact snowball hit{hit}");
        assert_eq!(hits(h).await?, hit);
        assert_eq!(count(h).await?, 20 - hit);
        assert_eq!(
            wall_appearance(owner).await?,
            if hit == 20 { 799 } else { 1441 },
            "map opens only after the committed twentieth input"
        );
    }
    let command = admitted.continuity.next_command_id;
    let (next, result, _) =
        super::item_craft::exchange(owner, *admitted, command, &last, None).await?;
    *admitted = next;
    assert_ne!(result, UseDisposition::Committed);
    assert_eq!(hits(h).await?, 20);
    assert_eq!(count(h).await?, 0);
    eprintln!(
        "POSTGRES_CASE_COMPLETED nimmersatt_counted_wall duration={:?}",
        started.elapsed()
    );
    Ok(())
}

#[test]
fn questpc5_nimmersatt_rascalio_premium_twenty_hits_and_once_breath() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-nimmersatt-whole")
}

#[test]
fn questpc5_nimmersatt_rascalio_premium_passage() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-nimmersatt-passage")
}

pub(super) async fn exercise_passage(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult<oteryn_protocol_oteryn::world_spatial_entities::EntityRef> {
    use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;
    assert!(
        oteryn_gameplay_quests::quest::reward_callbacks::load()?
            .iter()
            .any(|callback| callback.key == "oteryn:quest-reward/nimmersatt-passage"),
        "missing Rascalio Premium passage callback"
    );
    let [npc] =
        super::classic_quests::install_source(owner, admitted, ["oteryn:npc.rascalio"]).await?;
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    let mut selected = admitted
        .continuity
        .selected_capabilities
        .as_slice()
        .to_vec();
    selected.extend([3, 6, 25]);
    selected.sort_unstable();
    selected.dedup();
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered, &selected)
            .ok_or("NPC capabilities")?;
    super::quest_catalogue_chests::relocate_source_actor(
        owner,
        admitted,
        admitted.runtime_actor.ok_or("actor")?,
        Position {
            x: 32231,
            y: 31582,
            floor: -7,
        },
    )
    .await?;
    let premium = owner.premium_coordinator.take();
    let refused = say(owner, admitted, npc, "passage").await?;
    assert_ne!(
        refused,
        NpcIntentDisposition::Ok,
        "missing Premium registration must refuse passage"
    );
    let states:i64=sqlx::query_scalar("SELECT count(*) FROM game_character_quest_tracks WHERE track_key='oteryn:quest-progress/nimmersatt/stage'").fetch_one(&h.pool).await?;
    assert_eq!(states, 0);
    owner.premium_coordinator = premium;
    assert_eq!(
        say(owner, admitted, npc, "passage").await?,
        NpcIntentDisposition::Ok
    );
    let stage:i64=sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key='oteryn:quest-progress/nimmersatt/stage'").fetch_one(&h.pool).await?;
    assert_eq!(stage, 0);
    let _ = say(owner, admitted, npc, "passage").await?;
    let receipts:i64=sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE transition_key='oteryn:quest-reward/nimmersatt-passage'").fetch_one(&h.pool).await?;
    assert_eq!(receipts, 1);
    assert_eq!(count(h).await?, 0);
    Ok(npc)
}

pub(super) async fn exercise_whole(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;
    let npc = exercise_passage(owner, admitted, h).await?;
    let _ = say(owner, admitted, npc, "breath").await?;
    assert_eq!(breath(h).await?, 0, "no reward before the twentieth hit");
    wall(owner, admitted, h, false).await?;
    super::quest_catalogue_chests::relocate_source_actor(
        owner,
        admitted,
        admitted.runtime_actor.ok_or("actor")?,
        Position {
            x: 32231,
            y: 31582,
            floor: -7,
        },
    )
    .await?;
    let premium = owner.premium_coordinator.take();
    assert_ne!(
        say(owner, admitted, npc, "breath").await?,
        NpcIntentDisposition::Ok
    );
    assert_eq!(
        breath(h).await?,
        0,
        "expired or unavailable Premium cannot grant the terminal reward"
    );
    owner.premium_coordinator = premium;
    assert_eq!(
        say(owner, admitted, npc, "breath").await?,
        NpcIntentDisposition::Ok
    );
    assert_eq!(breath(h).await?, 1);
    let _ = say(owner, admitted, npc, "breath").await?;
    assert_eq!(breath(h).await?, 1);
    let stage:i64=sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key='oteryn:quest-progress/nimmersatt/stage'").fetch_one(&h.pool).await?;
    assert_eq!(stage, 2);
    let completions:i64=sqlx::query_scalar("SELECT count(*) FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key='oteryn:quest.authored.nimmersatt_s_lair' AND completed_character_revision IS NOT NULL").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        completions, 1,
        "terminal receipt completes the selected personal quest"
    );
    Ok(())
}
async fn breath(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i44179' AND lifecycle=1").fetch_one(&h.pool).await?)
}

async fn say(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    npc: oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
    name: &str,
) -> TestResult<oteryn_protocol_oteryn::npc_service::NpcIntentDisposition> {
    let callback = oteryn_gameplay_quests::quest::reward_callbacks::load()?
        .into_iter()
        .find(|callback| callback.key == format!("oteryn:quest-reward/nimmersatt-{name}"))
        .ok_or("Nimmersatt dialogue")?;
    let (disposition, _, _) =
        super::qengine_rewards::wire_grant(owner, admitted, npc, &callback).await?;
    Ok(disposition)
}
