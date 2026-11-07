/// The session runs over any byte stream: admission, join snapshot, and one `step` with its
/// sequenced delta, with no TLS or TCP anywhere.
#[test]
fn admits_joins_and_steps_over_an_in_memory_stream() -> Result<(), BoxError> {
    block_on(async {
        let (client, server) = tokio::io::duplex(64 * 1024);
        let server = tokio::spawn(scripted_server(server));
        let mut session = Session::admit(client, admission()?).await?;
        assert_eq!(session.next_command_id(), 7);
        assert_eq!(session.last_server_sequence(), 40);
        assert_eq!(session.join_snapshot().world_spatial_revision, 5);
        assert_eq!(session.join_snapshot().world_object_overlay_revision, 2);
        let outcome = session.step(StepDirection::East).await?;
        server.await??;
        assert_eq!(outcome.disposition, StepDisposition::Moved);
        assert_eq!(outcome.result_server_sequence, 41);
        let delta = outcome.world_spatial_delta.ok_or("Moved carries a delta")?;
        assert_eq!((delta.server_sequence, delta.new_revision), (42, 6));
        assert_eq!(session.world_spatial().actor_position.x, 1);
        assert_eq!(session.last_server_sequence(), 42);
        Ok(())
    })?
}

fn vitals(health: u32, mana: u32) -> ActorVitals {
    ActorVitals {
        health,
        max_health: 150,
        mana,
        max_mana: 55,
        soul: 100,
        harmony: 0,
        serene: false,
    }
}

/// Admits, sends a join snapshot that carries `ACTOR_VITALS` (revision 3) and then answers two
/// casts: `Cast` with its vitals delta, then `Rejected` (the closed server gate) with none.
async fn send_vitals_join(stream: &mut DuplexStream) -> Result<(), BoxError> {
    read_frame(&mut *stream).await?;
    write_frame(
        &mut *stream,
        &encode_server_accepted(&ServerAcceptedValue {
            game_session_id: GameSessionId::decode(&uuid_v7(1))?,
            world_id: WorldId::decode(&uuid_v7(2))?,
            channel_id: ChannelId::decode(&uuid_v7(3))?,
            connection_generation: 1,
            current_server_sequence: 0,
            next_command_id: 7,
            schema_revision: 1,
            selected_capabilities: &[],
        })?,
    )
    .await?;
    let overlay = encode_world_object_overlay_snapshot(&[])
        .map_err(|error| format!("overlay snapshot: {error:?}"))?;
    let vitals_payload = actor_spell::encode_actor_vitals(&vitals(150, 55))
        .map_err(|error| format!("vitals: {error:?}"))?;
    for frame in encode_single_chunk_snapshot(
        1,
        1,
        40,
        &[
            DomainSnapshot {
                domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                revision: 5,
                snapshot_type: SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
                payload: &spatial(0),
            },
            DomainSnapshot {
                domain_id: STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
                revision: 2,
                snapshot_type: SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1,
                payload: &overlay,
            },
            DomainSnapshot {
                domain_id: actor_spell::STATE_DOMAIN_ACTOR_VITALS,
                revision: 3,
                snapshot_type: actor_spell::SNAPSHOT_TYPE_ACTOR_VITALS_V1,
                payload: &vitals_payload,
            },
        ],
    )? {
        write_frame(&mut *stream, &frame).await?;
    }
    Ok(())
}

async fn cast_server(mut stream: DuplexStream) -> Result<(), BoxError> {
    send_vitals_join(&mut stream).await?;
    let command = read_frame(&mut stream).await?;
    let command = decode_wire_envelope(&command)?.client_command(1)?;
    assert_eq!(
        (command.command_id, command.command_type),
        (7, actor_spell::COMMAND_TYPE_WORLD_ACTOR_SPELL_CAST_INTENT)
    );
    assert_eq!(
        actor_spell::decode_spell_cast_intent(command.payload),
        Ok(SpellCastIntent {
            spell: NonZeroU32::new(2).ok_or("nonzero")?,
            target: SpellTarget::AttackTarget,
            aim_at_target: true,
        })
    );
    write_frame(
        &mut stream,
        &encode_command_result(
            1,
            41,
            7,
            CommandStatus::Accepted,
            &actor_spell::encode_spell_cast_result(SpellCastDisposition::Cast),
        )?,
    )
    .await?;
    write_frame(
        &mut stream,
        &encode_state_delta(
            1,
            42,
            actor_spell::STATE_DOMAIN_ACTOR_VITALS,
            3,
            4,
            actor_spell::DELTA_TYPE_ACTOR_VITALS_V1,
            &actor_spell::encode_actor_vitals(&vitals(150, 30))
                .map_err(|error| format!("vitals: {error:?}"))?,
        )?,
    )
    .await?;
    read_frame(&mut stream).await?;
    write_frame(
        &mut stream,
        &encode_command_result(
            1,
            43,
            8,
            CommandStatus::Rejected,
            &actor_spell::encode_spell_cast_result(SpellCastDisposition::Rejected),
        )?,
    )
    .await?;
    Ok(())
}

#[test]
fn a_cast_applies_the_vitals_delta_and_a_rejected_cast_changes_nothing() -> Result<(), BoxError> {
    block_on(async {
        let (client, server) = tokio::io::duplex(64 * 1024);
        let server = tokio::spawn(cast_server(server));
        let mut session = Session::admit(client, admission()?).await?;
        assert_eq!(session.actor_vitals(), Some(&vitals(150, 55)));
        let spell = NonZeroU32::new(2).ok_or("nonzero")?;
        let cast = session
            .cast_spell(spell, SpellTarget::AttackTarget, true)
            .await?;
        assert_eq!(cast.disposition, SpellCastDisposition::Cast);
        let delta = cast.actor_vitals_delta.ok_or("Cast carries a delta")?;
        assert_eq!((delta.server_sequence, delta.base_revision), (42, 3));
        assert_eq!(session.actor_vitals(), Some(&vitals(150, 30)));
        let rejected = session.cast_spell(spell, SpellTarget::None, false).await?;
        server.await??;
        assert_eq!(rejected.status, CommandStatus::Rejected);
        assert_eq!(rejected.disposition, SpellCastDisposition::Rejected);
        assert_eq!(rejected.actor_vitals_delta, None);
        assert_eq!(session.actor_vitals(), Some(&vitals(150, 30)));
        assert_eq!(session.last_server_sequence(), 43);
        // The session stays usable after a rejection.
        assert_eq!(session.next_command_id(), 9);
        Ok(())
    })?
}
