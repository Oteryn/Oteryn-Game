use super::fixture::{Harness, debug, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Offering, Position};
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, encode_item_use_intent,
};
use std::{sync::Arc, time::Duration};

pub(super) const NAME: &str = "qengine-rewards-shadow-nexus";
const TARGET: Position = Position {
    x: 101,
    y: 100,
    floor: -7,
};
const QUEST: &str = "oteryn:quest.the_inquisition_quest";
const TRACK: &str = "oteryn:quest-progress/quest/u8_2/the_inquisition_quest/questline";
const MISSION: &str = "oteryn:quest-progress/quest/u8_2/the_inquisition_quest/mission07";
const WATER: &str = "oteryn:item.tibia.i133";
const CHARGE: &str = "oteryn:quest-charge/inquisition/shadow-nexus";

pub(super) fn shadow_nexus_native_use_with_decays_and_spends_only_the_shared_charge_count_scenario()
-> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run(NAME, || {
        super::quest_world_callbacks_fixtures::scenario(NAME, TARGET, false)
    })
}

pub(super) fn callback(spend: bool) -> TestResult<Callback> {
    // Crystal 201189f8 actions_holy_water.lua:105-135, data/items/items.xml:21313-21336.
    let mut row = serde_json::json!({
        "key": if spend { "oteryn:quest-callback/nexus-spend-fixture" } else { "oteryn:quest-callback/nexus-charge-fixture" },
        "quest":QUEST,"trigger":"use","selector":{
            "appearance":7925,"position":TARGET
        },
        "reversible":false,"passages":[],"offerings":[],"storage":[],"return_on_failure":false,
        "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/others/actions_holy_water.lua:105-135"]
    });
    if spend {
        row["requires_world_admission"] = true.into();
        row["storage"] = serde_json::json!([
            {"track":TRACK,"minimum":0,"maximum":21,"set":22},
            {"track":MISSION,"minimum":0,"maximum":5,"set":2}
        ]);
        row["item_on_target"] =
            serde_json::json!({"item":WATER,"count":1,"selected_instance":true});
        row["effects"] = serde_json::json!({
            "charge_window":{"kind":"spend","group":CHARGE,"active":7931,"radius":3,
                "cues":[
                    {"position":{"x":99,"y":100,"floor":-7},"effect":50},
                    {"position":{"x":102,"y":100,"floor":-7},"effect":50},
                    {"position":TARGET,"effect":50}
                ],
                "announcement":"destroyed the shadow nexus! In 10 seconds it will return to its original state."}
        });
    } else {
        row["effects"] = serde_json::json!({
            "charge_window":{"kind":"charge","group":CHARGE,"tool":WATER,"radius":3,"cue":12,
                "announcement":"damaged the shadow nexus! You can't damage it while it's burning.",
                "phases":[
                    {"from":7925,"burning":7926,"ready":7927,"burn_ms":30000,"ready_ms":20000},
                    {"from":7927,"burning":7928,"ready":7929,"burn_ms":30000,"ready_ms":20000},
                    {"from":7929,"burning":7930,"ready":7931,"burn_ms":30000,"ready_ms":20000,"capacity":[4,5]}
                ]}
        });
    }
    Ok(serde_json::from_value(row)?)
}

fn appearance(owner: &ComposedFreshAdmission<'_, '_, '_>) -> TestResult<u16> {
    let map = owner.bundle_map().ok_or("map")?;
    let at = crate::map::overlay::TilePos {
        x: TARGET.x,
        y: TARGET.y,
        floor: TARGET.floor,
    };
    let tile = map
        .overlay
        .base()
        .tile(at.x, at.y, at.floor)
        .ok_or("nexus tile")?;
    let id = *tile.ids().get(1).ok_or("nexus placement")?;
    Ok(map
        .facts
        .base_entry(at, 1, id)
        .ok_or("nexus appearance")?
        .appearance_id)
}

async fn use_water(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult<UseDisposition> {
    let reference = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .and_then(|native| native.item_index())
        .and_then(|index| index.definition_ref("Item", WATER, "definition-r1"))
        .ok_or("holy water reference")?;
    let map = owner.bundle_map().ok_or("map")?;
    let key = oteryn_world_bundle::bundle::placement_key(TARGET.floor, TARGET.x, TARGET.y, 1)
        .ok_or("nexus key")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::ItemByDefinition(reference),
        use_with: None,
        use_with_position: None,
        use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget {
            placement,
            expected_revision: map.facts.object_revision(key),
        })),
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
    Ok(disposition)
}

async fn give_water(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    ordinal: u64,
) -> TestResult {
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    super::quest_world_callbacks_fixtures::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: TARGET,
            item: WATER.into(),
            count: 1,
        },
        ordinal,
        true,
    )
    .await
}

