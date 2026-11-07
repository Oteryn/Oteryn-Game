//! Actual shipped B3 MapBase source, then the existing native House owner.
use super::*;
use crate::content::{ReferenceItemField as F, ReferenceItemType};
use crate::gameplay_transport::{
    item_move::{ItemMoveStep, item_move},
    item_view::{ItemViewContinuity, SessionItemView},
    world_map::{MapUpdate, SessionMapView},
};
use oteryn_protocol_oteryn::item_view::{
    ItemMoveDestination, ItemMoveIntent, ItemMoveOutcome, ItemMoveSelection,
    encode_item_move_intent_for,
};

// Actual open interior next to the source House entrance. The shared
// synthetic POSITION is a blocking white-stone wall in the real B3.
pub(super) const PLACE: [i32; 3] = [32310, 32803, 7];

pub(super) fn world(
    world: crate::foundation::WorldId,
    channel: crate::foundation::ChannelId,
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    let bytes = std::fs::read(
        std::env::var_os("OTERYN_SRV7_EASEL_BUNDLE").ok_or("actual source easel bundle")?,
    )?;
    let b = oteryn_world_bundle::bundle::read(&bytes).map_err(debug)?;
    let pins = crate::map::BundlePins {
        digest: b.digest,
        project_format_version: b.manifest.identity.project_format_version.clone(),
        world_schema_version: b.manifest.identity.world_schema_version.clone(),
        content_revision: b.manifest.identity.content_revision.clone(),
        production: false,
    };
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    let index = native.item_index().ok_or("native Item index")?;
    crate::map::boot::boot(&bytes,&crate::map::boot::BootPins{bundle:pins,map_revision:crate::map::overlay::map_revision(&base),start:crate::map::overlay::TilePos{x:33572,y:31866,floor:-12}},world,channel,|key|{
  let rev=index.revision(key)?;let policy=native.item_policy(key,rev).map(|policy|policy.record());
  // Every key retains the real generation reference. Unqualified background
  // Item semantics remain conservative; the acquired easel is strictly qualified.
  if matches!(key,"oteryn:item.tibia.i14687"|"oteryn:item.tibia.i14688")&&policy.is_none(){return None;}

  Some(crate::map::facts::ItemDefinition{reference:index.definition_ref("Item",key,rev)?,solid:policy.and_then(|p|p.attributes.blocks_movement),
   blocks_projectile:policy.and_then(|p|p.attributes.blocks_projectile).unwrap_or(true),pickupable:policy.is_some_and(|p|matches!(&p.semantics.physical,F::Known(p) if p.pickupable==F::Known(true))),
   door:policy.is_some_and(|p|matches!(&p.semantics.classification,F::Known(c) if c.item_type==F::Known(ReferenceItemType::Door)))})
 }).map_err(debug)
}

pub(super) async fn acquire(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    reference: std::num::NonZeroU32,
) -> TestResult<[u8; 16]> {
    let actor = admitted.runtime_actor.ok_or("source actor")?;
    let mut items = SessionItemView::resume(ItemViewContinuity::default()).with_map_view();
    items
        .snapshot(
            owner
                .observe_character_inventory(actor, admitted.game_session_id)
                .await
                .ok_or("source inventory")?,
            None,
        )
        .map_err(debug)?;
    let Some(MapUpdate::Snapshot(snapshot)) = owner
        .observe_world_map(actor, &mut SessionMapView::default(), &mut items)
        .await
        .ok_or("actual source map")?
        .map_err(debug)?
    else {
        return Err("actual source snapshot".into());
    };
    let snapshot = oteryn_protocol_oteryn::world_map::decode_world_map_snapshot(&snapshot.payload)
        .map_err(debug)?;
    let handle = snapshot
        .tiles
        .iter()
        .find(|t| t.position.x == 33572 && t.position.y == 31867 && t.position.floor == -12)
        .and_then(|t| {
            t.items.iter().find_map(|i| match (i.definition, i.origin) {
                (MapDefinition::Item(r), MapOrigin::Handle(h)) if r == reference => Some(h),
                _ => None,
            })
        })
        .ok_or("actual globally placed easel handle")?;
    let selection = ItemMoveSelection {
        equip_drop: false,
        container_tree: false,
        world_interaction: true,
    };
    let payload = encode_item_move_intent_for(
        &ItemMoveIntent {
            source: handle,
            destination: ItemMoveDestination::MainBackpack,
        },
        selection,
    )
    .map_err(debug)?;
    let command = crate::gameplay_transport::connection::UseCommand {
        game_session_id: admitted.game_session_id,
        command_id: 1,
        item_fence: admitted.item_fence,
    };
    assert!(
        matches!(
            item_move(
                owner, actor, command, &payload, selection, false, &mut items
            )
            .await,
            ItemMoveStep::Result(ItemMoveOutcome::Moved, _)
        ),
        "actual command9 must materialize shipped source14687"
    );
    let bytes:Vec<u8>=sqlx::query_scalar("SELECT uuid_send(item_instance_id) FROM game_map_item_mint_receipts WHERE definition_production_key='oteryn:item.tibia.i14687'").fetch_one(&h.pool).await.map_err(|error|format!("real easel pickup mint receipt: {error}"))?;
    let id: [u8; 16] = bytes.try_into().map_err(|_| "actual acquired UUID")?;
    assert!(sqlx::query_scalar::<_,bool>("SELECT EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid)").bind(id.as_slice()).fetch_one(&h.pool).await?);
    admitted.continuity.next_command_id = 2;
    Ok(id)
}

