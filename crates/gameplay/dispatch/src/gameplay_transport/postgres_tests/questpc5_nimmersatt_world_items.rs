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
    let original = owner.quest_catalogue.as_ref().ok_or("catalogue")?;
    let tracks = callback
        .storage
        .iter()
        .map(|s| original.track(&s.track).cloned().ok_or("track"))
        .collect::<Result<Vec<_>, _>>()?;
    let setup = "oteryn:quest-transition/fixture-nimmersatt-accept";
    let catalogue = std::sync::Arc::new(
        QuestStateCatalogue::new(
            original.content_revision(),
            tracks,
            vec![
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
            ],
        )
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
