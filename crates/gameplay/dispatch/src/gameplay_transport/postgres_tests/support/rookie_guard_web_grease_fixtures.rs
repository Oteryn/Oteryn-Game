//! Crystal 201189f8 mission05_web_terror.lua:88-138,147-161;
//! spider_queen_wrap.lua:1-42 and spider_queen.lua:67-69.
use super::fixture::{Harness, debug};
use super::quest_world_callbacks_fixtures::{local, wire};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::world_callbacks::{self, Callback, Position};
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, QuestTrack};
use oteryn_protocol_oteryn::world_object::{
    UseDisposition, WorldObjectTarget, decode_use_result, encode_use_intent,
};
use oteryn_protocol_oteryn::world_spatial::{StepDirection, encode_step_intent};

pub(super) const NAME: &str = "rookie-guard-web-grease";
const STONE: Position = Position {
    x: 32014,
    y: 32096,
    floor: -11,
};
const WEB: Position = Position {
    x: 32015,
    y: 32096,
    floor: -12,
};
const WEB_ACTION: u16 = 40010;
// Crystal assigns the action without an appearance filter. The Fandom archive
// Spider Web entry lists 182 as an intact web appearance.
const WEB_APPEARANCE: u16 = 182;
const NEST: Position = Position {
    x: 32013,
    y: 32087,
    floor: -10,
};
const QUEEN: &str = "oteryn:creature.spider_queen";
const QUEST: &str = "oteryn:quest.the_rookie_guard_quest";
const TRACK: &str = "oteryn:quest-progress/quest/u9_1/the_rookie_guard/mission05";
const GREASE: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_rookie_guard/mission05_web_terror.lua:88-111";
const SAMPLE: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_rookie_guard/mission05_web_terror.lua:147-161";
const SAMPLE_PLACEMENT: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/startup/tables/item.lua:2439-2460";

const WRAP: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/spells/monster/spider_queen_wrap.lua:1-42";
const ATTACK: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/monster/bosses/spider_queen.lua:67-69";

pub(super) fn run_case()
-> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(NAME, || {
        super::quest_world_callbacks_fixtures::scenario(NAME, STONE, false)
    })
}

fn document() -> serde_json::Value {
    use serde_json::json;
    let grease = json!({"key":"oteryn:quest-callback/rookie-guard-grease", "quest":QUEST, "trigger":"use",
         "selector":{"position":STONE,"appearance":12727},"requires_world_admission":true,
         "passages":[],"offerings":[],"reversible":false,"return_on_failure":false,
         "activation":{"track":TRACK,"minimum":1,"maximum":2},
         "storage":[{"track":TRACK,"minimum":1,"maximum":2,"set":2}],
         "effects":{"invisibility_ms":120000,
           "creature_capture":{"creature":QUEEN,"interval_ms":5000,"range":1,"track":TRACK,"value":4,
               "condition_ms":30000,"combat_effect":17,"projectile":15,"look_type":422,"speed":[-300,-450],"teleport_delay_ms":4500,
               "landing":NEST,"message":"The spider queen caught you in her net and paralysed you!"}},
         "sources":[GREASE,WRAP,ATTACK]});
    let mut regrease = grease.clone();
    regrease["key"] = "oteryn:quest-callback/rookie-guard-grease-retry".into();
    regrease["activation"] = json!({"track":TRACK,"minimum":4,"maximum":4});
    regrease["storage"] = json!([{"track":TRACK,"minimum":4,"maximum":4,"set":2}]);
    regrease["effects"] = json!({"invisibility_ms":120000});
    regrease["sources"] = json!([GREASE]);
    let sample = json!({"key":"oteryn:quest-callback/rookie-guard-web-sample", "quest":QUEST, "trigger":"use",
         "selector":{"position":WEB,"action":WEB_ACTION},"requires_world_admission":true,
         "passages":[],"offerings":[],"reversible":false,"return_on_failure":false,
         "activation":{"track":TRACK,"minimum":2,"maximum":2},
         "storage":[{"track":TRACK,"minimum":2,"maximum":2,"set":3}],
         "sources":[SAMPLE,SAMPLE_PLACEMENT]});
    let mut recovered_sample = sample.clone();
    recovered_sample["key"] = "oteryn:quest-callback/rookie-guard-web-sample-after-wrap".into();
    recovered_sample["activation"] = json!({"track":TRACK,"minimum":4,"maximum":4});
    recovered_sample["storage"] = json!([{"track":TRACK,"minimum":4,"maximum":4,"set":3}]);
    json!({"schema":1,"callbacks":[grease,regrease,sample,recovered_sample]})
}

pub(super) fn callback() -> TestResult<Callback> {
    Ok(serde_json::from_value(document()["callbacks"][0].clone())?)
}

