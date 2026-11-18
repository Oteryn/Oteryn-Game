use super::TestResult;
use super::fixture::{Harness, debug, id};
use super::house_balloon::{Case, run_case};
use crate::foundation::{MessageType, decode_wire_envelope};
use crate::gameplay_transport::{
    ComposedFreshAdmission,
    connection::{AdmittedSession, FreshAdmissionAuthority, IDLE_LIVENESS, serve_admitted},
};
use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, decode_command_result, encode_client_command,
};
use oteryn_protocol_oteryn::{item_inscription, item_view, world_spatial_entities as entities};
use tokio::io::AsyncWriteExt;

#[test]
fn house_resident_tick_postgres() -> TestResult {
    run_case(Case::ResidentTick)
}

#[test]
fn house_resident_letter_6246_postgres() -> TestResult {
    run_case(Case::ResidentLetter)
}

pub(super) async fn exchange(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    letter: std::num::NonZeroU32,
    case: Case,
) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "INSERT INTO game_guilds(guild_id,world_id,name,state,formation_deadline,\
                          vice_deficit_since,revision,created_at) \
         VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'House Guild',2,9999999999999,1,1,1)",
    )
    .bind(id(90).as_slice())
    .bind(id(42).as_slice())
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO game_guild_members(character_id,guild_id,rank_level,title,joined_at) \
         VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,'',1)",
    )
    .bind(id(41).as_slice())
    .bind(id(90).as_slice())
    .execute(&mut *tx)
    .await?;
    sqlx::query("INSERT INTO game_item_text_revisions VALUES(encode($1,'hex')::uuid,1)")
        .bind(id(71).as_slice())
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "INSERT INTO game_item_texts \
         VALUES(encode($1,'hex')::uuid,'House letter',encode($2,'hex')::uuid,now(),1)",
    )
    .bind(id(71).as_slice())
    .bind(id(41).as_slice())
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let observe = async move {
        let mut capacity = false;
        let mut emblem = false;
        loop {
            let raw = super::item_look::read_frame(&mut client).await?;
            let envelope = decode_wire_envelope(&raw).map_err(debug)?;
            match envelope.message_type() {
                MessageType::SnapshotChunk if case == Case::ResidentLetter => {
                    let (_, domains) =
                        oteryn_protocol_oteryn::decode_snapshot_chunk(envelope.payload())
                            .map_err(debug)?;
                    for domain in domains {
                        if domain.domain_id != item_view::STATE_DOMAIN_CHARACTER_INVENTORY {
                            continue;
                        }
                        let inventory = item_view::decode_character_inventory_with_equip_drop(
                            domain.payload,
                            false,
                        )
                        .map_err(debug)?;
                        let handle = inventory
                            .entries
                            .iter()
                            .find(|entry| entry.item_definition_ref == letter)
                            .ok_or("letter handle")?
                            .handle;
                        let request = item_inscription::Request {
                            item_handle: handle.get(),
                        }
                        .encode()
                        .ok_or("request")?;
                        let frame = encode_client_command(
                            admitted.continuity.connection_generation,
                            &ClientCommandValue {
                                command_id: 1,
                                command_type: item_inscription::COMMAND_TYPE_ITEM_INSCRIPTION_READ,
                                payload: &request,
                            },
                        )
                        .map_err(debug)?;
                        client.write_u32(u32::try_from(frame.len())?).await?;
                        client.write_all(&frame).await?;
                    }
                }
                MessageType::CommandResult if case == Case::ResidentLetter => {
                    let reply = decode_command_result(envelope.payload()).map_err(debug)?;
                    assert_eq!(reply.status, CommandStatus::Accepted);
                    assert_eq!(
                        item_inscription::Reply::decode(reply.payload),
                        Some(item_inscription::Reply {
                            text: "House letter".into(),
                            writable: true,
                            max_length: 1999
                        })
                    );
                    break;
                }
                MessageType::StateDelta if case == Case::ResidentTick => {
                    let delta = oteryn_protocol_oteryn::decode_state_delta(envelope.payload())
                        .map_err(debug)?;
                    if delta.domain_id != 1 || delta.delta_type != 2 {
                        continue;
                    }
                    let fields = entities::EntityFields::selected(
                        admitted.continuity.selected_capabilities.as_slice(),
                    );
                    match entities::decode_world_spatial_entities_delta_with_sidecar(
                        delta.payload,
                        fields,
                    )
                    .map_err(debug)?
                    .2
                    {
                        Some(entities::DeltaSidecar::InventoryCapacity(value)) => {
                            capacity = value.free < value.capacity
                        }
                        Some(entities::DeltaSidecar::GuildEmblems(entries)) => {
                            emblem = entries.iter().any(|entry| {
                                entry.emblem == oteryn_protocol_oteryn::guild_emblem::Emblem::Member
                                    && Some(entry.entity.identity)
                                        == admitted
                                            .runtime_actor
                                            .map(|actor| actor.placement_identity())
                            })
                        }
                        _ => (),
                    }
                    if capacity && emblem {
                        break;
                    }
                }
                _ => (),
            }
        }
        client.shutdown().await?;
        Ok::<_, Box<dyn std::error::Error>>(())
    };
    let (_, result) = tokio::time::timeout(std::time::Duration::from_secs(12), async {
        tokio::join!(
            serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS),
            observe
        )
    })
    .await?;
    result?;
    // A retained session with the wrong named House cannot authorize any of these reads.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    let changed = sqlx::query(
        "UPDATE game_durability_reconnect_sessions \
         SET runtime_scope_house_key=$3, runtime_scope_instance_id=game_house_scope_instance_id(runtime_scope_world_id,$3) \
         WHERE game_session_id=encode($1,'hex')::uuid AND runtime_scope_house_key=$2",
    )
    .bind(admitted.game_session_id.as_bytes().as_slice())
    .bind(super::house_balloon::HOUSE)
    .bind("oteryn:content.house.wrong")
    .execute(&mut *tx)
    .await?;
    assert_eq!(changed.rows_affected(), 1);
    tx.commit().await?;
    assert!(owner.observe_inventory_capacity(admitted).await.is_none());
    assert!(owner.observe_guild_emblems(admitted).await.is_none());
    assert!(
        owner
            .item_inscription_read(
                admitted,
                Some(crate::gameplay_transport::item_view::ItemKey::Instance(id(
                    71
                )))
            )
            .await
            .is_none()
    );
    Ok(())
}
