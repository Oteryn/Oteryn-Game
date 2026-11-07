//! Task-shop cosmetic grants reach the existing admitted appearance owner.
//! The authored packet below is deliberately test-only; it is never production content.
use super::*;
use crate::content::native_gameplay::test_support::cosmetics_fixture::cosmetics_test_packet;
use crate::domain::appearance::AppearanceContent;
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use serde_json::{Value, json};

#[path = "cosmetic_atomicity.rs"]
mod cosmetic_atomicity;
#[path = "wheel_case.rs"]
mod wheel_case;

// This private test packet still obeys ProductionKey: `fixture` is a forbidden
// key marker, even though its explicitly synthetic authoring stays test-only.
const OUTFIT: &str = "oteryn:outfit.task_board_proposed";
const MOUNT: &str = "oteryn:mount.task_board_proposed";
const PROVENANCE: &str = "task-board-shop-v1";

fn source_look(packet: &Value, look: u32) -> TestResult<Value> {
    for shard in packet["source_shards"].as_array().ok_or("source shards")? {
        let source: Value = serde_json::from_str(shard["json"].as_str().ok_or("source JSON")?)?;
        for record in source["records"].as_array().ok_or("source records")? {
            if record["source_id"] == look {
                return Ok(
                    json!({"id":look,"source_record_sha256":record["source_record_sha256"]}),
                );
            }
        }
    }
    Err(format!("the pinned client has no test look {look}").into())
}

/// Keep the source-bound fallback, then explicitly author one earned outfit and mount.
/// Both sex choices are synthetic test facts. The source record binds the rendered look
/// only; it does not claim these authoring decisions for the production catalogue.
pub(super) fn cosmetics_packet() -> TestResult<Vec<u8>> {
    let mut packet = cosmetics_test_packet();
    let outfit_look = source_look(&packet, 1907)?;
    let mount_look = source_look(&packet, 1908)?;
    let author = &mut packet["authoring"];
    author["document"]["content_revision"] = "content-1".into();
    let mut outfit = author["document"]["outfits"][0].clone();
    outfit["key"] = OUTFIT.into();
    outfit["male"] = outfit_look.clone();
    outfit["female"] = outfit_look;
    outfit["starter_male"] = false.into();
    outfit["starter_female"] = false.into();
    outfit["starter_addons_male"] = 0.into();
    outfit["starter_addons_female"] = 0.into();
    author["document"]["outfits"]
        .as_array_mut()
        .ok_or("outfits")?
        .push(outfit);
    author["document"]["mounts"][0]["key"] = MOUNT.into();
    author["document"]["mounts"][0]["look"] = mount_look;
    // Offer 2 intentionally has no mapping: a numeric look alone must grant nothing.
    author["document"]["grant_mappings"] = json!([
        {"grant_key":"oteryn:taskboard/shop/offer-0","target":{"kind":"Outfit","outfit_key":OUTFIT},"compatible_provenance":[PROVENANCE]},
        {"grant_key":"oteryn:taskboard/shop/offer-1","target":{"kind":"Addon","outfit_key":OUTFIT,"addon":1},"compatible_provenance":[PROVENANCE]},
        {"grant_key":"oteryn:taskboard/shop/offer-6","target":{"kind":"Mount","mount_key":MOUNT},"compatible_provenance":[PROVENANCE]}
    ]);
    let originals = author["promotions"].as_array().ok_or("promotions")?.clone();
    let mut promotions = originals.clone();
    for original in &originals {
        let path = original["path"].as_str().ok_or("promotion path")?;
        if let Some(field) = path.strip_prefix("/outfits/0/") {
            let mut promotion = original.clone();
            promotion["path"] = format!("/outfits/1/{field}").into();
            promotions.push(promotion);
        }
    }
    let mut mapping = originals.first().ok_or("promotion template")?.clone();
    mapping["path"] = "/grant_mappings".into();
    promotions.push(mapping);
    for promotion in &mut promotions {
        let path = promotion["path"].as_str().ok_or("promotion path")?;
        promotion["value"] = author["document"]
            .pointer(path)
            .ok_or("promoted value")?
            .clone();
    }
    author["promotions"] = promotions.into();
    Ok(serde_json::to_vec(&packet)?)
}

async fn receipt_count(harness: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT count(*) FROM game_account_appearance_unlock_receipts WHERE source_kind='task_board' AND granting_character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).fetch_one(&harness.pool).await?)
}

