//! Actual fixed Farm torch. No source item is minted or assigned a fictitious UUID.
use super::*;
use crate::content::{ReferenceItemField as F, ReferenceItemType};
use crate::gameplay_transport::{
    item_view::{ItemViewContinuity, SessionItemView},
    world_map::{MapUpdate, SessionMapView},
};

pub(super) fn world(
    world: crate::foundation::WorldId,
    channel: crate::foundation::ChannelId,
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    let bytes = std::fs::read(
        std::env::var_os("OTERYN_SRV7_STATIC_HOUSE_BUNDLE")
            .ok_or("actual source fixed-House bundle")?,
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
    crate::map::boot::boot(&bytes,&crate::map::boot::BootPins{bundle:pins,map_revision:crate::map::overlay::map_revision(&base),start:crate::map::overlay::TilePos{x:32650,y:31890,floor:-2}},world,channel,|key|{
  let rev=index.revision(key)?;let policy=native.item_policy(key,rev).map(|policy|policy.record());
  // Every key retains the real generation reference. Unqualified background
  // Item semantics remain conservative; the exact fixed torch states are strictly qualified.
  if matches!(key,"oteryn:item.tibia.i2929"|"oteryn:item.tibia.i2928")&&policy.is_none(){return None;}

  Some(crate::map::facts::ItemDefinition{reference:index.definition_ref("Item",key,rev)?,solid:policy.and_then(|p|p.attributes.blocks_movement),
   blocks_projectile:policy.and_then(|p|p.attributes.blocks_projectile).unwrap_or(true),pickupable:policy.is_some_and(|p|matches!(&p.semantics.physical,F::Known(p) if p.pickupable==F::Known(true))),
   door:policy.is_some_and(|p|matches!(&p.semantics.classification,F::Known(c) if c.item_type==F::Known(ReferenceItemType::Door)))})
 }).map_err(debug)
}

pub(super) const HOUSE: &str = "oteryn:content.house.the_farms_5";
pub(super) const ENTRY: [i32; 3] = [32650, 31890, 2];
pub(super) const NEAR_TORCH: [i32; 3] = [32654, 31888, 2];
const TORCH: [i32; 3] = [32655, 31888, 2];
fn source_tiles() -> TestResult<std::collections::BTreeSet<[i32; 3]>> {
    for raw in [
        include_str!("../../../../../../../content/houses/houses-00000-00499.json"),
        include_str!("../../../../../../../content/houses/houses-00500-00994.json"),
    ] {
        let value: Value = serde_json::from_str(raw)?;
        for house in value["houses"].as_array().ok_or("source Houses")? {
            if house["identity"]["key"] == HOUSE {
                assert_eq!(house["provenance"]["engine_house"]["house_id"], 2844);
                return house["tiles"]
                    .as_array()
                    .ok_or("real Farm tiles")?
                    .iter()
                    .map(|tile| {
                        let a = tile.as_array().ok_or("source tile")?;
                        Ok([
                            i32::try_from(a[0].as_i64().ok_or("sourcex")?)?,
                            i32::try_from(a[1].as_i64().ok_or("sourcey")?)?,
                            i32::try_from(a[2].as_i64().ok_or("sourcez")?)?,
                        ])
                    })
                    .collect();
            }
        }
    }
    Err("source Farm identity absent".into())
}
async fn near_torch(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
) -> TestResult {
    use crate::gameplay_transport::world_spatial::{StepDirection, StepDisposition};
    let tiles = source_tiles()?;
    let mut historical = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *historical)
        .await?;
    for at in &tiles {
        let bytes: Vec<u8> = at.iter().flat_map(|part| part.to_be_bytes()).collect();
        sqlx::query("INSERT INTO game_house_tiles VALUES(encode($1,'hex')::uuid,$2,$3) ON CONFLICT DO NOTHING").bind(id(42).as_slice()).bind(HOUSE).bind(bytes).execute(&mut *historical).await?;
    }
    historical.commit().await?;
    let actor = admitted.runtime_actor.ok_or("actual Farm resident")?;
    let start = {
        let runtime = owner.runtime.lock().await;
        let state = runtime
            .house_actor_state(actor, admitted.game_session_id)
            .map_err(debug)?;
        [
            state.position.x,
            state.position.y,
            i32::from(state.position.floor),
        ]
    };
    assert!(
        tiles.contains(&start),
        "actual House handoff must land inside source Farm"
    );
    let mut pending = std::collections::VecDeque::from([start]);
    let mut previous = std::collections::BTreeMap::from([(start, None)]);
    let mut goal = None;
    while let Some(at) = pending.pop_front() {
        if at[2] == TORCH[2] && at[0].abs_diff(TORCH[0]) <= 1 && at[1].abs_diff(TORCH[1]) <= 1 {
            goal = Some(at);
            break;
        }
        for (dx, dy, direction) in [
            (0, -1, StepDirection::North),
            (1, 0, StepDirection::East),
            (0, 1, StepDirection::South),
            (-1, 0, StepDirection::West),
        ] {
            let next = [at[0] + dx, at[1] + dy, at[2]];
            if tiles.contains(&next)
                && owner.movement_cells.index().lookup(
                    owner.movement_cells.scope(),
                    crate::content::LogicalCell {
                        x: next[0],
                        y: next[1],
                        z: next[2],
                    },
                ) == Ok(crate::content::CollisionClass::Walkable)
                && let std::collections::btree_map::Entry::Vacant(entry) = previous.entry(next)
            {
                entry.insert(Some((at, direction)));
                pending.push_back(next);
            }
        }
    }
    let mut at = goal.ok_or("actual walkable Farm path to torch")?;
    let mut path = Vec::new();
    while let Some((from, direction)) = previous[&at] {
        path.push(direction);
        at = from;
    }
    for direction in path.into_iter().rev() {
        for retry in 0..8 {
            let (outcome, wait) = owner
                .house_paced_step(actor, admitted.game_session_id, direction)
                .await;
            if outcome.disposition == StepDisposition::Moved {
                break;
            }
            if let Some(wait) = wait {
                assert!(wait <= std::time::Duration::from_secs(2));
                tokio::time::sleep(wait + std::time::Duration::from_millis(2)).await;
            } else {
                return Err(format!("actual Farm STEP refused: {:?}", outcome.disposition).into());
            }
            assert!(retry < 7, "bounded actual House walking cadence");
        }
    }
    Ok(())
}
async fn observed(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    reference: std::num::NonZeroU32,
) -> TestResult<(
    oteryn_protocol_oteryn::world_object::WorldObjectTarget,
    Vec<u8>,
)> {
    let mut items = SessionItemView::resume(ItemViewContinuity::default()).with_map_view();
    let mut map = SessionMapView::default();
    let Some(MapUpdate::Snapshot(snapshot)) = owner
        .observe_house_item_map(admitted, &mut map, &mut items)
        .await
        .ok_or("current Farm projection")?
        .map_err(debug)?
    else {
        return Err("real Farm snapshot".into());
    };
    let decoded = oteryn_protocol_oteryn::world_map::decode_world_map_snapshot(&snapshot.payload)
        .map_err(debug)?;
    let source = decoded
        .tiles
        .iter()
        .find(|tile| {
            tile.position.x == TORCH[0]
                && tile.position.y == TORCH[1]
                && i32::from(tile.position.floor) == -TORCH[2]
        })
        .ok_or("actual source torch visible")?;
    let (ordinal, revision) = source
        .items
        .iter()
        .find_map(|item| {
            if item.definition == MapDefinition::Item(reference) {
                if let MapOrigin::BaseOrdinal {
                    ordinal,
                    object_revision,
                } = item.origin
                {
                    Some((ordinal, object_revision))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .ok_or("actual torch must stay BASE with no item UUID")?;
    let tile = crate::map::overlay::TilePos {
        x: u16::try_from(TORCH[0])?,
        y: u16::try_from(TORCH[1])?,
        floor: -i8::try_from(TORCH[2])?,
    };
    let mut placement = owner
        .bundle_map()
        .ok_or("real Farm bundle")?
        .overlay
        .base()
        .digest()
        .to_vec();
    placement.extend(
        crate::map::view::placement_key(tile, ordinal)
            .ok_or("actual torch base lineage")?
            .to_be_bytes(),
    );
    Ok((
        oteryn_protocol_oteryn::world_object::WorldObjectTarget {
            placement,
            expected_revision: revision,
        },
        snapshot.payload,
    ))
}
pub(super) async fn exercise(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    before: std::num::NonZeroU32,
    after: std::num::NonZeroU32,
) -> TestResult {
    near_torch(h, owner, admitted).await?;
    let (target, source_snapshot) = observed(owner, admitted, before).await?;
    let item_count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_instances")
        .fetch_one(&h.pool)
        .await?;
    let body = encode_item_use_intent(&UseIntent {
        target: UseTarget::WorldObject(target.clone()),
        use_with: None,
        use_with_map_item: None,
        use_with_position: None,
    })
    .map_err(debug)?;
    let capture = std::sync::Arc::new(std::sync::Mutex::new(
        super::wire::HouseWireCapture::default(),
    ));
    let (mut next, result, original, _) = exchange(
        owner,
        admitted,
        1,
        &body,
        HouseExchange {
            rotate: false,
            current: None,
            after: None,
            nested: None,
            foreign_after_snapshot: None,
            capture: Some(std::sync::Arc::clone(&capture)),
        },
    )
    .await?;
    assert_eq!(
        result,
        UseDisposition::Committed,
        "actual framed WorldObject USE of source House oven must reach its current named-House owner"
    );
    let (changed, after_snapshot) = observed(owner, next, after).await?;
    assert_eq!(changed.placement, target.placement);
    assert_eq!(changed.expected_revision, target.expected_revision + 1);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_instances")
            .fetch_one(&h.pool)
            .await?,
        item_count,
        "BASE USE does not mint a source or substitute a fabricated item UUID"
    );
    assert!(
        !capture
            .lock()
            .map_err(|_| "source Farm wire capture")?
            .map_deltas
            .is_empty(),
        "native House wire publishes BASE definition/light change"
    );
    owner.states.command_ingress.forget(next.game_session_id);
    next.continuity.next_command_id = 1;
    let (_, replayed, _, _) = exchange(
        owner,
        next,
        1,
        &original,
        HouseExchange {
            rotate: false,
            current: None,
            after: None,
            nested: None,
            foreign_after_snapshot: None,
            capture: None,
        },
    )
    .await?;
    assert_ne!(
        replayed,
        UseDisposition::Committed,
        "retained BASE CAS prevents a second toggle when command-ingress history was deliberately lost"
    );
    let (still_changed, _) = observed(owner, next, after).await?;
    assert_eq!(still_changed, changed);
    let actor = admitted.runtime_actor.ok_or("source viewer")?;
    let viewer = {
        let runtime = owner.runtime.lock().await;
        let actual = runtime
            .house_actor_state(actor, admitted.game_session_id)
            .map_err(debug)?;
        serde_json::json!({"house":actual.house,"generation":actual.generation.get(),"position":[actual.position.x,actual.position.y,actual.position.floor]})
    };
    let raw = capture.lock().map_err(|_| "real source native capture")?;
    println!(
        "SRV7_STATIC_HOUSE_CAPTURE {}",
        serde_json::json!({"house":HOUSE,"viewer":viewer,"source_item_id":2929,"source_house_id":2844,"source_position":TORCH,"source_snapshot_hex":hex(&source_snapshot),"after_snapshot_hex":hex(&after_snapshot),"world_map_snapshot_hex":hex(raw.maps.first().ok_or("actual wire source snapshot")?),"world_map_delta_hex":hex(raw.map_deltas.first().ok_or("actual source change delta")?)})
    );

    Ok(())
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
