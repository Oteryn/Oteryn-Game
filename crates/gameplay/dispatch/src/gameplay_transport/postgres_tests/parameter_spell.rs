//! PARAM-CAST-WIRE-1: admitted CHAT_V1 speech reaches the existing spell owner.
#![allow(clippy::expect_used)]
use super::appearance_change_postgres::observer_visibility_tests as observer;
use super::death_ml_loss_composed::{current_fence, production_generation_with_source};
use super::fixture::{Harness, debug, id, join_two, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::character_build::{
    BuildCause, BuildChangeRequest, BuildOccurrence, DurableBuildState, convert_vocation,
};
use crate::durability::character_progression::{
    ExperienceAwardRequest, ExperienceRewardOccurrence,
};
use crate::foundation::{
    ChannelContentPin, CharacterLease, CommandId, CommandRef, GameSessionId, MessageType,
    decode_wire_envelope,
};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    AdmittedSession, FirstEntryOutcome, FreshAdmissionAuthority, IDLE_LIVENESS, serve_admitted,
};
use crate::gameplay_transport::{ComposedFreshAdmission, item_fence_of};
use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, actor_spell, chat, decode_command_result,
    encode_client_command,
};
use std::time::Duration;
use tokio::io::AsyncWriteExt;

#[test]
fn parameter_spell_online_exiva_real_session_postgres() -> TestResult {
    scenario(0)
}
#[test]
fn parameter_spell_offline_and_unknown_exiva_real_session_postgres() -> TestResult {
    scenario(1)
}
#[test]
fn parameter_spell_ordinary_cast_unchanged_real_session_postgres() -> TestResult {
    scenario(2)
}

#[test]
fn parameter_spell_protected_exiva_real_session_postgres() -> TestResult {
    scenario(3)
}

fn scenario(case: u8) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(128 << 20)
        .spawn(move || {
            run(async move |admin| scenario_inner(admin, case).await)
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "parameter spell thread panicked")?
        .map_err(Into::into)
}

