//! Crystal 201189f8 actions_mission10_a_message_of_freedom_sceptre.lua:16-19,
//! creaturescripts_bosses_kill.lua:20-34.
use super::{
    AdmittedSession, ComposedFreshAdmission, TestResult,
    fixture::{Harness, debug, id},
};
use crate::foundation::{CommandId, CommandRef, GameSessionId, OwnerDamageCommand};
use crate::gameplay_transport::kill_reward::{self, KillSink};
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue,
    authored_bindings::{Bindings, Trigger},
    world_callbacks::{Callback, Offering, Position},
};
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, encode_item_use_intent,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::sync::Arc;

pub(super) const NAME: &str = "qewrath-incarnation";
const AWAKEN: &str = "oteryn:quest-callback/wrath/awaken-fury";
const QUEST: &str = "oteryn:quest.wrath_of_the_emperor_quest";
const MISSION: &str = "oteryn:quest-progress/quest/u8_6/wrath_of_the_emperor/mission10";
pub(super) const STATUE: Position = Position {
    x: 33048,
    y: 31085,
    floor: -15,
};
const SPAWN: Position = Position { x: 33052, ..STATUE };
const CLEAR: Position = Position { x: 33054, ..STATUE };
const CREATURE: &str = "oteryn:creature.fury_of_the_emperor";
const SCEPTRE: &str = "oteryn:item.tibia.i11362";
const SOURCE: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/wrath_of_the_emperor/actions_mission10_a_message_of_freedom_sceptre.lua:16-19";

#[test]
fn sceptre_awakens_the_statue_and_only_its_exact_incarnation_death_restores_it() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(NAME, || {
        super::quest_world_callbacks_fixtures::scenario(NAME, STATUE, false)
    })
}
fn fixture() -> TestResult<Bindings> {
    let row = json!({
        "key":AWAKEN,"quest":QUEST,"trigger":"use",
        "selector":{"position":STATUE,"unique":3193,"appearance":11427},
        "reversible":false,"passages":[],"offerings":[],"return_on_failure":false,
        "requires_world_admission":true,"storage":[
            {"track":MISSION,"minimum":0,"maximum":5,"set":0,"preserve":true}
        ],
        "item_on_target":{"item":SCEPTRE,"count":1,"reusable":true,"selected_instance":true},
        "sources":[SOURCE],"effects":{"transform":10797,"awakened_creature":{"creature":CREATURE,"position":SPAWN}}
    });
    let digest = Sha256::digest(serde_json::to_vec(&row)?)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    Ok(Bindings::parse(&serde_json::to_vec(
        &json!({"schema":1,"quests":[{
            "quest":QUEST,"recipe_sha256":digest,"sources":[SOURCE],
            "tracks":[{"key":MISSION,"initial":1,"minimum":0,"maximum":5}],
            "stages":[{"key":"awakening","completes":false,"trigger":{"kind":"map_use","callback":row}}]
        }]}),
    )?)?)
}

