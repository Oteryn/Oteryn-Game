//! Real USE commits must refresh the views in the command's dispatch turn.

use super::*;
use oteryn_protocol_oteryn::container_tree::{
    STATE_DOMAIN_CONTAINER_VIEWS, decode_container_views,
};
use oteryn_protocol_oteryn::world_object::{
    UseIntent, UseTarget, encode_item_use_intent, encode_use_item_intent,
};

#[derive(Clone, Copy)]
pub(super) enum Case {
    LastCharge,
    PotionFlask,
    ClosedBag,
    WithoutContainerCapability,
}

#[test]
fn item_use_last_charge_removes_the_entry_from_the_open_bag_in_the_same_turn() -> TestResult {
    run_scenario(Some(Case::LastCharge))
}

#[test]
fn item_use_potion_to_flask_updates_both_open_bags_in_the_same_turn() -> TestResult {
    run_scenario(Some(Case::PotionFlask))
}

#[test]
fn item_use_in_a_closed_bag_emits_no_update_for_unchanged_open_views() -> TestResult {
    run_scenario(Some(Case::ClosedBag))
}

#[test]
fn item_use_without_container_capability_emits_no_container_delta() -> TestResult {
    run_scenario(Some(Case::WithoutContainerCapability))
}

pub(super) async fn run(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    harness: &Harness,
    admitted: AdmittedSession,
    case: Case,
) -> TestResult {
    let source = match case {
        Case::LastCharge | Case::ClosedBag | Case::WithoutContainerCapability => 100,
        Case::PotionFlask => 110,
    };
    let hidden = matches!(case, Case::ClosedBag | Case::WithoutContainerCapability);
    if !hidden {
        let mut tx = harness.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role = replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE game_item_instances SET quantity = 1 WHERE item_instance_id = encode($1,'hex')::uuid")
            .bind(id(source).as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
    }
    let index = owner
        .active_generation
        .expect("generation")
        .native_gameplay()
        .expect("native")
        .item_index()
        .expect("index");
    let definition = |key| {
        index
            .definition_ref("Item", key, index.revision(key).expect("revision"))
            .expect("definition")
    };
    let use_handle =
        |handle| encode_use_item_intent(std::num::NonZeroU64::new(handle).expect("handle"));
    // Reuse the existing fixture: main=1, outer=2, inner=3, potion=4, food=5.
    let mut payloads = vec![use_handle(1), use_handle(2)];
    if matches!(case, Case::WithoutContainerCapability) {
        payloads.clear();
    }
    if hidden {
        payloads.push(
            encode_item_use_intent(&UseIntent {
                target: UseTarget::ItemByDefinition(definition("oteryn:item.tibia.i3577")),
                use_with: None,
                use_with_map_item: None,
                use_with_position: None,
            })
            .map_err(debug)?,
        );
    } else {
        payloads.push(use_handle(3));
        payloads.push(use_handle(if matches!(case, Case::PotionFlask) {
            4
        } else {
            5
        }));
    }
    let use_command = u64::try_from(payloads.len())?;
    let output = wire_dispatch::exchange(owner, admitted, &payloads).await?;
    let mut cursor = 0;
    let mut last_result = 0;
    let mut refreshes = Vec::new();
    let mut views_revision = None;
    while cursor < output.len() {
        let length = u32::from_be_bytes(output[cursor..cursor + 4].try_into()?) as usize;
        let frame =
            crate::foundation::decode_wire_envelope(&output[cursor + 4..cursor + 4 + length])?;
        match frame.message_type() {
            crate::foundation::MessageType::CommandResult => {
                let result = oteryn_protocol_oteryn::decode_command_result(frame.payload())?;
                assert_eq!(result.command_id, last_result + 1);
                assert_eq!(
                    result.payload,
                    oteryn_protocol_oteryn::world_object::encode_use_result(
                        UseDisposition::Committed
                    )
                    .map_err(debug)?
                );
                last_result = result.command_id;
            }
            crate::foundation::MessageType::StateDelta => {
                let delta = oteryn_protocol_oteryn::decode_state_delta(frame.payload())?;
                if delta.domain_id == STATE_DOMAIN_CONTAINER_VIEWS {
                    if let Some(previous) = views_revision {
                        assert_eq!(delta.base_revision, previous);
                    }
                    assert_eq!(delta.new_revision, delta.base_revision + 1);
                    views_revision = Some(delta.new_revision);
                    if last_result == use_command {
                        refreshes.push(decode_container_views(delta.payload).map_err(debug)?);
                    }
                }
            }
            _ => {}
        }
        cursor += 4 + length;
    }
    assert_eq!(last_result, use_command, "every command completed");
    assert_eq!(harness.count("game_item_use_records").await?, 1);
    let quantity: i64 = sqlx::query_scalar(
        "SELECT quantity FROM game_item_instances WHERE item_instance_id = encode($1,'hex')::uuid",
    )
    .bind(id(source).as_slice())
    .fetch_one(&harness.pool)
    .await?;
    if hidden {
        assert_eq!(quantity, 2);
        assert!(refreshes.is_empty(), "unchanged views emit no delta");
        if matches!(case, Case::WithoutContainerCapability) {
            assert!(views_revision.is_none(), "domain 14 requires capability 14");
        }
        return Ok(());
    }
    assert_eq!(quantity, 0);
    assert_eq!(
        refreshes.len(),
        1,
        "USE emits its domain 14 update before disconnect"
    );
    let views = &refreshes[0].views;
    assert_eq!(views.len(), 3);
    let inner = views
        .iter()
        .find(|v| v.container_handle.get() == 3)
        .expect("inner view");
    assert_eq!(inner.entries.len(), 1);
    let remaining = &inner.entries[0];
    let (key, count, handle) = match case {
        Case::LastCharge => ("oteryn:item.tibia.i266", 2, 4),
        Case::PotionFlask => ("oteryn:item.tibia.i3577", 3, 5),
        Case::ClosedBag | Case::WithoutContainerCapability => unreachable!(),
    };
    assert_eq!(remaining.item_definition_ref, definition(key));
    assert_eq!(remaining.count.get(), count);
    assert_eq!(
        remaining.handle.get(),
        handle,
        "unchanged item keeps its handle"
    );
    let main = views
        .iter()
        .find(|v| v.container_handle.get() == 1)
        .expect("main view");
    match case {
        Case::LastCharge => assert_eq!(main.entries.len(), 1),
        Case::PotionFlask => {
            assert_eq!(main.entries.len(), 2);
            let flask = main
                .entries
                .iter()
                .find(|e| e.item_definition_ref == definition("oteryn:item.tibia.i285"))
                .expect("the committed potion-to-flask output is visible");
            assert_eq!(flask.count.get(), 1);
            assert!(flask.handle.get() > 5, "the output gets a fresh handle");
        }
        Case::ClosedBag | Case::WithoutContainerCapability => unreachable!(),
    }
    Ok(())
}
