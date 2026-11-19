use super::*;
use serde_json::json;

pub(super) const CLAIM: &str = "oteryn:reward-claim.nrwicked-forgemaster";
const ACCESS_CALLBACK: &str = "oteryn:quest-reward/nrwicked-forgemaster-key";
const QUEST: &str = "oteryn:quest.no_rest_for_the_wicked_quest";
const ACCESS: &str = "oteryn:quest-progress/crystalserver/quest/u14_10/no_rest_for_the_wicked/forge_master/access_door";
const DOOR_STAGE: &str = "oteryn:quest-progress/crystalserver/quest/u14_10/no_rest_for_the_wicked/forgemaster_door/questline";
pub(super) const CLAIMED: &str = "oteryn:quest-progress/crystalserver/quest/u14_10/no_rest_for_the_wicked/forgemaster_door/reward";
pub(super) const CHEST: (u16, u16, i8) = (34061, 32334, -14);
const REWARDS: [u16; 11] = [
    49520, 49522, 49523, 49524, 49525, 49526, 49527, 49530, 49528, 49529, 50250,
];

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AccessPolicy {
    Verified,
    SeparateRewardMarkers,
    AccessFlagOnly,
}

fn key(item: u16) -> String {
    format!("oteryn:item.tibia.i{item}")
}

pub(super) fn selectors() -> impl Iterator<Item = (u16, u16)> {
    (57237..=57247).map(|uid| {
        let x = match uid {
            57237 => CHEST.0 - 2,
            // This selector breaks the sequence of reward item IDs.
            57244 => CHEST.0,
            _ => CHEST.0 + (uid - 57237),
        };
        (x, uid)
    })
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
    policy: AccessPolicy,
) -> TestResult<std::sync::Arc<oteryn_gameplay_quests::quest::QuestStateCatalogue>> {
    use oteryn_gameplay_quests::quest::{QuestStateCatalogue, authored_bindings::Bindings};
    let digest: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    let shared_marker = policy != AccessPolicy::SeparateRewardMarkers;
    let claim_track = |uid| {
        if shared_marker {
            CLAIMED.to_owned()
        } else {
            format!("{CLAIMED}/{uid}")
        }
    };
    let mut tracks = vec![json!({"key": ACCESS, "initial": 0, "minimum": 0, "maximum": 1})];
    let mut requirements = vec![json!({"track":ACCESS,"minimum":1,"maximum":1})];
    let mut key_grant = vec![json!({"track":ACCESS,"minimum":0,"maximum":0,"set":1})];
    if policy != AccessPolicy::AccessFlagOnly {
        tracks.push(json!({"key":DOOR_STAGE,"initial":2,"minimum":0,"maximum":3}));
        requirements.push(json!({"track":DOOR_STAGE,"minimum":3,"maximum":3}));
        key_grant.push(json!({"track":DOOR_STAGE,"minimum":2,"maximum":2,"set":3}));
    }
    if shared_marker {
        tracks.push(json!({"key": CLAIMED, "initial": 0, "minimum": 0, "maximum": 1}));
    } else {
        tracks.extend(selectors().map(
            |(_, uid)| json!({"key": claim_track(uid), "initial": 0, "minimum": 0, "maximum": 1}),
        ));
    }
    let mut stages: Vec<_> = selectors().zip(REWARDS).map(|((_, uid), reward)| json!({
        "key":format!("claim-{uid}"),"completes":false,"trigger":{
            "kind":"npc_dialogue","callback":{
                "key":callback_key(uid),"quest":QUEST,"npc":"","keywords":[],"topic":0,"next_topic":0,
                "world_claim":{"key":CLAIM,"revision":"nrwicked-r1","authority_sha256":digest},
                "requires":requirements,
                "storage":[{"track":claim_track(uid),"minimum":0,"maximum":0,"set":1}],
                "give":{"item":key(reward),"count":1,"contents":[]},
                "take":[],"present":[],"outfit":null,"achievement":null,"reply":["You have found a reward."],"sources":["Crystal Forgemaster reward fixture"]
            }
        }
    })).collect();
    stages.push(json!({
        "key": "access", "completes": false, "trigger": {
            "kind": "npc_dialogue", "callback": {
                "key": ACCESS_CALLBACK, "quest": QUEST, "npc": "oteryn:npc.the_forgemaster",
                "keywords": ["key"], "topic": 0, "next_topic": 0,
                "storage": key_grant,
                "give": null, "outfit": null, "achievement": null, "take": [], "present": [],
                "reply": ["You have proven yourself worthy."], "sources": ["Crystal Forgemaster key fixture"]
            }
        }
    }));
    let bindings = Bindings::parse(&serde_json::to_vec(&json!({
        "schema":1,"quests":[{
            "quest":QUEST,"recipe_sha256":"1".repeat(64),"sources":["Crystal Forgemaster reward fixture"],
            "tracks":tracks,
            "stages":stages
        }]
    }))?)?;
    let base = QuestStateCatalogue::empty("content-1").ok_or("fixture revision")?;
    Ok(std::sync::Arc::new(bindings.attach(&base)?))
}

