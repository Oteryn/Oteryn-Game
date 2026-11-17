use super::*;
use oteryn_protocol_oteryn::server_modal::{Answer, Reply};

#[test]
fn server_modal_hireling_store_selection_dispatches_cancel_rename_and_sex_once() -> TestResult {
    if !enabled() {
        return Ok(());
    }
    super::super::kill_reward_e2e::scenario_thread::run("modal-hireling-store", || {
        super::scenario(Case::Modal)
    })
}

pub(super) async fn scenario(
    harness: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    ledger: &Arc<SyncMutex<Ledger>>,
) -> TestResult {
    // Store-only fixtures do not bind combat leases; modal admission proves the
    // same exact actor/character lease as the other current-player projections.
    let fence = admitted.item_fence.ok_or("modal fence")?;
    {
        let mut runtime = owner.runtime.lock().await;
        runtime
            .bind_attacker_lease(
                admitted.runtime_actor.ok_or("modal actor")?,
                admitted.game_session_id,
                crate::foundation::CharacterLease::new(
                    crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                        .map_err(debug)?,
                    fence.character_lease_generation,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        assert!(
            crate::gameplay_transport::battle_instinct_status::actor_is_current(
                &runtime, *admitted
            )
        );
    }
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let mut next = admitted.continuity.next_command_id;
    let serving = serve_admitted(&mut server, *admitted, owner, IDLE_LIVENESS);
    let playing = async move {
        for (seed, offer, price, name) in [(70, 25438, 250, Some("Morgan")), (71, 25437, 120, None)]
        {
            let custom = name.map(|name| wire::StoreHirelingCustomization {
                hireling_id: None,
                name: Some(name.into()),
                sex: None,
            });
            let (_, payload) = command(seed, offer, price, custom)?;
            let result = exchange(&mut client, &mut next, (6196, payload.clone())).await?;
            assert_eq!(result.0, CommandStatus::Accepted);
            let Reply::Dialog(dialog) = Reply::decode(&result.1).map_err(debug)? else {
                return Err("selection dialog".into());
            };
            assert_eq!(dialog.title, "Choose a Hireling");
            assert_eq!(dialog.choices.len(), 1);
            let repeated = exchange(&mut client, &mut next, (6196, payload)).await?;
            assert_eq!(repeated.1, result.1);
            let queried = exchange(&mut client, &mut next, (736, Vec::new())).await?;
            assert_eq!(queried.1, result.1);
            assert!(dialog.choices[0].1.contains(if name.is_some() {
                "Alex Moon"
            } else {
                "Morgan"
            }));
            assert_eq!(
                ledger.lock().expect("ledger").debits,
                if name.is_some() { 1 } else { 2 }
            );
            let answer = Answer {
                dialog_id: dialog.id,
                button_id: 1,
                choice_id: Some(dialog.choices[0].0),
            };
            let invalid = Answer {
                choice_id: Some(99),
                ..answer
            };
            assert_eq!(
                exchange(
                    &mut client,
                    &mut next,
                    (737, invalid.encode().map_err(debug)?)
                )
                .await?
                .0,
                CommandStatus::Rejected
            );
            for _ in 0..2 {
                let result = exchange(
                    &mut client,
                    &mut next,
                    (737, answer.encode().map_err(debug)?),
                )
                .await?;
                assert_eq!(result.0, CommandStatus::Accepted);
                assert_eq!(
                    Reply::decode(&result.1).map_err(debug)?,
                    Reply::Answered(answer)
                );
            }
            let result = exchange(&mut client, &mut next, (736, Vec::new())).await?;
            let Reply::Dialog(receipt) = Reply::decode(&result.1).map_err(debug)? else {
                return Err("purchase result dialog".into());
            };
            assert!(receipt.message.contains("Accepted"));
            let close = Answer {
                dialog_id: receipt.id,
                button_id: 1,
                choice_id: None,
            };
            assert_eq!(
                exchange(
                    &mut client,
                    &mut next,
                    (737, close.encode().map_err(debug)?)
                )
                .await?
                .0,
                CommandStatus::Accepted
            );
        }
        let state: (String, i16) = sqlx::query_as("SELECT name,sex FROM game_house_hirelings")
            .fetch_one(&harness.pool)
            .await?;
        assert_eq!(state, ("Morgan".into(), 2));
        assert_eq!(ledger.lock().expect("ledger").debits, 3);
        let (_, payload) = command(72, 25437, 120, None)?;
        let result = exchange(&mut client, &mut next, (6196, payload)).await?;
        let Reply::Dialog(dialog) = Reply::decode(&result.1).map_err(debug)? else {
            return Err("cancel dialog".into());
        };
        let answer = Answer {
            dialog_id: dialog.id,
            button_id: 2,
            choice_id: None,
        };
        assert_eq!(
            exchange(
                &mut client,
                &mut next,
                (737, answer.encode().map_err(debug)?)
            )
            .await?
            .0,
            CommandStatus::Accepted
        );
        assert_eq!(ledger.lock().expect("ledger").debits, 3);
        let (_, payload) = command(73, 25437, 120, None)?;
        let result = exchange(&mut client, &mut next, (6196, payload)).await?;
        let Reply::Dialog(dialog) = Reply::decode(&result.1).map_err(debug)? else {
            return Err("retry dialog".into());
        };
        let answer = Answer {
            dialog_id: dialog.id,
            button_id: 1,
            choice_id: Some(dialog.choices[0].0),
        };
        ledger.lock().expect("ledger").lose_next_answer = true;
        assert_eq!(
            exchange(
                &mut client,
                &mut next,
                (737, answer.encode().map_err(debug)?)
            )
            .await?
            .0,
            CommandStatus::Rejected
        );
        let cancel = Answer {
            button_id: 2,
            choice_id: None,
            ..answer
        };
        assert_eq!(
            exchange(
                &mut client,
                &mut next,
                (737, cancel.encode().map_err(debug)?)
            )
            .await?
            .0,
            CommandStatus::Rejected
        );
        assert_eq!(
            exchange(
                &mut client,
                &mut next,
                (737, answer.encode().map_err(debug)?)
            )
            .await?
            .0,
            CommandStatus::Accepted
        );
        assert_eq!(ledger.lock().expect("ledger").debits, 4);
        let sex: i16 = sqlx::query_scalar("SELECT sex FROM game_house_hirelings")
            .fetch_one(&harness.pool)
            .await?;
        assert_eq!(sex, 1);
        client.shutdown().await?;
        Ok(())
    };
    let (_, result) = tokio::join!(serving, playing);
    result
}

async fn exchange(
    client: &mut tokio::io::DuplexStream,
    next: &mut u64,
    command: (u32, Vec<u8>),
) -> TestResult<(CommandStatus, Vec<u8>)> {
    let bytes = oteryn_protocol_oteryn::encode_client_command(
        1,
        &oteryn_protocol_oteryn::ClientCommandValue {
            command_id: *next,
            command_type: command.0,
            payload: &command.1,
        },
    )
    .map_err(debug)?;
    *next += 1;
    client
        .write_all(&u32::try_from(bytes.len())?.to_be_bytes())
        .await?;
    client.write_all(&bytes).await?;
    loop {
        let length = client.read_u32().await? as usize;
        let mut bytes = vec![0; length];
        client.read_exact(&mut bytes).await?;
        let envelope = decode_wire_envelope(&bytes).map_err(debug)?;
        if envelope.message_type() == MessageType::CommandResult {
            let result =
                oteryn_protocol_oteryn::decode_command_result(envelope.payload()).map_err(debug)?;
            return Ok((result.status, result.payload.to_vec()));
        }
    }
}