pub(super) fn activate(
    world: crate::foundation::WorldId,
) -> TestResult<(
    crate::content::ContentActivationController,
    crate::content::NativeEntryContentPin,
)> {
    use crate::content::native_gameplay::{PinnedGameplayBytes, test_support::try_activate_input};
    use serde_json::json;
    use sha2::{Digest, Sha256};
    let hex = |bytes: &[u8]| {
        Sha256::digest(bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    };
    let mut input = super::death_ml_loss_composed_fixtures::production_input_with_source("content-1", None)?;
    let mut appearances: serde_json::Value = serde_json::from_slice(
        &input
            .spell_appearances
            .as_ref()
            .ok_or("pinned native spell appearance document")?
            .bytes,
    )?;
    let mut cocoon = json!({
        "outfit_key":"oteryn:outfit.source.crystal.look422","outfit_revision":"source-outfit-r1",
        "look_type":422,"colours":[0,0,0,0],"addons":0,"mount_key":null,"creature":null,
        "source":{
            "revision":"201189f871d23597b82caf88fd611454a2ddefb8",
            "path":"data-global/scripts/spells/monster/spider_queen_wrap.lua",
            "git_blob":"b0621abdcc35fa20a512712c3abc8502ca45d113",
            "sha256":"6f51784cf952ba4572e175dfb8daf01ff1e7dfaf7e4c978d99d2199b9f151ecf",
            "explicit_fields":{"lookType":422},
            "defaulted_fields":["lookAddons","lookBody","lookFeet","lookHead","lookLegs","lookMount","lookTypeEx"],
            "default_source":{
                "revision":"201189f871d23597b82caf88fd611454a2ddefb8",
                "path":"src/creatures/creatures_definitions.hpp",
                "git_blob":"fc4ae0cf45a56ee7c363b0f452acfb14cbd3c63a",
                "sha256":"8fbc70bf2416b6e76f17cce3dc1b075f8a5451f0f85b522b0a4ae34cf2cccf94"
            }
        }
    });
    cocoon["qualification_sha256"] = hex(&serde_json::to_vec(&cocoon)?).into();
    appearances["records"]
        .as_array_mut()
        .ok_or("native appearance records")?
        .push(cocoon);
    let bytes = serde_json::to_vec(&appearances)?;
    input.spell_appearances = Some(PinnedGameplayBytes {
        sha256: hex(&bytes),
        bytes,
    });
    try_activate_input(world, &input).map_err(|error| debug(error).into())
}

pub(super) fn world(callback: &Callback) -> TestResult<crate::map::boot::BundleWorld> {
    super::quest_world_callbacks_fixtures::world_cells(
        callback,
        STONE,
        12727,
        1,
        &[(WEB, WEB_APPEARANCE, oteryn_world_bundle::sector::Attrs {
            action: Some(WEB_ACTION),
            ..Default::default()
        })],
        &[],
        None,
        &[NEST, WEB],
    )
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let document = document();
    let fixture = QuestStateCatalogue::new(
        "content-1",
        vec![QuestTrack {
            key: TRACK.into(),
            quest: QUEST.into(),
            initial: 1,
            min: -1,
            max: 5,
        }],
        vec![],
    )
    .map_err(debug)?;
    owner.quest_catalogue = Some(std::sync::Arc::new(world_callbacks::fixture_catalogue(
        fixture,
        &document.to_string(),
    )?));
    owner
        .states
        .quest_callbacks
        .fixture
        .set(serde_json::from_value(document["callbacks"].clone())?)
        .map_err(|_| "callback fixture already installed")?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    assert_eq!(
        use_object(owner, admitted, STONE).await?,
        UseDisposition::Committed
    );
    assert_eq!(stage(h).await?, 2);
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let session = admitted.game_session_id;
    {
        let runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        let player = states.get(&runtime, actor, session).ok_or("player")?;
        let now = owner.owner_now().get();
        assert!(player.owned_invisible_at(now).map_err(debug)?);
        assert!(
            !player
                .owned_invisible_at(now + 120_000_000)
                .map_err(debug)?
        );
    }
    let (origin, queen) = {
        let mut runtime = owner.runtime.lock().await;
        let native = owner
            .active_generation
            .and_then(|active| active.native_gameplay())
            .ok_or("native")?;
        runtime
            .install_companion_policies(native.policies().clone())
            .map_err(debug)?;
        let origin = runtime
            .read_actor_position(actor)
            .map_err(debug)?
            .position();
        let health = runtime
            .companion_policy(QUEEN)
            .map_err(debug)?
            .maximum_health;
        let queen = runtime
            .commit_quest_creature_spawn(&[(
                QUEEN,
                health,
                local(Position {
                    y: STONE.y - 1,
                    ..STONE
                }),
            )])
            .map_err(debug)?;
        (origin, queen[0])
    };
    rewind(owner, 6)?;
    owner.drain_monster_melee().await;
    owner.drain_quest_encounter_deadlines().await;
    assert_eq!(
        stage(h).await?,
        2,
        "the queen cannot target a greased player"
    );

    rewind(owner, 120)?;
    for _ in 0..4 {
        owner.drain_monster_melee().await;
        if owner.states.quest_callbacks.next_encounter_due_us().is_some() {
            break;
        }
        rewind(owner, 5)?;
    }
    assert!(
        owner.states.quest_callbacks.next_encounter_due_us().is_some(),
        "the native AI queued a wrap after the first grease expired"
    );
    assert_eq!(
        use_object(owner, admitted, STONE).await?,
        UseDisposition::Committed
    );
    owner.drain_quest_encounter_deadlines().await;
    assert_eq!(
        stage(h).await?,
        2,
        "greasing again before the queued wrap resolves keeps the player invisible"
    );

    rewind(owner, 120)?;
    for _ in 0..4 {
        owner.drain_monster_melee().await;
        owner.drain_quest_encounter_deadlines().await;
        if stage(h).await? == 4 {
            break;
        }
        rewind(owner, 5)?;
    }
    assert_eq!(
        stage(h).await?,
        4,
        "the actual native target schedules the wrap"
    );
    {
        let mut runtime = owner.runtime.lock().await;
        assert_eq!(
            runtime
                .read_actor_position(actor)
                .map_err(debug)?
                .position(),
            origin
        );
        let states = owner.states.spell_states.lock().await;
        let player = states.get(&runtime, actor, session).ok_or("player")?;
        assert!(
            player
                .owned_conditions()
                .active_speed_delta(owner.owner_now().get())
                < 0
        );
        drop(states);
        runtime.remove_encounter_actor(queen).map_err(debug)?;
    }
    rewind(owner, 5)?;
    owner.drain_quest_encounter_deadlines().await;
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position(),
        local(NEST)
    );
    // The native nest has a walkable exit to the south.
    let move_id = admitted.continuity.next_command_id;
    wire(
        owner,
        admitted,
        move_id,
        1,
        encode_step_intent(StepDirection::South),
    )
    .await?;
    {
        let runtime = owner.runtime.lock().await;
        let current = runtime.read_actor_position(actor).map_err(debug)?;
        assert_eq!(
            current.position(),
            local(Position {
                y: NEST.y + 1,
                ..NEST
            })
        );
    }
    // The fixture crop omits the passage from the nest to the sampling corridor.
    relocate(owner, actor, session, local(Position { x: WEB.x - 1, ..WEB })).await?;
    rewind(owner, 2)?;
    assert_eq!(
        use_object(owner, admitted, WEB).await?,
        UseDisposition::Committed
    );
    assert_eq!(stage(h).await?, 3);
    relocate(owner, actor, session, origin).await?;
    rewind(owner, 2)?;
    assert_ne!(
        use_object(owner, admitted, STONE).await?,
        UseDisposition::Committed
    );
    assert_eq!(stage(h).await?, 3);
    Ok(())
}

