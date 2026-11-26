/// A fixture authority for `ACCOUNT_ACHIEVEMENTS_QUERY` dispatch: every call is recorded and
/// answered with one canned reply.
struct AchievementsAuthority {
    calls: RefCell<Vec<AccountAchievementsRequest>>,
    reply: AccountAchievementsReply,
    commands: Option<crate::gameplay_transport::session_ingress::SessionCommands>,
}

impl FreshAdmissionAuthority for AchievementsAuthority {
    async fn admit(
        &self,
        _attempt: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        Err(AdmissionRefusal::Rejected)
    }

    async fn observe(&self, _actor: ExactActorRef) -> Option<WorldSpatialObservation> {
        Some(at(0))
    }

    fn session_commands(
        &self,
    ) -> Option<&crate::gameplay_transport::session_ingress::SessionCommands> {
        self.commands.as_ref()
    }

    async fn logout(&self, _: AdmittedSession) -> LogoutReply {
        LogoutReply::Result(LogoutResult::Accepted)
    }

    async fn account_achievements(
        &self,
        request: AccountAchievementsRequest,
    ) -> AccountAchievementsReply {
        self.calls.borrow_mut().push(request);
        self.reply.clone()
    }
}

fn achievements_command(id: u64, query: &[u8]) -> Vec<u8> {
    let mut payload = Vec::new();
    scalar(&mut payload, 1, id);
    scalar(
        &mut payload,
        2,
        u64::from(COMMAND_TYPE_ACCOUNT_ACHIEVEMENTS_QUERY),
    );
    bytes(&mut payload, 4, query);
    envelope(7, ADMITTED_GENERATION, &payload)
}

/// Display contract §4: the query reads the admitted controller's account, never one from the
/// payload; a malformed query or a session without a controller is REJECTED without a read;
/// an over-bound reply is REJECTED with `PAYLOAD_LIMIT_EXCEEDED` and an integrity fault with
/// `ACCOUNT_DATA_INTEGRITY` (owner decision 2026-09-30), each with no rows.
#[test]
fn account_achievements_query_reads_only_the_controller_account() -> Result<(), Box<dyn Error>> {
    const ACCOUNT: [u8; 16] = uuid_v7(0x55);
    let request = |page, command_id| AccountAchievementsRequest {
        completion: oteryn_protocol_oteryn::account_achievements::AchievementCompletion::Completed,
        account_id: ACCOUNT,
        page,
        game_session_id: SESSION,
        command_id,
    };
    run(async {
        let world_id = WorldId::decode(&WORLD)?;
        let channel_id = ChannelId::decode(&CHANNEL)?;
        let session = |controller| AdmittedSession {
            game_session_id: GameSessionId::decode(&SESSION).expect("session"),
            world_id,
            channel_id,
            runtime_actor: Some(ExactActorRef::transport_fixture(world_id, channel_id)),
            first_entry: FirstEntryOutcome::Positioned,
            controller,
            continuity: SessionContinuity::FRESH,
            item_fence: None,
        };
        let controller = Some(ControllerBinding {
            transport: AuthenticatedTransportRefV1::decode(&[0x5a; 16])?,
            account_id: ACCOUNT,
        });
        let authority = AchievementsAuthority {
            calls: RefCell::new(Vec::new()),
            reply: AccountAchievementsReply::Page(vec![0x08, 0x02]),
            commands: None,
        };
        let (_end, frames) = drive_session(
            &authority,
            session(controller),
            &[
                // Page 3; a payload has no field for an account.
                achievements_command(1, &[0x08, 0x03]),
                // An unknown field (as if naming an account) does not decode.
                achievements_command(2, &[0x08, 0x00, 0x12, 0x01, 0x07]),
            ],
        )
        .await?;
        let mut expected = baseline();
        expected.extend([
            encode_command_result(
                ADMITTED_GENERATION,
                1,
                1,
                CommandStatus::Accepted,
                &[0x08, 0x02],
            )?,
            encode_command_result(ADMITTED_GENERATION, 2, 2, CommandStatus::Rejected, &[])?,
        ]);
        assert_eq!(frames, expected);
        assert_eq!(*authority.calls.borrow(), [request(3, 1)]);

        // No controller binding: no account to read.
        let (_end, frames) =
            drive_session(&authority, session(None), &[achievements_command(1, &[])]).await?;
        assert_eq!(
            frames.last(),
            Some(&encode_command_result(
                ADMITTED_GENERATION,
                1,
                1,
                CommandStatus::Rejected,
                &[]
            )?)
        );
        assert_eq!(authority.calls.borrow().len(), 1);

        for error in [
            FoundationProtocolError::PayloadLimitExceeded,
            FoundationProtocolError::AccountDataIntegrity,
        ] {
            let failing = AchievementsAuthority {
                calls: RefCell::new(Vec::new()),
                reply: AccountAchievementsReply::Terminal(error),
                commands: None,
            };
            let (_end, frames) = drive_session(
                &failing,
                session(controller),
                &[achievements_command(1, &[])],
            )
            .await?;
            let terminal = encode_command_error_result(ADMITTED_GENERATION, 1, 1, error)?;
            assert_ne!(
                terminal,
                encode_command_result(ADMITTED_GENERATION, 1, 1, CommandStatus::Rejected, &[])?
            );
            assert_eq!(frames.last(), Some(&terminal), "{error:?}");
            assert_eq!(*failing.calls.borrow(), [request(0, 1)]);
        }
        Ok(())
    })
}

