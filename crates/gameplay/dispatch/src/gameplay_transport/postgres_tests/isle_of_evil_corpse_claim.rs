//! Direct USE of native boss corpses commits one item and one personal claim each.
use super::death_ml_loss_composed_fixtures::production_generation_with_pin;
use super::fixture::{Harness, debug, id};
use super::item_look_fixtures::seed_main_backpack;
use super::kill_reward_e2e_fixtures::scenario_thread::run_async;
use super::quest_world_callbacks_fixtures::local;
use super::questpc7_native_deaths_fixtures::{lethal_creature, prepare_native_hunter};
use super::{ComposedFreshAdmission, Dependencies, TestResult, prepare_current_session};
use crate::gameplay_transport::{
    connection::{FreshAdmissionAuthority, UseCommand},
    item_view::{ItemKey, ItemViewError, SessionItemView},
};
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue, authored_bindings::Bindings, world_callbacks::Position,
};
use oteryn_protocol_oteryn::world_object::UseDisposition;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::sync::Arc;

const QUEST: &str = "oteryn:quest.the_isle_of_evil_quest";
const MISSION: &str = "oteryn:quest-progress/crystalserver/quest/u8_5/the_isle_of_evil/mission07";
const COUNTER: &str = "oteryn:quest-progress/isle-of-evil/corpse-claims";
// Crystal 201189f8 actions_corpse_bosses.lua:4-30; the pool belongs to each corpse.
const CORPSES: [(&str, u16, &[u16]); 6] = [
    ("doctor_perhaps", 9402, &[9399, 9383, 9372, 9373]),
    ("dirtbeard", 9403, &[9374, 9401, 9375, 9382]),
    ("evil_mastermind", 9404, &[9391]),
    ("monstor", 9405, &[9381, 9386, 9380, 9396]),
    ("mephiles", 9406, &[9376, 9387, 9400, 9377]),
    ("boogey", 9407, &[9379, 9385, 9384, 9378]),
];

#[test]
fn direct_corpse_claim_pays_once_and_finishes_after_all_six_bosses() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_async(async move |_| {
        let h =
            Harness::create_with_admission_seed(admin, "isle_of_evil_corpse_claim", false, false)
                .await?;
        exercise(&h).await?;
        h.cleanup().await
    })
}

fn catalogue() -> TestResult<QuestStateCatalogue> {
    let mut tracks = vec![
        json!({"key":MISSION,"initial":1,"minimum":0,"maximum":2}),
        json!({"key":COUNTER,"initial":0,"minimum":0,"maximum":6}),
    ];
    let stages = CORPSES.iter().map(|(boss, appearance, rewards)| {
        let claim = format!("oteryn:quest-progress/isle-of-evil/corpses/{boss}");
        tracks.push(json!({"key":claim,"initial":0,"minimum":0,"maximum":1}));
        json!({"key":boss,"completes":false,"trigger":{"kind":"corpse_use","callback":{
            "key":format!("oteryn:quest-reward/isle-of-evil/corpses/{boss}"),"quest":QUEST,
            "npc":"","keywords":[],"topic":0,"next_topic":0,
            "counter":{"track":COUNTER,"maximum":6},
            "direct_corpse_claim":{"appearance":appearance,"completion_track":MISSION,
                "rewards":rewards.iter().map(|id| format!("oteryn:item.tibia.i{id}")).collect::<Vec<_>>()},
            "storage":[{"track":claim,"minimum":0,"maximum":0,"set":1},
                       {"track":MISSION,"minimum":1,"maximum":1,"set":2}],
            "take":[],"give":null,"outfit":null,"achievement":null,
            "magic_effects":[{"effect":"CONST_ME_MAGIC_BLUE","target":"player"}],
            "reply":["Recovered a reward from the boss corpse."],
            "sources":["Crystal 201189f8 actions_corpse_bosses.lua:36-67"]
        }}})
    }).collect::<Vec<_>>();
    let recipe = format!("{:x}", Sha256::digest(serde_json::to_vec(&CORPSES)?));
    let source = json!({"schema":1,"quests":[{"quest":QUEST,"tracks":tracks,"stages":stages,
        "recipe_sha256":recipe,
        "sources":["Crystal 201189f8 actions_corpse_bosses.lua:4-67"]}]});
    let base = QuestStateCatalogue::empty("content-1").ok_or("quest catalogue")?;
    Ok(Bindings::parse(&serde_json::to_vec(&source)?)?.attach(&base)?)
}