async fn relocate(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    session: crate::foundation::GameSessionId,
    destination: crate::foundation::MovementLocalPosition,
) -> TestResult {
    let mut runtime = owner.runtime.lock().await;
    let current = runtime.read_actor_position(actor).map_err(debug)?;
    let map = owner.bundle_map().ok_or("map")?;
    let doors = crate::map::floor::BundleDoors(map.facts.doors());
    let proof = crate::map::floor::prepare_quest_callback_teleport(
        &runtime,
        owner.movement_cells(),
        actor,
        session,
        current,
        destination,
        false,
        &doors,
    )
    .map_err(debug)?;
    runtime.commit_source_step(proof).map_err(debug)?;
    Ok(())
}

async fn use_object(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    at: Position,
) -> TestResult<UseDisposition> {
    let map = owner.bundle_map().ok_or("map")?;
    let key = oteryn_world_bundle::bundle::placement_key(at.floor, at.x, at.y, 1).ok_or("key")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let payload = encode_use_intent(&WorldObjectTarget {
        placement,
        expected_revision: map.facts.object_revision(key),
    })
    .map_err(debug)?;
    let command = admitted.continuity.next_command_id;
    let reply = super::quest_world_callbacks_fixtures::wire(owner, admitted, command, 2, payload).await?;
    Ok(decode_use_result(&reply).map_err(debug)?)
}

fn rewind(owner: &mut ComposedFreshAdmission<'_, '_, '_>, seconds: u64) -> TestResult {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(seconds))
        .ok_or("clock")?;
    Ok(())
}

async fn stage(h: &Harness) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1")
            .bind(TRACK)
            .fetch_one(&h.pool)
            .await?,
    )
}
