/// A session joined without an ACTOR_VITALS snapshot (`vitals_revision` 0) has no base to
/// delta from: a committed cast answers its result and the session stays connected.
#[test]
fn spell_cast_without_a_joined_vitals_snapshot_answers_and_stays_connected()
-> Result<(), Box<dyn Error>> {
    use super::super::actor_spell::{encode_spell_cast_intent, tests as spell};
    run(async {
        let (runtime, actor, session) = spell::runtime_with_player(0x72);
        let mut states = super::super::actor_spell::ChannelSpellStates::default();
        states
            .initialize(
                &runtime,
                actor,
                session,
                spell::FACTS,
                (0, 0),
                oteryn_simulation_determinism::SemanticTimeMicros::from_micros(0),
            )
            .ok_or("initialize")?;
        let authority = SpellAuthority {
            runtime,
            states: RefCell::new(states),
            book: crate::spell::cast::v1_spell_book()?,
            casts: Cell::new(0),
            hide_vitals: true,
        };
        let exura = encode_spell_cast_intent(&spell::exura());
        let admitted = AdmittedSession {
            game_session_id: session,
            world_id: WorldId::decode(&uuid_v7(0x60))?,
            channel_id: ChannelId::decode(&uuid_v7(0x61))?,
            runtime_actor: Some(actor),
            first_entry: FirstEntryOutcome::Positioned,
            controller: None,
            continuity: SessionContinuity::FRESH,
            item_fence: None,
        };
        let (end, frames) = drive_session(
            &authority,
            admitted,
            &[cast_command(1, &exura), cast_command(2, &exura)],
        )
        .await?;
        let mut expected: Vec<Vec<u8>> = encode_single_chunk_snapshot(
            ADMITTED_GENERATION,
            1,
            0,
            &[
                DomainSnapshot {
                    domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                    revision: 1,
                    snapshot_type: SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
                    payload: &encode_world_spatial(&at(0)),
                },
                empty_overlay_snapshot(),
            ],
        )?
        .into();
        expected.push(encode_command_result(
            ADMITTED_GENERATION,
            1,
            1,
            CommandStatus::Accepted,
            &encode_spell_cast_result(SpellCastDisposition::Cast),
        )?);
        // The session survived the paid cast: the next command is still answered.
        expected.push(encode_command_result(
            ADMITTED_GENERATION,
            2,
            2,
            CommandStatus::Accepted,
            &encode_spell_cast_result(SpellCastDisposition::CoolingDown),
        )?);
        assert_eq!(frames, expected);
        // Only the peer's close ended the session; no vitals base was ever recorded.
        let ConnectionEnd::AdmittedThenDisconnected(ended) = end else {
            return Err("ended".into());
        };
        assert_eq!(ended.continuity.vitals_revision, 0);
        assert_eq!(authority.casts.get(), 2);
        Ok(())
    })
}

/// Spell cast §4 over the real owner and the real serve loop: after a committed cast, a
/// same-GameSession resume (a second `serve_admitted` over the same owner state, with the
/// initialization the first-entry step repeats) re-snapshots the PAID vitals, and the
/// cooldown is still running: a second cast pays nothing.
#[test]
fn spell_same_session_resume_snapshots_the_paid_vitals_and_keeps_the_cooldown()
-> Result<(), Box<dyn Error>> {
    use super::super::actor_spell::{encode_spell_cast_intent, tests as spell};
    run(async {
        let (runtime, actor, session) = spell::runtime_with_player(0x71);
        let mut states = super::super::actor_spell::ChannelSpellStates::default();
        states
            .initialize(
                &runtime,
                actor,
                session,
                spell::FACTS,
                (0, 0),
                oteryn_simulation_determinism::SemanticTimeMicros::from_micros(0),
            )
            .ok_or("initialize")?;
        spell::wound(&mut states, actor, session, 100);
        let authority = SpellAuthority {
            runtime,
            states: RefCell::new(states),
            book: crate::spell::cast::v1_spell_book()?,
            casts: Cell::new(0),
            hide_vitals: false,
        };
        let exura = encode_spell_cast_intent(&spell::exura());
        let admitted = |continuity| -> Result<AdmittedSession, Box<dyn Error>> {
            Ok(AdmittedSession {
                game_session_id: session,
                world_id: WorldId::decode(&uuid_v7(0x60))?,
                channel_id: ChannelId::decode(&uuid_v7(0x61))?,
                runtime_actor: Some(actor),
                first_entry: FirstEntryOutcome::Positioned,
                controller: None,
                continuity,
                item_fence: None,
            })
        };
        drive_session(
            &authority,
            admitted(SessionContinuity::FRESH)?,
            &[cast_command(1, &exura)],
        )
        .await?;
        let (revision, paid) = authority
            .observe_vitals(actor, session)
            .await
            .ok_or("vitals")?;
        assert_eq!(revision, 2);
        assert_eq!((paid.mana, paid.max_mana), (70, 90));
        assert!(paid.health > 100 && paid.health <= paid.max_health);
        // The reconnect's owner step: initialization of a present actor keeps its state.
        authority
            .states
            .borrow_mut()
            .initialize(
                &authority.runtime,
                actor,
                session,
                spell::FACTS,
                (0, 0),
                oteryn_simulation_determinism::SemanticTimeMicros::from_micros(0),
            )
            .ok_or("reinitialize")?;
        let resumed = SessionContinuity {
            next_command_id: 2,
            server_sequence: 3,
            ..SessionContinuity::FRESH
        };
        let (_end, frames) =
            drive_session(&authority, admitted(resumed)?, &[cast_command(2, &exura)]).await?;
        let mut expected: Vec<Vec<u8>> = encode_single_chunk_snapshot(
            ADMITTED_GENERATION,
            1,
            3,
            &[
                DomainSnapshot {
                    domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                    revision: 1,
                    snapshot_type: SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
                    payload: &encode_world_spatial(&at(0)),
                },
                empty_overlay_snapshot(),
                DomainSnapshot {
                    domain_id: STATE_DOMAIN_ACTOR_VITALS,
                    revision: 2,
                    snapshot_type: SNAPSHOT_TYPE_ACTOR_VITALS_V1,
                    payload: &encode_actor_vitals(&paid).map_err(|_| "vitals")?,
                },
            ],
        )?
        .into();
        // Still cooling down: answered, accepted, and no delta (nothing paid).
        expected.push(encode_command_result(
            ADMITTED_GENERATION,
            4,
            2,
            CommandStatus::Accepted,
            &encode_spell_cast_result(SpellCastDisposition::CoolingDown),
        )?);
        assert_eq!(frames, expected);
        assert_eq!(
            authority.observe_vitals(actor, session).await,
            Some((2, paid))
        );
        Ok(())
    })
}
