//! The production text writer with owner-observed depot view data at its custody seam.
use super::fixture::{Harness, debug, id, run};
use super::{TestResult, prepare_current_session};
use crate::domain::item_definition::TypedDefinitionRef;
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::durability::item_text::{
    ItemTextWriteError, ItemTextWriteOutcome, ItemTextWriteRequest,
};
use crate::durability::item_transfer::{CurrentCharacterItemFence, scope_of};
use crate::durability::mail_text_policy::NativeGameplayMailTextPolicyExt;
use crate::durability::spell_owner_commit::SpellLanePermit;
use crate::foundation::{CommandId, CommandRef, ConnectionGeneration};
use std::time::Duration;

const TOWN: u16 = 7;
const BOX: u8 = 17;
const OTHER_BOX: u8 = 1;

#[test]
fn depot_nested_text_writer_checks_current_custody_and_replays_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            run(async |admin| {
                let h =
                    Harness::create_with_admission_seed(admin, "depot_nested_text", true, false)
                        .await?;
                let result = exercise(&h).await;
                h.cleanup().await?;
                result
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "nested depot text scenario panicked")??;
    Ok(())
}

async fn exercise(h: &Harness) -> TestResult {
    let admission = prepare_current_session(h).await?;
    let current = FreshAdmissionStore::from_root(h.root.clone())
        .current_session_at(admission.operation().authorization.candidate_session)
        .await?
        .0;
    let fence = crate::gameplay_transport::item_fence_of(current).ok_or("current item fence")?;
    let (world, channel) = scope_of(&fence)?;
    let character = fence.character_id.as_bytes().as_slice();
    let (controller, activation) =
        super::death_ml_loss_composed_fixtures::production_generation_with_pin(world, "content-1")?;
    let active = controller.active().ok_or("active content generation")?;
    let native = active.native_gameplay().ok_or("native gameplay")?;
    let definition = |source_id| {
        let item = native.item_policy_for_source_id(source_id)?;
        let def = &item.record().production_definition;
        Some(TypedDefinitionRef {
            family: def.family.clone(),
            production_key: def.production_key.clone(),
            revision_ref: def.revision_ref.clone(),
        })
    };
    let backpack = definition(2854).ok_or("backpack definition")?;
    let letter = definition(3505).ok_or("letter definition")?;
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice())
        .bind(activation.identity().server_artifact_digest().as_slice())
        .bind(activation.identity().client_artifact_digest().as_slice())
        .bind(activation.frame_binding().digest().as_slice())
        .execute(&h.pool).await?;

    let (root, inner, leaf) = (id(83), id(82), id(81));
    // Admit an existing depot tree as fixture state. The production writer below
    // still runs with all durable fences and triggers enabled.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    for (item, def) in [(&root, &backpack), (&inner, &backpack), (&leaf, &letter)] {
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,$5,1,1,game_character_uuid_v7())")
            .bind(item.as_slice()).bind(world.as_bytes().as_slice())
            .bind(&def.family).bind(&def.production_key).bind(&def.revision_ref)
            .execute(&mut *tx).await?;
    }
    for (child, parent) in [(&inner, &root), (&leaf, &inner)] {
        sqlx::query("INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,game_character_uuid_v7())")
            .bind(child.as_slice()).bind(world.as_bytes().as_slice()).bind(parent.as_slice())
            .execute(&mut *tx).await?;
    }
    sqlx::query("INSERT INTO game_item_character_depot_locations VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4,$5,1)")
        .bind(root.as_slice()).bind(world.as_bytes().as_slice()).bind(character)
        .bind(i32::from(TOWN)).bind(i16::from(BOX)).execute(&mut *tx).await?;
    sqlx::query(
        "INSERT INTO game_character_depot_box_counters VALUES(encode($1,'hex')::uuid,$2,$3,2,1)",
    )
    .bind(character)
    .bind(i32::from(TOWN))
    .bind(i16::from(BOX))
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let permit = SpellLanePermit::of_fresh_scope(fence.runtime_scope).await?;
    let write = async |command, text: &str, fence: CurrentCharacterItemFence, box_view| {
        h.root
            .commit_item_text_rewrite(
                &permit,
                &authority,
                &h.node,
                fence,
                native.mail_text_policy(&letter)?,
                ItemTextWriteRequest {
                    command: CommandRef::new(
                        fence.game_session_id,
                        CommandId::new(command).map_err(|_| ItemTextWriteError::InvalidInput)?,
                    ),
                    item_instance_id: leaf,
                    definition: letter.clone(),
                    text: text.into(),
                },
                box_view,
                None,
            )
            .await
    };

    const TEXT: &str = "Please leave the spare lantern by the door.";
    let view = Some((TOWN, BOX));
    for (command, wrong_view) in [
        (1, None),
        (2, Some((TOWN + 1, BOX))),
        (3, Some((TOWN, OTHER_BOX))),
    ] {
        assert!(matches!(
            write(command, TEXT, fence, wrong_view).await,
            Err(ItemTextWriteError::NotReachable)
        ));
    }
    let stale = CurrentCharacterItemFence {
        connection_generation: ConnectionGeneration::new(fence.connection_generation.get() + 1)
            .map_err(debug)?,
        ..fence
    };
    assert!(matches!(
        write(4, TEXT, stale, view).await,
        Err(ItemTextWriteError::AuthorityRejected)
    ));

    let written = write(5, TEXT, fence, view).await?;
    assert_eq!(
        written,
        ItemTextWriteOutcome {
            text_revision: 1,
            changed: true,
            replayed: false,
        }
    );
    let counters: (String, i32) = sqlx::query_as(
        "SELECT next_ordinal::text,committed FROM game_character_depot_box_counters",
    )
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(counters, ("2".into(), 1));

    // Rebox the locked root while another write waits on it. Leave both
    // nested edges and the letter's exact identity intact.
    let mut moved = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *moved)
        .await?;
    let blocker: i32 = sqlx::query_scalar(
        "SELECT pg_backend_pid() FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid FOR UPDATE",
    )
    .bind(root.as_slice())
    .fetch_one(&mut *moved)
    .await?;
    let rebox = async move {
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let waiting: bool = sqlx::query_scalar(
                    "SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE datname=current_database() AND $1=ANY(pg_blocking_pids(pid)))",
                )
                .bind(blocker)
                .fetch_one(&h.pool)
                .await?;
                if waiting {
                    return Ok::<_, sqlx::Error>(());
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .map_err(|_| "text writer did not wait on its depot ancestor")??;
        let relocated = sqlx::query("UPDATE game_item_character_depot_locations SET box_number=$2 WHERE item_instance_id=encode($1,'hex')::uuid")
            .bind(root.as_slice())
            .bind(i16::from(OTHER_BOX))
            .execute(&mut *moved)
            .await?;
        assert_eq!(relocated.rows_affected(), 1);
        sqlx::query("UPDATE game_character_depot_box_counters SET box_number=$1 WHERE character_id=encode($2,'hex')::uuid AND town_id=$3 AND box_number=$4")
            .bind(i16::from(OTHER_BOX)).bind(character).bind(i32::from(TOWN)).bind(i16::from(BOX))
            .execute(&mut *moved).await?;
        moved.commit().await?;
        Ok::<_, Box<dyn std::error::Error>>(())
    };

    let replacement = "The lantern is gone.";
    let (refused, reboxed) = tokio::join!(write(6, replacement, fence, view), rebox);
    reboxed?;
    assert!(matches!(refused, Err(ItemTextWriteError::NotReachable)));
    assert_eq!(
        write(5, TEXT, fence, None).await?,
        ItemTextWriteOutcome {
            replayed: true,
            ..written
        }
    );
    assert!(matches!(
        write(5, replacement, fence, None).await,
        Err(ItemTextWriteError::ConflictingCommand)
    ));

    let stored: (String, String, bool, i64) = sqlx::query_as(
        "SELECT text,text_revision::text,writer_character_id=encode($2,'hex')::uuid,\
         (SELECT count(*) FROM game_item_text_write_receipts) \
         FROM game_item_texts WHERE item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(leaf.as_slice())
    .bind(character)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(stored, (TEXT.into(), "1".into(), true, 1));
    eprintln!("I04-DEPOT_PG nested text custody and replay assertions completed");
    Ok(())
}