async fn exercise(h: &Harness) -> TestResult {
    prepare_current_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world = Dependencies::new()?.admitted.world_id;
    let (controller, pin) = production_generation_with_pin(world, "content-1")?;
    let generation = controller.active().ok_or("active content")?;
    let start = pin.entry_start();
    let room = pin.qualified_room().clone();
    let deps = Dependencies::with_content_and_capacity(
        pin.into_channel_pin(),
        room,
        crate::spell::cast::v1_spell_book().map_err(debug)?,
        32,
    )?;
    let mut owner = deps.owner(h, &authority);
    owner.active_generation = Some(generation);
    owner.quest_catalogue = Some(Arc::new(catalogue()?));
    let admitted = deps.admitted;
    let actor = deps.actor;
    owner
        .runtime
        .lock()
        .await
        .initialize_first_entry_position(actor)
        .map_err(debug)?;
    seed_main_backpack(h, &[]).await?;
    prepare_native_hunter(&owner, &admitted).await?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(debug)?
        .ok_or("fence")?;
    let mut items = SessionItemView::resume(Default::default()).with_map_view();
    let mut expected = Vec::new();
    for (index, (boss, appearance, rewards)) in CORPSES.into_iter().enumerate() {
        let creature = format!("oteryn:creature.{boss}");
        let at = Position {
            x: u16::try_from(start.x + 1)?,
            y: u16::try_from(start.y)?,
            floor: i8::try_from(-start.floor)?,
        };
        let target = {
            let mut runtime = owner.runtime.lock().await;
            let health = runtime
                .companion_policy(&creature)
                .map_err(debug)?
                .maximum_health;
            let target = runtime
                .admit_encounter_actor(&creature, health, local(at))
                .map_err(debug)?;
            runtime
                .install_creature_policy(target, &creature)
                .map_err(debug)?;
            target
        };
        let ordinal = 1000 + 10 * u64::try_from(index)?;
        lethal_creature(&owner, &admitted, target, ordinal, &creature).await?;
        owner
            .runtime
            .lock()
            .await
            .remove_encounter_actor(target)
            .map_err(debug)?;
        let key = format!("oteryn:item.tibia.i{appearance}");
        let corpse = owner
            .runtime
            .lock()
            .await
            .visible_corpses()
            .into_iter()
            .find(|corpse| corpse.item.production_key == key)
            .ok_or_else(|| format!("native {boss} corpse"))?;
        let key = ItemKey::Entity(corpse.identity);
        let handle = items
            .map_view(&[key], |table| {
                table.handle(&key).ok_or(ItemViewError::Encode)
            })
            .map_err(debug)?;
        let mut use_ = UseCommand {
            game_session_id: admitted.game_session_id,
            command_id: ordinal + 1,
            item_fence: Some(fence.item_fence()),
        };
        assert_eq!(
            claim(&owner, actor, use_, handle, corpse.identity).await?,
            UseDisposition::Committed
        );
        assert_eq!(
            claim(&owner, actor, use_, handle, corpse.identity).await?,
            UseDisposition::Committed
        );
        use_.command_id += 1;
        assert_eq!(
            claim(&owner, actor, use_, handle, corpse.identity).await?,
            UseDisposition::Rejected
        );
        assert!(
            owner
                .runtime
                .lock()
                .await
                .bound_corpse(corpse.identity)
                .is_some()
        );

        let item: String = sqlx::query_scalar(
            "SELECT plan #>> '{give,item}' FROM game_quest_reward_sources WHERE transition_key=$1",
        )
        .bind(format!("oteryn:quest-reward/isle-of-evil/corpses/{boss}"))
        .fetch_one(&h.pool)
        .await?;
        assert!(
            rewards
                .iter()
                .any(|id| item == format!("oteryn:item.tibia.i{id}"))
        );
        expected.push(item);
        let mission: i64 = sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2")
            .bind(id(41).as_slice()).bind(MISSION).fetch_one(&h.pool).await?;
        assert_eq!(mission, if index == 5 { 2 } else { 1 });
    }
    let mut carried: Vec<String> = sqlx::query_scalar("SELECT i.definition_production_key FROM game_item_container_entries e JOIN game_item_instances i USING(item_instance_id) WHERE e.character_id=encode($1,'hex')::uuid AND i.lifecycle=1 ORDER BY i.definition_production_key")
        .bind(id(41).as_slice()).fetch_all(&h.pool).await?;
    carried.retain(|item| expected.contains(item));
    expected.sort();
    assert_eq!(
        carried, expected,
        "one minted reward from each of the six corpses"
    );
    Ok(())
}

async fn claim(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    command: UseCommand,
    handle: oteryn_protocol_oteryn::item_view::ItemHandle,
    identity: [u8; 16],
) -> TestResult<UseDisposition> {
    Ok(owner
        .use_direct_corpse_claim(actor, command, handle, identity)
        .await
        .map_err(debug)?
        .ok_or("native direct corpse USE route")?
        .disposition)
}
