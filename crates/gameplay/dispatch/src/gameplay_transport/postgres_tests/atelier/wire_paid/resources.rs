//! Command 484 projects current custody without changing paid receipts or ExAction.
use super::*;
use oteryn_protocol_oteryn::atelier::resources as resource_wire;

pub(super) async fn bundle_fixture(
    h: &Harness,
    pin: &crate::content::NativeEntryContentPin,
    world: crate::foundation::WorldId,
) -> TestResult<(
    crate::foundation::ChannelContentPin,
    crate::content::QualifiedNativeEntryRoom,
)> {
    // Reuse admit_cap_2's real bundle loader and room adapter. Capability 272 is
    // offered only by a bundle World; an entry-room fixture cannot negotiate it.
    let map = crate::map::spell_tiles::tests::movement_arena::world();
    let entry = pin.qualified_room();
    let cells = map.movement_cells(entry.movement_cells()).map_err(debug)?;
    let start = crate::content::NativeEntryStart {
        x: 100,
        y: 100,
        floor: 7,
    };
    let room = entry
        .bundle_view(
            cells,
            map.frame_binding_digest(world),
            map.map_revision_digest(),
            start,
        )
        .ok_or("Atelier resource bundle room")?;
    let channel_pin = crate::foundation::ChannelContentPin::from_activation(
        world,
        pin.activation_sequence(),
        pin.identity().server_artifact_digest(),
        pin.identity().client_artifact_digest(),
        room.frame_binding().digest(),
        room.map_revision_digest(),
        (start.x, start.y, start.floor),
    );
    // The Content owner records the actual bundle frame, as in admit_cap_2.
    // Admission subsequently compares this evidence with the real Channel pin.
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,NULL,$4,$5,$6)")
        .bind(id(42).as_slice()).bind(id(43).as_slice())
        .bind(pin.activation_sequence().to_string())
        .bind(pin.identity().server_artifact_digest().as_slice())
        .bind(pin.identity().client_artifact_digest().as_slice())
        .bind(room.frame_binding().digest().as_slice()).execute(&h.pool).await?;
    let stored: (Vec<u8>, Vec<u8>, Vec<u8>) = sqlx::query_as("SELECT server_artifact_digest,client_artifact_digest,frame_binding_digest FROM game_content_activations WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid ORDER BY activation_sequence DESC LIMIT 1")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        stored.0.as_slice(),
        pin.identity().server_artifact_digest().as_slice()
    );
    assert_eq!(
        stored.1.as_slice(),
        pin.identity().client_artifact_digest().as_slice()
    );
    assert_eq!(
        stored.2.as_slice(),
        room.frame_binding().digest().as_slice()
    );
    Ok((channel_pin, room))
}

async fn read_unchanged(
    h: &Harness,
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    expected: resource_wire::ResourceBalances,
) -> TestResult<Vec<u8>> {
    let before = scope_footprint(h).await?;
    let (status, bytes) = send_atelier_status(
        owner,
        admitted,
        resource_wire::COMMAND_TYPE_ATELIER_RESOURCES_QUERY,
        &[],
    )
    .await?;
    assert_eq!(
        status,
        oteryn_protocol_oteryn::CommandStatus::Accepted,
        "expected resource snapshot {expected:?}; selected {:?}",
        admitted.continuity.selected_capabilities.as_slice()
    );
    assert_eq!(
        resource_wire::decode_reply(&bytes).map_err(debug)?,
        expected
    );
    assert_eq!(
        scope_footprint(h).await?,
        before,
        "resource reads cannot change custody, balances, revisions or receipts"
    );
    Ok(bytes)
}

async fn workshop_unchanged(
    h: &Harness,
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    upgraded: Option<u16>,
) -> TestResult<wire::AtelierReply> {
    let before = scope_footprint(h).await?;
    let reply = query(owner, admitted).await?;
    assert!(!reply.refused);
    assert_eq!(reply.grades.len(), 69);
    assert_eq!(reply.grades.iter().filter(|row| row.kind == 0).count(), 46);
    assert_eq!(reply.grades.iter().filter(|row| row.kind == 1).count(), 23);
    assert!(
        reply
            .grades
            .windows(2)
            .all(|pair| (pair[0].kind, pair[0].mod_id) < (pair[1].kind, pair[1].mod_id))
    );
    assert!(
        reply
            .grades
            .iter()
            .any(|row| row.kind == 1 && row.mod_id == 58)
    );
    assert!(
        !reply
            .grades
            .iter()
            .any(|row| row.kind == 1 && row.mod_id == 93),
        "the sorcerer Workshop cannot list a monk-only Supreme modifier"
    );
    for row in &reply.grades {
        assert_eq!(
            row.grade,
            u8::from(row.kind == 0 && upgraded == Some(row.mod_id))
        );
    }
    let persisted = super::super::stored(h).await?;
    assert_eq!(
        persisted.grades.len(),
        usize::from(upgraded.is_some()),
        "zero-grade catalogue rows must never become stored upgrades"
    );
    if let Some(modifier) = upgraded {
        assert_eq!(
            persisted.grades.get(&(FragmentKind::Basic, modifier)),
            Some(&1)
        );
    }
    assert_eq!(
        scope_footprint(h).await?,
        before,
        "Workshop queries cannot change stored grades, balances, revisions or receipts"
    );
    Ok(reply)
}

