//! Real USE commits with an explicitly synthetic admitted cosmetic catalogue.
use super::*;
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::connection::AdmittedSession;
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::num::NonZeroU32;

#[derive(Clone, Copy, Debug)]
pub(super) enum Case {
    Parts,
    OmittedGrant,
    Outfit,
    Afflicted,
    ConcurrentGrant,
    ProductionOutfit,
    ProductionParts,
    ProductionAddonDoll,
    ProductionMount,
    RandomOutfit,
    Glider { missing: bool },
}

#[path = "race.rs"]
mod race;

#[test]
fn item_cosmetics_competing_source_grant_prevents_fourth_item_spend() -> TestResult {
    run_all(false, false, Some(Case::ConcurrentGrant))
}

#[test]
fn item_cosmetics_parts_replay_duplicate_and_appearance_consumer() -> TestResult {
    run_all(false, false, Some(Case::Parts))
}
#[test]
fn item_cosmetics_omitted_appearance_rolls_back_contribution_and_source() -> TestResult {
    run_all(false, false, Some(Case::OmittedGrant))
}
#[test]
fn item_cosmetics_outfit_addons_prerequisite_duplicate_and_full_achievement() -> TestResult {
    run_all(false, false, Some(Case::Outfit))
}
#[test]
fn item_cosmetics_ground_afflicted_commits_six_nested_pieces() -> TestResult {
    run_all(true, true, Some(Case::Afflicted))
}

async fn appearance(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult<crate::gameplay_transport::appearance_change::observation::Observation> {
    let actor = admitted.runtime_actor.ok_or("actor")?;
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
        .ok_or_else(|| "appearance observation".into())
}

async fn scalar(h: &Harness, sql: &'static str) -> TestResult<i64> {
    Ok(sqlx::query_scalar(sql).fetch_one(&h.pool).await?)
}

async fn item_quantity(h: &Harness, item: u8) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(id(item).as_slice())
    .fetch_one(&h.pool)
    .await?)
}

