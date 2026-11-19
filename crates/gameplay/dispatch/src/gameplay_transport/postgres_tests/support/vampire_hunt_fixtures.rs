use super::fixture::{Harness, debug, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::map::view::MapFacts;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Offering, Position};
use oteryn_protocol_oteryn::world_object::{
    UseDisposition, UseIntent, UseTarget, WorldObjectTarget, encode_item_use_intent,
};
use std::sync::Arc;

pub(super) const PREFIX: &str = "qengine-rewards-vampire-hunt-";
const TARGET: Position = Position {
    x: 101,
    y: 100,
    floor: -7,
};
const NORTH: Position = Position { y: 99, ..TARGET };
const GROUND_APPEARANCE: u16 = 353;
const QUEST: &str = "oteryn:quest.the_inquisition_quest";
const TRACK: &str = "oteryn:quest-progress/quest/u8_2/the_inquisition_quest/questline";
const MISSION: &str = "oteryn:quest-progress/quest/u8_2/the_inquisition_quest/mission03";
const CANDLE: &str = "oteryn:item.tibia.i3083";

pub(super) fn vampire_hunt_uses_all_three_altar_items_for_weakened_count_only_scenario()
-> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run("vampire hunt", || {
        for suffix in ["missing", "all"] {
            super::quest_world_callbacks_fixtures::scenario(
                &format!("{PREFIX}{suffix}"),
                TARGET,
                false,
            )?;
        }
        Ok(())
    })
}

pub(super) fn vampire_hunt_routes_both_native_ground_targets_to_their_summon_branches_scenario()
-> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run("vampire hunt native ground", || {
        for (suffix, target) in [
            ("native-north-missing", NORTH),
            ("native-south-all", TARGET),
        ] {
            super::quest_world_callbacks_fixtures::scenario(
                &format!("{PREFIX}{suffix}"),
                target,
                false,
            )?;
        }
        Ok(())
    })
}

pub(super) fn native_ground(name: &str) -> bool {
    name.strip_prefix(PREFIX)
        .is_some_and(|suffix| suffix.starts_with("native-"))
}

fn altars() -> Vec<Offering> {
    [101, 103, 105]
        .into_iter()
        .map(|x| Offering {
            position: Position {
                x,
                y: 101,
                ..TARGET
            },
            item: CANDLE.into(),
            count: 1,
        })
        .collect()
}