pub(super) async fn exercise(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    source: [u8; 16],
    before_ref: std::num::NonZeroU32,
    after_ref: std::num::NonZeroU32,
) -> TestResult {
    let before:serde_json::Value=sqlx::query_scalar("SELECT to_jsonb(i) FROM game_item_instances i WHERE item_instance_id=encode($1,'hex')::uuid").bind(source.as_slice()).fetch_one(&h.pool).await?;
    // Placement uses the genuine native command9 owner after actual MapBase pickup.
    let capture = std::sync::Arc::new(std::sync::Mutex::new(super::wire::HouseWireCapture {
        move_destination: Some(ItemMoveDestination::Ground(
            oteryn_protocol_oteryn::item_view::WorldTilePosition {
                x: PLACE[0],
                y: PLACE[1],
                floor: -i16::try_from(PLACE[2])?,
            },
        )),
        ..Default::default()
    }));
    let (admitted, placed, _, _) = exchange(
        owner,
        admitted,
        1,
        &[],
        HouseExchange {
            rotate: false,
            current: Some(before_ref),
            after: None,
            nested: None,
            foreign_after_snapshot: None,
            capture: Some(std::sync::Arc::clone(&capture)),
        },
    )
    .await?;
    assert_eq!(placed, UseDisposition::Committed);
    assert_eq!(
        capture
            .lock()
            .map_err(|_| "easel move capture")?
            .move_outcome,
        Some(ItemMoveOutcome::Moved)
    );
    assert!(sqlx::query_scalar::<_,bool>("SELECT EXISTS(SELECT 1 FROM game_item_house_interior_locations WHERE item_instance_id=encode($1,'hex')::uuid)").bind(source.as_slice()).fetch_one(&h.pool).await?);
    let body = oteryn_protocol_oteryn::world_rotate::encode_rotate_intent(
        &UseTarget::Item(std::num::NonZeroU64::MIN),
        Some(before_ref),
    )
    .map_err(debug)?;
    let (_, outcome, _, _) = exchange(
        owner,
        admitted,
        2,
        &body,
        HouseExchange {
            rotate: true,
            current: Some(before_ref),
            after: Some(after_ref),
            nested: None,
            foreign_after_snapshot: None,
            capture: None,
        },
    )
    .await?;
    assert_eq!(
        outcome,
        UseDisposition::Committed,
        "real acquired bare easel must rotate within its own current Wiki identity"
    );
    let after:serde_json::Value=sqlx::query_scalar("SELECT to_jsonb(i) FROM game_item_instances i WHERE item_instance_id=encode($1,'hex')::uuid").bind(source.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        after["definition_production_key"],
        "oteryn:item.tibia.i14688"
    );
    assert_eq!(
        after["minted_transaction_id"],
        before["minted_transaction_id"]
    );
    assert!(sqlx::query_scalar::<_,bool>("SELECT EXISTS(SELECT 1 FROM game_map_item_mint_receipts WHERE item_instance_id=encode($1,'hex')::uuid AND transaction_id=$2::uuid)").bind(source.as_slice()).bind(after["minted_transaction_id"].as_str().ok_or("mint lineage")?).fetch_one(&h.pool).await?);
    Ok(())
}
