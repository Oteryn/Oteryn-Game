//! Oberon's chat challenge through framed local speech and the shared damage gate.
use super::fixture::debug;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::foundation::{
    CarrierError, ChannelRuntimeV1, CommandId, CommandRef, ExactActorRef, MovementLocalPosition,
    OwnerDamageCommand,
};
use oteryn_gameplay_encounters::bosses::{
    phase_controller::oberon,
    source_callbacks::oberon_think::{OBERON_KEY, OBERON_LIVES, OBERON_SUMMONS},
};
use oteryn_protocol_oteryn::chat::{self, ChatDisposition, ChatIntent, ChatSpeechMode};

#[test]
fn seclib1_oberon_local_say_lifts_immunity_and_limits_summons() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::seclib1_falcon::scenario(
        "seclib1_oberon_local_say_lifts_immunity_and_limits_summons",
        true,
    )
}

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    attacker: AdmittedSession,
    mut speaker: AdmittedSession,
    at: MovementLocalPosition,
) -> TestResult {
    speaker.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,
            &[7],
        )
        .ok_or("chat negotiation")?;
    owner.states.chat.admit(
        speaker.game_session_id,
        "Falcon challenger",
        owner.owner_now(),
    );
    let (boss, maximum) = {
        let mut runtime = owner.runtime.lock().await;
        let maximum = runtime
            .companion_policy(OBERON_KEY)
            .map_err(debug)?
            .maximum_health;
        let boss = runtime
            .admit_encounter_actor(
                OBERON_KEY,
                maximum,
                MovementLocalPosition {
                    x: at.x + 1,
                    y: at.y - 4,
                    ..at
                },
            )
            .map_err(debug)?;
        runtime
            .install_creature_policy(boss, OBERON_KEY)
            .map_err(debug)?;
        (boss, maximum)
    };
    let threshold = maximum / 5;
    let damage_to_threshold = maximum - threshold;
    hit(
        &mut *owner.runtime.lock().await,
        attacker,
        boss,
        40,
        damage_to_threshold,
    )?;
    let _clock = owner.freeze_owner_clock_for_test();
    for life in 0..u64::from(OBERON_LIVES) {
        owner.states.test_owner_time.store(
            10_000_001 + life * 1_000_000,
            std::sync::atomic::Ordering::Relaxed,
        );
        let sequence = life + 1;
        {
            let mut runtime = owner.runtime.lock().await;
            assert!(
                oberon::think(&mut runtime, boss, sequence, |_| 0)
                    .map_err(debug)?
                    .is_some()
            );
            assert_eq!(
                runtime.live_creature_health(boss),
                Some(u64::try_from(maximum)?)
            );
            let summons = runtime
                .current_live_creature_census()
                .map_err(debug)?
                .into_iter()
                .filter(|actor| {
                    runtime.matches_live_creature_identity(*actor, OBERON_SUMMONS[0].as_bytes())
                })
                .count();
            assert_eq!(summons, usize::try_from(sequence)?);
            assert!(
                oberon::think(&mut runtime, boss, sequence, |_| 0)
                    .map_err(debug)?
                    .is_none()
            );
            assert_eq!(
                try_hit(&mut runtime, attacker, boss, 50 + life * 2, 1),
                Err(CarrierError::PlanConflict)
            );
        }
        // Crystal's first reply, sent by the second player through ordinary local chat.
        let payload = chat::encode_chat_intent(&ChatIntent::Say {
            mode: ChatSpeechMode::Say,
            text: "HOW APPROPRIATE, YOU LOOK LIKE SOMETHING WORMS ALREADY GOT THE BETTER OF!"
                .into(),
        })
        .map_err(debug)?;
        let command = speaker.continuity.next_command_id;
        let reply = super::quest_world_callbacks::wire(
            owner,
            &mut speaker,
            command,
            chat::COMMAND_TYPE_CHAT_INTENT,
            payload,
        )
        .await?;
        assert_eq!(
            chat::decode_chat_intent_result(&reply)
                .map_err(debug)?
                .disposition,
            ChatDisposition::Ok
        );
        hit(
            &mut *owner.runtime.lock().await,
            attacker,
            boss,
            51 + life * 2,
            damage_to_threshold,
        )?;
        assert_eq!(
            owner.runtime.lock().await.live_creature_health(boss),
            Some(u64::try_from(threshold)?)
        );
    }
    let mut runtime = owner.runtime.lock().await;
    assert!(
        oberon::think(&mut runtime, boss, 4, |_| 0)
            .map_err(debug)?
            .is_none()
    );
    hit(&mut runtime, attacker, boss, 60, threshold)?;
    assert_eq!(runtime.live_creature_health(boss), None);
    Ok(())
}

fn hit(
    runtime: &mut ChannelRuntimeV1,
    player: AdmittedSession,
    boss: ExactActorRef,
    command: u64,
    damage: i64,
) -> TestResult {
    try_hit(runtime, player, boss, command, damage).map_err(debug)?;
    Ok(())
}

fn try_hit(
    runtime: &mut ChannelRuntimeV1,
    player: AdmittedSession,
    boss: ExactActorRef,
    command: u64,
    damage: i64,
) -> Result<(), CarrierError> {
    runtime
        .borrow_exact_actor_commit_at(command)
        .commit_damage_for_bound_attacker(
            boss,
            player.runtime_actor.ok_or(CarrierError::PlanConflict)?,
            CommandRef::new(
                player.game_session_id,
                CommandId::new(command).map_err(|_| CarrierError::PlanConflict)?,
            ),
            0,
            OwnerDamageCommand {
                target: OBERON_KEY.as_bytes(),
                occurrence: player.game_session_id.as_bytes(),
                binding: b"seclib1-oberon-native-damage",
                damage,
            },
        )?;
    Ok(())
}
