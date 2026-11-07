//! Elathriel: exact paid copper key through source conversation and native wallet/MINT owners.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
#[test]
fn draconia_elathriel_carried_price_exact_key_replay_and_repurchase() -> TestResult {
    scenario("draconia-elathriel-coins")
}
#[test]
fn draconia_elathriel_bank_price_exact_key_replay_and_repurchase() -> TestResult {
    scenario("draconia-elathriel-bank")
}
#[test]
fn draconia_elathriel_full_backpack_rolls_back_bank_fee_and_key() -> TestResult {
    scenario("draconia-elathriel-full")
}
fn scenario(name: &'static str) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run(name, move || {
        super::quest_world_callbacks::scenario(
            name,
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    });
    eprintln!(
        "POSTGRES_CASE_{} {name} duration={:?}",
        if result.is_ok() {
            "COMPLETED"
        } else {
            "FAILED"
        },
        started.elapsed()
    );
    result
}
async fn counts(h: &Harness) -> TestResult<(i64, i64)> {
    Ok(sqlx::query_as("SELECT (SELECT count(*) FROM game_item_fee_burns WHERE cause_kind=18),(SELECT count(*) FROM game_item_keys)").fetch_one(&h.pool).await?)
}
async fn bank(h: &Harness, amount: i64) -> TestResult {
    let uuid = crate::durability::item_fee_burn::test_support::uuid;
    // Fixture-only funding retains the ordinary bank ledger head. Gameplay below
    // uses the restricted runtime payment and MINT owners, with no seeded key/quest flag.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    let seed = format!(
        "INSERT INTO game_account_bank_operations(operation_occurrence_id,transaction_id,event_id,kind,request_binding,acting_character_id,account_id,world_id,channel_id,runtime_scope_ownership_generation,amount,outcome,occurred_at,envelope_sha256,committed_at,planned_platinum_item_instance_id,planned_gold_item_instance_id,backpack_item_instance_id) VALUES({},{},{},1,sha256('seed'),{},{},{},{},1,{amount},0,1,sha256('seed'),0,{},{},{}); INSERT INTO game_account_bank_entries(entry_id,transaction_id,account_id,world_id,kind,amount,balance_before,balance_after,acting_character_id) VALUES({},{},{},{},1,{amount},0,{amount},{}); INSERT INTO game_account_bank_balances VALUES({},{},{amount},{});",
        uuid(80),
        uuid(81),
        uuid(82),
        uuid(41),
        uuid(40),
        uuid(42),
        uuid(43),
        uuid(83),
        uuid(84),
        uuid(70),
        uuid(85),
        uuid(81),
        uuid(40),
        uuid(42),
        uuid(41),
        uuid(40),
        uuid(42),
        uuid(85)
    );
    sqlx::raw_sql(sqlx::AssertSqlSafe(seed))
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

async fn acknowledge_probe(
    client: &mut tokio::io::DuplexStream,
    generation: u64,
    payload: &[u8],
) -> Result<(), String> {
    use tokio::io::AsyncWriteExt;
    let probe =
        oteryn_protocol_oteryn::decode_liveness_probe(payload).map_err(|e| format!("{e:?}"))?;
    let frame = oteryn_protocol_oteryn::encode_liveness_ack(generation, probe, 0)
        .map_err(|e| format!("{e:?}"))?;
    client
        .write_all(
            &u32::try_from(frame.len())
                .map_err(|e| e.to_string())?
                .to_be_bytes(),
        )
        .await
        .map_err(|e| e.to_string())?;
    client.write_all(&frame).await.map_err(|e| e.to_string())
}

