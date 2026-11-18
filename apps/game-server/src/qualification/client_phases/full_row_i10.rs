//! I10 concrete player scenario, derived from the matrix/audit and the item-action owners:
//! 1. Admit the node-boot character and retain its complete native inventory view.
//! 2. Send HOUSE_FURNITURE_WRAP `Wrap` for carried/house furniture; expect an accepted reply
//!    with a committed revision and an inventory/house delta replacing it with its kit.
//! 3. Send `Unwrap` for that construction kit at a free house cell; expect an accepted reply,
//!    a later revision, and a delta moving the constructed furniture into house custody.
//! 4. Use an ordinarily held destroy weapon on the furniture; expect the USE reply and the
//!    furniture/remains delta (a failed deterministic roll must leave the furniture unchanged).
//! 5. Use an ordinarily held fluid container on a fluid source and then on the constructed
//!    furniture/map target; expect committed USE replies and subtype/field deltas for fill/pour.
//! 6. Repeat wrap with a stale revision; expect a `Stale` refusal and no inventory delta.
//! 7. Log out, freshly admit the character, and re-check the complete durable inventory view.
//!
//! House ownership and placed furnishings are prerequisites. Resolve their current
//! durable identities and exercise the mutations through the admitted player session.
use super::*;
use oteryn_dev_client::client_runtime::house_transfer as transfer;
use oteryn_dev_client::{CharacterInventory, DevClientSession, ItemHandle};
use oteryn_protocol_oteryn::world_map::{MapDefinition, MapOrigin, MapTile};
use oteryn_protocol_oteryn::{CommandStatus, house_wrap as wrap, world_object};
use std::num::NonZeroU32;

/// The refusal is the server's correct answer outside a house, so the row fails on the missing
/// precondition: House ownership is only settled by the Control-side auction settlement, which
/// no player command reaches during the run.
const GAP: &str = "step=wrap-observed=NotInHouse;needed=owned-house;settlement-only:crates/gameplay/durability/src/durability/house_ownership/settlement.rs";

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = verdict::indexed_timeout(Duration::from_secs(90), verdict::observe(journey(clients))).await;
    let (status, detail) = match result {
        Ok(Ok(())) => ("PASS", "lifecycle-and-fresh-admission-validated".to_owned()),
        Ok(Err(error)) => classify(&error.to_string()),
        Err(_) => ("FAIL", "step=journey observed=timeout".to_owned()),
    };
    evidence(&format!(
        "SEAM_EVIDENCE e2e_all scenario=I10.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn classify(error: &str) -> (&'static str, String) {
    if error.contains("NotInHouse") {
        ("FAIL", GAP.to_owned())
    } else {
        ("FAIL", compact(error))
    }
}

fn compact(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join("-")
}

async fn wrap_command(
    client: &mut DevClientSession,
    request: wrap::Request,
) -> TestResult<wrap::Reply> {
    let reply = client
        .session_mut()
        .gated_command(
            wrap::CAPABILITY_HOUSE_FURNITURE_WRAP_V1,
            wrap::COMMAND_WRAP,
            &request.encode(),
        )
        .await?;
    let disposition = wrap::Reply::decode(&reply.payload).ok_or_else(|| {
        format!(
            "step={:?} observed=invalid-payload:{:?}",
            request.action, reply.payload
        )
    })?;
    let expected = match disposition {
        wrap::Reply::Committed(_) => CommandStatus::Accepted,
        wrap::Reply::Refused(_) => CommandStatus::Rejected,
    };
    if reply.status != expected {
        return Err(format!(
            "step={:?} observed=status:{:?},reply:{disposition:?}",
            request.action, reply.status
        )
        .into());
    }
    Ok(disposition)
}

fn validate_use_reply(
    step: &str,
    status: CommandStatus,
    disposition: world_object::UseDisposition,
    observed: bool,
) -> TestResult {
    if status != CommandStatus::Accepted || disposition != world_object::UseDisposition::Committed {
        return Err(format!(
            "step={step} observed=status:{status:?},disposition:{disposition:?} needed=accepted-committed-with-state-delta"
        ).into());
    }
    if !observed {
        return Err(format!(
            "step={step} observed=status:{status:?},disposition:{disposition:?} needed=observable-furniture-or-fluid-state-delta"
        ).into());
    }
    Ok(())
}

async fn transfer_house(client: &mut DevClientSession, house: &str, exit: bool) -> TestResult {
    let intent = transfer::HouseTransferIntent {
        exit,
        house: house.to_owned(),
    };
    let payload = transfer::encode_house_transfer_intent(&intent)
        .map_err(|error| format!("house transfer encode: {error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(
            transfer::CAPABILITY_HOUSE_TRANSFER_V1,
            transfer::COMMAND_TYPE_HOUSE_TRANSFER,
            &payload,
        )
        .await?;
    let ack = transfer::decode_house_transfer_ack(&reply.payload)
        .map_err(|error| format!("house transfer reply {:?}: {error:?}", reply.status))?;
    if reply.status != CommandStatus::Accepted || ack.house.as_deref() != (!exit).then_some(house) {
        return Err(format!("house transfer did not commit the requested scope: {ack:?}").into());
    }
    Ok(())
}

async fn furniture_custody(
    db: &mut sqlx::PgConnection,
    item: [u8; 16],
    character: [u8; 16],
) -> TestResult<(String, String, Option<Vec<u8>>, bool)> {
    Ok(sqlx::query_as(
        "SELECT i.definition_production_key,i.state_revision::text,h.spatial_position,
                EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.item_instance_id=i.item_instance_id AND e.character_id=encode($2,'hex')::uuid)
           FROM game_item_instances i LEFT JOIN game_item_house_interior_locations h USING(item_instance_id,world_id)
          WHERE i.item_instance_id=encode($1,'hex')::uuid AND i.lifecycle=1",
    ).bind(item.as_slice()).bind(character.as_slice()).fetch_one(db).await?)
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    let reply = client.session_mut().logout().await?;
    if reply.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("step=logout observed={reply:?}").into());
    }
    Ok(())
}

