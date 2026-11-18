use super::*;
use crate::content::native_gameplay::NativeGameplayInput;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

pub(super) const CLAIM: &str = "oteryn:reward-claim.nrwicked-forgemaster";
const QUEST: &str = "oteryn:quest.no_rest_for_the_wicked_quest";
const ACCESS: &str = "oteryn:quest-progress/crystalserver/quest/u14_10/no_rest_for_the_wicked/forge_master/access_door";
const CLAIMED: &str = "oteryn:quest-progress/crystalserver/quest/u14_10/no_rest_for_the_wicked/forgemaster_door/reward";
pub(super) const CHEST: (u16, u16, i8) = (34061, 32334, -14);
const REWARDS: [u16; 11] = [
    49520, 49522, 49523, 49524, 49525, 49526, 49527, 49530, 49528, 49529, 50250,
];

fn key(item: u16) -> String {
    format!("oteryn:item.tibia.i{item}")
}

pub(super) fn selectors() -> impl Iterator<Item = (u16, u16)> {
    (57237..=57247).enumerate().map(|(index, uid)| {
        let x = match index {
            0 => CHEST.0,
            1 => CHEST.0 - 2,
            _ => CHEST.0 + (uid - 57238),
        };
        (x, uid)
    })
}

pub(super) fn provision_items(input: &mut NativeGameplayInput) -> TestResult {
    let profiles = input.item_profiles.as_mut().ok_or("item profiles")?;
    let mut items: Value = serde_json::from_slice(&profiles.bytes)?;
    let bindings: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../imports/crystalserver/bindings/items.json"
    )))?;
    let canonical: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../content/items/definitions/items-27000-27499.json"
    )))?;
    let additional: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../content/items/definitions/items-27500-27999.json"
    )))?;
    let rows = items["records"].as_array_mut().ok_or("profile rows")?;
    for reward in REWARDS {
        let key = key(reward);
        let exact = bindings["bindings"]
            .as_array()
            .ok_or("Crystal bindings")?
            .iter()
            .find(|r| r["target"]["key"] == key)
            .ok_or("Crystal reward identity")?;
        let item = canonical["records"]
            .as_array()
            .ok_or("canonical items")?
            .iter()
            .chain(
                additional["records"]
                    .as_array()
                    .ok_or("additional canonical items")?,
            )
            .find(|r| r["definition"]["identity"]["key"] == key)
            .ok_or("canonical reward definition")?;
        rows.push(json!({
            "production_definition": {"family":"Item","production_key":key,"revision_ref":"definition-r1"},
            "production_binding": exact,
            "production_binding_qualification": null,
            "authoring": {"item":exact["target"]},
            "attributes": {"speed_bonus":0,"spell_augments":[]},
            "admission": {"materializable":true,"stack_class":"NonStackable","legal_destinations":["CharacterInventory"]},
            "semantics":item["definition"]["semantics"]
        }));
    }
    profiles.bytes = serde_json::to_vec(&items)?;
    profiles.sha256 = Sha256::digest(&profiles.bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    Ok(())
}

fn callback_key(uid: u16) -> String {
    format!("oteryn:quest-reward/nrwicked-forgemaster-{uid}")
}

pub(super) fn claim_shard() -> String {
    let placements: Vec<_> = selectors().zip(REWARDS).map(|((x, uid), reward)| json!({
        "appearance_tibia_id": 2472,
        "source_binding": {"native_entry_ordinal":1,"project_position":{"x":x,"y":CHEST.1,"z":-CHEST.2}},
        "reward":{"items":[{"count":1,"item":{"family":"Item","key":key(reward),"revision":"definition-r1"}}]},
        "reward_callback":callback_key(uid)
    })).collect();
    json!({"records":[{"definition":{
        "definition_profile":"native_authored_v1",
        "identity":{"key":CLAIM,"revision":"nrwicked-r1"},
        "readiness":"ready",
        "claim":{"per":"character","repeat":{"kind":"once"}},
        "placements":placements
    }}]})
    .to_string()
}

pub(super) fn catalogue(
    digest: [u8; 32],
) -> TestResult<std::sync::Arc<oteryn_gameplay_quests::quest::QuestStateCatalogue>> {
    use oteryn_gameplay_quests::quest::{QuestStateCatalogue, authored_bindings::Bindings};
    let digest: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    let stages: Vec<_> = selectors().zip(REWARDS).map(|((_, uid), reward)| json!({
        "key":format!("claim-{uid}"),"completes":false,"trigger":{
            "kind":"npc_dialogue","callback":{
                "key":callback_key(uid),"quest":QUEST,"npc":"","keywords":[],"topic":0,"next_topic":0,
                "world_claim":{"key":CLAIM,"revision":"nrwicked-r1","authority_sha256":digest},
                "requires":[{"track":ACCESS,"minimum":1,"maximum":1}],
                "storage":[{"track":CLAIMED,"minimum":0,"maximum":0,"set":1}],
                "give":{"item":key(reward),"count":1,"contents":[]},
                "take":[],"present":[],"outfit":null,"achievement":null,"reply":["You have found a reward."],"sources":["Crystal Forgemaster reward fixture"]
            }
        }
    })).collect();
    let bindings = Bindings::parse(&serde_json::to_vec(&json!({
        "schema":1,"quests":[{
            "quest":QUEST,"recipe_sha256":"1".repeat(64),"sources":["Crystal Forgemaster reward fixture"],
            "tracks":[{"key":ACCESS,"initial":0,"minimum":0,"maximum":1},
                      {"key":CLAIMED,"initial":0,"minimum":0,"maximum":1}],
            "stages":stages
        }]
    }))?)?;
    let base = QuestStateCatalogue::empty("content-1").ok_or("fixture revision")?;
    Ok(std::sync::Arc::new(bindings.attach(&base)?))
}

pub(super) async fn allow_access(harness: &Harness, character: &[u8; 16]) -> TestResult {
    sqlx::query(
        "INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision)
         SELECT character_id,$2,$3,1,GREATEST(character_revision, 2) FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(character.as_slice())
    .bind(ACCESS)
    .bind(QUEST)
    .execute(&harness.pool)
    .await?;
    Ok(())
}

pub(super) async fn use_chest(
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &crate::gameplay_transport::connection::AdmittedSession,
    actor: crate::foundation::ExactActorRef,
    command_id: u64,
    target: &WorldObjectTarget,
) -> TestResult<UseDisposition> {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(1))
        .ok_or("Forgemaster use clock")?;
    let current = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "Forgemaster claim fence read failed")?
        .ok_or("Forgemaster claim fence")?;
    let result = owner
        .use_bundle_chest(
            actor,
            crate::gameplay_transport::connection::UseCommand {
                game_session_id: admitted.game_session_id,
                command_id,
                item_fence: Some(current.item_fence()),
            },
            owner.bundle_map().ok_or("Forgemaster map")?,
            target,
        )
        .await
        .ok_or("Forgemaster use did not bind")?;
    Ok(result.disposition)
}

pub(super) async fn claim_count(harness: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT count(*) FROM game_reward_claim_mint_receipts WHERE claim_production_key=$1",
    )
    .bind(CLAIM)
    .fetch_one(&harness.pool)
    .await?)
}
