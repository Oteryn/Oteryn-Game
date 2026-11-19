//! Crystal 201189f8, data-global/scripts/quests/the_new_frontier/action_beaver.lua:12-44
//! and data-global/monster/mammals/thieving_squirrel.lua:63-65.
use super::{
    AdmittedSession, ComposedFreshAdmission, TestResult,
    fixture::{Harness, debug},
};
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue,
    authored_bindings::Bindings,
    world_callbacks::{Offering, Position},
};
use oteryn_protocol_oteryn::world_object::UseDisposition;
use serde_json::json;
use std::sync::Arc;

pub(super) const NAME: &str = "qengine-rewards-frontier-tree-bait";
pub(super) const SQUIRRELS: &str = "qengine-rewards-frontier-tree-squirrels";
pub(super) const WOLVES: &str = "qengine-rewards-frontier-tree-wolves";
const AT: Position = Position {
    x: 33294,
    y: 31677,
    floor: -15,
};
const QUEST: &str = "oteryn:quest.authored.draccoon_herald_outfits_quest";
const TRACK: &str = "oteryn:quest-progress/tree-bait-fixture/marked";
const BAIT: &str = "oteryn:item.tibia.i9843";
const THIEF: &str = "oteryn:creature.thieving_squirrel";

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    scenario: &str,
) -> TestResult {
    let (groups, count) = match scenario {
        SQUIRRELS => (
            json!([{"creature":"oteryn:creature.enraged_squirrel","count":3}]),
            3,
        ),
        WOLVES => (
            json!([
                {"creature":"oteryn:creature.wolf","count":5},
                {"creature":"oteryn:creature.war_wolf","count":1}
            ]),
            6,
        ),
        NAME => (json!([{"creature":THIEF,"count":1}]), 1),
        _ => return Err("unknown tree scenario".into()),
    };
    let stolen = scenario == NAME;
    let mut callback = super::authored_world_rewards_fixtures::placeholder(AT)?;
    callback.key = "oteryn:quest-callback/tree-bait-fixture/mark".into();
    callback.requires_world_admission = true;
    callback.storage[0].track = TRACK.into();
    callback.sources = vec!["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_new_frontier/action_beaver.lua:12-44".into()];
    let mut callback = serde_json::to_value(callback)?;
    callback["item_on_target"] = json!({
        "item":BAIT,"count":1,"selected_instance":true,"reusable":!stolen,
        "replacement":null,"target_replacement":null
    });
    callback["effects"] = json!({"spawn_creatures":groups,"magic":{"position":AT,"effect":11}});
    let document = json!({"schema":1,"quests":[{"quest":QUEST,
        "recipe_sha256":"2222222222222222222222222222222222222222222222222222222222222222",
        "sources":["synthetic target-item spawn engine fixture"],
        "tracks":[{"key":TRACK,"initial":0,"minimum":0,"maximum":1}],
        "stages":[{"key":"mark","completes":false,"trigger":{"kind":"map_use","callback":callback}}]}]});
    // The tree stays in place in every donor branch. Reject a conflicting
    // target mutation rather than committing storage and silently skipping it.
    for (field, value) in [
        ("target_remove", json!(true)),
        ("target_replacement", json!(2118)),
    ] {
        let mut invalid = document.clone();
        invalid["quests"][0]["stages"][0]["trigger"]["callback"]["item_on_target"][field] = value;
        assert!(
            Bindings::parse(&serde_json::to_vec(&invalid)?).is_err(),
            "{field}"
        );
    }
    let bindings = Bindings::parse(&serde_json::to_vec(&document)?)?;
    let base = QuestStateCatalogue::empty("content-1").ok_or("revision")?;
    owner.quest_catalogue = Some(Arc::new(bindings.attach(&base)?));
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    super::quest_world_callbacks_fixtures::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: AT,
            item: BAIT.into(),
            count: 1,
        },
        4620,
        true,
    )
    .await?;
    // Clicking the tree without selecting the bait cannot steal it.
    super::secret_service_dock_fire_fixtures::assert_bare_use_refused(owner, admitted, AT).await?;
    assert_eq!(
        super::qengine_voucher_stories_fixtures::quantity(h, 9843).await?,
        1
    );
    assert!(
        owner
            .runtime
            .lock()
            .await
            .current_live_creature_census()
            .map_err(debug)?
            .is_empty()
    );
    assert_eq!(
        super::secret_service_dock_fire_fixtures::use_selected_tool(owner, admitted, AT, BAIT)
            .await?,
        UseDisposition::Committed
    );
    assert_eq!(
        super::qengine_voucher_stories_fixtures::quantity(h, 9843).await?,
        i64::from(!stolen)
    );
    let target = crate::map::overlay::TilePos {
        x: AT.x,
        y: AT.y,
        floor: AT.floor,
    };
    assert_eq!(
        owner
            .bundle_map()
            .ok_or("map")?
            .facts
            .movement_effects_since(0)
            .iter()
            .filter(|(_, position, effect)| *position == target && *effect == 11)
            .count(),
        count
    );
    if !stolen {
        assert_eq!(
            owner
                .runtime
                .lock()
                .await
                .current_live_creature_census()
                .map_err(debug)?
                .len(),
            count
        );
        assert_ne!(
            super::secret_service_dock_fire_fixtures::use_selected_tool(owner, admitted, AT, BAIT)
                .await?,
            UseDisposition::Committed
        );
        assert_eq!(
            owner
                .runtime
                .lock()
                .await
                .current_live_creature_census()
                .map_err(debug)?
                .len(),
            count
        );
        return Ok(());
    }
    let thief = {
        let runtime = owner.runtime.lock().await;
        let actors = runtime.current_live_creature_census().map_err(debug)?;
        let [thief] = actors.as_slice() else {
            return Err("exactly one spawned thief".into());
        };
        *thief
    };
    assert_eq!(
        super::qengine_kill_stories_fixtures::queue_death_using(
            owner,
            admitted,
            THIEF,
            20000,
            Some(thief)
        )
        .await?,
        thief
    );
    super::qengine_kill_stories_fixtures::drain_deaths(owner, admitted, &[]).await?;
    let bait_in_corpse: i64 = sqlx::query_scalar("SELECT coalesce(sum(i.quantity),0)::bigint FROM game_item_corpse_container_entries e JOIN game_item_instances i USING(item_instance_id) WHERE i.lifecycle=1 AND i.definition_production_key=$1")
        .bind(BAIT).fetch_one(&h.pool).await?;
    assert_eq!(bait_in_corpse, 1);
    let marked: i64 =
        sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1")
            .bind(TRACK)
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(marked, 1);
    // A fresh command with another flask must still respect the durable mark.
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    super::quest_world_callbacks_fixtures::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: AT,
            item: BAIT.into(),
            count: 1,
        },
        4621,
        true,
    )
    .await?;
    let bait_before = super::qengine_voucher_stories_fixtures::quantity(h, 9843).await?;
    assert_ne!(
        super::secret_service_dock_fire_fixtures::use_selected_tool(owner, admitted, AT, BAIT)
            .await?,
        UseDisposition::Committed
    );
    assert_eq!(
        super::qengine_voucher_stories_fixtures::quantity(h, 9843).await?,
        bait_before
    );
    assert!(
        owner
            .runtime
            .lock()
            .await
            .current_live_creature_census()
            .map_err(debug)?
            .is_empty()
    );
    Ok(())
}