async fn advance(owner: &mut ComposedFreshAdmission<'_, '_, '_>, seconds: u64) -> TestResult {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(Duration::from_secs(seconds))
        .ok_or("clock")?;
    owner.tick_timed_items().await;
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let catalogue = owner.quest_catalogue.as_ref().ok_or("catalogue")?;
    let digest = catalogue
        .definition_hash(QUEST)
        .ok_or("prior quest definition")?;
    let mut repeat = callback(true)?;
    repeat.key = "oteryn:quest-callback/nexus-repeat-fixture".into();
    repeat.storage.retain(|storage| storage.track == TRACK);
    repeat.storage[0].minimum = 22;
    repeat.storage[0].maximum = 25;

    owner.quest_catalogue = Some(Arc::new(catalogue.with_world_callback_fixtures(vec![
        callback(false)?,
        callback(true)?,
        repeat,
    ])?));
    sqlx::query(
        "INSERT INTO game_character_quest_states(character_id,quest_key,pinned_content_revision,definition_hash,started_character_revision,committed_character_revision)
         SELECT character_id,$2,'content-1',$3,character_revision,character_revision
         FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
    ).bind(id(41).as_slice()).bind(QUEST).bind(digest.as_slice()).execute(&h.pool).await?;
    sqlx::query(
        "INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision)
         SELECT s.character_id,v.track,s.quest_key,v.stage,s.committed_character_revision
         FROM game_character_quest_states s
         CROSS JOIN (VALUES ($2::text,21::bigint),($3::text,4::bigint)) AS v(track,stage)
         WHERE s.character_id=encode($1,'hex')::uuid AND s.quest_key=$4",
    ).bind(id(41).as_slice()).bind(TRACK).bind(MISSION).bind(QUEST).execute(&h.pool).await?;
    give_water(owner, admitted, 975).await?;
    assert_eq!(appearance(owner)?, 7925);
    for (burning, ready) in [(7926, 7927), (7928, 7929), (7930, 7931)] {
        assert_eq!(use_water(owner, admitted).await?, UseDisposition::Committed);
        assert_eq!(appearance(owner)?, burning);
        advance(owner, 31).await?;
        assert_eq!(appearance(owner)?, ready);
    }
    let capacity: i16 = sqlx::query_scalar(
        "SELECT remaining FROM game_quest_world_charge_windows WHERE charge_group=$1",
    )
    .bind(CHARGE)
    .fetch_one(&h.pool)
    .await?;
    assert!((4..=5).contains(&capacity));
    // The map fixture uses a sampled owner clock. Bring the SQL observation into
    // the same ready window without sleeping or replacing the writer.
    sqlx::query(
        "UPDATE game_quest_world_charge_windows SET ready_at=floor(extract(epoch FROM clock_timestamp())*1000)::bigint
         WHERE charge_group=$1",
    ).bind(CHARGE).execute(&h.pool).await?;
    for attempt in 0..capacity {
        if attempt == 1 {
            // Crystal actions_holy_water.lua:122-132 rewrites questline even above 22,
            // while later characters keep their existing Mission07.
            sqlx::query(
                "UPDATE game_character_quest_tracks SET value=CASE WHEN track_key=$1 THEN 25 ELSE 1 END
                 WHERE character_id=encode($2,'hex')::uuid AND track_key IN ($1,$3)",
            ).bind(TRACK).bind(id(41).as_slice()).bind(MISSION).execute(&h.pool).await?;
        }
        if attempt > 0 {
            give_water(owner, admitted, 976 + u64::try_from(attempt)?).await?;
        }
        assert_eq!(use_water(owner, admitted).await?, UseDisposition::Committed);
        assert_eq!(
            appearance(owner)?,
            7931,
            "an admitted debit leaves the remaining window visible"
        );
        if attempt == 0 {
            let map = owner.bundle_map().ok_or("map")?;
            let mut lights: Vec<_> = map
                .facts
                .movement_effects_since(0)
                .into_iter()
                .filter_map(|(_, at, effect)| (effect == 50).then_some(at.x))
                .collect();
            lights.sort_unstable();
            assert_eq!(lights, [99, 101, 102]);
        }
        let (stage, mission): (i64, i64) = sqlx::query_as(
            "SELECT (SELECT value FROM game_character_quest_tracks WHERE track_key=$1),
                    (SELECT value FROM game_character_quest_tracks WHERE track_key=$2)",
        )
        .bind(TRACK)
        .bind(MISSION)
        .fetch_one(&h.pool)
        .await?;
        assert_eq!((stage, mission), (22, if attempt == 0 { 2 } else { 1 }));

        advance(owner, 1).await?;
    }
    give_water(owner, admitted, 989).await?;
    assert_eq!(use_water(owner, admitted).await?, UseDisposition::Committed);
    assert_eq!(appearance(owner)?, 7925);
    let outcome: (i64,i64,i64,i16) = sqlx::query_as(
        "SELECT (SELECT value FROM game_character_quest_tracks WHERE track_key=$1),
                (SELECT value FROM game_character_quest_tracks WHERE track_key=$2),
                (SELECT count(*) FROM game_item_instances WHERE lifecycle=1 AND definition_production_key=$3),
                (SELECT remaining FROM game_quest_world_charge_windows WHERE charge_group=$4)",
    ).bind(TRACK).bind(MISSION).bind(WATER).bind(CHARGE).fetch_one(&h.pool).await?;
    assert_eq!(outcome, (22, 1, 1, 0));
    Ok(())
}