pub(super) async fn allow_access(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &crate::gameplay_transport::connection::AdmittedSession,
) -> TestResult {
    use crate::durability::quest_state::{
        QuestCause, QuestTransitionOutcome, QuestTransitionRequest,
    };
    use crate::durability::reward_claim_mint::QuestRewardAdmission;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "Forgemaster key fence read failed")?
        .ok_or("Forgemaster key fence")?;
    let native = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .ok_or("Forgemaster native generation")?;
    let catalogue = owner
        .quest_catalogue
        .clone()
        .ok_or("Forgemaster catalogue")?;
    let weights = NativeItemWeights(native);
    let result = owner
        .revision_sequencer
        .acquire(fence.character_id)
        .await
        .commit_quest_reward(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            QuestTransitionRequest {
                transition_key: ACCESS_CALLBACK.into(),
                cause: QuestCause::Command(CommandRef::new(
                    admitted.game_session_id,
                    CommandId::new(99).map_err(debug)?,
                )),
            },
            catalogue,
            QuestRewardAdmission {
                native,
                weights: &weights,
                fee_payment: None,
                actor_conditions: None,
                carried_use: None,
                creature_use: None,
                peer_use: None,
            },
            &owner.premium_coordinator,
        )
        .await
        .map_err(debug)?;
    assert!(
        matches!(result, QuestTransitionOutcome::Committed(_)),
        "{result:?}"
    );
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
        .use_world_object(
            actor,
            crate::gameplay_transport::connection::UseCommand {
                game_session_id: admitted.game_session_id,
                command_id,
                item_fence: Some(current.item_fence()),
            },
            target.clone(),
        )
        .await;
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

pub(super) const SERVED_CHEST: (u16, u16, i8) = (34101, 32346, -14);

pub(super) fn assert_production_choices(
    placements: &[crate::content::world_reward_claims::BoundPlacement],
) -> TestResult {
    use oteryn_world_bundle::bundle::placement_key;

    assert_eq!(placements.len(), REWARDS.len());
    for ((x, uid), reward) in (34087..=34107).step_by(2).zip(57237..=57247).zip(REWARDS) {
        let (_, y, floor) = SERVED_CHEST;
        let ordinal = if x == 34107 { 3 } else { 2 };
        let selector = placement_key(floor, x, y, ordinal).ok_or("Forgemaster chest selector")?;
        let bound = placements
            .iter()
            .find(|bound| bound.bundle_placement_key == selector)
            .ok_or("Forgemaster chest missing at its source selector")?;
        assert_eq!(
            bound.reward_callback.as_deref(),
            Some(callback_key(uid).as_str())
        );
        let [item] = bound.placement.items.as_slice() else {
            return Err("Forgemaster chest must offer exactly one item".into());
        };
        assert_eq!(item.item.key().as_str(), key(reward));
        assert_eq!(item.count, 1);
    }
    Ok(())
}