#[test]
fn malformed_achievement_query_allows_first_logout() -> Result<(), Box<dyn Error>> {
    use crate::gameplay_transport::capabilities::PRODUCTION_OFFERED_CAPABILITIES;
    use oteryn_protocol_oteryn::logout::{
        CAPABILITY_LOGOUT_V1, COMMAND_TYPE_LOGOUT_INTENT, encode_logout_intent, encode_logout_result,
    };
    run(async {
        let mut admitted = positioned()?;
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(PRODUCTION_OFFERED_CAPABILITIES, &[CAPABILITY_LOGOUT_V1])
                .ok_or("logout capability")?;
        admitted.controller = Some(ControllerBinding {
            transport: AuthenticatedTransportRefV1::decode(&[0x5a; 16])?,
            account_id: uuid_v7(0x55),
        });
        let authority = AchievementsAuthority {
            calls: RefCell::new(Vec::new()),
            reply: AccountAchievementsReply::Rejected,
            commands: Some(Default::default()),
        };
        let logout = oteryn_protocol_oteryn::encode_client_command(
            ADMITTED_GENERATION,
            &oteryn_protocol_oteryn::ClientCommandValue {
                command_id: 2,
                command_type: COMMAND_TYPE_LOGOUT_INTENT,
                payload: &encode_logout_intent(),
            },
        )?;
        let (end, frames) = drive_session(
            &authority,
            admitted,
            &[achievements_command(1, &[0x08]), logout],
        )
        .await?;
        assert!(matches!(end, ConnectionEnd::AdmittedThenLoggedOut(_)), "{end:?}");
        let mut expected = baseline();
        expected.extend([
            encode_command_result(ADMITTED_GENERATION, 1, 1, CommandStatus::Rejected, &[])?,
            encode_command_result(
                ADMITTED_GENERATION,
                2,
                2,
                CommandStatus::Accepted,
                &encode_logout_result(LogoutResult::Accepted)
                    .map_err(|error| format!("{error:?}"))?,
            )?,
        ]);
        assert_eq!(frames, expected);
        assert!(authority.calls.borrow().is_empty());
        Ok(())
    })
}

/// A fixture authority for `HIGHSCORES_QUERY_V1` dispatch: every call is recorded and
/// answered with one canned reply.
struct HighscoresAuthority {
    calls: RefCell<Vec<(WorldId, GameSessionId, HighscoresQuery)>>,
    reply: HighscoresReply,
}

impl FreshAdmissionAuthority for HighscoresAuthority {
    async fn admit(
        &self,
        _attempt: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        Err(AdmissionRefusal::Rejected)
    }

    async fn observe(&self, _actor: ExactActorRef) -> Option<WorldSpatialObservation> {
        Some(at(0))
    }

    async fn highscores(
        &self,
        world: WorldId,
        session: GameSessionId,
        query: HighscoresQuery,
    ) -> HighscoresReply {
        self.calls.borrow_mut().push((world, session, query));
        self.reply.clone()
    }
}

fn highscores_command(id: u64, query: &[u8]) -> Vec<u8> {
    let mut payload = Vec::new();
    scalar(&mut payload, 1, id);
    scalar(&mut payload, 2, u64::from(COMMAND_TYPE_HIGHSCORES_QUERY_V1));
    bytes(&mut payload, 4, query);
    envelope(7, ADMITTED_GENERATION, &payload)
}