async fn scenario_inner(admin: String, case: u8) -> TestResult {
    let h = Harness::create_with_admission_seed(
        admin,
        &format!("parameter_spell_{case}"),
        false,
        false,
    )
    .await?;
    eprintln!("PARAM-CAST-WIRE-1 PostgreSQL harness created, case={case}");
    prepare_current_session(&h).await?;
    h.seed_pvp_target().await?;
    if case != 1 {
        observer::admit(&h).await?;
    }
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, pin) = production_generation_with_source(
        world,
        "content-1",
        Some(include_bytes!(
            "../../../../../../tools/content-schema/native-gameplay/canary-thalom-world.json"
        )),
    )
    .map_err(debug)?;
    let active = controller.active().ok_or("active")?;
    let native = active.native_gameplay().ok_or("native")?;
    let room = pin.qualified_room().clone();
    let cp = ChannelContentPin::from_activation(
        world,
        1,
        pin.identity().server_artifact_digest(),
        pin.identity().client_artifact_digest(),
        room.frame_binding().digest(),
        room.map_revision_digest(),
        (2, 11, -7),
    );
    let mut deps = Dependencies::with_content(cp, room.clone(), native.spell_book().clone())?;
    deps.combat_room = Some(room);
    let mut owner = deps.owner(&h, &authority);
    owner.active_generation = Some(active);
    let mut admitted = deps.admitted;
    let actor = deps.actor;
    let session = admitted.game_session_id;
    owner.admit_progression_session(&mut admitted).await;
    let progression = owner.progression_binding(session).ok_or("progression")?;
    let formula = native.training_formula().ok_or("training")?;
    let druid = convert_vocation(formula, &DurableBuildState::default(), "druid").map_err(debug)?;
    h.root
        .commit_character_build(
            &authority,
            &h.node,
            current_fence(&owner, session).await?,
            BuildChangeRequest {
                occurrence: BuildOccurrence::from_bytes(id(90)).map_err(debug)?,
                cause: BuildCause::VocationChoice,
                before: DurableBuildState::default(),
                after: druid.clone(),
                pruned_stance: None,
            },
            formula,
        )
        .await
        .map_err(debug)?;
    h.root
        .commit_character_experience(
            &authority,
            &h.node,
            current_fence(&owner, session).await?,
            ExperienceAwardRequest {
                occurrence: ExperienceRewardOccurrence::from_bytes(id(91)).map_err(debug)?,
                amount: oteryn_simulation_determinism::ExactI64::new(98_800),
                context: progression.context.clone(),
                policy_revision: progression.policy_revision.clone(),
                reward_revision: progression.reward_revision.clone(),
                policy: progression.policy.clone(),
            },
        )
        .await
        .map_err(debug)?;
    {
        let mut runtime = deps.runtime.lock().await;
        runtime
            .initialize_first_entry_position(actor)
            .map_err(debug)?;
        runtime
            .bind_attacker_lease(
                actor,
                session,
                CharacterLease::new(
                    crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?,
                    2,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        let mut states = owner.states.spell_states.lock().await;
        assert!(
            states
                .initialize(
                    &runtime,
                    actor,
                    session,
                    crate::spell::cast::CharacterCastFacts {
                        level: 20,
                        ..crate::gameplay_transport::actor_spell::tests::FACTS
                    },
                    (0, 0),
                    owner.owner_now()
                )
                .is_some()
        );
        assert!(states.load_owned_build(&runtime, actor, session, &druid));
        states
            .get_mut(&runtime, actor, session)
            .ok_or("caster")?
            .enable_owned_training(&druid, formula)
            .map_err(debug)?;
    }
    let target_session = GameSessionId::decode(&id(110)).map_err(debug)?;
    if case != 1 {
        observer::initialize(&deps, &owner).await?;
    }
    sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1),(encode($2,'hex')::uuid,1)").bind(id(41).as_slice()).bind(id(101).as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)").bind(id(42).as_slice()).bind(id(43).as_slice()).bind(pin.identity().server_artifact_digest().as_slice()).bind(pin.identity().client_artifact_digest().as_slice()).bind(pin.frame_binding().digest().as_slice()).execute(&h.pool).await?;
    for current_session in [Some(session), (case != 1).then_some(target_session)]
        .into_iter()
        .flatten()
    {
        h.root
            .initialize_familiar_group(
                &authority,
                &h.node,
                current_fence(&owner, current_session).await?,
            )
            .await
            .map_err(debug)?;
    }
    if case == 0 || case == 3 {
        allow_location(&owner, target_session, native.source_digest(), case == 0).await?;
    }
    let current =
        crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
            .current_session_at(session)
            .await?
            .0;
    admitted.item_fence = item_fence_of(current);
    admitted.first_entry = FirstEntryOutcome::Positioned;
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), &[6, 7])
            .ok_or("chat capabilities")?;
    let before = owner
        .observe_vitals(actor, session)
        .await
        .ok_or("before vitals")?
        .1
        .mana;
    let words = if case == 2 {
        "exura"
    } else {
        "  EXIVA prefix\"Fixture Watcher\"  "
    };
    let payload = chat::encode_chat_intent(&chat::ChatIntent::Say {
        mode: chat::ChatSpeechMode::Say,
        text: words.into(),
    })
    .map_err(debug)?;
    let mut commands = vec![
        (chat::COMMAND_TYPE_CHAT_INTENT, payload.clone()),
        (chat::COMMAND_TYPE_CHAT_INTENT, payload),
    ];
    let (results, feedback) = exchange(&owner, &mut admitted, &commands, true, case != 2).await?;
    assert_eq!(results[1].0, CommandStatus::DuplicateReplay);
    let result = chat::decode_chat_intent_result(&results[0].1).map_err(debug)?;
    assert_eq!(
        result.disposition,
        if case == 1 || case == 3 {
            chat::ChatDisposition::Rejected
        } else {
            chat::ChatDisposition::Ok
        },
        "{results:?}"
    );
    let after = owner
        .observe_vitals(actor, session)
        .await
        .ok_or("after vitals")?
        .1
        .mana;
    assert_eq!(before - after, if case == 1 || case == 3 { 0 } else { 20 });
    if case != 2 {
        let expected = if case == 1 {
            "A player with this name is not online."
        } else if case == 3 {
            "The character you are trying to find with Exiva is currently protected from your spell."
        } else {
            "Fixture Watcher is standing next to you."
        };
        assert!(feedback.iter().any(|line| matches!(line, chat::ChatLine::Private { speaker_name, text } if speaker_name == "Game" && text == expected)), "{feedback:?}");
        assert!(owner.states.chat.drain(target_session).is_empty());
    }
    // A second input after the source cooldown uses a new command, not another cast path.
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(Duration::from_secs(5))
        .ok_or("clock advance")?;
    if case == 1 {
        commands = vec![(
            chat::COMMAND_TYPE_CHAT_INTENT,
            chat::encode_chat_intent(&chat::ChatIntent::Say {
                mode: chat::ChatSpeechMode::Say,
                text: "exiva Nobody".into(),
            })
            .map_err(debug)?,
        )];
    } else if case == 2 {
        let index = (1..=deps.spells.source_len())
            .filter_map(|i| u32::try_from(i).ok().and_then(std::num::NonZeroU32::new))
            .find(|i| {
                deps.spells
                    .indexed(*i)
                    .is_some_and(|s| s.name == "Light Healing")
            })
            .ok_or("exura index")?;
        commands = vec![(
            actor_spell::COMMAND_TYPE_WORLD_ACTOR_SPELL_CAST_INTENT,
            actor_spell::encode_spell_cast_intent(&actor_spell::SpellCastIntent {
                spell: index,
                target: actor_spell::SpellTarget::None,
                aim_at_target: false,
            }),
        )];
    } else {
        return Ok(());
    }
    assert_eq!(admitted.continuity.next_command_id, 2);
    let (next, _) = exchange(&owner, &mut admitted, &commands, false, case == 1).await?;
    if case == 1 {
        assert_eq!(
            chat::decode_chat_intent_result(&next[0].1)
                .map_err(debug)?
                .disposition,
            chat::ChatDisposition::Rejected
        );
        assert_eq!(
            owner
                .observe_vitals(actor, session)
                .await
                .ok_or("offline mana")?
                .1
                .mana,
            after
        );
    } else {
        assert_eq!(
            actor_spell::decode_spell_cast_result(&next[0].1).map_err(debug)?,
            actor_spell::SpellCastDisposition::Cast
        );
        assert_eq!(
            after
                - owner
                    .observe_vitals(actor, session)
                    .await
                    .ok_or("ordinary mana")?
                    .1
                    .mana,
            20
        );
    }
    Ok(())
}

