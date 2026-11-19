//! Crystal 201189f8 data-global/npc/marvin.lua:57-85.
use std::sync::Arc;

use super::classic_quests_fixtures::{install, talk};
use super::fixture::{Harness, debug, id, run};
use super::{
    AdmittedSession, ComposedFreshAdmission, Dependencies, TestResult, prepare_current_session,
};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::gameplay_transport::connection::FirstEntryOutcome;
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, QuestTrack, authored_bindings::Bindings};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

const DARK_TRAILS: &str = "oteryn:quest.dark_trails_quest";
const ORAMOND: &str = "oteryn:quest.oramond_quest";
const VOTES: &str = "oteryn:quest-progress/quest/u10_50/oramond_quest/voting_points";
const ARCHIVES: &str = "oteryn:quest-progress/quest/u10_50/dark_trails/mission08";

pub(super) fn vote_case(admin: String, database: &'static str, before: i64, after: i64) -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            run(async |_| {
                let h = Harness::create_with_admission_seed(admin, database, false, false).await?;
                let result = Box::pin(exercise(&h, before, after)).await;
                let cleanup = h.cleanup().await;
                result?;
                cleanup
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "archivist vote case panicked")??;
    Ok(())
}

fn catalogue(before: i64) -> TestResult<QuestStateCatalogue> {
    let base = QuestStateCatalogue::new(
        "content-1",
        vec![QuestTrack {
            key: VOTES.into(),
            quest: ORAMOND.into(),
            initial: before,
            min: -1,
            max: i64::MAX,
        }],
        vec![],
    )
    .map_err(debug)?;
    let source =
        "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/marvin.lua:77-85";
    // Crystal marvin.lua:79-83 decrements a positive balance and always
    // credits the mission. The conditional branches use existing authored
    // state operations and recheck the selected balance under the owner lock.
    let debit = serde_json::json!({
        "key":"oteryn:quest-reward/fixture/archive-vote","quest":DARK_TRAILS,"npc":"oteryn:npc.marvin",
        "topic":0,"next_topic":0,"keywords":["yes"],
        "storage":[{"track":ARCHIVES,"minimum":-1,"maximum":0,"set":1}],
        "participants":[{
            "quest":ORAMOND,"storage":[],"state_ops":[{
                "track":VOTES,
                "from":{"kind":"between","minimum":0,"maximum":i64::MAX},
                "effect":{"kind":"add_at_least","value":-1,"minimum":0}
            }]
        }],
        "take":[],"reply":["Vote recorded."],"sources":[source]
    });
    let mut unset = debit.clone();
    unset["key"] = serde_json::json!("oteryn:quest-reward/fixture/archive-vote-unset");
    unset["participants"] = serde_json::json!([]);
    unset["requires"] = serde_json::json!([{"track":VOTES,"minimum":-1,"maximum":-1}]);
    let binding = Bindings::parse(serde_json::json!({
        "schema":1,"quests":[{
            "quest":DARK_TRAILS,
            "recipe_sha256":<sha2::Sha256 as sha2::Digest>::digest(source).iter().map(|byte| format!("{byte:02x}")).collect::<String>(),
            "tracks":[{"key":ARCHIVES,"initial":-1,"minimum":-1,"maximum":1}],
            "stages":[{"key":"s1","completes":false,"trigger":{
                "kind":"npc_dialogue","callback":debit,"alternatives":[unset]
            }}],
            "sources":[source]
        }]
    }).to_string().as_bytes())?;
    Ok(binding.attach(&base)?)
}

async fn exercise(h: &Harness, before: i64, after: i64) -> TestResult {
    with_npc(h, Arc::new(catalogue(before)?), "oteryn:npc.marvin", async |owner, admitted, marvin| {
    talk(owner, admitted, marvin, 1, "hi").await?;
    let reply = talk(owner, admitted, marvin, 2, "yes").await?;
    assert_eq!(reply, ["Vote recorded."]);
    let read = || async {
        sqlx::query_as::<_, (String, i64, String)>(
            "SELECT track_key, value, committed_character_revision::text FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key IN ($2,$3) ORDER BY track_key",
        ).bind(id(41).as_slice()).bind(ARCHIVES).bind(VOTES).fetch_all(&h.pool).await
    };
    let rows = read().await?;
    assert_eq!(rows.len(), if before < 0 { 1 } else { 2 });
    assert_eq!((&*rows[0].0, rows[0].1), (ARCHIVES, 1));
    if before >= 0 {
        assert_eq!((&*rows[1].0, rows[1].1), (VOTES, after));
        assert_eq!(rows[0].2, rows[1].2);
    }
    let _ = talk(owner, admitted, marvin, 2, "yes").await?;
    assert_eq!(read().await?, rows);
    Ok(())
    }).await
}