pub(super) fn production_chests(
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    use crate::map::{boot, overlay::TilePos};
    use oteryn_world_bundle::{bundle, sector, writer};

    let (bytes, mut pins) = boot::tests::bundle();
    let mut manifest = bundle::read(&bytes)?.manifest;
    let (chosen, y, floor) = SERVED_CHEST;
    let (west, east) = (34085, 34108);
    manifest.world = bundle::Extent {
        min_x: west, min_y: y - 1, max_x: east + 1, max_y: y + 1, floors: vec![floor],
    };
    manifest.palette.truncate(1);
    for id in [2187, 6381, 6385, 2472] {
        let key = format!("oteryn:item.tibia.i{id}");
        let definition = boot::native_item_definition(Some(native), &key).ok_or("chest tile definition")?;
        manifest.palette.push(bundle::PaletteEntry {
            key, family: bundle::Family::Item, id: definition.reference.get() - 1, terrain: None,
        });
    }
    let mut sectors = std::collections::BTreeMap::<_, Vec<_>>::new();
    let stack_item = |palette| sector::Item { palette, depth: 0, attrs: Default::default() };
    for row in [y - 1, y] {
        for x in west..=east {
            let mut items = vec![stack_item(0)];
            if row == y && (34087..=34107).contains(&x) && x % 2 == 1 {
                if x == 34107 {
                    items.extend([stack_item(2), stack_item(3)]);
                } else {
                    items.push(stack_item(1));
                }
                items.push(stack_item(4));
            }
            sectors.entry((x / 32, row / 32)).or_default().push(sector::Tile {
                x, y: row, flags: 0, house: 0, zones: vec![], items,
            });
        }
    }
    let mut sectors: Vec<_> = sectors.into_iter().map(|((sx, sy), tiles)| {
        bundle::Sector { floor, sx, sy, tiles }
    }).collect();
    sectors.sort_by_key(bundle::order);
    let bytes = writer::write(&manifest, &sectors, &Default::default())?;
    pins.digest = bundle::read(&bytes)?.digest;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    boot::boot(
        &bytes,
        &boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start: TilePos { x: chosen - 1, y, floor },
        },
        crate::foundation::WorldId::decode(&id(42)).map_err(debug)?,
        crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
        |key| boot::native_item_definition(Some(native), key),
    ).map_err(|error| debug(error).into())
}

pub(super) async fn ask_for_key(
    h: &Harness,
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &crate::gameplay_transport::connection::AdmittedSession,
    character: oteryn_gameplay_domain::domain::CharacterId,
) -> TestResult {
    use super::super::classic_quests_fixtures::{install, talk};
    const NPC: &str = "oteryn:npc.the_forgemaster";
    const HARD_KILLS: &str = "oteryn:quest-progress/crystalserver/quest/u14_10/no_rest_for_the_wicked/hard_kills";

    let [npc] = install(owner, admitted, [NPC]).await?;
    talk(owner, admitted, npc, 10, "hi").await?;
    let refusal = talk(owner, admitted, npc, 11, "key").await?;
    assert_eq!(
        refusal,
        ["You are not ready yet. Proove yourself worthy first."]
    );
    let rows = owner
        .root
        .read_character_quest_state(owner.character, character)
        .await
        .map_err(debug)?;
    assert_ne!(rows.tracks().get(ACCESS), Some(&1));
    assert_ne!(rows.tracks().get(DOOR_STAGE), Some(&3));

    for (track, value) in [(HARD_KILLS, 10_i64), (DOOR_STAGE, 2)] {
        let inserted = sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) SELECT character_id,$2,$3,$4,character_revision FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
            .bind(character.as_bytes().as_slice()).bind(track).bind(QUEST).bind(value)
            .execute(&h.pool).await?;
        assert_eq!(inserted.rows_affected(), 1);
    }
    owner.refresh_quest_session(admitted.game_session_id, true).await;
    let lines = talk(owner, admitted, npc, 12, "key").await?;
    assert_eq!(
        lines,
        ["You have proven yourself worthy. The door to the Forgemaster's chamber is now accessible."]
    );
    let rows = owner.root.read_character_quest_state(owner.character, character).await.map_err(debug)?;
    assert_eq!(rows.tracks().get(ACCESS), Some(&1));
    assert_eq!(rows.tracks().get(DOOR_STAGE), Some(&3));
    Ok(())
}