// A conversation spans several commands on ONE connection. Re-opening the
// one-command movement helper correctly clears conversation engagement.
pub(super) async fn wire_grant(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    npc: EntityRef,
    words: &[&str],
) -> TestResult<(
    NpcIntentDisposition,
    Vec<oteryn_protocol_oteryn::appearance::Catalogue>,
    u64,
)> {
    use crate::foundation::{MessageType, decode_wire_envelope};
    use crate::gameplay_transport::connection::{ConnectionEnd, IDLE_LIVENESS, serve_admitted};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let words = words.to_vec();
    let last = admitted.continuity.next_command_id + u64::try_from(words.len())? - 1;
    let generation = admitted.continuity.connection_generation;
    let frames = words
        .into_iter()
        .enumerate()
        .map(|(i, text)| {
            let payload =
                oteryn_protocol_oteryn::npc_service::encode_npc_talk_intent(&NpcTalkIntent {
                    npc_actor: npc,
                    text: text.into(),
                })
                .map_err(debug)?;
            oteryn_protocol_oteryn::encode_client_command(
                generation,
                &oteryn_protocol_oteryn::ClientCommandValue {
                    command_id: admitted.continuity.next_command_id + u64::try_from(i)?,
                    command_type: 7,
                    payload: &payload,
                },
            )
            .map_err(debug)
        })
        .collect::<TestResult<Vec<_>>>()?;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let peer = tokio::spawn(async move {
        let mut catalogues = Vec::new();
        let mut result = None;
        let mut window = tokio::time::Instant::now();
        for (index, frame) in frames.iter().enumerate() {
            // Drive the actual NPC0-RL-04 admission window, rather than
            // disabling its limiter for Navigator's ten-message conversation.
            if index != 0 && index % 4 == 0 {
                tokio::time::sleep_until(window + std::time::Duration::from_secs(1)).await;
                window = tokio::time::Instant::now();
            }
            client
                .write_all(
                    &u32::try_from(frame.len())
                        .map_err(|e| e.to_string())?
                        .to_be_bytes(),
                )
                .await
                .map_err(|e| e.to_string())?;
            client.write_all(frame).await.map_err(|e| e.to_string())?;
            loop {
                let mut prefix = [0; 4];
                client
                    .read_exact(&mut prefix)
                    .await
                    .map_err(|e| e.to_string())?;
                let mut bytes = vec![0; u32::from_be_bytes(prefix) as usize];
                client
                    .read_exact(&mut bytes)
                    .await
                    .map_err(|e| e.to_string())?;
                let envelope = decode_wire_envelope(&bytes).map_err(|e| format!("{e:?}"))?;
                if envelope.message_type() == MessageType::LivenessProbe {
                    acknowledge_probe(&mut client, generation, envelope.payload()).await?;
                }
                if envelope.message_type() == MessageType::StateDelta {
                    let delta = oteryn_protocol_oteryn::decode_state_delta(envelope.payload())
                        .map_err(|e| format!("{e:?}"))?;
                    if delta.domain_id == 21
                        && delta.delta_type
                            == oteryn_protocol_oteryn::appearance::DELTA_TYPE_APPEARANCE_CATALOGUE
                    {
                        catalogues.push(
                            oteryn_protocol_oteryn::appearance::Catalogue::decode(delta.payload)
                                .map_err(|e| format!("{e:?}"))?,
                        );
                    }
                }
                if envelope.message_type() == MessageType::CommandResult {
                    let command = oteryn_protocol_oteryn::decode_command_result(envelope.payload())
                        .map_err(|e| format!("{e:?}"))?;
                    let disposition =
                        oteryn_protocol_oteryn::npc_service::decode_npc_intent_result(
                            command.payload,
                        )
                        .map_err(|e| format!("{e:?}"))?;
                    if command.command_id == last {
                        result = Some(disposition);
                    } else {
                        assert_eq!(
                            disposition,
                            NpcIntentDisposition::Ok,
                            "conversation command {}",
                            command.command_id
                        );
                    }
                    if (index + 1) % 4 == 0 {
                        window = tokio::time::Instant::now();
                    }
                    break;
                }
            }
        }
        let has_outfit = |_catalogues: &[oteryn_protocol_oteryn::appearance::Catalogue]| true;
        let mut shut = has_outfit(&catalogues);
        if shut {
            client.shutdown().await.map_err(|e| e.to_string())?;
        }
        // Keep the real connection alive until the normal appearance refresh
        // publishes the earned choice, then drain effects through EOF.
        let drain_deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            let mut prefix = [0; 4];
            match tokio::time::timeout_at(drain_deadline, client.read_exact(&mut prefix))
                .await
                .map_err(|_| "earned appearance catalogue did not arrive".to_owned())?
            {
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(e.to_string()),
            }
            let mut bytes = vec![0; u32::from_be_bytes(prefix) as usize];
            client
                .read_exact(&mut bytes)
                .await
                .map_err(|e| e.to_string())?;
            let envelope = decode_wire_envelope(&bytes).map_err(|e| format!("{e:?}"))?;
            if !shut && envelope.message_type() == MessageType::LivenessProbe {
                acknowledge_probe(&mut client, generation, envelope.payload()).await?;
            }
            if envelope.message_type() == MessageType::StateDelta {
                let delta = oteryn_protocol_oteryn::decode_state_delta(envelope.payload())
                    .map_err(|e| format!("{e:?}"))?;
                if delta.domain_id == 21
                    && delta.delta_type
                        == oteryn_protocol_oteryn::appearance::DELTA_TYPE_APPEARANCE_CATALOGUE
                {
                    catalogues.push(
                        oteryn_protocol_oteryn::appearance::Catalogue::decode(delta.payload)
                            .map_err(|e| format!("{e:?}"))?,
                    );
                }
            }
            if !shut && has_outfit(&catalogues) {
                client.shutdown().await.map_err(|e| e.to_string())?;
                shut = true;
            }
        }
        Ok::<_, String>((result.ok_or("no reward result")?, catalogues, last))
    });
    let end = serve_admitted(&mut server, *admitted, owner, IDLE_LIVENESS).await;
    drop(server);
    if let ConnectionEnd::AdmittedThenDisconnected(a) | ConnectionEnd::AdmittedThenClosed(a, _) =
        end
    {
        *admitted = a;
    }
    peer.await?.map_err(Into::into)
}