async fn reject_unchanged(
    h: &Harness,
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    payload: &[u8],
) -> TestResult {
    let before = scope_footprint(h).await?;
    let (status, bytes) = send_atelier_status(
        owner,
        admitted,
        resource_wire::COMMAND_TYPE_ATELIER_RESOURCES_QUERY,
        payload,
    )
    .await?;
    assert_eq!(status, oteryn_protocol_oteryn::CommandStatus::Rejected);
    assert!(bytes.is_empty());
    assert_eq!(scope_footprint(h).await?, before);
    Ok(())
}

async fn nest_coins(h: &Harness) -> TestResult {
    // nest_asset already installed the native bag250 and lesser gems at ordinal1.
    // Only the fixture changes placement; the subsequent command uses runtime SQL.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    assert_eq!(sqlx::query("INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) SELECT item_instance_id,world_id,encode($2,'hex')::uuid,2,placed_transaction_id FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(id(235).as_slice()).bind(id(250).as_slice()).execute(&mut *tx).await?.rows_affected(), 1);
    assert_eq!(
        sqlx::query(
            "DELETE FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid"
        )
        .bind(id(235).as_slice())
        .execute(&mut *tx)
        .await?
        .rows_affected(),
        1
    );
    tx.commit().await?;
    Ok(())
}