async fn carried_snapshot(
    h: &Harness,
    item: [u8; 16],
) -> TestResult<(serde_json::Value, serde_json::Value)> {
    Ok(sqlx::query_as(
        "SELECT to_jsonb(i),to_jsonb(e) FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id,world_id) WHERE i.item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(item.as_slice())
    .fetch_one(&h.pool)
    .await?)
}

async fn fiery_remaining(h: &Harness) -> TestResult<i64> {
    scalar(h, "SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i36938'").await
}

async fn grant_count(h: &Harness) -> TestResult<i64> {
    scalar(h, "SELECT count(*) FROM game_account_appearance_unlock_receipts WHERE source_kind='item_action'").await
}

async fn receipt_count(h: &Harness) -> TestResult<i64> {
    scalar(
        h,
        "SELECT count(*) FROM game_item_consume_transform_records",
    )
    .await
}

pub(super) async fn execute(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    mut admitted: AdmittedSession,
    case: Case,
    reference: impl Fn(&str) -> Option<NonZeroU32>,
) -> TestResult {
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(std::num::NonZeroU64::new(1).ok_or("handle")?),
        use_with: None,
        use_with_map_item: None,
        use_with_position: None,
    })
    .map_err(debug)?;
    match case {
        Case::Glider { missing } => {
            return super::glider::execute(owner, h, admitted, reference, missing).await;
        }
        Case::RandomOutfit | Case::ProductionAddonDoll => {
            return super::stock::random_outfit(owner, h, admitted, reference, None).await;
        }
        Case::Parts | Case::OmittedGrant | Case::ConcurrentGrant | Case::ProductionParts => {
            let source = reference("oteryn:item.tibia.i36938").ok_or("source")?;
            assert!(
                !appearance(owner, &admitted)
                    .await?
                    .catalogue
                    .mounts
                    .iter()
                    .any(|k| k
                        == if matches!(case, Case::ProductionParts) {
                            "oteryn:content.mount.singeing_steed"
                        } else {
                            packet::MOUNT
                        })
            );
            let mut last_payload = Vec::new();
            for command in 1..=4 {
                if command == 4 && matches!(case, Case::ConcurrentGrant) {
                    return race::execute(owner, h, admitted, &payload, source).await;
                }
                if command == 4 && matches!(case, Case::OmittedGrant) {
                    return omission(owner, h, admitted, &payload, source).await;
                }
                let (end, result, actual) = super::super::rust_lottery::exchange(
                    owner,
                    admitted,
                    command,
                    &payload,
                    Some(source),
                    None,
                )
                .await?;
                admitted = end;
                last_payload = actual;
                assert_eq!(result, UseDisposition::Committed);
                assert_eq!(fiery_remaining(h).await?, 5 - i64::try_from(command)?);
                assert_eq!(scalar(h, "SELECT contributions FROM game_character_item_cosmetic_progress WHERE component_key='fiery-horseshoe'").await?, i64::try_from(command)?);
                assert_eq!(grant_count(h).await?, i64::from(command == 4));
                assert_eq!(receipt_count(h).await?, i64::try_from(command)?);
            }
            let (end, result, _) =
                super::super::rust_lottery::exchange(owner, admitted, 4, &last_payload, None, None)
                    .await?;
            admitted = end;
            assert_eq!(result, UseDisposition::Committed);
            assert_eq!(receipt_count(h).await?, 4);
            assert_eq!(fiery_remaining(h).await?, 1);
            assert_eq!(grant_count(h).await?, 1);
            assert!(
                appearance(owner, &admitted)
                    .await?
                    .catalogue
                    .mounts
                    .iter()
                    .any(|k| k
                        == if matches!(case, Case::ProductionParts) {
                            "oteryn:content.mount.singeing_steed"
                        } else {
                            packet::MOUNT
                        })
            );
            let (end, result, _) = super::super::rust_lottery::exchange(
                owner,
                admitted,
                5,
                &payload,
                Some(source),
                None,
            )
            .await?;
            admitted = end;
            assert_eq!(result, UseDisposition::Committed);
            assert_eq!(
                fiery_remaining(h).await?,
                1,
                "already-owned acquisition never spends"
            );
            assert_eq!(scalar(h, "SELECT contributions FROM game_character_item_cosmetic_progress WHERE component_key='fiery-horseshoe'").await?, 4);
            assert_eq!(grant_count(h).await?, 1);
            assert_eq!(
                receipt_count(h).await?,
                5,
                "duplicate choice retains its immutable no-spend outcome"
            );
            assert!(
                appearance(owner, &admitted)
                    .await?
                    .catalogue
                    .mounts
                    .iter()
                    .any(|k| k
                        == if matches!(case, Case::ProductionParts) {
                            "oteryn:content.mount.singeing_steed"
                        } else {
                            packet::MOUNT
                        })
            );
        }
        Case::Outfit | Case::ProductionOutfit => {
            // Exercise the existing chat owner's local feedback without adding an
            // unrelated CHAT wire capability to this item transition fixture.
            owner.states.chat.admit(
                admitted.game_session_id,
                "Cosmetic fixture",
                owner.owner_now(),
            );
            let mut duplicate_payload = Vec::new();
            let mut duplicate_source = None;
            let mut duplicate_receipt = None;
            for (command, item, expected, grants) in [
                (1, 16253, UseDisposition::RequirementNotMet, 0),
                (2, 16252, UseDisposition::Committed, 1),
                (3, 16253, UseDisposition::Committed, 2),
                (4, 16254, UseDisposition::Committed, 3),
                (5, 16252, UseDisposition::Committed, 3),
            ] {
                let source = reference(&format!("oteryn:item.tibia.i{item}")).ok_or("source")?;
                if command == 5 {
                    // The publisher orders carried entries by descending placement ordinal.
                    // Capture the remaining published identity rather than assuming which
                    // of the two original base pieces the first USE consumed.
                    let inventory = owner
                        .observe_character_inventory(
                            admitted.runtime_actor.ok_or("actor")?,
                            admitted.game_session_id,
                        )
                        .await
                        .ok_or("duplicate source inventory")?;
                    let mut candidates = inventory
                        .entries
                        .iter()
                        .filter(|entry| entry.item_definition_ref == source);
                    let entry = candidates.next().ok_or("published duplicate source")?;
                    assert!(
                        candidates.next().is_none(),
                        "one base piece remains published"
                    );
                    let crate::gameplay_transport::item_view::ItemKey::Instance(item_id) =
                        entry.key
                    else {
                        return Err("duplicate source is not an instance".into());
                    };
                    assert_eq!(entry.count.get(), 1);
                    let before = carried_snapshot(h, item_id).await?;
                    assert_eq!(before.0["quantity"], 1);
                    assert_eq!(before.0["lifecycle"], 1);
                    duplicate_source = Some((item_id, before));
                }
                let (end, result, actual) = super::super::rust_lottery::exchange(
                    owner,
                    admitted,
                    command,
                    &payload,
                    Some(source),
                    None,
                )
                .await?;
                admitted = end;
                assert_eq!(result, expected);
                assert_eq!(grant_count(h).await?, grants);
                let feedback = owner.states.chat.drain(admitted.game_session_id);
                if command == 5 {
                    assert_eq!(
                        feedback,
                        vec![oteryn_protocol_oteryn::chat::ChatLine::Private {
                            speaker_name: "Game".into(),
                            text: "You already own this outfit part.".into(),
                        }],
                        "the no-spend outcome delivers the exact donor feedback once"
                    );
                    let (source_id, before) = duplicate_source
                        .as_ref()
                        .ok_or("captured duplicate source")?;
                    let (receipt_source, intent, consume_source, source_count, receipt): (
                        Vec<u8>,
                        Vec<u8>,
                        bool,
                        i64,
                        serde_json::Value,
                    ) = sqlx::query_as(
                        "SELECT uuid_send(source_item_instance_id),intent,consume_source,source_count,to_jsonb(r) FROM game_item_consume_transform_records r WHERE game_session_id=encode($1,'hex')::uuid AND command_id=5",
                    )
                    .bind(admitted.game_session_id.as_bytes().as_slice())
                    .fetch_one(&h.pool)
                    .await?;
                    assert_eq!(receipt_source.as_slice(), source_id.as_slice());
                    assert_eq!(intent, actual, "receipt binds the actual framed USE");
                    assert!(!consume_source);
                    assert_eq!(source_count, 0);
                    assert_eq!(
                        carried_snapshot(h, *source_id).await?,
                        *before,
                        "duplicate USE preserves the exact published item and custody"
                    );
                    duplicate_receipt = Some(receipt);
                    duplicate_payload = actual;
                } else {
                    assert!(feedback.is_empty());
                }
                assert_eq!(scalar(h, "SELECT count(*) FROM game_account_achievements WHERE achievement_key='oteryn:achievement/funghitastic'").await?,
                    i64::from(command >= 4), "only both addons complete the source achievement");
            }
            let (end, result, _) = super::super::rust_lottery::exchange(
                owner,
                admitted,
                5,
                &duplicate_payload,
                None,
                None,
            )
            .await?;
            admitted = end;
            assert_eq!(result, UseDisposition::Committed);
            assert!(
                owner.states.chat.drain(admitted.game_session_id).is_empty(),
                "a historical duplicate receipt does not deliver feedback again"
            );
            let (source_id, before) = duplicate_source.ok_or("captured duplicate source")?;
            assert_eq!(
                carried_snapshot(h, source_id).await?,
                before,
                "historical replay preserves the same item and custody"
            );
            let replayed_receipt: serde_json::Value = sqlx::query_scalar(
                "SELECT to_jsonb(r) FROM game_item_consume_transform_records r WHERE game_session_id=encode($1,'hex')::uuid AND command_id=5",
            )
            .bind(admitted.game_session_id.as_bytes().as_slice())
            .fetch_one(&h.pool)
            .await?;
            assert_eq!(
                replayed_receipt,
                duplicate_receipt.ok_or("captured duplicate receipt")?,
                "historical replay preserves the exact source-bound receipt"
            );
            assert_eq!(receipt_count(h).await?, 4);
            let seen = appearance(owner, &admitted).await?;
            let outfit = seen
                .catalogue
                .outfits
                .iter()
                .find(|o| {
                    o.key
                        == if matches!(case, Case::ProductionOutfit) {
                            "oteryn:content.outfit.soil_guardian_outfits"
                        } else {
                            packet::FUNGUS
                        }
                })
                .ok_or("earned outfit")?;
            assert_eq!(outfit.permitted_addons, 3);
        }
        Case::ProductionMount => {
            let (end, result, actual) = super::super::rust_lottery::exchange(
                owner,
                admitted,
                1,
                &payload,
                reference("oteryn:item.tibia.i23538"),
                None,
            )
            .await?;
            assert_eq!(result, UseDisposition::Committed);
            assert_eq!(item_quantity(h, 71).await?, 0);
            assert!(
                appearance(owner, &end)
                    .await?
                    .catalogue
                    .mounts
                    .iter()
                    .any(|key| key == "oteryn:content.mount.sparkion_mount")
            );
            let (_, replayed, _) =
                super::super::rust_lottery::exchange(owner, end, 1, &actual, None, None).await?;
            assert_eq!(replayed, UseDisposition::Committed);
            assert_eq!(grant_count(h).await?, 1);
            assert_eq!(receipt_count(h).await?, 1);
        }
        Case::Afflicted => {
            let before: serde_json::Value = sqlx::query_scalar("SELECT to_jsonb(g) FROM game_item_ground_locations g WHERE item_instance_id=encode($1,'hex')::uuid")
                .bind(id(71).as_slice()).fetch_one(&h.pool).await?;
            let (end, result, actual) = super::super::rust_lottery::exchange(
                owner,
                admitted,
                1,
                &payload,
                reference("oteryn:item.tibia.i12551"),
                None,
            )
            .await?;
            admitted = end;
            assert_eq!(result, UseDisposition::Committed);
            assert_eq!(
                item_quantity(h, 71).await?,
                1,
                "Ground piece is the trigger, not a carried ingredient"
            );
            assert_eq!(sqlx::query_scalar::<_, serde_json::Value>("SELECT to_jsonb(g) FROM game_item_ground_locations g WHERE item_instance_id=encode($1,'hex')::uuid")
                .bind(id(71).as_slice()).fetch_one(&h.pool).await?, before);
            assert_eq!(
                scalar(h, "SELECT count(*) FROM game_item_nested_entries").await?,
                0
            );
            assert_eq!(
                scalar(
                    h,
                    "SELECT count(*) FROM game_item_instances WHERE lifecycle=2"
                )
                .await?,
                6
            );
            assert_eq!(grant_count(h).await?, 1);
            assert_eq!(receipt_count(h).await?, 1);
            assert!(
                appearance(owner, &admitted)
                    .await?
                    .catalogue
                    .outfits
                    .iter()
                    .any(|o| o.key == packet::AFFLICTED)
            );
            let (_, result, _) =
                super::super::rust_lottery::exchange(owner, admitted, 1, &actual, None, None)
                    .await?;
            assert_eq!(result, UseDisposition::Committed);
            assert_eq!(grant_count(h).await?, 1);
            assert_eq!(receipt_count(h).await?, 1);
        }
    }
    eprintln!("W19 POSTGRES_CASE_COMPLETED item_cosmetics positive={case:?}");
    Ok(())
}