async fn unpaired_stateless_receipt_is_rejected(h: &Harness) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *tx)
        .await?;
    let shared_barter_preserved: bool = sqlx::query_scalar("SELECT game_quest_reward_mint_claim_key(encode($1,'hex')::uuid,encode($2,'hex')::uuid,17,'oteryn:quest-reward/shared-barter',$3::jsonb)=game_quest_reward_mint_claim_key_qengine_base(encode($1,'hex')::uuid,encode($2,'hex')::uuid,17,'oteryn:quest-reward/shared-barter',$3::jsonb)")
        .bind(super::fixture::id(41).as_slice()).bind(super::fixture::id(50).as_slice())
        .bind(serde_json::json!({"storage":[],"take":[{"item":"oteryn:item.tibia.i3723","count":1}],"give":{"item":"oteryn:item.tibia.i3206","count":1}}))
        .fetch_one(&mut *tx).await?;
    assert!(
        shared_barter_preserved,
        "monetary extension must preserve shared barter claims"
    );
    sqlx::query("INSERT INTO game_character_quest_receipts(character_id,cause_kind,cause_id,cause_ordinal,transition_key,quest_key,request_binding,pinned_content_revision,definition_hash,completes,track_keys,values_before,values_after,original_character_revision,committed_character_revision,level_before,level_after,experience_before,experience_after,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision,committed_at) SELECT character_id,'command',game_character_uuid_v7(),999999,'oteryn:quest-reward/forged-stateless','oteryn:quest.draconia_quest',decode('00','hex'),'test-1',decode(repeat('00',32),'hex'),false,ARRAY[]::text[],ARRAY[]::bigint[],ARRAY[]::bigint[],1000000,1000001,1,1,0,0,'test-1','test-1','test-1','test-1','test-1','test-1','test-1','test-1',1 FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
        .bind(super::fixture::id(41).as_slice()).execute(&mut *tx).await?;
    let error = sqlx::query("SET CONSTRAINTS game_quest_reward_value_only_complete, game_quest_stateless_paid_complete IMMEDIATE")
        .execute(&mut *tx)
        .await
        .err()
        .ok_or("unpaired zero-effect receipt was accepted")?;
    let database = error
        .as_database_error()
        .ok_or("constraint database error")?;
    assert_eq!(database.code().as_deref(), Some("23514"));
    assert_eq!(
        database.message(),
        "value-only quest lacks exact same-transaction source"
    );
    tx.rollback().await?;
    eprintln!("POSTGRES_STATELESS_ORPHAN_REFUSED sqlstate=23514 runtime_role=true");
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    name: &str,
) -> TestResult {
    let installed: bool = sqlx::query_scalar(
        "SELECT count(*)=17 FROM _sqlx_migrations WHERE version IN (999,1000,1001,1002,1003,1004,1005,1006,1200,1201,1202,1203,1204,1205,1294,1295,1296) AND success",
    )
    .fetch_one(&h.pool)
    .await?;
    assert!(
        installed,
        "the actual PG schema must include the stateless paid receipt proof"
    );
    eprintln!(
        "POSTGRES_MIGRATION_REGISTRY qengine8=true qbase6=true lane_paid3=true migration1294=true migration1295=true migration1296=true"
    );
    unpaired_stateless_receipt_is_rejected(h).await?;
    let [npc] = super::classic_quests::install(owner, admitted, ["oteryn:npc.elathriel"]).await?;
    let mut selected = admitted
        .continuity
        .selected_capabilities
        .as_slice()
        .to_vec();
    selected.extend([3, 6]);
    selected.sort_unstable();
    selected.dedup();
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            owner.offered_capabilities(),
            &selected,
        )
        .ok_or("NPC capabilities")?;
    if !name.ends_with("coins") {
        sqlx::query(
            "INSERT INTO game_type2_audit_activation VALUES(1,now()) ON CONFLICT DO NOTHING",
        )
        .execute(&h.pool)
        .await?;
    }
    let before = counts(h).await?;
    assert_eq!(before, (0, 0));
    let (result, _, _) = wire_grant(owner, admitted, npc, &["hi", "key", "yes"]).await?;
    assert_eq!(result, NpcIntentDisposition::Rejected, "insufficient funds");
    assert_eq!(counts(h).await?, before);
    let offering = Offering {
        position: Position {
            x: 33293,
            y: 31677,
            floor: -15,
        },
        item: "oteryn:item.tibia.i3035".into(),
        count: 100,
    };
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "fence")?
        .ok_or("fence")?;
    if name.ends_with("coins") {
        super::quest_world_callbacks::input_item(owner, fence.item_fence(), &offering, 800, true)
            .await?;
    } else {
        bank(h, 10000).await?;
    }
    if name.ends_with("full") {
        let native = owner
            .active_generation
            .and_then(|g| g.native_gameplay())
            .ok_or("native")?;
        let facts = crate::gameplay_transport::item_ref_admission::generation_item_facts(
            native,
            &crate::durability::item_mint::TypedDefinitionRef {
                family: "Item".into(),
                production_key: "oteryn:item.tibia.i2854".into(),
                revision_ref: "definition-r1".into(),
            },
        )
        .map_err(debug)?;
        for n in 0..facts.container_capacity.ok_or("backpack capacity")? {
            super::quest_world_callbacks::input_item(
                owner,
                fence.item_fence(),
                &Offering {
                    item: "oteryn:item.tibia.i2854".into(),
                    count: 1,
                    ..offering.clone()
                },
                810 + u64::from(n),
                true,
            )
            .await?;
        }
        let (result, _, _) = wire_grant(owner, admitted, npc, &["hi", "key", "yes"]).await?;
        assert_eq!(result, NpcIntentDisposition::Rejected, "full backpack");
        assert_eq!(counts(h).await?, before);
        let balance: i64 = sqlx::query_scalar("SELECT balance FROM game_account_bank_balances")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(balance, 10000);
    } else {
        // An affirmative outside topic1, or text that merely contains it,
        // cannot authorize a charge or a key grant.
        for words in [&["hi", "yes"][..], &["hi", "key", "yes please"][..]] {
            let _ = wire_grant(owner, admitted, npc, words).await?;
            assert_eq!(
                counts(h).await?,
                before,
                "only exact yes at topic1 can charge"
            );
        }
        // Exact source cancellation must not consume value or grant a key.
        let (result, _, _) = wire_grant(owner, admitted, npc, &["hi", "key", "no"]).await?;
        assert_eq!(result, NpcIntentDisposition::Ok);
        assert_eq!(counts(h).await?, before);
        for index in 0..2 {
            let (result, _, command) =
                wire_grant(owner, admitted, npc, &["hi", "key", "yes"]).await?;
            assert_eq!(result, NpcIntentDisposition::Ok);
            let expected = i64::from(index + 1);
            assert_eq!(counts(h).await?, (expected, expected));
            let keys:Vec<(String,i64,i32)>=sqlx::query_as("SELECT i.definition_production_key,i.quantity,k.key_number FROM game_item_keys k JOIN game_item_instances i USING(item_instance_id) ORDER BY i.item_instance_id").fetch_all(&h.pool).await?;
            assert_eq!(
                keys,
                vec![("oteryn:item.tibia.i2970".into(), 1, 3012); usize::try_from(expected)?]
            );
            let payload =
                oteryn_protocol_oteryn::npc_service::encode_npc_talk_intent(&NpcTalkIntent {
                    npc_actor: npc,
                    text: "yes".into(),
                })
                .map_err(debug)?;
            let reply =
                super::quest_world_callbacks::wire(owner, admitted, command, 7, payload).await?;
            assert_eq!(
                oteryn_protocol_oteryn::npc_service::decode_npc_intent_result(&reply)
                    .map_err(debug)?,
                NpcIntentDisposition::Ok
            );
            assert_eq!(counts(h).await?, (expected, expected), "literal replay");
        }
        let paid:(i64,i64,i64)=sqlx::query_as("SELECT count(*),sum(fee_gold_units)::bigint,sum(bank_debit_gold_units)::bigint FROM game_item_fee_burns WHERE cause_kind=18").fetch_one(&h.pool).await?;
        assert_eq!(
            paid,
            (2, 10000, if name.ends_with("bank") { 10000 } else { 0 })
        );
    }
    Ok(())
}