async fn install_gold_pouch(
    h: &Harness,
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
) -> TestResult {
    use crate::durability::item_loop_move::ItemLoopContent;
    use crate::durability::item_mint::TypedDefinitionRef;
    use crate::gameplay_transport::item_ref_admission::{NativeItemWeights, pinned_index};
    let native = owner
        .active_generation
        .and_then(|active| active.native_gameplay())
        .ok_or("native Gold Pouch content")?;
    let revision = {
        let runtime = owner.runtime.lock().await;
        pinned_index(&runtime, owner.active_generation)
            .ok_or("pouch item index")?
            .revision("oteryn:item.tibia.i23721")
            .ok_or("pouch definition revision")?
            .to_owned()
    };
    let definition = TypedDefinitionRef {
        family: "Item".into(),
        production_key: "oteryn:item.tibia.i23721".into(),
        revision_ref: revision.clone(),
    };
    assert!(
        NativeItemWeights(native)
            .loop_facts(&definition)
            .is_some_and(|facts| facts.container_capacity.is_some()),
        "the fixture must use the admitted native Gold Pouch, not an ordinary Inbox bag"
    );
    // Same native Store Inbox pouch shape as postgres_tests/quick_loot.rs:103-139.
    // Provisioning is fixture-only; display and the following paid command use
    // the production restricted runtime owners.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    assert_eq!(sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) SELECT encode($1,'hex')::uuid,world_id,'Item','oteryn:item.tibia.i23721',$3,1,1,encode($1,'hex')::uuid FROM game_item_instances WHERE item_instance_id=encode($2,'hex')::uuid")
        .bind(id(246).as_slice()).bind(id(210).as_slice()).bind(revision).execute(&mut *tx).await?.rows_affected(), 1);
    sqlx::query("INSERT INTO game_item_character_inbox_locations(character_id,item_instance_id,world_id,ordinal) SELECT encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,coalesce(max(ordinal),0)+1 FROM game_item_character_inbox_locations WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).bind(id(246).as_slice()).bind(id(42).as_slice()).execute(&mut *tx).await?;
    assert_eq!(sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) SELECT encode($1,'hex')::uuid,world_id,definition_family,definition_production_key,definition_revision_ref,100,1,encode($1,'hex')::uuid FROM game_item_instances WHERE item_instance_id=encode($2,'hex')::uuid")
        .bind(id(247).as_slice()).bind(id(235).as_slice()).execute(&mut *tx).await?.rows_affected(), 1);
    sqlx::query("INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,encode($1,'hex')::uuid)")
        .bind(id(247).as_slice()).bind(id(42).as_slice()).bind(id(246).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

async fn pouch_state(h: &Harness) -> TestResult<serde_json::Value> {
    Ok(sqlx::query_scalar("SELECT jsonb_build_object('pouch',to_jsonb(p),'inbox',to_jsonb(loc),'coin',to_jsonb(i),'entry',to_jsonb(n)) FROM game_item_instances p JOIN game_item_character_inbox_locations loc ON loc.item_instance_id=p.item_instance_id JOIN game_item_nested_entries n ON n.parent_item_instance_id=p.item_instance_id JOIN game_item_instances i ON i.item_instance_id=n.item_instance_id WHERE p.item_instance_id=encode($1,'hex')::uuid AND i.item_instance_id=encode($2,'hex')::uuid")
        .bind(id(246).as_slice()).bind(id(247).as_slice()).fetch_one(&h.pool).await?)
}

async fn pouch_display_preserves_fee_exclusion(
    h: &Harness,
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    revealed: &wire::AtelierReply,
    after_reveal: resource_wire::ResourceBalances,
) -> TestResult {
    install_gold_pouch(h, owner).await?;
    let pouch_before = pouch_state(h).await?;
    read_unchanged(
        h,
        owner,
        admitted,
        resource_wire::ResourceBalances {
            carried_gold: 1_875_000,
            ..after_reveal
        },
    )
    .await?;
    assert_eq!(
        pouch_state(h).await?,
        pouch_before,
        "live display preserves the Store Inbox locator and every pouch coin"
    );

    // D174 still excludes Gold Pouch as a fee source. Display is not an admission
    // to spend it: this fee must use 875000 ordinary coins plus 1125000 bank gold.
    let live = workshop_unchanged(h, owner, admitted, None).await?;
    assert!(
        revealed.grades.is_empty(),
        "Reveal receipt keeps its sparse original grades"
    );
    let modifier = live
        .grades
        .iter()
        .find(|row| {
            row.kind == 0
                && row.grade == 0
                && live
                    .gems
                    .iter()
                    .all(|gem| !gem.modifiers.contains(&row.mod_id))
        })
        .ok_or("compatible Basic modifier absent from every revealed gem")?
        .mod_id;
    assert!(
        revealed
            .gems
            .iter()
            .all(|gem| !gem.modifiers.contains(&modifier))
    );
    owner.clock_origin = Instant::now()
        .checked_sub(Duration::from_millis(490))
        .ok_or("advance pouch payment ExAction")?;
    let payload = wire::encode_mutate(&wire::MutateRequest {
        occurrence: id(181),
        expected_revision: revealed.revision,
        mutation: wire::Mutation::GradeUp {
            kind: 0,
            mod_id: modifier,
        },
    })
    .map_err(debug)?;
    let reply_bytes =
        send_atelier(owner, admitted, wire::COMMAND_TYPE_ATELIER_MUTATE, &payload).await?;
    let reply = wire::decode_reply(&reply_bytes).map_err(debug)?;
    assert!(!reply.refused);
    assert_eq!(reply.revision, revealed.revision + 1);
    assert_eq!(
        reply.grades,
        vec![wire::GradeView {
            kind: 0,
            mod_id: modifier,
            grade: 1
        }],
        "the mutation receipt carries the stored upgrade, not the live zero-grade catalogue"
    );
    let fee: (i64, i64) = sqlx::query_as("SELECT fee_gold_units,bank_debit_gold_units FROM game_item_fee_burns WHERE cause_occurrence_id=encode($1,'hex')::uuid")
        .bind(id(181).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(fee, (2_000_000, 1_125_000));
    let paid = resource_wire::ResourceBalances {
        bank_gold: 998_875_000,
        carried_gold: 1_000_000,
        basic_fragments: 102,
        ..after_reveal
    };
    read_unchanged(h, owner, admitted, paid).await?;
    let current = workshop_unchanged(h, owner, admitted, Some(modifier)).await?;
    assert_eq!(current.revision, reply.revision);
    assert_eq!(
        pouch_state(h).await?,
        pouch_before,
        "the paid owner must not debit the displayed pouch gold"
    );
    let committed = scope_footprint(h).await?;
    assert_eq!(
        send_atelier(owner, admitted, wire::COMMAND_TYPE_ATELIER_MUTATE, &payload).await?,
        reply_bytes
    );
    assert_eq!(scope_footprint(h).await?, committed);
    assert_eq!(pouch_state(h).await?, pouch_before);
    read_unchanged(h, owner, admitted, paid).await?;
    assert_eq!(
        workshop_unchanged(h, owner, admitted, Some(modifier))
            .await?
            .grades,
        current.grades
    );
    Ok(())
}

pub(super) async fn check_unselected(
    h: &Harness,
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    assert_eq!(
        admitted.continuity.selected_capabilities.as_slice(),
        &[wire::CAPABILITY_WHEEL_ALLOCATION_V1]
    );
    reject_unchanged(h, owner, admitted, &[]).await
}

pub(super) async fn check(
    h: &Harness,
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    before: &wire::AtelierReply,
) -> TestResult {
    assert!(
        admitted
            .continuity
            .selected_capabilities
            .contains(wire::CAPABILITY_WHEEL_ALLOCATION_V1)
    );
    assert!(
        admitted
            .continuity
            .selected_capabilities
            .contains(resource_wire::CAPABILITY_ATELIER_RESOURCES_V1)
    );
    let baseline = resource_wire::ResourceBalances {
        bank_gold: 1_000_000_000,
        carried_gold: 1_000_000,
        lesser_gems: 2,
        regular_gems: 2,
        greater_gems: 2,
        basic_fragments: 100,
        supreme_fragments: 100,
    };
    read_unchanged(h, owner, admitted, baseline).await?;
    assert_eq!(
        workshop_unchanged(h, owner, admitted, None).await?.grades,
        before.grades
    );
    reject_unchanged(h, owner, admitted, &[1]).await?;

    nest_asset(h, 220, 2).await?;
    nest_coins(h).await?;
    custody::clone_carried(h, 229, 252, 7, 9).await?;
    custody::clone_carried(h, 232, 253, 11, 10).await?;
    stow_asset_command(owner, admitted, 229, 100, 150).await?;
    stow_asset_command(owner, admitted, 232, 100, 151).await?;
    stow_asset_command(owner, admitted, 223, 2, 152).await?;
    let current = resource_wire::ResourceBalances {
        regular_gems: 0, // Stashed unrevealed gems never enter the carried count.
        basic_fragments: 107,
        supreme_fragments: 111,
        ..baseline
    };
    let first = read_unchanged(h, owner, admitted, current).await?;
    assert_eq!(read_unchanged(h, owner, admitted, current).await?, first);

    // Resources uses the same future-origin clock as the existing ExAction case.
    // PostgreSQL latency cannot accidentally expire a wrongly charged read window.
    advance_ex_action(owner)?;
    assert_eq!(owner.owner_now().get(), 0);
    read_unchanged(h, owner, admitted, current).await?;
    let stale_fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "current resource fence")?
        .ok_or("current resource fence")?;
    let payload = wire::encode_mutate(&wire::MutateRequest {
        occurrence: id(180),
        expected_revision: before.revision,
        mutation: wire::Mutation::Reveal { quality: 0 },
    })
    .map_err(debug)?;
    let reply_bytes =
        send_atelier(owner, admitted, wire::COMMAND_TYPE_ATELIER_MUTATE, &payload).await?;
    let reply = wire::decode_reply(&reply_bytes).map_err(debug)?;
    assert!(
        !reply.refused,
        "resource queries must not consume the fresh Atelier ExAction"
    );
    assert_eq!(reply.revision, before.revision + 1);
    assert_eq!(reply.gems.len(), before.gems.len() + 1);
    assert_eq!(owner.owner_now().get(), 0);
    let after_payment = resource_wire::ResourceBalances {
        carried_gold: 875_000,
        lesser_gems: 1,
        ..current
    };
    let committed = scope_footprint(h).await?;
    read_unchanged(h, owner, admitted, after_payment).await?;
    assert_eq!(
        owner.owner_now().get(),
        0,
        "the successful live query runs inside the closed mutation window"
    );

    assert!(matches!(
        owner
            .root
            .read_character_atelier_resources(owner.character, owner.holder, stale_fence,)
            .await,
        Err(CharacterProgressionError::CharacterRevisionMismatch)
    ));
    assert_eq!(
        scope_footprint(h).await?,
        committed,
        "a stale expected revision cannot read or alter resource state"
    );
    assert_eq!(
        send_atelier(owner, admitted, wire::COMMAND_TYPE_ATELIER_MUTATE, &payload).await?,
        reply_bytes,
        "the paid mutation replays its original bytes despite live resource queries"
    );
    assert_eq!(
        scope_footprint(h).await?,
        committed,
        "receipt replay cannot charge a second time"
    );
    read_unchanged(h, owner, admitted, after_payment).await?;
    pouch_display_preserves_fee_exclusion(h, owner, admitted, &reply, after_payment).await?;
    Ok(())
}
