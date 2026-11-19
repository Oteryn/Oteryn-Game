//! Crystal 201189f8: the Plague entrance and Sacrifice flame consume their exact sources.
use super::fixture::{Harness, debug};
use super::quest_world_callbacks_fixtures::{input_item, local, wire};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::item_use::fluid::{ItemFluidChange, ItemFluidRequest};
use crate::gameplay_transport::connection::UseCommand;
use crate::gameplay_transport::item_view::ItemKey;
use oteryn_gameplay_quests::quest::{QuestComparison, QuestEffect, QuestEffectKind, QuestTransition};
use oteryn_gameplay_quests::quest::world_callbacks::{self, Callback, Offering, Position};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
use oteryn_protocol_oteryn::world_spatial::{StepDirection, encode_step_intent};

pub(super) const PLAGUE: &str = "banshee-plague-offerings";
pub(super) const SACRIFICE: &str = "banshee-sacrifice-offering";
const QUEST: &str = "oteryn:quest.the_queen_of_the_banshees_quest";
const ENTRY: Position = Position {
    x: 32250,
    y: 31892,
    floor: -14,
};
const SACRIFICE_APPROACH: Position = Position { x: ENTRY.x - 1, ..ENTRY };
// Crystal startup/tables/teleport.lua:234-240, paired portals.
const PLAGUE_ENTRY: Position = Position {
    x: 32176,
    y: 31869,
    floor: -15,
};
const ALTAR: Position = Position {
    x: 32243,
    y: 31892,
    floor: -14,
};
const PLAGUE_LANDING: Position = Position {
    y: PLAGUE_ENTRY.y - 6,
    ..PLAGUE_ENTRY
};
const PLAGUE_EAST: Position = Position { x: PLAGUE_ENTRY.x + 1, ..PLAGUE_ENTRY };
const PLAGUE_EAST_APPROACH: Position = Position { x: PLAGUE_EAST.x + 1, ..PLAGUE_EAST };
const PLAGUE_EAST_LANDING: Position = Position { x: PLAGUE_EAST.x, ..PLAGUE_LANDING };
const SACRIFICE_LANDING: Position = Position {
    x: 32261,
    y: 31849,
    floor: -15,
};
const PLAGUE_TRACK: &str = "oteryn:quest-progress/quest/u7_2/the_queen_of_the_banshees/second_seal";
const PLAGUE_COMPLETE: &str = "oteryn:quest-callback/fixture/plague-seal-complete";
const SACRIFICE_TRACK: &str =
    "oteryn:quest-progress/quest/u7_2/the_queen_of_the_banshees/fourth_seal";
const SACRIFICE_DOOR: &str =
    "oteryn:quest-progress/quest/u7_2/the_queen_of_the_banshees/fourth_seal_door";
const BLOOD: u8 = 5;
const BLOOD_CUES: [Position; 4] = [
    Position { x: 32242, y: 31891, ..ALTAR },
    Position { y: 31891, ..ALTAR },
    Position { x: 32242, y: 31893, ..ALTAR },
    Position { y: 31893, ..ALTAR },
];

pub(super) fn run_first_case() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(PLAGUE, || {
        super::quest_world_callbacks_fixtures::scenario(PLAGUE, PLAGUE_ENTRY, false)
    })
}

pub(super) fn run_second_case() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(SACRIFICE, || {
        super::quest_world_callbacks_fixtures::scenario(SACRIFICE, ENTRY, false)
    })
}

pub(super) fn matches(name: &str) -> bool {
    matches!(name, PLAGUE | SACRIFICE)
}

