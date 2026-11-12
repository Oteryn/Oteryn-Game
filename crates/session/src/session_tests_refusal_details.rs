// Production session decode, including capability negotiation and the next-result reset.
#[test]
fn refusal_details_survive_step_and_use_exchange() -> Result<(), BoxError> {
    std::thread::Builder::new().stack_size(64 << 20).spawn(||block_on(async {
        let selected: &[u32] = &[66];
        let (client, mut server)=tokio::io::duplex(65536);
        let peer=tokio::spawn(async move {
            join_peer(&mut server,selected,selected,&[]).await?;
            for (sequence, use_result) in [(41,false),(42,true),(43,false)] {
                let bytes=read_frame(&mut server).await?;
                let envelope=decode_wire_envelope(&bytes)?;
                let command=envelope.client_command(1)?;
                let mut payload=if use_result {world_object::encode_use_result(UseDisposition::TooFar).map_err(|e|format!("{e:?}"))?} else {encode_step_result(StepDisposition::Blocked)};
                if sequence!=43 {oteryn_protocol_oteryn::refusal_detail::append_refusal_detail(&mut payload,selected,Some(oteryn_protocol_oteryn::refusal_detail::RefusalDetail::TooFarAway));}
                write_frame(&mut server,&encode_command_result(1,sequence,command.command_id,CommandStatus::Accepted,&payload)?).await?;
            }
            Ok::<(),BoxError>(())
        });
        let mut session=Session::admit(client,admission_with(selected)?).await?;
        session.step(StepDirection::North).await?;
        assert_eq!(session.last_refusal_detail(),Some(oteryn_protocol_oteryn::refusal_detail::RefusalDetail::TooFarAway));
        session.use_object(b"door",2).await?;
        assert_eq!(session.last_refusal_detail().map(|d|d.reference_message()),Some("Too far away."));
        session.step(StepDirection::North).await?;
        assert_eq!(session.last_refusal_detail(),None);
        peer.await??;
        Ok(())
    }))?.join().map_err(|_|"refusal-details test panicked")??
}

#[test]
fn refusal_suffix_requires_selection_and_rejects_unknown_or_success_detail() -> Result<(), BoxError>
{
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            block_on(async {
                let cases: [(&'static [u32], bool, u8); 3] =
                    [(&[], false, 2), (&[66], false, 9), (&[66], true, 2)];
                for (selected, moved, detail) in cases {
                    let (client, mut server) = tokio::io::duplex(65536);
                    let peer = tokio::spawn(async move {
                        join_peer(&mut server, selected, selected, &[]).await?;
                        let wire = read_frame(&mut server).await?;
                        let envelope = decode_wire_envelope(&wire)?;
                        let command = envelope.client_command(1)?;
                        let mut payload = encode_step_result(if moved {
                            StepDisposition::Moved
                        } else {
                            StepDisposition::Blocked
                        });
                        payload.extend([120, detail]);
                        write_frame(
                            &mut server,
                            &encode_command_result(
                                1,
                                41,
                                command.command_id,
                                CommandStatus::Accepted,
                                &payload,
                            )?,
                        )
                        .await?;
                        Ok::<(), BoxError>(())
                    });
                    let mut session = Session::admit(client, admission_with(selected)?).await?;
                    let result = session.step(StepDirection::North).await;
                    if selected.is_empty() {
                        assert!(matches!(result, Err(SessionError::WorldSpatial(_))));
                    } else if detail == 9 {
                        assert!(matches!(result, Err(SessionError::ItemView(_))));
                    } else {
                        assert!(matches!(result, Err(SessionError::SessionUnusable)));
                    }
                    assert_eq!(session.last_refusal_detail(), None);
                    peer.await??;
                }
                Ok(())
            })
        })?
        .join()
        .map_err(|_| "refusal suffix test panicked")??
}
