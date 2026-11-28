//! Carried Crusher USE through normal handle dispatch, durable custody and replay.
use super::fixture::{Harness, debug, id};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{ChannelContentPin, ConnectionGeneration};
use crate::gameplay_transport::connection::UseCommand;
use crate::gameplay_transport::item_view::{ItemKey, SessionItemView, inventory_from_backpack};
use oteryn_protocol_oteryn::timed_item::{
    TimedAction, TimedDisposition, TimedItemIntent, decode_timed_item_result,
    encode_timed_item_intent,
};
use oteryn_protocol_oteryn::world_object::{MapUseTarget, UseDisposition, UseIntent, UseTarget};
use prost::Message;

#[test]
fn crusher_item_use_commits_charge_and_fragments_once() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, false)
}

#[test]
fn crusher_item_use_from_open_nested_bags() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, true)
}

fn run(admin: String, nested: bool) -> TestResult {
    std::thread::Builder::new()
        .stack_size(128 << 20)
        .spawn(move || -> Result<(), String> {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|error| error.to_string())?
                .block_on(Box::pin(scenario(admin, nested)))
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "Crusher case thread panicked")??;
    Ok(())
}

async fn scenario(admin: String, nested: bool) -> TestResult {
    let h = Harness::create_with_admission_seed(admin, "crusher", true, false).await?;
    super::reset();
    prepare_current_session(&h).await?;
    super::atelier_fixtures::fixture::eligible(&h.pool).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world = crate::bestiary_postgres_harness::fence(1)?
        .runtime_scope
        .world_id();
    let (controller, pin) =
        crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None);
    let active = controller
        .active()
        .ok_or("Crusher fixture active generation")?;
    let mut deps = Dependencies::with_content(
        ChannelContentPin::from_activation(
            world,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            pin.frame_binding().digest(),
            pin.qualified_room().map_revision_digest(),
            (6, 0, 7),
        ),
        crate::content::qualify_native_entry_room(world).map_err(debug)?,
        crate::spell::cast::v1_spell_book().map_err(debug)?,
    )?;
    deps.runtime
        .get_mut()
        .initialize_pinned_test_position(
            deps.actor,
            crate::foundation::MovementLocalPosition {
                x: 6,
                y: 0,
                floor: 7,
            },
        )
        .map_err(debug)?;
    let mut owner = deps.owner(&h, &authority);
    owner.active_generation = Some(active);
    deps.initialize(&owner).await;
    let session = deps.admitted.game_session_id;
    let current =
        crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
            .current_session_at(session)
            .await?
            .0;
    let fence = crate::gameplay_transport::item_fence_of(current).ok_or("Crusher item fence")?;
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(id(42).as_slice()).bind(id(43).as_slice())
        .bind(pin.identity().server_artifact_digest().as_slice())
        .bind(pin.identity().client_artifact_digest().as_slice())
        .bind(pin.frame_binding().digest().as_slice()).execute(&h.pool).await?;
    if nested {
        check_nested(&h, &owner, deps.actor, fence).await?;
        drop(owner);
        drop(authority);
        drop(seal);
        return h.cleanup().await;
    }
    super::item_look_fixtures::seed_main_backpack(
        &h,
        &[
            ("oteryn:item.tibia.i46627", 1),
            ("oteryn:item.tibia.i44608", 7),
            ("oteryn:item.tibia.i44609", 2),
            ("oteryn:item.tibia.i44610", 2),
            ("oteryn:item.tibia.i3577", 1),
        ],
    )
    .await?;
    let bag = owner
        .root
        .read_character_backpack(owner.character, fence.character_id)
        .await
        .map_err(debug)?
        .ok_or("Crusher backpack")?;
    let index = active
        .native_gameplay()
        .ok_or("Crusher native data")?
        .item_index()
        .ok_or("Crusher item index")?;
    let project = |definition: &crate::durability::item_mint::TypedDefinitionRef| {
        index.definition_ref(
            &definition.family,
            &definition.production_key,
            &definition.revision_ref,
        )
    };
    let mut view = SessionItemView::resume(Default::default());
    view.snapshot(
        inventory_from_backpack(Some(&bag), project).ok_or("Crusher inventory projection")?,
        None,
    )
    .map_err(debug)?;
    let handle = |seed| {
        view.table()
            .handle(&ItemKey::Instance(id(seed)))
            .ok_or("Crusher item handle")
    };
    let intent = |target| -> TestResult<_> {
        Ok(UseIntent {
            target: UseTarget::Item(handle(71)?),
            use_with: None,
            use_with_map_item: Some(MapUseTarget::Item(handle(target)?)),
            use_with_position: None,
        })
    };
    let dispatch = async |command_id, target, item_fence| -> TestResult<_> {
        let outcome = crate::gameplay_transport::mining::dispatch_map_use(
            &owner,
            deps.actor,
            UseCommand {
                game_session_id: session,
                command_id,
                item_fence: Some(item_fence),
            },
            intent(target)?,
            Some(&view),
        )
        .await;
        Ok(outcome.map(|outcome| outcome.disposition))
    };
    let call = async |command_id, target, item_fence| {
        dispatch(command_id, target, item_fence)
            .await?
            .ok_or_else(|| Box::<dyn std::error::Error>::from("Crusher USE outcome"))
    };
    assert_eq!(call(1, 75, fence).await?, UseDisposition::NothingToUse);
    let binding = fence.connection_generation.get() + 1;
    let stale = crate::durability::item_transfer::CurrentCharacterItemFence {
        connection_generation: ConnectionGeneration::new(binding).map_err(debug)?,
        ..fence
    };
    assert_eq!(call(2, 72, stale).await?, UseDisposition::Rejected);
    assert_eq!(call(3, 72, fence).await?, UseDisposition::Committed);
    assert_eq!(call(3, 72, fence).await?, UseDisposition::Committed);
    assert_eq!(call(3, 73, fence).await?, UseDisposition::Rejected);
    // Force an error after the gem, tool and fragments were written, at the
    // final craft audit insert. The transaction must roll back all three.
    sqlx::raw_sql(
        "CREATE FUNCTION test_crusher_refuse_audit() RETURNS trigger LANGUAGE plpgsql AS $$
         BEGIN
           IF EXISTS (SELECT 1 FROM game_item_craft_records
                       WHERE transaction_id=NEW.transaction_id AND recipe='oteryn:gem-atelier.crush') THEN
             RAISE EXCEPTION 'test Crusher rollback';
           END IF;
           RETURN NEW;
         END $$;
         CREATE TRIGGER test_crusher_refuse_audit BEFORE INSERT ON game_item_audit_outbox
         FOR EACH ROW EXECUTE FUNCTION test_crusher_refuse_audit();",
    ).execute(&h.pool).await?;
    assert_eq!(dispatch(4, 73, fence).await?, None);
    sqlx::query("DROP TRIGGER test_crusher_refuse_audit ON game_item_audit_outbox")
        .execute(&h.pool)
        .await?;
    let rolled_back: (i32, i64, i64) = sqlx::query_as(
        "SELECT (SELECT charges FROM game_item_timed_states WHERE item_instance_id=encode($1,'hex')::uuid),
                (SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($2,'hex')::uuid),
                (SELECT count(*) FROM game_item_craft_records WHERE recipe='oteryn:gem-atelier.crush')",
    ).bind(id(71).as_slice()).bind(id(73).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(rolled_back, (9, 2, 1));
    for (command, target) in [(4, 73), (5, 74)] {
        assert_eq!(
            call(command, target, fence).await?,
            UseDisposition::Committed
        );
    }
    let stored_charge: i32 = sqlx::query_scalar(
        "SELECT charges FROM game_item_timed_states WHERE item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(id(71).as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(stored_charge, 7);
    let bag = owner
        .root
        .read_character_backpack(owner.character, fence.character_id)
        .await
        .map_err(debug)?
        .ok_or("Crusher updated backpack")?;
    let projected =
        inventory_from_backpack(Some(&bag), project).ok_or("Crusher native projection")?;
    assert!(
        projected
            .entries
            .iter()
            .any(|entry| entry.key == ItemKey::Instance(id(71)))
    );
    for (seed, quantity) in [(72, 6), (73, 1), (74, 1)] {
        assert!(bag.entries.iter().any(
            |entry| entry.item.item_instance_id == id(seed) && entry.item.quantity == quantity
        ));
    }
    let outputs: Vec<(String, i64)> = sqlx::query_as(
        "SELECT definition_production_key,quantity FROM game_item_instances
         JOIN game_item_craft_records ON minted_transaction_id=transaction_id
         WHERE game_item_craft_records.recipe='oteryn:gem-atelier.crush' ORDER BY command_id",
    )
    .fetch_all(&h.pool)
    .await?;
    assert_eq!(outputs.len(), 3);
    for ((key, amount), (output, range)) in outputs.iter().zip([
        ("oteryn:item.tibia.i46625", 1..=4),
        ("oteryn:item.tibia.i46625", 2..=8),
        ("oteryn:item.tibia.i46626", 1..=4),
    ]) {
        assert_eq!(key, output);
        assert!(range.contains(amount));
    }
    let gameplay = owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "Crusher resource fence read")?
        .ok_or("Crusher resource fence")?;
    let balances = owner
        .root
        .read_character_atelier_resources(owner.character, owner.holder, gameplay)
        .await
        .map_err(debug)?;
    assert_eq!(
        (
            balances.lesser_gems,
            balances.regular_gems,
            balances.greater_gems
        ),
        (6, 1, 1)
    );
    assert_eq!(
        balances.basic_fragments,
        u64::try_from(outputs[0].1 + outputs[1].1)?
    );
    assert_eq!(balances.supreme_fragments, u64::try_from(outputs[2].1)?);
    let timed = async |command_id, action| -> TestResult<_> {
        let payload = encode_timed_item_intent(TimedItemIntent {
            action,
            target: handle(71)?,
            expected_revision: 3,
            npc: None,
        })
        .map_err(debug)?;
        let bytes = owner
            .timed_item_command(
                deps.actor,
                UseCommand {
                    game_session_id: session,
                    command_id,
                    item_fence: Some(fence),
                },
                &payload,
                Some(ItemKey::Instance(id(71))),
            )
            .await
            .ok_or("Crusher timed result")?;
        Ok(decode_timed_item_result(&bytes).map_err(debug)?)
    };
    let inspection = timed(20, TimedAction::Inspect).await?;
    assert_eq!(inspection.disposition, TimedDisposition::Ok);
    assert_eq!(inspection.charges, Some(7));
    let metadata = owner
        .root
        .read_inspection_metadata(owner.character, world, id(71))
        .await
        .map_err(debug)?
        .ok_or("Crusher inspection metadata")?;
    assert_eq!((metadata.charges, metadata.remaining_ms), (Some(7), None));
    assert_eq!(
        timed(21, TimedAction::Use).await?.disposition,
        TimedDisposition::Rejected
    );

    for command in 6..=11 {
        assert_eq!(call(command, 72, fence).await?, UseDisposition::Committed);
    }
    assert_eq!(call(12, 74, fence).await?, UseDisposition::Committed);
    // Both handles are gone after the final charge. The ordinary map USE
    // fallback still finds the exact historical craft receipt.
    let empty = SessionItemView::resume(Default::default());
    let replay = async |command_id| -> TestResult<_> {
        crate::gameplay_transport::mining::dispatch_map_use(
            &owner,
            deps.actor,
            UseCommand {
                game_session_id: session,
                command_id,
                item_fence: Some(fence),
            },
            intent(74)?,
            Some(&empty),
        )
        .await
        .map(|outcome| outcome.disposition)
        .ok_or_else(|| "Crusher expiry replay outcome".into())
    };
    assert_eq!(replay(12).await?, UseDisposition::Committed);
    assert_eq!(replay(13).await?, UseDisposition::StaleState);
    let after = owner
        .root
        .read_character_backpack(owner.character, fence.character_id)
        .await
        .map_err(debug)?
        .ok_or("Crusher final backpack")?;
    assert!(
        after
            .entries
            .iter()
            .all(|entry| ![id(71), id(74)].contains(&entry.item.item_instance_id))
    );
    assert_eq!(
        owner
            .current_quest_fence(session)
            .await
            .map_err(|()| "Crusher final gameplay fence read")?
            .ok_or("Crusher final gameplay fence")?
            .expected_character_revision,
        gameplay.expected_character_revision,
    );
    assert!(
        owner
            .root
            .read_inspection_metadata(owner.character, world, id(71))
            .await
            .map_err(debug)?
            .is_none()
    );
    let expiry: (i16, i32) = sqlx::query_as(
        "SELECT cause,charges_before FROM game_item_timed_state_writes
         WHERE item_instance_id=encode($1,'hex')::uuid ORDER BY expected_revision DESC LIMIT 1",
    )
    .bind(id(71).as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(expiry, (3, 1));
    let atomic: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_item_craft_records c
         JOIN game_item_timed_state_writes w ON w.created_xact_id=c.created_xact_id
         JOIN game_item_audit_outbox a ON a.transaction_id=c.transaction_id AND a.created_xact_id=c.created_xact_id
         WHERE c.recipe='oteryn:gem-atelier.crush' AND w.item_instance_id=encode($1,'hex')::uuid",
    ).bind(id(71).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        atomic, 10,
        "each craft and its timed write and audit share one commit"
    );
    let (envelope, transaction): (Vec<u8>, String) = sqlx::query_as(
        "SELECT a.envelope,w.transaction_id::text FROM game_item_craft_records c
         JOIN game_item_timed_state_writes w ON w.created_xact_id=c.created_xact_id
         JOIN game_item_audit_outbox a ON a.transaction_id=c.transaction_id
         WHERE c.recipe='oteryn:gem-atelier.crush' AND c.command_id=12",
    )
    .fetch_one(&h.pool)
    .await?;
    use crate::durability::item_mint_audit::{
        EventEnvelopeV1, OneItemOperationV1, OneItemTransactionV1,
    };
    let envelope = EventEnvelopeV1::decode(envelope.as_slice())?;
    let operation = OneItemTransactionV1::decode(envelope.payload.as_slice())?;
    let Some(OneItemOperationV1::Craft(craft)) = operation.operation else {
        return Err("Crusher audit operation".into());
    };
    let proof: serde_json::Value = serde_json::from_slice(&craft.proof)?;
    assert_eq!(
        proof["crusher"]["transaction"].as_str(),
        Some(transaction.as_str())
    );
    assert_eq!(proof["crusher"]["charges_after"].as_u64(), Some(0));
    eprintln!("POSTGRES_CASE_COMPLETED crusher_item_use_commits_charge_and_fragments_once");
    drop(owner);
    drop(authority);
    drop(seal);
    h.cleanup().await
}