pub(super) async fn with_npc(
    h: &Harness,
    catalogue: Arc<QuestStateCatalogue>,
    npc: &str,
    exercise: impl AsyncFnOnce(
        &mut ComposedFreshAdmission<'_, '_, '_>,
        &AdmittedSession,
        EntityRef,
    ) -> TestResult,
) -> TestResult {
    with_npc_items(h, catalogue, npc, &[], exercise).await
}

pub(super) struct FixtureItem {
    pub id: u16,
    pub name: &'static str,
    pub weight: u16,
    pub stackable: bool,
}

pub(super) fn install_fixture_item_profiles(
    input: &mut crate::content::native_gameplay::NativeGameplayInput,
    items: &[FixtureItem],
) -> TestResult {
    use serde_json::{Value, json};

    let mut profiles: Value = serde_json::from_slice(
        &input
            .item_profiles
            .as_ref()
            .ok_or("native item profiles")?
            .bytes,
    )?;
    let records = profiles["records"]
        .as_array_mut()
        .ok_or("profile records")?;
    for item in items {
        let key = format!("oteryn:item.tibia.i{}", item.id);
        let mut record = records
            .iter()
            .find(|record| {
                record["production_definition"]["production_key"]
                    == if item.stackable {
                        "oteryn:item.tibia.i3031"
                    } else {
                        "oteryn:item.tibia.i123"
                    }
            })
            .ok_or("plain test item profile template")?
            .clone();
        record["production_definition"]["production_key"] = json!(key);
        record["production_binding"]["target"]["key"] = json!(key);
        record["production_binding"]["external_id"] = json!(item.id.to_string());
        record["authoring"]["item"]["key"] = json!(format!("candidate:item/{}", item.id));
        record["semantics"]["presentation"]["value"]["name"] =
            json!({"state":"KNOWN","value":item.name});
        record["semantics"]["physical"]["value"]["weight"] =
            json!({"state":"KNOWN","value":item.weight});
        records.push(record);
    }
    input.item_profiles = Some(pin_fixture_json(&profiles)?);
    Ok(())
}