fn document(name: &str) -> serde_json::Value {
    use serde_json::json;
    let plague = name == PLAGUE;
    let track = if plague {
        PLAGUE_TRACK
    } else {
        SACRIFICE_TRACK
    };
    let source = if plague {
        "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_queen_of_the_banshees/movement-2-second_seal_pearl.lua:3-33"
    } else {
        "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_queen_of_the_banshees/movement-4-fourth_seal_flame.lua:3-30"
    };
    let offerings = if plague {
        json!([
            {"position":{"x":32173,"y":31871,"floor":-15},"item":"oteryn:item.tibia.i3026","count":1},
            {"position":{"x":32180,"y":31871,"floor":-15},"item":"oteryn:item.tibia.i3027","count":1}
        ])
    } else {
        json!([])
    };
    let effects = if plague {
        json!({"teleport_effect":11})
    } else {
        json!({
            "fluid_offering":{"position":ALTAR,"subtype":BLOOD,"missing_return_effect":11,
                "placement_cues":BLOOD_CUES.map(|position| json!({"position":position,"effect":1}))},
            "magic":{"position":ALTAR,"effect":14},
            "teleport_effect":14
        })
    };
    let mut storage = vec![json!({"track":track,"minimum":-1,"maximum":0,"set":1,"preserve":plague})];
    let mut sources = vec![source];
    let selector = if plague {
        json!({"position":PLAGUE_ENTRY,"appearance":1949,"action":35002})
    } else {
        json!({"position":ENTRY,"appearance":1959,"unique":35016})
    };
    if !plague {
        storage.push(json!({"track":SACRIFICE_DOOR,"minimum":-1,"maximum":1,"set":1}));
        sources.extend([
            "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_queen_of_the_banshees/movement-4-fourth_seal_sacrifice.lua:3-15",
            "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/lib/quests/the_queen_of_the_banshees.lua:97-105",
            "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/startup/tables/teleport.lua:575-579",
        ]);
    } else {
        sources.push("crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/startup/tables/teleport.lua:233-240");
    }
    let primary = json!({
        "key":format!("oteryn:quest-callback/{name}"),"quest":QUEST,
        "trigger":"step_in","selector":selector,
        "reversible":false,"passages":[],"teleport":if plague { PLAGUE_LANDING } else { SACRIFICE_LANDING },
        "requires_world_admission":true,"storage":storage,
        "offerings":offerings,"effects":effects,"return_on_failure":true,"sources":sources
    });
    let mut callbacks = vec![primary.clone()];
    if plague {
        let mut east = primary;
        east["key"] = "oteryn:quest-callback/banshee-plague-offerings-east".into();
        east["selector"]["position"] = json!(PLAGUE_EAST);
        east["teleport"] = json!(PLAGUE_EAST_LANDING);
        callbacks.push(east);
    }
    json!({"schema":1,"callbacks":callbacks})
}

pub(super) fn callback(name: &str) -> TestResult<Callback> {
    Ok(serde_json::from_value(
        document(name)["callbacks"][0].clone(),
    )?)
}