async fn check_nested(
    h: &Harness,
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    fence: crate::durability::item_transfer::CurrentCharacterItemFence,
) -> TestResult {
    use crate::gameplay_transport::connection::FreshAdmissionAuthority;
    use crate::gameplay_transport::container_view::{ContainerPlan, ContainerViewState};
    use oteryn_protocol_oteryn::container_tree::{ContainerViewIntent, ContainerViewOutcome};

    super::item_look_fixtures::seed_main_backpack(
        h,
        &[
            ("oteryn:item.tibia.i2854", 1),
            ("oteryn:item.tibia.i46627", 1),
            ("oteryn:item.tibia.i44608", 2),
            ("oteryn:item.tibia.i46627", 1),
            ("oteryn:item.tibia.i44610", 1),
        ],
    )
    .await?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "WITH moved AS (
           DELETE FROM game_item_container_entries
            WHERE item_instance_id IN (encode($1,'hex')::uuid,encode($2,'hex')::uuid)
            RETURNING item_instance_id,world_id,placement_ordinal,placed_transaction_id
         )
         INSERT INTO game_item_nested_entries
           (item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id)
         SELECT item_instance_id,world_id,encode($3,'hex')::uuid,placement_ordinal,placed_transaction_id
           FROM moved",
    ).bind(id(72).as_slice()).bind(id(73).as_slice()).bind(id(71).as_slice())
        .execute(&mut *tx).await?;
    sqlx::query(
        "INSERT INTO game_item_timed_states(item_instance_id,charges,state_revision)
         VALUES(encode($1,'hex')::uuid,2,1)",
    )
    .bind(id(72).as_slice())
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    let bag = owner
        .root
        .read_character_backpack(owner.character, fence.character_id)
        .await
        .map_err(debug)?
        .ok_or("nested Crusher backpack")?;
    let index = owner
        .active_generation
        .and_then(|active| active.native_gameplay())
        .and_then(|native| native.item_index())
        .ok_or("nested Crusher item index")?;
    let mut items = SessionItemView::resume(Default::default()).with_container_tree();
    items
        .snapshot(
            inventory_from_backpack(Some(&bag), |definition| {
                index.definition_ref(
                    &definition.family,
                    &definition.production_key,
                    &definition.revision_ref,
                )
            })
            .ok_or("nested Crusher main projection")?,
            None,
        )
        .map_err(debug)?;
    assert_eq!(items.table().handle(&ItemKey::Instance(id(72))), None);
    assert_eq!(items.table().handle(&ItemKey::Instance(id(73))), None);
    let bag_handle = items
        .table()
        .handle(&ItemKey::Instance(id(71)))
        .ok_or("nested bag handle")?;
    let mut views = ContainerViewState::default();
    let ContainerPlan::Observe(pending) = views
        .plan(
            &mut items,
            ContainerViewIntent::Open {
                handle: bag_handle,
                replace_view: None,
            },
        )
        .map_err(debug)?
    else {
        return Err("nested bag must open via its handle".into());
    };
    let observation = owner
        .observe_container(actor, Some(fence), pending.key())
        .await;
    let (disposition, delta) = views
        .apply(&mut items, pending, observation)
        .map_err(debug)?;
    assert_eq!(disposition, ContainerViewOutcome::Opened);
    assert!(delta.is_some());
    let handle = |seed| {
        items
            .table()
            .handle(&ItemKey::Instance(id(seed)))
            .ok_or("Crusher operand handle")
    };
    for (command_id, crusher, gem) in [(1, 72, 73), (2, 74, 73), (3, 72, 75)] {
        let outcome = crate::gameplay_transport::mining::dispatch_map_use(
            owner,
            actor,
            UseCommand {
                game_session_id: fence.game_session_id,
                command_id,
                item_fence: Some(fence),
            },
            UseIntent {
                target: UseTarget::Item(handle(crusher)?),
                use_with: None,
                use_with_map_item: Some(MapUseTarget::Item(handle(gem)?)),
                use_with_position: None,
            },
            Some(&items),
        )
        .await
        .ok_or("nested Crusher result")?;
        assert_eq!(
            outcome.disposition,
            UseDisposition::Committed,
            "command {command_id}"
        );
        if command_id == 1 {
            let checkpoint: (i32, i64) = sqlx::query_as(
                "SELECT t.charges,i.quantity FROM game_item_timed_states t
                 JOIN game_item_nested_entries n ON n.item_instance_id=t.item_instance_id
                 JOIN game_item_instances i ON i.item_instance_id=encode($2,'hex')::uuid
                 WHERE t.item_instance_id=encode($1,'hex')::uuid",
            )
            .bind(id(72).as_slice())
            .bind(id(73).as_slice())
            .fetch_one(&h.pool)
            .await?;
            assert_eq!(checkpoint, (1, 1));
        }
    }
    let remaining: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_item_nested_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid",
    ).bind(id(71).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(remaining, 0);
    let retired: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_item_instances WHERE item_instance_id IN
         (encode($1,'hex')::uuid,encode($2,'hex')::uuid) AND lifecycle=2 AND quantity=0",
    )
    .bind(id(72).as_slice())
    .bind(id(73).as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(retired, 2);
    let outputs: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_item_craft_records r JOIN game_item_container_entries e
            ON e.placed_transaction_id=r.transaction_id
          WHERE r.recipe='oteryn:gem-atelier.crush' AND e.parent_item_instance_id=encode($1,'hex')::uuid",
    ).bind(id(70).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(outputs, 3);
    let timed_links: Vec<(i16, i32, i32)> = sqlx::query_as(
        "SELECT w.cause,w.charges_before,w.charges_after
         FROM game_item_crusher_nested_tool_proofs p
         JOIN game_item_craft_records r ON r.transaction_id=p.craft_transaction_id
         JOIN game_item_timed_state_writes w ON w.transaction_id=p.timed_transaction_id
            AND w.item_instance_id=p.item_instance_id AND w.created_xact_id=r.created_xact_id
         ORDER BY r.command_id",
    )
    .fetch_all(&h.pool)
    .await?;
    assert_eq!(timed_links, [(1, 2, 1), (3, 1, 1)]);
    eprintln!("POSTGRES_CASE_COMPLETED crusher_item_use_from_open_nested_bags");
    Ok(())
}