async fn allow_location(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    session: GameSessionId,
    digest: [u8; 32],
    allowed: bool,
) -> TestResult {
    use crate::durability::spell_item_transaction as items;
    let fence = current_fence(owner, session).await?.item_fence();
    let permit = owner.spell_lane_permit().await.ok_or("spell lane")?;
    let command = CommandRef::new(session, CommandId::new(1).map_err(debug)?);
    let mut context = (owner, permit);
    owner
        .root
        .try_issue_semantic_pass()?
        .run_with_context(&mut context, move |holder, deadline, (owner, permit)| {
            Box::pin(async move {
                let mut tx = items::begin_spell_owner_transaction(holder, deadline).await?;
                let authority = items::assert_spell_item_authority_in_transaction(
                    &mut tx,
                    permit,
                    owner.root,
                    owner.character,
                    owner.holder,
                    &fence,
                    command,
                    digest,
                )
                .await
                .map_err(|_| crate::durability::DurabilityError::Unavailable)?;
                let mut preferences =
                    crate::durability::spell_privacy::ExivaPreferences::source_initial_preferences(
                    );
                preferences.allow_all = allowed;
                preferences.allow_own_guild = false;
                preferences.allow_own_party = false;
                preferences.allow_vip_list = false;
                crate::durability::spell_privacy::write_exiva_preferences_in_transaction(
                    &mut tx,
                    &authority,
                    None,
                    &preferences,
                )
                .await
                .map_err(|_| crate::durability::DurabilityError::Unavailable)?;
                tx.commit().await?;
                Ok(())
            })
        })
        .await?;
    Ok(())
}

type WireResults = (Vec<(CommandStatus, Vec<u8>)>, Vec<chat::ChatLine>);

async fn exchange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    commands: &[(u32, Vec<u8>)],
    replay: bool,
    feedback_expected: bool,
) -> TestResult<WireResults> {
    let session = *admitted;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serving = async move { serve_admitted(&mut server, session, owner, IDLE_LIVENESS).await };
    let driving = async move {
        loop {
            let frame = super::item_look::read_frame(&mut client).await?;
            if decode_wire_envelope(&frame).map_err(debug)?.message_type()
                == MessageType::SnapshotCommit
            {
                break;
            }
        }
        let mut results = Vec::new();
        let mut feedback = Vec::new();
        for (offset, (kind, payload)) in commands.iter().enumerate() {
            let command_id =
                session.continuity.next_command_id + if replay { 0 } else { offset as u64 };
            let frame = encode_client_command(
                session.continuity.connection_generation,
                &ClientCommandValue {
                    command_id,
                    command_type: *kind,
                    payload,
                },
            )
            .map_err(debug)?;
            client.write_u32(u32::try_from(frame.len())?).await?;
            client.write_all(&frame).await?;
            loop {
                let frame = super::item_look::read_frame(&mut client).await?;
                let envelope = decode_wire_envelope(&frame).map_err(debug)?;
                if envelope.message_type() == MessageType::StateDelta {
                    let delta = oteryn_protocol_oteryn::decode_state_delta(envelope.payload())
                        .map_err(debug)?;
                    if delta.domain_id == 12 && delta.delta_type == 1 {
                        feedback.push(chat::decode_chat_line(delta.payload).map_err(debug)?);
                    }
                }
                if envelope.message_type() == MessageType::CommandResult {
                    let result = decode_command_result(envelope.payload()).map_err(debug)?;
                    results.push((result.status, result.payload.to_vec()));
                    break;
                }
            }
        }
        while feedback_expected && feedback.is_empty() {
            let frame = super::item_look::read_frame(&mut client).await?;
            let envelope = decode_wire_envelope(&frame).map_err(debug)?;
            if envelope.message_type() == MessageType::StateDelta {
                let delta = oteryn_protocol_oteryn::decode_state_delta(envelope.payload())
                    .map_err(debug)?;
                if delta.domain_id == 12 && delta.delta_type == 1 {
                    feedback.push(chat::decode_chat_line(delta.payload).map_err(debug)?);
                }
            }
        }
        client.shutdown().await?;
        Ok((results, feedback))
    };
    let (end, results): (_, TestResult<_>) =
        tokio::time::timeout(Duration::from_secs(45), join_two(serving, driving)).await?;
    match end {
        crate::gameplay_transport::connection::ConnectionEnd::AdmittedThenDisconnected(current) => {
            *admitted = current
        }
        other => return Err(format!("parameter serving ended: {other:?}").into()),
    }
    results
}