/// Materialize the actual donor items in the isolated generation for the two
/// cases that need them. The main Item policy has their identities only.
pub(super) fn fixture_generation(
    world: crate::foundation::WorldId,
    scenario: &str,
) -> TestResult<(
    crate::content::ContentActivationController,
    crate::content::NativeEntryContentPin,
)> {
    use sha2::{Digest, Sha256};
    let mut input =
        super::death_ml_loss_composed_fixtures::production_input_with_source("content-1", None)?;
    let pin = input.item_profiles.as_mut().ok_or("item profile input")?;
    let mut document: serde_json::Value = serde_json::from_slice(&pin.bytes)?;
    let records = document["records"]
        .as_array_mut()
        .ok_or("item profile records")?;
    let bindings: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../imports/crystalserver/bindings/items.json"
    )))?;
    let bindings = bindings["bindings"]
        .as_array()
        .ok_or("source item identities")?;
    let mut add_profile =
        |source: u16, prototype: u16, weight: u32, capacity: Option<u8>| -> TestResult {
            let key = format!("oteryn:item.tibia.i{source}");
            if records
                .iter()
                .any(|record| record["production_definition"]["production_key"] == key.as_str())
            {
                return Ok(());
            }
            let prototype = format!("oteryn:item.tibia.i{prototype}");
            let mut record = records
                .iter()
                .find(|record| {
                    record["production_definition"]["production_key"] == prototype.as_str()
                })
                .ok_or("item profile prototype")?
                .clone();
            let external = source.to_string();
            record["production_binding"] = bindings
                .iter()
                .find(|binding| binding["external_id"] == external.as_str())
                .ok_or("item source identity")?
                .clone();
            record["production_binding_qualification"] = serde_json::Value::Null;
            record["production_definition"]["production_key"] = json!(key);
            record["authoring"]["item"]["key"] = json!(format!("candidate:item/{source}"));
            record["authoring"]
                .as_object_mut()
                .ok_or("item authoring")?
                .remove("taxonomy");
            record["semantics"]["physical"]["value"]["weight"]["value"] = json!(weight);
            if let Some(capacity) = capacity {
                record["semantics"]["container"]["value"]["capacity"]["value"] = json!(capacity);
                // The fixture inspects the original corpse immediately after settling death.
                record["semantics"]["temporal"] = json!({"state":"NOT_APPLICABLE"});
            }
            records.push(record);
            Ok(())
        };
    if matches!(scenario, NAME | SQUIRRELS | WOLVES) {
        // Crystal data/items/items.xml:475-479, 2925-2928, 26267-26270.
        add_profile(276, 111, 0, Some(15))?;
        add_profile(836, 841, 10, None)?;
        add_profile(9843, 5467, 180, None)?;
    } else {
        // Crystal data/items/items.xml:810-813.
        add_profile(402, 5467, 50, None)?;
    }
    pin.bytes = serde_json::to_vec(&document)?;
    pin.sha256 = Sha256::digest(&pin.bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    if scenario == NAME {
        // The native corpse and loot owner reads materialization and capacity
        // from its own pin. Preserve the donor squirrel's existing loot rolls.
        let pin = input.loot_tables.as_mut().ok_or("loot tables input")?;
        let mut document: serde_json::Value = serde_json::from_slice(&pin.bytes)?;
        let items = document["items"].as_array_mut().ok_or("loot item facts")?;
        for (key, capacity) in [("oteryn:item.tibia.i276", Some(15)), (BAIT, None)] {
            let facts = items
                .iter_mut()
                .find(|facts| facts["item"]["key"] == key)
                .ok_or("squirrel loot item facts")?;
            facts["materializable"] = json!(true);
            facts["container_capacity"] = json!(capacity);
        }
        pin.bytes = serde_json::to_vec(&document)?;
        pin.sha256 = Sha256::digest(&pin.bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
    }
    crate::content::native_gameplay::test_support::try_activate_input(world, &input).map_err(debug)
}
