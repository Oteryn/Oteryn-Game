//! M03 server-payload replay. The owning client paths are `Session::step`/`exchange_step` and
//! `Session::cast_spell`/`exchange_cast` in `session_commands.rs`; the floor payload reaches
//! `Session::apply_delta` in `domain_dispatch.rs`, which updates the state exposed by
//! `Session::world_spatial` in `session_access.rs`.
use super::*;

#[test]
fn real_floor_change_and_exhaust_refusal_replay_into_session_state() -> Result<(), BoxError> {
    block_on(async {
        let (
            before_payload,
            floor_payload,
            vitals_payload,
            step_payload,
            refusal_payload,
            expected,
        ) = if let Some(directory) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
            let directory = std::path::Path::new(&directory);
            (
                std::fs::read(directory.join("m03-spatial-before.bin"))?,
                std::fs::read(directory.join("m03-spatial-floor.bin"))?,
                std::fs::read(directory.join("m03-vitals-before.bin"))?,
                std::fs::read(directory.join("m03-step-result.bin"))?,
                std::fs::read(directory.join("m03-cast-refusal.bin"))?,
                std::fs::read_to_string(directory.join("m03-expected.txt"))?,
            )
        } else {
            let before = WireSpatialObservation {
                content_generation: CONTENT_GENERATION,
                actor_position: ActorPosition {
                    x: 32369,
                    y: 32238,
                    floor: 7,
                },
            };
            let floor = WireSpatialObservation {
                actor_position: ActorPosition {
                    x: 32370,
                    y: 32238,
                    floor: 6,
                },
                ..before
            };
            let vitals = ActorVitals {
                health: 150,
                mana: 55,
                ..ActorVitals::default()
            };
            (
                encode_world_spatial(&before),
                encode_world_spatial(&floor),
                actor_spell::encode_actor_vitals(&vitals)
                    .map_err(|error| format!("vitals fixture: {error:?}"))?,
                encode_step_result(StepDisposition::Moved),
                actor_spell::encode_spell_cast_result(SpellCastDisposition::CoolingDown),
                "32369 32238 7\n32370 32238 6\n55\n".to_owned(),
            )
        };
        let expected_values = expected
            .split_whitespace()
            .map(str::parse::<i32>)
            .collect::<Result<Vec<_>, _>>()?;
        let [
            before_x,
            before_y,
            before_floor,
            after_x,
            after_y,
            after_floor,
            mana,
        ] = expected_values.as_slice()
        else {
            return Err("invalid M03 expected capture".into());
        };

        let (client, mut server) = tokio::io::duplex(256 * 1024);
        let peer = tokio::spawn(async move {
            join_peer_with(
                &mut server,
                &[13],
                &[],
                &[],
                (SNAPSHOT_TYPE_WORLD_SPATIAL_V1, before_payload),
                &[(
                    actor_spell::STATE_DOMAIN_ACTOR_VITALS,
                    1,
                    actor_spell::SNAPSHOT_TYPE_ACTOR_VITALS_V1,
                    vitals_payload,
                )],
            )
            .await?;
            play_step(&mut server, Step::ReadCommand).await?;
            play_step(
                &mut server,
                Step::Send(encode_command_result(
                    1,
                    41,
                    7,
                    CommandStatus::Accepted,
                    &step_payload,
                )?),
            )
            .await?;
            play_step(
                &mut server,
                Step::Send(encode_state_delta(
                    1,
                    42,
                    STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                    5,
                    6,
                    DELTA_TYPE_WORLD_SPATIAL_V1,
                    &floor_payload,
                )?),
            )
            .await?;
            play_step(&mut server, Step::ReadCommand).await?;
            play_step(
                &mut server,
                Step::Send(encode_command_result(
                    1,
                    43,
                    8,
                    CommandStatus::Accepted,
                    &refusal_payload,
                )?),
            )
            .await?;
            let mut rest = Vec::new();
            let _ = server.read_to_end(&mut rest).await;
            Ok::<(), BoxError>(())
        });

        let mut session = Session::admit(client, admission()?).await?;
        assert_eq!(
            session.world_spatial().actor_position,
            ActorPosition {
                x: *before_x,
                y: *before_y,
                floor: i16::try_from(*before_floor)?,
            }
        );
        let moved = session.step(StepDirection::East).await?;
        assert_eq!(moved.disposition, StepDisposition::Moved);
        assert_eq!(
            session.world_spatial().actor_position,
            ActorPosition {
                x: *after_x,
                y: *after_y,
                floor: i16::try_from(*after_floor)?,
            }
        );
        let stable = *session.world_spatial();
        let spell = std::num::NonZeroU32::new(1).ok_or("nonzero spell")?;
        let refused = session.cast_spell(spell, SpellTarget::None, false).await?;
        assert_eq!(refused.disposition, SpellCastDisposition::CoolingDown);
        assert_eq!(refused.actor_vitals_delta, None);
        assert_eq!(*session.world_spatial(), stable);
        assert_eq!(
            session.actor_vitals().map(|vitals| vitals.mana),
            Some(u32::try_from(*mana)?)
        );
        drop(session);
        peer.await??;
        Ok(())
    })?
}