/// HIGHSCORES0 §6: command type 14 dispatches only under capability 9, with the admitted
/// session's World and GameSession; a malformed query is REJECTED without a read, and the
/// snapshot outcomes are FND-02 error results 1200 and 1201 without a payload.
#[test]
fn highscores_query_reads_only_the_admitted_session_under_capability_9()
-> Result<(), Box<dyn Error>> {
    let selected = |capabilities: &[u32]| -> Result<AdmittedSession, Box<dyn Error>> {
        let mut admitted = positioned()?;
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(PRODUCTION_OFFERED_CAPABILITIES, capabilities)
                .ok_or("bounded")?;
        Ok(admitted)
    };
    let capability = oteryn_protocol_oteryn::highscores::CAPABILITY_HIGHSCORES_V1;
    run(async {
        let admitted = selected(&[capability])?;
        let (world, session) = (admitted.world_id, admitted.game_session_id);
        let authority = HighscoresAuthority {
            calls: RefCell::new(Vec::new()),
            reply: HighscoresReply::Result(vec![0x08, 0x01]),
        };
        let (_end, frames) = drive_session(
            &authority,
            admitted,
            &[
                // Category 1, all vocations, page 1, the current snapshot.
                highscores_command(1, &[0x08, 0x01, 0x18, 0x01]),
                // Page 0 does not decode.
                highscores_command(2, &[0x08, 0x01]),
            ],
        )
        .await?;
        let mut expected = baseline();
        expected.extend([
            encode_command_result(
                ADMITTED_GENERATION,
                1,
                1,
                CommandStatus::Accepted,
                &[0x08, 0x01],
            )?,
            encode_command_result(ADMITTED_GENERATION, 2, 2, CommandStatus::Rejected, &[])?,
        ]);
        assert_eq!(frames, expected);
        let query = HighscoresQuery {
            world: None,
            details: false,
            category: 1,
            vocation: 0,
            page: 1,
            snapshot_id: 0,
        };
        assert_eq!(*authority.calls.borrow(), [(world, session, query)]);

        // Not selected: the command is not dispatched.
        let (_end, frames) = drive_session(
            &authority,
            selected(&[])?,
            &[highscores_command(1, &[0x08, 0x01, 0x18, 0x01])],
        )
        .await?;
        assert_ne!(
            frames.last(),
            Some(&encode_command_result(
                ADMITTED_GENERATION,
                1,
                1,
                CommandStatus::Accepted,
                &[0x08, 0x01]
            )?)
        );
        assert_eq!(authority.calls.borrow().len(), 1);

        for error in [
            FoundationProtocolError::NotSupported,
            FoundationProtocolError::SnapshotExpired,
        ] {
            let failing = HighscoresAuthority {
                calls: RefCell::new(Vec::new()),
                reply: HighscoresReply::Terminal(error),
            };
            let (_end, frames) = drive_session(
                &failing,
                selected(&[capability])?,
                &[highscores_command(1, &[0x08, 0x0c, 0x18, 0x01])],
            )
            .await?;
            let terminal = encode_command_error_result(ADMITTED_GENERATION, 1, 1, error)?;
            assert_eq!(frames.last(), Some(&terminal), "{error:?}");
            assert_eq!(failing.calls.borrow().len(), 1);
        }
        Ok(())
    })
}

/// T143 keeps admitted authority separate from the requested public read World.
#[test]
fn cross_world_highscores_route_forwards_selection_without_replacing_session_authority()
-> Result<(), Box<dyn Error>> {
    run(async {
        let mut admitted = positioned()?;
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(PRODUCTION_OFFERED_CAPABILITIES, &[9])
                .ok_or("selected")?;
        let query = HighscoresQuery {
            world: Some(oteryn_protocol_oteryn::WorldId::decode(&uuid_v7(90))?),
            details: true,
            category: 2,
            vocation: 1,
            page: 2,
            snapshot_id: 300,
        };
        let authority = HighscoresAuthority {
            calls: RefCell::new(Vec::new()),
            reply: HighscoresReply::Result(vec![0x08, 1]),
        };
        let payload = oteryn_protocol_oteryn::highscores::encode_highscores_query(query)
            .map_err(|e| format!("{e:?}"))?;
        drive_session(&authority, admitted, &[highscores_command(1, &payload)]).await?;
        assert_eq!(
            *authority.calls.borrow(),
            [(admitted.world_id, admitted.game_session_id, query)]
        );
        assert_ne!(
            query.world.map(|w| *w.as_bytes()),
            Some(*admitted.world_id.as_bytes())
        );
        Ok(())
    })
}
