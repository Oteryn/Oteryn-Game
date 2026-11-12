//! The optional quote follows the actual fenced item reader and never mutates inventory.
use super::TestResult;
use super::fixture::{Harness, debug};
use crate::gameplay_transport::item_view::{
    ItemViewContinuity, SessionItemView, inventory_from_backpack,
};
use crate::gameplay_transport::{ComposedFreshAdmission, connection::AdmittedSession};
use oteryn_protocol_oteryn::{ClientCommandView, imbuement_details as wire};
#[test]
fn imbuement_details_postgres_reads_canonical_materials_and_fails_closed_on_item_session_fences()
-> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| super::imbuement::details_scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "imbuement details test panicked")??;
    Ok(())
}
pub(super) async fn exercise(
    harness: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
) -> TestResult {
    use crate::gameplay_transport::imbuement::ImbuementReply;
    let before: (i64, i64) = sqlx::query_as("SELECT count(*)::bigint,COALESCE(sum(quantity),0)::bigint FROM game_item_instances WHERE lifecycle=1").fetch_one(&harness.pool).await?;
    let fence = admitted.item_fence.ok_or("fence")?;
    let native = owner
        .active_generation
        .and_then(|active| active.native_gameplay())
        .ok_or("native")?;
    let index = native.item_index().ok_or("index")?;
    let backpack = owner
        .root
        .read_character_backpack(owner.character, fence.character_id)
        .await?;
    let inventory = inventory_from_backpack(backpack.as_ref(), |definition| {
        index.definition_ref(
            &definition.family,
            &definition.production_key,
            &definition.revision_ref,
        )
    })
    .ok_or("inventory projection")?;
    let sword = inventory
        .entries
        .iter()
        .find(|entry| {
            Some(entry.item_definition_ref)
                == index.definition_ref("Item", "oteryn:item.tibia.i3264", "definition-r1")
        })
        .ok_or("sword")?
        .key;
    let mut views = SessionItemView::resume(ItemViewContinuity::default());
    views.snapshot(inventory, None).map_err(debug)?;
    let item = views.table().handle(&sword).ok_or("sword handle")?;
    let payload = wire::encode_query(item);
    let command = ClientCommandView {
        command_id: 1,
        command_type: wire::COMMAND_TYPE_IMBUEMENT_DETAILS,
        payload: &payload,
    };
    let ImbuementReply::Payload(bytes) = crate::gameplay_transport::imbuement_details::query(
        owner,
        &admitted,
        command,
        Some(&views),
    )
    .await
    else {
        return Err("missing material projection".into());
    };
    let reply = wire::decode_reply(&bytes).map_err(debug)?;
    assert_eq!(reply.item, item);
    let quote = reply
        .quotes
        .iter()
        .find(|quote| (quote.family, quote.tier) == (1, 1))
        .ok_or("basic strike quote")?;
    assert!(quote.recipe_available);
    assert_eq!(quote.gold, 7500);
    assert_eq!(quote.success_percent, 100);
    assert_eq!(
        quote.materials,
        vec![wire::Material {
            definition_key: "oteryn:item.tibia.i11444".into(),
            quantity: 20
        }]
    );
    let mut stale = admitted;
    stale.continuity.connection_generation += 1;
    assert!(matches!(
        crate::gameplay_transport::imbuement_details::query(owner, &stale, command, Some(&views))
            .await,
        ImbuementReply::Rejected
    ));
    let missing = wire::encode_query(
        oteryn_protocol_oteryn::item_view::ItemHandle::new(u64::MAX)
            .ok_or("invalid handle fixture")?,
    );
    let wrong_item = ClientCommandView {
        command_id: 2,
        command_type: wire::COMMAND_TYPE_IMBUEMENT_DETAILS,
        payload: &missing,
    };
    assert!(matches!(
        crate::gameplay_transport::imbuement_details::query(
            owner,
            &admitted,
            wrong_item,
            Some(&views)
        )
        .await,
        ImbuementReply::Rejected
    ));
    let after: (i64, i64) = sqlx::query_as("SELECT count(*)::bigint,COALESCE(sum(quantity),0)::bigint FROM game_item_instances WHERE lifecycle=1").fetch_one(&harness.pool).await?;
    assert_eq!(after, before);
    Ok(())
}