fn inventory_state(client: &DevClientSession) -> TestResult<CharacterInventory> {
    let mut inventory = client
        .inventory()
        .cloned()
        .ok_or("missing I10 inventory view")?;
    for entry in inventory
        .entries
        .iter_mut()
        .chain(inventory.main_backpack.iter_mut())
        .chain(inventory.equipment.iter_mut().map(|entry| &mut entry.item))
    {
        entry.handle = ItemHandle::MIN;
    }
    Ok(inventory)
}

async fn destroy(
    client: &mut DevClientSession,
    db: &mut sqlx::PgConnection,
    house: &str,
    world: &[u8; 16],
    weapon: ItemHandle,
    index: &crate::content::item_ref::ItemDefinitionIndex,
) -> TestResult {
    let session = *client.join_snapshot().game_session_id.as_bytes();
    let targets: Vec<(Vec<u8>, String, Vec<u8>)> = sqlx::query_as(
        "SELECT uuid_send(i.item_instance_id),i.definition_production_key,h.spatial_position
           FROM game_item_house_interior_locations h JOIN game_item_instances i USING(item_instance_id,world_id)
          WHERE h.world_id=encode($1,'hex')::uuid AND h.house_key=$2 AND i.lifecycle=1
            AND i.definition_production_key IN ('oteryn:item.tibia.i2295','oteryn:item.tibia.i2296','oteryn:item.tibia.i4995')
          ORDER BY h.spatial_position,i.item_instance_id LIMIT 64",
    ).bind(world.as_slice()).bind(house).fetch_all(&mut *db).await?;
    let mut projected = BTreeMap::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let (instance, handle, original) = loop {
        if projected.is_empty()
            && let Some(map) = client.session_mut().world_map_snapshot()
        {
            for tile in map.tiles {
                projected.insert(
                    (tile.position.x, tile.position.y, tile.position.floor),
                    tile,
                );
            }
        }
        for event in client.take_events() {
            if let oteryn_dev_client::SessionEvent::WorldMap(delta) = event {
                for at in delta.value.cleared {
                    projected.remove(&(at.x, at.y, at.floor));
                }
                for tile in delta.value.tiles {
                    projected.insert(
                        (tile.position.x, tile.position.y, tile.position.floor),
                        tile,
                    );
                }
            }
        }
        let found = targets.iter().find_map(|(instance, key, cell)| {
            let [x, y, z] =
                crate::durability::consume_transform::HouseActionContext::decode_spatial_position(
                    cell,
                )?;
            let revision = index.revision(key)?;
            let definition = index.definition_ref("Item", key, revision)?;
            let at = (x, y, -i16::try_from(z).ok()?);
            let tile = projected.get(&at).filter(|tile: &&MapTile| !tile.more)?;
            let mut matching = tile
                .items
                .iter()
                .filter(|item| item.definition == MapDefinition::Item(definition));
            let (Some(item), None) = (matching.next(), matching.next()) else {
                return None;
            };
            let MapOrigin::Handle(handle) = item.origin else {
                return None;
            };
            Some((instance, handle, key))
        });
        if let Some(found) = found {
            break found;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("step=destroy observed=no-unambiguous-placed-furniture-handle".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    };
    let disposition =
        super::full_row_i11::use_ingredient(client, weapon, Some(handle), None).await?;
    let receipt: Option<(String, bool)> = sqlx::query_as(
        "SELECT outcome, CASE WHEN outcome='COMMITTED' THEN
                   remains_item IS NOT NULL AND EXISTS(SELECT 1 FROM game_item_house_interior_locations h
                       WHERE h.item_instance_id=(remains_item->>'item_instance_id')::uuid AND h.world_id=r.world_id AND h.house_key=r.house_key)
               ELSE before_item=after_item AND EXISTS(SELECT 1 FROM game_item_house_interior_locations h JOIN game_item_instances i USING(item_instance_id,world_id)
                       WHERE h.item_instance_id=r.target AND i.definition_production_key=$3 AND i.lifecycle=1) END
           FROM game_house_furniture_destroy_receipts r
          WHERE r.game_session_id=encode($1,'hex')::uuid AND r.target=encode($2,'hex')::uuid
          ORDER BY command_id DESC LIMIT 1",
    ).bind(session.as_slice()).bind(instance.as_slice()).bind(original)
        .fetch_optional(&mut *db).await?;
    validate_use_reply(
        "destroy",
        CommandStatus::Accepted,
        disposition,
        receipt.is_some_and(|(_, observed)| observed),
    )
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    if E2eMode::from_env()? == E2eMode::Journey {
        evidence("e2e_game_gap scenario=I10.full cause=reachable-owned-house-and-placed-furniture-unavailable");
        return Err(verdict::MissingServerRoute(concat!("source:", file!(), ":", line!())).into());
    }

    use super::full_row_i11::{self as fluids, OVENS, VESSEL};
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes.clone(),
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut db = sqlx::PgConnection::connect(clients.url).await?;
    let character = clients.characters[1];
    let index = provisioning::item_index()?;
    let fixture_house = prepare_house_fixture(&mut db, clients.world, clients.channel, character, &index).await?;
    let mut client = connect_world(clients, 0xb0).await?;
    let (house, item, revision, acl, cell): (String, Vec<u8>, String, String, Vec<u8>) = sqlx::query_as(
        "SELECT p.house_key,uuid_send(i.item_instance_id),i.state_revision::text,p.acl_revision::text,h.spatial_position
           FROM game_house_properties p JOIN game_item_house_interior_locations h USING(world_id,house_key)
           JOIN game_item_instances i USING(item_instance_id,world_id)
          WHERE p.world_id=encode($1,'hex')::uuid AND p.owner_character_id=encode($2,'hex')::uuid
            AND p.house_key=$3 AND i.definition_production_key='oteryn:item.tibia.i6355' AND i.lifecycle=1
          ORDER BY p.house_key,h.spatial_position,i.item_instance_id LIMIT 1",
    ).bind(clients.world.as_slice()).bind(character.as_slice()).bind(fixture_house).fetch_optional(&mut db).await?
        .ok_or("step=wrap observed=no-owned-house-with-placed-furniture needed=ordinary-house-settlement-and-furnishing")?;
    let item = item.as_slice().try_into()?;
    let revision: u64 = revision.parse()?;
    let position =
        crate::durability::consume_transform::HouseActionContext::decode_spatial_position(&cell)
            .ok_or("owned furniture has invalid position")?;
    let weapon_ref = provisioning::item_reference(&index, 3265)?;
    if fluids::carried_item(&client, weapon_ref).is_err() {
        super::full_row_i17c::acquire(&mut client, world, 3265, 1).await?;
    }
    let weapon = fluids::carried_item(&client, weapon_ref)?.handle;
    let entry = super::full_row_s05::entrance(&house)?;
    let path = route(
        world,
        tile(client.world_spatial().actor_position)?,
        |at| at == entry,
    )
    .ok_or("step=enter-house observed=no-route-to-owned-house")?;
    walk(&mut client, world, &path).await?;
    transfer_house(&mut client, &house, false).await?;

    let request = wrap::Request {
        action: wrap::Action::Wrap,
        item,
        expected_revision: revision,
        acl_revision: acl.parse()?,
        position,
    };
    let kit = provisioning::item_reference(&index, 6371)?;
    let previous_kits = fluids::quantity(&client, kit)?;
    let wrapped = wrap_command(&mut client, request).await?;
    let wrap::Reply::Committed(wrapped_revision) = wrapped else {
        return Err(format!("step=wrap observed={wrapped:?}").into());
    };
    wait_for_kit_count(&mut client, kit, previous_kits + 1).await?;
    if revision.checked_add(1) != Some(wrapped_revision)
        || furniture_custody(&mut db, item, character).await?
            != (
                "oteryn:item.tibia.i6371".into(),
                wrapped_revision.to_string(),
                None,
                true,
            )
    {
        return Err("step=wrap missing exact kit conversion or custody transition".into());
    }

    let unwrapped = wrap_command(
        &mut client,
        wrap::Request {
            action: wrap::Action::Unwrap,
            expected_revision: wrapped_revision,
            ..request
        },
    )
    .await?;
    let wrap::Reply::Committed(unwrapped_revision) = unwrapped else {
        return Err(format!("step=unwrap observed={unwrapped:?}").into());
    };
    wait_for_kit_count(&mut client, kit, previous_kits).await?;
    let placed = furniture_custody(&mut db, item, character).await?;
    if wrapped_revision.checked_add(1) != Some(unwrapped_revision)
        || placed
            != (
                "oteryn:item.tibia.i6355".into(),
                unwrapped_revision.to_string(),
                Some(cell),
                false,
            )
    {
        return Err("step=unwrap missing exact furniture conversion or house placement".into());
    }
    destroy(&mut client, &mut db, &house, &clients.world, weapon, &index).await?;
    let before_refusal = inventory_state(&client)?;
    let refused = wrap_command(&mut client, request).await?;
    client.service_liveness(Duration::from_millis(100)).await?;
    if refused != wrap::Reply::Refused(wrap::Refusal::Stale)
        || inventory_state(&client)? != before_refusal
        || furniture_custody(&mut db, item, character).await? != placed
    {
        return Err(format!(
            "step=stale-refusal observed={refused:?}; expected Stale and unchanged custody"
        )
        .into());
    }
    transfer_house(&mut client, &house, true).await?;
    let vessel = fluids::water_vessel(&mut client, world, &bytes, &index, true).await?;
    let (target, at) = fluids::walk_to_map_item(&mut client, world, &bytes, "pour-target", |key| {
        OVENS.contains(&key)
    })
    .await?;
    let splash = provisioning::item_reference(&index, 2886)?;
    client.take_events();
    let disposition = fluids::use_ingredient(&mut client, vessel, None, Some(target)).await?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let mut saw_splash = false;
    while tokio::time::Instant::now() < deadline {
        saw_splash |= client.take_events().into_iter().any(|event| {
            let oteryn_dev_client::SessionEvent::WorldMap(delta) = event else {
                return false;
            };
            delta.value.tiles.iter().any(|tile| {
                tile.position.x == i32::from(at.x)
                    && tile.position.y == i32::from(at.y)
                    && tile.position.floor == i16::from(at.floor)
                    && tile.items.iter().any(|item| {
                        item.definition == MapDefinition::Item(splash) && item.sub_type == 1
                    })
            })
        });
        if saw_splash {
            break;
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
    validate_use_reply(
        "fluid-pour",
        CommandStatus::Accepted,
        disposition,
        saw_splash,
    )?;
    fluids::wait_subtype(&mut client, vessel, 0).await?;

    let before_logout = inventory_state(&client)?;
    let session = client.join_snapshot().game_session_id;
    logout(&mut client).await?;
    let mut fresh = connect_world(clients, 0xb1).await?;
    let vessel_ref = provisioning::item_reference(&index, VESSEL)?;
    if fresh.join_snapshot().game_session_id == session
        || inventory_state(&fresh)? != before_logout
        || fluids::carried_item(&fresh, vessel_ref)?.sub_type != 0
        || furniture_custody(&mut db, item, character).await? != placed
    {
        return Err(
            "step=fresh-admission observed=durable-furniture-or-inventory-state-changed".into(),
        );
    }
    final_logout(&mut fresh).await
}

async fn wait_for_kit_count(
    client: &mut DevClientSession,
    kit: NonZeroU32,
    expected: u64,
) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while super::full_row_i11::quantity(client, kit)? != expected {
        if tokio::time::Instant::now() >= deadline {
            return Err("step=wrap observed=missing-kit-inventory-delta".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verdict_depends_on_observed_reply() {
        assert_eq!(classify("step=wrap observed=NotInHouse"), ("FAIL", GAP.to_owned()));
        assert_eq!(classify("step=wrap observed=Blocked").0, "FAIL");
    }

    #[test]
    fn use_steps_require_committed_reply_and_observable_delta() {
        let rejected = validate_use_reply(
            "destroy",
            CommandStatus::Rejected,
            world_object::UseDisposition::Rejected,
            false,
        );
        let rejected = rejected.err().map(|error| error.to_string());
        assert!(
            rejected.is_some_and(|error| error.contains("step=destroy observed=status:Rejected"))
        );

        let unverified = validate_use_reply(
            "fluid-fill",
            CommandStatus::Accepted,
            world_object::UseDisposition::Committed,
            false,
        );
        let unverified = unverified.err().map(|error| error.to_string());
        assert!(unverified.is_some_and(|error| {
            error.contains("needed=observable-furniture-or-fluid-state-delta")
        }));
    }
}

#[test]
fn furniture_and_fluid_use_can_pass_with_exact_state_evidence() {
    assert!(
        validate_use_reply(
            "fluid-fill",
            CommandStatus::Accepted,
            world_object::UseDisposition::Committed,
            true
        )
        .is_ok()
    );
}

const FIXTURE_HOUSES: &[(&str, [[i32; 3]; 2])] = &[
    ("oteryn:content.house.alai_flats_flat_01", [[32377, 32258, 7], [32378, 32258, 7]]),
    ("oteryn:content.house.alai_flats_flat_02", [[32382, 32258, 7], [32383, 32258, 7]]),
    ("oteryn:content.house.alai_flats_flat_03", [[32376, 32264, 7], [32376, 32265, 7]]),
];
const FIXTURE_ITEMS: [u32; 2] = [6355, 2295];

/// I10 owns the initial condition for the disposable node-boot world. The
/// measured wrap, destruction, and persistence operations are player commands.
async fn prepare_house_fixture(
    db: &mut sqlx::PgConnection,
    world: [u8; 16],
    channel: [u8; 16],
    character: [u8; 16],
    index: &crate::content::item_ref::ItemDefinitionIndex,
) -> TestResult<&'static str> {
    let mut tx = db.begin().await?;
    let mut chosen = None;
    for &(house, positions) in FIXTURE_HOUSES {
        sqlx::query(
            "WITH property AS (
               INSERT INTO game_house_properties
                 (world_id,house_key,kind,state,rent_gold,content_digest,owner_character_id,owner_account_id,acquisition_receipt_ref,paid_until)
               SELECT r.world_id,$2,'private_house',3,50000,a.server_artifact_digest,r.character_id,r.account_id,
                      game_character_uuid_v7(),statement_timestamp()+interval '30 days'
                 FROM game_character_roots r
                 CROSS JOIN LATERAL (
                   SELECT server_artifact_digest FROM game_content_activations
                   WHERE world_id=r.world_id AND channel_id=encode($4,'hex')::uuid
                   ORDER BY activation_sequence DESC LIMIT 1
                 ) a
                 WHERE r.world_id=encode($1,'hex')::uuid AND r.character_id=encode($3,'hex')::uuid
               ON CONFLICT (world_id,house_key) DO NOTHING RETURNING *
             ), slot AS (
               INSERT INTO game_account_housing_slots(account_id,world_id,house_key)
               SELECT owner_account_id,world_id,house_key FROM property
             )
             INSERT INTO game_house_ownership(world_id,house_key,owner_character_id,ownership_revision,acquisition_receipt_ref)
             SELECT world_id,house_key,owner_character_id,ownership_generation,acquisition_receipt_ref FROM property",
        ).bind(world.as_slice()).bind(house).bind(character.as_slice())
            .bind(channel.as_slice()).execute(&mut *tx).await?;
        // Serialize ownership verification and fixture placement against
        // settlement and other invocations of this fixture.
        let owned: bool = sqlx::query_scalar(
            "SELECT coalesce(p.owner_character_id=encode($3,'hex')::uuid AND p.state=3
                      AND p.content_digest=(
                        SELECT a.server_artifact_digest FROM game_content_activations a
                         WHERE a.world_id=p.world_id AND a.channel_id=encode($4,'hex')::uuid
                         ORDER BY a.activation_sequence DESC LIMIT 1
                      ),false)
               FROM game_house_properties p
              WHERE p.world_id=encode($1,'hex')::uuid AND p.house_key=$2 FOR UPDATE OF p",
        ).bind(world.as_slice()).bind(house).bind(character.as_slice()).bind(channel.as_slice())
            .fetch_optional(&mut *tx).await?.unwrap_or(false);
        if owned {
            chosen = Some((house, positions));
            break;
        }
    }
    let (house, positions) = chosen.ok_or("step=fixture all I10 houses are occupied or unavailable")?;
    // Use the same fixture-only insertion path as the native wrap dispatch test.
    sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
    for (id, position) in FIXTURE_ITEMS.into_iter().zip(positions) {
        let key = format!("oteryn:item.tibia.i{id}");
        let revision = index.revision(&key).ok_or("I10 fixture item definition is absent")?;
        let cell: Vec<u8> = position.into_iter().flat_map(i32::to_be_bytes).collect();
        sqlx::query(
            "INSERT INTO game_house_tiles(world_id,house_key,spatial_position)
             VALUES(encode($1,'hex')::uuid,$2,$3) ON CONFLICT DO NOTHING",
        ).bind(world.as_slice()).bind(house).bind(&cell).execute(&mut *tx).await?;
        sqlx::query(
            "WITH target AS (
               INSERT INTO game_item_instances
                 (item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id)
               SELECT game_character_uuid_v7(),encode($1,'hex')::uuid,'Item',$4,$5,1,1,game_character_uuid_v7()
               WHERE NOT EXISTS (
                 SELECT 1 FROM game_item_house_interior_locations h JOIN game_item_instances i USING(item_instance_id,world_id)
                 WHERE h.world_id=encode($1,'hex')::uuid AND h.house_key=$2 AND h.spatial_position=$3
                 AND i.lifecycle=1 AND i.definition_production_key=$4
               ) RETURNING item_instance_id,world_id
             ), placement AS (
               INSERT INTO game_item_house_interior_locations
                 (item_instance_id,world_id,house_key,spatial_position,stack_ordinal,placed_transaction_id)
               SELECT item_instance_id,world_id,$2,$3,1+coalesce((
                 SELECT max(stack_ordinal) FROM game_item_house_interior_locations
                 WHERE world_id=target.world_id AND house_key=$2 AND spatial_position=$3
               ),0),game_character_uuid_v7() FROM target
               RETURNING *
             )
             INSERT INTO game_item_house_reclaim_provenance
               (item_instance_id,world_id,house_key,reclaim_subject_character_id,placement_transaction_id,provenance_revision)
             SELECT item_instance_id,world_id,house_key,encode($6,'hex')::uuid,placed_transaction_id,1 FROM placement",
        ).bind(world.as_slice()).bind(house).bind(&cell).bind(&key)
            .bind(revision).bind(character.as_slice()).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(house)
}

#[test]
fn round3_stock_house_fixtures_supply_both_mechanics() -> TestResult {
    let catalogue: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../content/houses/houses-00000-00499.json"
    ))?;
    let houses = catalogue["houses"].as_array().ok_or("house catalogue")?;
    let index = provisioning::item_index()?;
    for &(key, positions) in FIXTURE_HOUSES {
        let house = houses.iter().find(|record| record["identity"]["key"] == key)
            .ok_or("I10 fixture house")?;
        assert_ne!(positions[0], positions[1]);
        for (id, position) in FIXTURE_ITEMS.into_iter().zip(positions) {
            assert!(house["tiles"].as_array().is_some_and(|cells| cells.contains(&serde_json::json!(position))));
            provisioning::item_reference(&index, id)?;
        }
    }
    Ok(())
}

#[test]
fn round4_house_fixture_falls_back_when_first_house_is_vacant() -> TestResult {
    use crate::bestiary_postgres_harness::{self as pg, Harness};
    let Some(admin) = pg::configured_admin() else { return Ok(()); };
    pg::runtime()?.block_on(async {
        let h = Harness::create(admin, "i10_furniture_fallback", true).await?;
        let result = async {
            let world = pg::id(pg::WORLD);
            let channel = pg::id(pg::CHANNEL);
            let character = pg::id(pg::CHARACTER);
            let mut db = h.pool.acquire().await?;
            let mut tx = db.begin().await?;
            sqlx::query(
                "INSERT INTO game_content_activations(world_id,channel_id,activation_sequence,
                  server_artifact_digest,client_artifact_digest,frame_binding_digest,issued_by,issued_at)
                 VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,$3,$3,$3,'test',1)
                 ON CONFLICT DO NOTHING",
            ).bind(world.as_slice()).bind(channel.as_slice()).bind([7_u8; 32].as_slice())
                .execute(&mut *tx).await?;
            sqlx::query(
                "WITH vacant AS (
                   INSERT INTO game_house_properties(world_id,house_key,kind,state,rent_gold,content_digest)
                   VALUES(encode($1,'hex')::uuid,$2,'private_house',1,50000,$3) RETURNING *
                 )
                 INSERT INTO game_house_ownership(world_id,house_key,ownership_revision,acquisition_receipt_ref)
                 SELECT world_id,house_key,ownership_generation,game_character_uuid_v7() FROM vacant",
            ).bind(world.as_slice()).bind(FIXTURE_HOUSES[0].0).bind([7_u8; 32].as_slice())
                .execute(&mut *tx).await?;
            tx.commit().await?;
            let index = provisioning::item_index()?;
            let mut second_db = h.pool.acquire().await?;
            let (first, second) = tokio::join!(
                prepare_house_fixture(&mut db, world, channel, character, &index),
                prepare_house_fixture(&mut second_db, world, channel, character, &index),
            );
            assert_eq!(first?, FIXTURE_HOUSES[1].0);
            assert_eq!(second?, FIXTURE_HOUSES[1].0);
            let original_state: i16 = sqlx::query_scalar(
                "SELECT state FROM game_house_properties WHERE world_id=encode($1,'hex')::uuid AND house_key=$2",
            ).bind(world.as_slice()).bind(FIXTURE_HOUSES[0].0).fetch_one(&mut *db).await?;
            assert_eq!(original_state, 1);
            let placed: Vec<(String, String)> = sqlx::query_as(
                "SELECT h.house_key,i.definition_production_key
                   FROM game_item_house_interior_locations h JOIN game_item_instances i USING(item_instance_id,world_id)
                  WHERE h.world_id=encode($1,'hex')::uuid ORDER BY i.definition_production_key",
            ).bind(world.as_slice()).fetch_all(&mut *db).await?;
            assert_eq!(placed, vec![
                (FIXTURE_HOUSES[1].0.into(), "oteryn:item.tibia.i2295".into()),
                (FIXTURE_HOUSES[1].0.into(), "oteryn:item.tibia.i6355".into()),
            ]);
            Ok(())
        }.await;
        h.cleanup().await?;
        result
    })
}