async fn appearance(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult<crate::gameplay_transport::appearance_change::observation::Observation> {
    let actor = admitted.runtime_actor.ok_or("current runtime actor")?;
    owner
        .appearance_observation(
            actor,
            admitted.game_session_id,
            vec![EntityRef {
                identity: actor.placement_identity(),
                generation: actor.actor_local_generation(),
            }],
        )
        .await
        .ok_or_else(|| "current appearance observation".into())
}

pub(super) async fn cosmetics_case(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    harness: &Harness,
    admitted: &mut AdmittedSession,
    inject_omission: bool,
) -> TestResult {
    // Admission seed predates the appearance fixture. Supply its historical creation sex;
    // every purchase and every earned-fact read below uses the production fences normally.
    let mut fixture = harness.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *fixture)
        .await?;
    sqlx::query("UPDATE game_character_roots SET sex=2 WHERE character_id=encode($1,'hex')::uuid AND sex IS NULL")
        .bind(id(41).as_slice()).execute(&mut *fixture).await?;
    fixture.commit().await?;

    let original = full_board(owner, admitted, 4, 0, "").await?.points;
    fixture_track(harness, task_board::POINTS, 1_000_000).await?;
    let before = appearance(owner, admitted).await?;
    assert!(
        !before
            .catalogue
            .outfits
            .iter()
            .any(|entry| entry.key == OUTFIT)
    );
    assert!(!before.catalogue.mounts.iter().any(|key| key == MOUNT));
    assert_eq!(
        full_board(owner, admitted, 16, 1, "").await?.refusal,
        8,
        "an addon cannot be purchased before its base outfit"
    );
    assert_eq!(receipt_count(harness).await?, 0);
    if inject_omission {
        cosmetic_atomicity::omission_case(owner, harness, admitted).await?;
        return Ok(());
    }

    let base_id = admitted.continuity.next_command_id;
    let base = full_board(owner, admitted, 16, 0, "").await?;
    assert_eq!((base.refusal, base.points), (0, 900_000));
    assert!(
        base.shop
            .iter()
            .any(|entry| entry.index == 0 && entry.bought)
    );
    let replay = serve_from(owner, admitted, base_id, &[full_request(16, 0, "")]).await?;
    assert_eq!(replay[0].1, CommandStatus::DuplicateReplay);
    assert_eq!(wire::full::decode_reply(&replay[0].2).map_err(debug)?, base);
    let duplicate = full_board(owner, admitted, 16, 0, "").await?;
    assert_eq!((duplicate.refusal, duplicate.points), (7, 900_000));
    assert_eq!(receipt_count(harness).await?, 1);
    let addon = full_board(owner, admitted, 16, 1, "").await?;
    assert_eq!((addon.refusal, addon.points), (0, 865_000));
    let missing_mapping = full_board(owner, admitted, 16, 2, "").await?;
    assert_eq!(
        (missing_mapping.refusal, missing_mapping.points),
        (17, 865_000)
    );
    let mount = full_board(owner, admitted, 16, 6, "").await?;
    assert_eq!((mount.refusal, mount.points), (0, 720_000));
    assert_eq!(receipt_count(harness).await?, 3);
    assert_eq!(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_task_board_cosmetic_grants WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).fetch_one(&harness.pool).await?, 3);

    // This consumer reads task_board provenance through the existing account appearance
    // owner; the JSON purchase index alone cannot put these options into the catalogue.
    let after = appearance(owner, admitted).await?;
    let outfit = after
        .catalogue
        .outfits
        .iter()
        .find(|entry| entry.key == OUTFIT)
        .ok_or("earned outfit")?;
    assert_eq!((outfit.look_type, outfit.permitted_addons), (1907, 1));
    assert!(after.catalogue.mounts.iter().any(|key| key == MOUNT));
    let current = crate::gameplay_transport::character_gameplay_fence::current(
        owner,
        admitted.game_session_id,
        false,
    )
    .await
    .ok_or("current appearance fence")?;
    let cosmetics = owner
        .task_board_catalogue()
        .ok_or("task catalogue")?
        .cosmetics
        .ok_or("admitted cosmetics")?;
    let reloaded = owner
        .root
        .read_current_character_appearance_state(
            owner.character,
            owner.holder,
            current,
            cosmetics.earned_grant_keys(),
        )
        .await
        .map_err(debug)?;
    assert_eq!(
        reloaded
            .earned
            .iter()
            .filter(|(_, provenance)| provenance == PROVENANCE)
            .count(),
        3
    );
    println!(
        "<TASK_BOARD> POSTGRES_CASE_COMPLETED: cosmetic_dispatch_grant_existing_consumer_and_replay"
    );

    unavailable_catalogue_case(owner, harness, admitted).await?;
    let promotion = full_board(owner, admitted, 16, 23, "").await?;
    assert_eq!((promotion.refusal, promotion.points), (0, 719_900));
    let mut connection = harness.pool.acquire().await?;
    let points = crate::durability::task_board::read_promotion_points_in_transaction(
        &mut connection,
        current.character_id,
    )
    .await
    .map_err(debug)?;
    assert_eq!(points, 1);
    drop(connection);
    println!(
        "<TASK_BOARD> POSTGRES_CASE_COMPLETED: purchased_promotion_is_available_to_wheel_owner"
    );
    wheel_case::wheel_case(owner, harness, admitted).await?;
    fixture_track(harness, task_board::POINTS, i64::try_from(original)?).await?;
    Ok(())
}

async fn unavailable_catalogue_case(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    harness: &Harness,
    admitted: &AdmittedSession,
) -> TestResult {
    use crate::durability::quest_state::QuestCause;
    use crate::durability::task_board::{TaskBoardCommand, TaskBoardRefusal};
    use crate::foundation::{CommandId, CommandRef};
    use oteryn_gameplay_quests::quest::task_board::full::TaskBoardAction;
    let mut catalogue = owner.task_board_catalogue().ok_or("task catalogue")?;
    catalogue.cosmetics = None;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence read")?
        .ok_or("current fence")?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let request = TaskBoardCommand {
        cause: QuestCause::Command(CommandRef::new(
            admitted.game_session_id,
            CommandId::new(1_200_000).map_err(debug)?,
        )),
        action: TaskBoardAction::BuyShop { offer: 3 },
        payload_binding: full_request(16, 3, ""),
        fee_change: None,
    };
    let refused = slot
        .commit_task_board(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            request,
            std::sync::Arc::new(catalogue),
            None,
        )
        .await
        .map_err(debug)?;
    assert_eq!(
        refused.refusal,
        Some(TaskBoardRefusal::UnsupportedEntitlement)
    );
    assert_eq!(refused.view.balances.points, 720_000);
    assert_eq!(receipt_count(harness).await?, 3);
    println!(
        "<TASK_BOARD> POSTGRES_CASE_COMPLETED: unavailable_cosmetic_catalogue_refuses_without_charge"
    );
    Ok(())
}