pub(super) fn world(name: &str, callback: &Callback) -> TestResult<crate::map::boot::BundleWorld> {
    if name == SACRIFICE {
        super::quest_world_callbacks_fixtures::world_with_cells(callback, ENTRY, &[ALTAR])
    } else {
        super::quest_world_callbacks_fixtures::world_cells(
            callback,
            PLAGUE_ENTRY,
            1949,
            1,
            &[(PLAGUE_EAST, 1949, oteryn_world_bundle::sector::Attrs {
                action: Some(35002),
                ..Default::default()
            })],
            &[],
            None,
            &[PLAGUE_EAST_LANDING],
        )
    }
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    name: &str,
) -> TestResult {
    let callback = callback(name)?;
    let catalogue = owner.quest_catalogue.as_deref().ok_or("catalogue")?;
    let catalogue = if name == PLAGUE {
        catalogue.with_fixture_transition(QuestTransition {
            key: PLAGUE_COMPLETE.into(),
            quest: QUEST.into(),
            effects: vec![QuestEffect {
                track: PLAGUE_TRACK.into(),
                from: QuestComparison::Lt(1),
                effect: QuestEffectKind::Set(1),
            }],
            completes: false,
            experience: None,
            world_change: None,
        }).map_err(debug)?
    } else {
        catalogue.clone()
    };
    let document = document(name);
    let catalogue = world_callbacks::fixture_catalogue(
        catalogue,
        &document.to_string(),
    )?;
    owner.quest_catalogue = Some(std::sync::Arc::new(catalogue));
    owner
        .states
        .quest_callbacks
        .fixture
        .set(serde_json::from_value(document["callbacks"].clone())?)
        .map_err(|_| "callback fixture already installed")?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let origin = owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?
        .position();
    if name == PLAGUE {
        input_item(
            owner,
            fence.item_fence(),
            &callback.offerings[0],
            301,
            false,
        )
        .await?;
    } else {
        pour(owner, admitted, h, 3, 301).await?;
        move_to_flame(owner, admitted).await?;
    }
    let step = encode_step_intent(StepDirection::East);
    wire(owner, admitted, 1, 1, step.clone()).await?;
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position(),
        origin
    );
    let count = |query: &'static str| sqlx::query_scalar::<_, i64>(query).fetch_one(&h.pool);
    if name == PLAGUE {
        assert_eq!(
            count("SELECT count(*) FROM game_quest_callback_consumptions").await?,
            0
        );
        input_item(
            owner,
            fence.item_fence(),
            &callback.offerings[1],
            302,
            false,
        )
        .await?;
    } else {
        assert_eq!(
            count("SELECT count(*) FROM game_item_fluid_splashes WHERE fluid_subtype=3").await?,
            1
        );
        assert!(blood_cues(owner)?.is_empty());
        assert_eq!(effects_at(owner, SACRIFICE_APPROACH)?, vec![11]);
        pour(owner, admitted, h, BLOOD, 302).await?;
        assert_eq!(blood_cues(owner)?, BLOOD_CUES);
        move_to_flame(owner, admitted).await?;
    }
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    wire(owner, admitted, 2, 1, step).await?;
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position(),
        local(callback.teleport.ok_or("seal landing")?)
    );
    if name == PLAGUE {
        assert_eq!(
            count("SELECT count(*) FROM game_quest_callback_consumptions").await?,
            2
        );
        assert_eq!(
            count("SELECT count(*) FROM game_character_quest_tracks WHERE value=1").await?,
            0
        );
        exercise_east_plague(owner, admitted, h, &callback.offerings).await?;
    } else {
        assert_eq!(
            count("SELECT count(*) FROM game_item_fluid_splashes WHERE fluid_subtype=5").await?,
            0
        );
        assert_eq!(
            count("SELECT count(*) FROM game_item_fluid_splashes WHERE fluid_subtype=3").await?,
            1
        );
        assert_eq!(
            count("SELECT count(*) FROM game_character_quest_tracks WHERE value=1").await?,
            2
        );
        assert_eq!(effects_at(owner, ALTAR)?, vec![14]);
        assert_eq!(effects_at(owner, SACRIFICE_LANDING)?, vec![14]);

        // Crystal movement-4-fourth_seal_flame.lua:9-12 rejects an already
        // completed seal before it looks at the blood on the altar.
        pour(owner, admitted, h, BLOOD, 303).await?;
        move_to_flame(owner, admitted).await?;
        let command = admitted.continuity.next_command_id;
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(2))
            .ok_or("clock")?;
        wire(owner, admitted, command, 1, encode_step_intent(StepDirection::East)).await?;
        let returned = owner.runtime.lock().await.read_actor_position(actor).map_err(debug)?;
        assert_eq!(returned.position(), local(SACRIFICE_APPROACH));
        assert_eq!(effects_at(owner, SACRIFICE_APPROACH)?, vec![11]);
        assert_eq!(
            count("SELECT count(*) FROM game_item_fluid_splashes WHERE fluid_subtype=5").await?,
            1
        );
    }
    Ok(())
}

async fn exercise_east_plague(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    offerings: &[Offering],
) -> TestResult {
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let session = admitted.game_session_id;
    let west = encode_step_intent(StepDirection::West);
    for (completed, first_mint, destination) in [
        (false, 303, PLAGUE_EAST_LANDING),
        (true, 305, PLAGUE_EAST_APPROACH),
    ] {
        if completed {
            // Crystal movement-2-second_seal_pearl.lua:9-13 checks the seal before either pearl.
            super::qengine_outfit_stories_fixtures::commit_fixture(owner, admitted, PLAGUE_COMPLETE, 500)
                .await?;
            owner.refresh_quest_session(session, false).await;
        }
        let fence = owner
            .current_quest_fence(session)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?;
        for (mint, offering) in (first_mint..).zip(offerings) {
            input_item(owner, fence.item_fence(), offering, mint, false).await?;
        }
        super::l6_native_signs_fixtures::relocate(owner, actor, session, PLAGUE_EAST_APPROACH).await?;
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(2))
            .ok_or("clock")?;
        let command = admitted.continuity.next_command_id;
        wire(owner, admitted, command, 1, west.clone()).await?;
        assert_eq!(
            owner.runtime.lock().await.read_actor_position(actor).map_err(debug)?.position(),
            local(destination)
        );
        let consumptions: i64 = sqlx::query_scalar("SELECT count(*) FROM game_quest_callback_consumptions")
            .fetch_one(&h.pool).await?;
        assert_eq!(consumptions, 4, "only the two successful arrivals consume pearls");
    }
    Ok(())
}

