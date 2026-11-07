//! Steward command-7 authorization feeds the native command-79 stash owner and current House ACL.
use super::{Harness, TestResult, debug, id};
use super::cook::{Client, count, drive, prepare_npc};
use crate::foundation::{ExactActorRef, MessageType, decode_wire_envelope};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::connection::{AdmittedSession, FreshAdmissionAuthority, UseCommand};
use oteryn_protocol_oteryn::{ClientCommandValue, CommandStatus, decode_command_result,
    decode_snapshot_chunk, encode_client_command};
use oteryn_protocol_oteryn::item_view::{ItemHandle, STATE_DOMAIN_CHARACTER_INVENTORY,
    decode_character_inventory};
use oteryn_protocol_oteryn::stash::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt, DuplexStream};

pub(super) async fn scenario(h: &Harness, owner: &ComposedFreshAdmission<'_, '_, '_>,
    mut admitted: AdmittedSession, actor: ExactActorRef, hireling: [u8;16]) -> TestResult {
    super::super::item_look::seed_main_backpack(h, &[("oteryn:item.tibia.i3028", 100)]).await?;
    guest_fixture(h, hireling).await?;
    let npc = prepare_npc(owner, admitted, actor, hireling).await?;
    // The viewer owns no skill; it is the current landlord's Steward that serves this guest.
    let viewer_skills: i64 = sqlx::query_scalar("SELECT count(*) FROM game_hireling_entitlements WHERE character_id=encode($1,'hex')::uuid AND entitlement_id=1003")
        .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(viewer_skills, 0);
    let generation = admitted.continuity.connection_generation;
    let (mut server, stream) = tokio::io::duplex(1 << 20);
    let mut client = Client::new(stream,admitted);
    let client_run = async {
        client.talk(npc,"hi").await?;
        let opened = client.talk(npc,"stash").await?;
        assert!(opened.lines.iter().any(|line|line.contains("supply stash is available")));
        let first = client.next_command;
        let mut item = client.first_item;
        send(&mut client.stream, generation, first, StashIntent { page: 0, action: StashAction::Browse }).await?;
        let browse = result(&mut client.stream, first, CommandStatus::Accepted, &mut item).await?;
        assert_eq!(browse.outcome, StashOutcome::Opened);
        assert!(browse.rows.is_empty());
        let stow = StashIntent { page: 0, action: StashAction::Stow(item.ok_or("real inventory snapshot handle")?) };
        send(&mut client.stream, generation, first + 1, stow).await?;
        let stowed = result(&mut client.stream, first + 1, CommandStatus::Accepted, &mut item).await?;
        assert_eq!((stowed.outcome, stowed.stowed_quantity), (StashOutcome::Committed, 100));
        assert_eq!(stowed.rows.len(), 1);
        assert_eq!(stowed.rows[0].quantity, 100);
        assert_eq!(stowed.rows[0].definition_key, "oteryn:item.tibia.i3028");
        send(&mut client.stream, generation, first + 1, stow).await?;
        let duplicate = result(&mut client.stream, first + 1, CommandStatus::DuplicateReplay, &mut item).await?;
        assert_eq!(duplicate, stowed);
        let withdraw = StashIntent { page: 0, action: StashAction::Withdraw {
            row: stowed.rows[0].handle, amount: 37,
        } };
        send(&mut client.stream, generation, first + 2, withdraw).await?;
        let withdrawn = result(&mut client.stream, first + 2, CommandStatus::Accepted, &mut item).await?;
        assert_eq!(withdrawn.outcome, StashOutcome::Committed);
        assert_eq!(withdrawn.rows[0].quantity, 63);
        assert_ne!(withdrawn.rows[0].handle, stowed.rows[0].handle);
        send(&mut client.stream, generation, first + 2, withdraw).await?;
        assert_eq!(result(&mut client.stream, first + 2, CommandStatus::DuplicateReplay, &mut item).await?, withdrawn);

        // Revoke only the durable ACL: the player, NPC and remembered open-stash UI do not move.
        revoke_guest(h).await?;
        send(&mut client.stream, generation, first + 3, StashIntent { page: 0, action: StashAction::Browse }).await?;
        let denied = result(&mut client.stream, first + 3, CommandStatus::Accepted, &mut item).await?;
        assert_eq!(denied.outcome, StashOutcome::NoLocker);
        assert!(denied.rows.is_empty());
        send(&mut client.stream, generation, first + 4, StashIntent { page: 0, action: StashAction::Withdraw {
            row: withdrawn.rows[0].handle, amount: 1,
        } }).await?;
        assert_eq!(result(&mut client.stream, first + 4, CommandStatus::Accepted, &mut item).await?.outcome,
            StashOutcome::NoLocker, "an old UI row cannot bypass the current House ACL");
        client.close().await?;
        Ok::<_, Box<dyn std::error::Error>>((first, stow, stowed, withdraw, withdrawn))
    };
    let (current,(first,stow,stowed,withdraw,withdrawn)) = drive(owner,admitted,&mut server,client_run).await?;
    admitted = current;
    let command = |command_id| UseCommand {
        game_session_id: admitted.game_session_id, command_id, item_fence: admitted.item_fence,
    };
    // Lost reply/restart probes durable history before ephemeral handles and current service ACL.
    for (number, intent, expected) in [(first + 1, stow, &stowed), (first + 2, withdraw, &withdrawn)] {
        let replay = owner.stash(actor, command(number), intent, None, false).await.ok_or("native durable stash replay")?;
        assert_eq!((replay.transaction, replay.outcome, replay.stowed_quantity),
            (expected.transaction, expected.outcome, expected.stowed_quantity));
        assert!(replay.rows.is_empty(), "historical receipt grants no new stash observation after ACL revocation");
    }
    assert_eq!(count(h, "SELECT count(*) FROM game_character_stash_operations").await?, 2);
    assert_eq!(count(h, "SELECT count(*) FROM game_character_stash_entries").await?, 2);
    assert_eq!(count(h, "SELECT count(*) FROM game_character_stash_audit_outbox").await?, 2);
    assert_eq!(count(h, "SELECT quantity FROM game_character_stash_balances WHERE definition_key='oteryn:item.tibia.i3028'").await?, 63);
    assert_eq!(count(h, "SELECT coalesce(sum(i.quantity),0)::bigint FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) WHERE i.definition_production_key='oteryn:item.tibia.i3028' AND i.lifecycle=1").await?, 37);
    let scopes: Vec<(String, String)> = sqlx::query_as("SELECT house_key,house_scope_ownership_generation::text FROM game_character_stash_operations ORDER BY command_id")
        .fetch_all(&h.pool).await?;
    let scope_generation = admitted.item_fence.ok_or("House fence")?.scope_ownership_generation.get().to_string();
    assert_eq!(scopes, vec![(super::HOUSE.into(), scope_generation.clone()), (super::HOUSE.into(), scope_generation)]);
    Ok(())
}