async fn omission(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    admitted: AdmittedSession,
    payload: &[u8],
    source: NonZeroU32,
) -> TestResult {
    sqlx::raw_sql("CREATE SEQUENCE w19d_test_grant_hits; CREATE FUNCTION w19d_test_omit_grant() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN PERFORM nextval('w19d_test_grant_hits'); RETURN NULL; END $$; CREATE TRIGGER w19d_test_omit_receipt BEFORE INSERT ON game_account_appearance_unlock_receipts FOR EACH ROW WHEN(NEW.source_kind='item_action') EXECUTE FUNCTION w19d_test_omit_grant(); CREATE TRIGGER w19d_test_omit_fact BEFORE INSERT ON game_account_appearance_unlocks FOR EACH ROW WHEN(NEW.unlock_key='oteryn:item-grant.mount.184') EXECUTE FUNCTION w19d_test_omit_grant();")
        .execute(&h.pool).await?;
    let result =
        super::super::rust_lottery::exchange(owner, admitted, 4, payload, Some(source), None).await;
    sqlx::raw_sql("DROP TRIGGER w19d_test_omit_receipt ON game_account_appearance_unlock_receipts; DROP TRIGGER w19d_test_omit_fact ON game_account_appearance_unlocks; DROP FUNCTION w19d_test_omit_grant();")
        .execute(&h.pool).await?;
    let hits = scalar(h, "SELECT last_value FROM w19d_test_grant_hits").await?;
    assert!(
        hits >= 2,
        "both appearance participants were deliberately omitted"
    );
    assert!(
        result.is_err(),
        "the missing grant cannot acknowledge a committed fourth contribution"
    );
    assert_eq!(fiery_remaining(h).await?, 2);
    assert_eq!(scalar(h, "SELECT contributions FROM game_character_item_cosmetic_progress WHERE component_key='fiery-horseshoe'").await?, 3);
    assert_eq!(grant_count(h).await?, 0);
    assert_eq!(receipt_count(h).await?, 3);
    eprintln!(
        "W19 POSTGRES_CASE_COMPLETED item_cosmetics grant_omission_atomic_rollback hits={hits}"
    );
    Ok(())
}