fn blood_cues(owner: &ComposedFreshAdmission<'_, '_, '_>) -> TestResult<Vec<Position>> {
    Ok(owner
        .bundle_map()
        .ok_or("map")?
        .facts
        .movement_effects_since(0)
        .iter()
        .filter(|(_, _, effect)| *effect == 1)
        .map(|(_, tile, _)| Position { x: tile.x, y: tile.y, floor: tile.floor })
        .collect())
}

fn effects_at(owner: &ComposedFreshAdmission<'_, '_, '_>, position: Position) -> TestResult<Vec<u16>> {
    let map = owner.bundle_map().ok_or("map")?;
    Ok(map.facts.movement_effects_since(0).into_iter().filter_map(|(_, at, effect)| {
        ((at.x, at.y, at.floor) == (position.x, position.y, position.floor)).then_some(effect)
    }).collect())
}

async fn move_to_flame(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult {
    super::l6_native_signs_fixtures::relocate(
        owner,
        admitted.runtime_actor.ok_or("actor")?,
        admitted.game_session_id,
        SACRIFICE_APPROACH,
    ).await
}

async fn pour(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
    subtype: u8,
    command: u64,
) -> TestResult {
    const VESSEL: &str = "oteryn:item.tibia.i2880";
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?
        .item_fence();
    let vessel = Offering {
        position: ALTAR,
        item: VESSEL.into(),
        count: 1,
    };
    if command == 301 {
        input_item(owner, fence, &vessel, 380, true).await?;
    }
    let id: sqlx::types::Uuid = sqlx::query_scalar(
        "SELECT item_instance_id FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1"
    ).bind(VESSEL).fetch_one(&h.pool).await?;
    let transaction: sqlx::types::Uuid = sqlx::query_scalar("SELECT game_character_uuid_v7()")
        .fetch_one(&h.pool)
        .await?;
    owner
        .root
        .commit_item_fluid(
            owner.character,
            owner.holder,
            fence,
            ItemFluidRequest {
                command_id: command + 100,
                transaction_id: *transaction.as_bytes(),
                change: ItemFluidChange {
                    item_instance_id: *id.as_bytes(),
                    definition: crate::durability::item_mint::TypedDefinitionRef {
                        family: "Item".into(),
                        production_key: VESSEL.into(),
                        revision_ref: "definition-r1".into(),
                    },
                    fluid_before: 0,
                    fluid_after: subtype,
                },
                other: None,
            },
        )
        .await
        .map_err(debug)?;
    let map = owner.bundle_map().ok_or("map")?;
    let mut placement = map.overlay.base().digest().to_vec();
    let key = oteryn_world_bundle::bundle::placement_key(ALTAR.floor, ALTAR.x, ALTAR.y, 0)
        .ok_or("placement")?;
    placement.extend_from_slice(&key.to_be_bytes());
    let actor = admitted.runtime_actor.ok_or("actor")?;
    super::l6_native_signs_fixtures::relocate(
        owner,
        actor,
        admitted.game_session_id,
        Position { x: ALTAR.x - 1, ..ALTAR },
    ).await?;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let result = owner
        .use_fluid_map(
            actor,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id: command,
                item_fence: Some(fence),
            },
            ItemKey::Instance(*id.as_bytes()),
            WorldObjectTarget {
                placement,
                expected_revision: 0,
            },
        )
        .await
        .ok_or("fluid USE")?;
    assert_eq!(result.disposition, UseDisposition::Committed);
    Ok(())
}