async fn guest_fixture(h: &Harness, hireling: [u8;16]) -> TestResult {
    // Seed a settled landlord/guest relation independently of the property-transfer tests.
    // All service reads, stash conversions and native item custody below use production owners.
    let landlord = id(121);
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica").execute(&mut *tx).await?;
    let landlord_root: String = sqlx::query_scalar("SELECT (to_jsonb(r)|| \
        jsonb_build_object('character_id',encode($2,'hex')::uuid,'name','Fixture Landlord'))::text \
        FROM game_character_roots r WHERE r.character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).bind(landlord.as_slice()).fetch_one(&mut *tx).await?;
    super::insert_character_root_copy(&mut tx, &landlord_root).await?;
    sqlx::query("UPDATE game_house_properties SET owner_character_id=encode($1,'hex')::uuid,ownership_generation=ownership_generation+1,revision=revision+1,acl_revision=acl_revision+1 WHERE world_id=encode($2,'hex')::uuid AND house_key=$3")
        .bind(landlord.as_slice()).bind(id(42).as_slice()).bind(super::HOUSE).execute(&mut *tx).await?;
    sqlx::query("UPDATE game_house_ownership SET owner_character_id=encode($1,'hex')::uuid,ownership_revision=ownership_revision+1 WHERE world_id=encode($2,'hex')::uuid AND house_key=$3")
        .bind(landlord.as_slice()).bind(id(42).as_slice()).bind(super::HOUSE).execute(&mut *tx).await?;
    sqlx::query("UPDATE game_house_hirelings SET owner_character_id=encode($1,'hex')::uuid WHERE hireling_id=encode($2,'hex')::uuid")
        .bind(landlord.as_slice()).bind(hireling.as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_house_acl_entries(world_id,house_key,list_key,character_id) VALUES(encode($1,'hex')::uuid,$2,'guest',encode($3,'hex')::uuid)")
        .bind(id(42).as_slice()).bind(super::HOUSE).bind(id(41).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_hireling_entitlements(character_id,entitlement_id) VALUES(encode($1,'hex')::uuid,1003)")
        .bind(landlord.as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

async fn revoke_guest(h: &Harness) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica").execute(&mut *tx).await?;
    sqlx::query("DELETE FROM game_house_acl_entries WHERE world_id=encode($1,'hex')::uuid AND house_key=$2 AND character_id=encode($3,'hex')::uuid")
        .bind(id(42).as_slice()).bind(super::HOUSE).bind(id(41).as_slice()).execute(&mut *tx).await?;
    sqlx::query("UPDATE game_house_properties SET revision=revision+1,acl_revision=acl_revision+1 WHERE world_id=encode($1,'hex')::uuid AND house_key=$2")
        .bind(id(42).as_slice()).bind(super::HOUSE).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

async fn send(client: &mut DuplexStream, generation: u64, command_id: u64, intent: StashIntent) -> TestResult {
    let payload = encode_stash_intent(intent).map_err(debug)?;
    let frame = encode_client_command(generation, &ClientCommandValue {
        command_id, command_type: COMMAND_TYPE_STASH_INTENT, payload: &payload,
    }).map_err(debug)?;
    client.write_u32(u32::try_from(frame.len())?).await?;
    client.write_all(&frame).await?;
    Ok(())
}

async fn result(client: &mut DuplexStream, command: u64, expected: CommandStatus,
    item: &mut Option<ItemHandle>) -> TestResult<StashReply> {
    loop {
        let length = usize::try_from(client.read_u32().await?)?;
        assert!(length <= 1 << 20);
        let mut bytes = vec![0; length]; client.read_exact(&mut bytes).await?;
        let envelope = decode_wire_envelope(&bytes).map_err(debug)?;
        if envelope.message_type() == MessageType::SnapshotChunk {
            let (_, domains) = decode_snapshot_chunk(envelope.payload()).map_err(debug)?;
            for domain in domains {
                if domain.domain_id == STATE_DOMAIN_CHARACTER_INVENTORY {
                    let inventory = decode_character_inventory(domain.payload).map_err(debug)?;
                    *item = inventory.entries.first().map(|entry| entry.handle);
                }
            }
        } else if envelope.message_type() == MessageType::CommandResult {
            let reply = decode_command_result(envelope.payload()).map_err(debug)?;
            assert_eq!(reply.command_id, command);
            assert_eq!(reply.status, expected);
            return decode_stash_reply(reply.payload).map_err(debug);
        }
    }
}