pub(super) fn callback(all: bool, ground: Option<Position>) -> TestResult<Callback> {
    // Crystal startup/tables/item.lua:330-336 and startup/others/functions.lua:76-88
    // bind both ground entries as well as their upper objects.
    let (key, selector) = match ground {
        Some(position) => (
            format!(
                "oteryn:quest-callback/vampire-hunt-ground-{}-{all}",
                position.y
            ),
            serde_json::json!({
                "native":{"position":position,"ordinal":0,"appearance":GROUND_APPEARANCE}
            }),
        ),
        None => (
            format!("oteryn:quest-callback/vampire-hunt-fixture-{all}"),
            serde_json::json!({"position":TARGET,"ordinal":1,"action":2002}),
        ),
    };
    // Crystal 201189f8 actions_vampire_hunt.lua:1-32.
    let required = altars();
    let offerings = if all { required.clone() } else { Vec::new() };
    let creature = if all {
        "oteryn:creature.the_weakened_count"
    } else {
        "oteryn:creature.the_count"
    };
    Ok(serde_json::from_value(serde_json::json!({
        "key":key,
        "quest":QUEST,"trigger":"use",
        "selector":selector,
        "requires_world_admission":true,
        "reversible":false,"passages":[],"offerings":offerings,"return_on_failure":false,
        "storage":[
            {"track":TRACK,"minimum":8,"maximum":8,"set":9},
            {"track":MISSION,"minimum":0,"maximum":6,"set":4}
        ],
        "effects":{"use_summon":{
            "creature":creature,"altar_branch":{"all_present":all,"altars":required}
        }},
        "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_inquisition_quest/actions_vampire_hunt.lua:1-32"]
    }))?)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    name: &str,
) -> TestResult {
    let all = name.ends_with("all");
    let native = native_ground(name);
    let target = if name.ends_with("north-missing") {
        NORTH
    } else {
        TARGET
    };
    {
        let native = owner
            .active_generation
            .and_then(|g| g.native_gameplay())
            .ok_or("native")?;
        let mut runtime = owner.runtime.lock().await;
        native
            .install_companion_policies(&mut runtime)
            .map_err(debug)?;
    }
    let catalogue = owner.quest_catalogue.as_ref().ok_or("catalogue")?;
    let digest = catalogue
        .definition_hash(QUEST)
        .ok_or("prior quest definition")?;
    let mut bindings = vec![
        callback(true, native.then_some(target))?,
        callback(false, native.then_some(target))?,
    ];
    if native {
        let other = if target == NORTH { TARGET } else { NORTH };
        bindings.extend([callback(true, Some(other))?, callback(false, Some(other))?]);
    }
    owner.quest_catalogue = Some(Arc::new(catalogue.with_world_callback_fixtures(bindings)?));
    sqlx::query(
        "INSERT INTO game_character_quest_states(character_id,quest_key,pinned_content_revision,definition_hash,started_character_revision,committed_character_revision)
         SELECT character_id,$2,'content-1',$3,character_revision,character_revision
         FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
    ).bind(id(41).as_slice()).bind(QUEST).bind(digest.as_slice()).execute(&h.pool).await?;
    sqlx::query(
        "INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision)
         SELECT character_id,$2,quest_key,8,committed_character_revision FROM game_character_quest_states
         WHERE character_id=encode($1,'hex')::uuid AND quest_key=$3",
    ).bind(id(41).as_slice()).bind(TRACK).bind(QUEST).execute(&h.pool).await?;
    let session = admitted.game_session_id;
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    for (i, altar) in altars().iter().take(if all { 3 } else { 2 }).enumerate() {
        super::quest_world_callbacks_fixtures::input_item(
            owner,
            fence.item_fence(),
            altar,
            970 + u64::try_from(i)?,
            false,
        )
        .await?;
    }
    let ordinal = u8::from(!native);
    let key = oteryn_world_bundle::bundle::placement_key(target.floor, target.x, target.y, ordinal)
        .ok_or("source")?;
    let map = owner.bundle_map().ok_or("map")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::WorldObject(WorldObjectTarget {
            placement,
            expected_revision: map.facts.object_revision(key),
        }),
        use_with: None,
        use_with_map_item: None,
        use_with_position: None,
    })
    .map_err(debug)?;
    let (next, disposition, _) = super::item_craft_fixtures::exchange(
        owner,
        *admitted,
        admitted.continuity.next_command_id,
        &payload,
        None,
    )
    .await?;
    *admitted = next;
    assert_eq!(disposition, UseDisposition::Committed);
    let runtime = owner.runtime.lock().await;
    let (creature, summoned, _) = runtime
        .positioned_melee_census()
        .map_err(debug)?
        .into_iter()
        .find(|(_, at, session)| {
            session.is_none()
                && at.position() == super::quest_world_callbacks_fixtures::local(target)
        })
        .ok_or("summoned count")?;
    assert_eq!(
        summoned.position(),
        super::quest_world_callbacks_fixtures::local(target)
    );
    let expected = if all {
        b"oteryn:creature.the_weakened_count".as_slice()
    } else {
        b"oteryn:creature.the_count".as_slice()
    };
    assert!(runtime.matches_live_creature_identity(creature, expected));
    drop(runtime);
    let used: (i64,i64) = sqlx::query_as(
        "SELECT (SELECT value FROM game_character_quest_tracks WHERE track_key=$1),
            (SELECT count(*) FROM game_item_instances WHERE lifecycle=1 AND definition_production_key=$2)",
    ).bind(MISSION).bind(CANDLE).fetch_one(&h.pool).await?;
    assert_eq!(used, (4, if all { 0 } else { 2 }));
    let receipt: String = sqlx::query_scalar(
        "SELECT transition_key FROM game_character_quest_receipts WHERE quest_key=$1",
    )
    .bind(QUEST)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(receipt, callback(all, native.then_some(target))?.key);
    Ok(())
}