fn pin_fixture_json(
    document: &serde_json::Value,
) -> TestResult<crate::content::native_gameplay::PinnedGameplayBytes> {
    let bytes = serde_json::to_vec(document)?;
    Ok(crate::content::native_gameplay::PinnedGameplayBytes {
        sha256: <sha2::Sha256 as sha2::Digest>::digest(&bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
        bytes,
    })
}

pub(super) async fn with_npc_items(
    h: &Harness,
    catalogue: Arc<QuestStateCatalogue>,
    npc: &str,
    items: &[FixtureItem],
    exercise: impl AsyncFnOnce(
        &mut ComposedFreshAdmission<'_, '_, '_>,
        &AdmittedSession,
        EntityRef,
    ) -> TestResult,
) -> TestResult {
    with_npc_items_and_loot(h, catalogue, npc, items, None, exercise).await
}

pub(super) struct FixtureCorpseLoot {
    pub corpse: &'static str,
    pub table: &'static str,
}

pub(super) async fn with_npc_items_and_loot(
    h: &Harness,
    catalogue: Arc<QuestStateCatalogue>,
    npc: &str,
    items: &[FixtureItem],
    corpse: Option<FixtureCorpseLoot>,
    exercise: impl AsyncFnOnce(
        &mut ComposedFreshAdmission<'_, '_, '_>,
        &AdmittedSession,
        EntityRef,
    ) -> TestResult,
) -> TestResult {
    prepare_current_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world_id = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
    let corpse_world = corpse.is_some();
    let (controller, activated) =
        if items.is_empty() && !corpse_world {
            super::death_ml_loss_composed_fixtures::production_generation_with_pin(world_id, "content-1")?
        } else {
            use crate::content::native_gameplay::test_support;
            use serde_json::{Value, json};
            let source = corpse_world.then_some(
                include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../tools/content-schema/native-gameplay/canary-thalom-world.json"
                ))
                .as_slice(),
            );
            let mut input =
                super::death_ml_loss_composed_fixtures::production_input_with_source("content-1", source)?;
            install_fixture_item_profiles(&mut input, items)?;
            if let Some(corpse) = corpse {
                let mut loot: Value = serde_json::from_slice(
                    &input
                        .loot_tables
                        .as_ref()
                        .ok_or("native loot tables")?
                        .bytes,
                )?;
                let facts = loot["items"]
                    .as_array_mut()
                    .ok_or("loot item facts")?
                    .iter_mut()
                    .find(|facts| facts["item"]["key"] == corpse.corpse)
                    .ok_or("fixture corpse loot facts")?;
                // The fixture isolates death MINT and decay from unrelated loot admission.
                facts["materializable"] = json!(true);
                let table = loot["tables"]
                    .as_array_mut()
                    .ok_or("loot table definitions")?
                    .iter_mut()
                    .find(|table| table["identity"]["key"] == corpse.table)
                    .ok_or("fixture creature loot table")?;
                table["entries"] = json!([]);
                input.loot_tables = Some(pin_fixture_json(&loot)?);
            }
            test_support::try_activate_input(world_id, &input).map_err(debug)?
        };
    let generation = controller.active().ok_or("active content")?;
    let room = activated.qualified_room();
    let start = room.entry_start();
    let digest = generation.identity().server_artifact_digest();
    let client_digest = generation.identity().client_artifact_digest();
    let frame = activated.frame_binding().digest();
    let pin = ChannelContentPin::from_activation(
        world_id,
        1,
        digest,
        client_digest,
        frame,
        room.map_revision_digest(),
        (start.x, start.y, start.floor),
    );
    let mut deps = Dependencies::with_content_and_capacity(
        pin,
        room.clone(),
        crate::spell::cast::v1_spell_book().map_err(debug)?,
        if corpse_world { 3 } else { 2 },
    )?;
    if corpse_world {
        deps.combat_room = Some(room.clone());
    }
    deps.runtime
        .lock()
        .await
        .initialize_first_entry_position(deps.actor)
        .map_err(debug)?;
    let mut owner = deps.owner(h, &authority);
    owner.active_generation = Some(generation);
    owner.quest_catalogue = Some(catalogue);
    let mut admitted = deps.admitted;
    admitted.first_entry = FirstEntryOutcome::Positioned;

    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(digest.as_slice()).bind(client_digest.as_slice())
        .bind(frame.as_slice()).execute(&h.pool).await?;
    let [npc] = install(&owner, &admitted, [npc]).await?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    admitted.item_fence = Some(fence.item_fence());
    let fresh = super::fresh_fixture::fresh::Source::new(0)?.request()?;
    h.root
        .initialize_admission_equipment(
            &authority,
            &h.node,
            &fence.item_fence(),
            digest,
            fresh.operation(),
        )
        .await
        .map_err(debug)?;
    let native = generation.native_gameplay().ok_or("native gameplay")?;
    let template = crate::durability::starter_grant::StarterTemplate {
        definition: crate::durability::item_mint::TypedDefinitionRef {
            family: "Item".into(),
            production_key: "oteryn:item.tibia.i2854".into(),
            revision_ref: "definition-r1".into(),
        },
        ..crate::durability::starter_grant::StarterTemplate::main()
    };
    h.root
        .grant_starter_items(
            &authority,
            &h.node,
            &fence.item_fence(),
            &crate::gameplay_transport::item_ref_admission::NativeItemWeights(native),
            &template,
        )
        .await
        .map_err(debug)?;
    deps.initialize(&owner).await;
    owner.admit_progression_session(&mut admitted).await;
    owner.admit_quest_session(&admitted).await;
    Box::pin(exercise(&mut owner, &admitted, npc)).await
}