pub(super) fn world() -> TestResult<crate::map::boot::BundleWorld> {
    let bindings = fixture()?;
    let Trigger::MapUse { callback } = &bindings.quests[0].stages[0].trigger else {
        return Err("awakening binding".into());
    };
    let callback: &Callback = callback;
    super::quest_world_callbacks_fixtures::world(callback, STATUE)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let base = QuestStateCatalogue::empty("content-1").ok_or("quest catalogue")?;
    owner.quest_catalogue = Some(Arc::new(fixture()?.attach(&base)?));
    let session = admitted.game_session_id;
    owner.refresh_quest_session(session, false).await;
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    super::quest_world_callbacks_fixtures::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: STATUE,
            item: SCEPTRE.into(),
            count: 1,
        },
        610,
        true,
    )
    .await?;
    let map = owner.bundle_map().ok_or("map")?;
    let key = oteryn_world_bundle::bundle::placement_key(STATUE.floor, STATUE.x, STATUE.y, 1)
        .ok_or("statue key")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let intent = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(std::num::NonZeroU64::new(1).ok_or("handle")?),
        use_with: None,
        use_with_position: None,
        use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget {
            placement,
            expected_revision: 0,
        })),
    })
    .map_err(debug)?;
    let reference = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .and_then(|n| n.item_index())
        .and_then(|i| i.definition_ref("Item", SCEPTRE, "definition-r1"))
        .ok_or("sceptre reference")?;
    let partner_session = GameSessionId::decode(&id(51)).map_err(debug)?;
    let partner = {
        let mut runtime = owner.runtime.lock().await;
        let reserved = runtime
            .reserve_fresh_session(partner_session)
            .map_err(debug)?;
        let partner = runtime.commit_fresh_session(reserved).map_err(debug)?;
        runtime
            .initialize_source_pinned_lab_player_position(
                partner,
                partner_session,
                super::quest_world_callbacks_fixtures::local(SPAWN),
            )
            .map_err(debug)?;
        partner
    };
    let (next, blocked, _) = super::item_craft_fixtures::exchange(
        owner,
        *admitted,
        admitted.continuity.next_command_id,
        &intent,
        Some(reference),
    )
    .await?;
    *admitted = next;
    assert_eq!(blocked, UseDisposition::Occupied);
    assert_eq!(map.facts.object_revision(key), 0);
    let receipts: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1",
    )
    .bind(AWAKEN)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(receipts, 0, "a blocked spawn cannot settle the statue USE");
    {
        let mut runtime = owner.runtime.lock().await;
        let expected = runtime.read_actor_position(partner).map_err(debug)?;
        assert_eq!(
            expected.position(),
            super::quest_world_callbacks_fixtures::local(SPAWN)
        );
        let closed = crate::map::floor::BundleDoors(&map.facts);
        let step = crate::map::floor::prepare_quest_encounter_ejection(
            &runtime,
            owner.movement_cells(),
            partner,
            partner_session,
            expected,
            super::quest_world_callbacks_fixtures::local(CLEAR),
            &closed,
        )
        .map_err(debug)?;
        runtime.commit_source_step(step).map_err(debug)?;
    }
    let (next, disposition, _) = super::item_craft_fixtures::exchange(
        owner,
        *admitted,
        admitted.continuity.next_command_id,
        &intent,
        Some(reference),
    )
    .await?;
    *admitted = next;
    assert_eq!(disposition, UseDisposition::Committed);
    let at = crate::map::overlay::TilePos {
        x: STATUE.x,
        y: STATUE.y,
        floor: STATUE.floor,
    };
    let tile = map
        .overlay
        .base()
        .tile(at.x, at.y, at.floor)
        .ok_or("statue tile")?;
    let base_id = *tile.ids().get(1).ok_or("native statue")?;
    let appearance = || {
        map.facts
            .base_entry(at, 1, base_id)
            .map(|facts| facts.appearance_id)
    };
    assert_eq!(appearance(), Some(10797));
    let mut runtime = owner.runtime.lock().await;
    let states = owner.states.spell_states.lock().await;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let target = runtime
        .positioned_melee_census()
        .map_err(debug)?
        .into_iter()
        .find(|(who, pos, _)| {
            runtime.matches_live_creature_identity(*who, CREATURE.as_bytes())
                && pos.position() == super::quest_world_callbacks_fixtures::local(SPAWN)
        })
        .ok_or("incarnation")?
        .0;
    let lease = runtime
        .bound_attacker_lease(actor, session)
        .map_err(debug)?;
    let time = owner.owner_now();
    let now = time.get() / 1000;
    let health = runtime.companion_snapshot(target).map_err(debug)?.health;
    let decoy = runtime
        .admit_encounter_actor(
            CREATURE,
            health,
            super::quest_world_callbacks_fixtures::local(Position {
                x: SPAWN.x + 1,
                ..SPAWN
            }),
        )
        .map_err(debug)?;
    runtime
        .install_creature_policy(decoy, CREATURE)
        .map_err(debug)?;
    let queue = kill_reward::KillSettlementQueue::default();
    let sink = KillSink {
        prey: Some(&states),
        activity: &[],
        queue: &queue,
        rewards: owner.reward_table(),
        charms: None,
    };
    let principal = crate::combat::CapturedRewardPrincipal {
        character: lease.character_id(),
        lease_generation: lease.generation(),
        session,
        actor,
    };
    // Crystal 201189f8 creaturescripts_bosses_kill.lua:20–34 restores the
    // source statue. The dispatch owner binds the restoration to its exact spawn.
    for (dying, command, expected_appearance, revision) in
        [(decoy, 9901, 10797, 1), (target, 9902, 11427, 2)]
    {
        runtime
            .borrow_exact_actor_commit_at(now)
            .commit_damage_for_bound_attacker(
                dying,
                actor,
                CommandRef::new(session, CommandId::new(command).map_err(debug)?),
                0,
                OwnerDamageCommand {
                    target: CREATURE.as_bytes(),
                    occurrence: session.as_bytes(),
                    binding: b"qewrath-incarnation",
                    damage: health,
                },
            )
            .map_err(debug)?;
        for _ in 0..2 {
            kill_reward::record_creature_kill_with_portals(
                &mut runtime,
                sink,
                dying,
                principal,
                now,
                Some(time),
                kill_reward::portal_context(Some(map)),
            );
        }
        assert_eq!(appearance(), Some(expected_appearance));
        assert_eq!(map.facts.object_revision(key), revision);
    }
    assert_eq!(
        runtime
            .read_actor_position(partner)
            .map_err(debug)?
            .position(),
        super::quest_world_callbacks_fixtures::local(CLEAR)
    );
    Ok(())
}
