//! Qualify the current-actor prerequisite through real native NPC command7.
//! Controlled condition predecessor only; this does not claim a beer producer.
use super::*;
fn conversations(
    frames: &[Vec<u8>],
) -> TestResult<Vec<oteryn_protocol_oteryn::npc_service::NpcConversation>> {
    let mut result = Vec::new();
    for frame in frames {
        let envelope = crate::foundation::decode_wire_envelope(frame).map_err(debug)?;
        if envelope.message_type() != crate::foundation::MessageType::StateDelta {
            continue;
        }
        let delta =
            oteryn_protocol_oteryn::decode_state_delta(envelope.payload()).map_err(debug)?;
        if delta.domain_id == oteryn_protocol_oteryn::npc_service::STATE_DOMAIN_NPC_CONVERSATION {
            result.push(
                oteryn_protocol_oteryn::npc_service::decode_npc_conversation(delta.payload)
                    .map_err(debug)?,
            );
        }
    }
    Ok(result)
}
pub(super) async fn exercise(
    h: &Harness,
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut crate::gameplay_transport::connection::AdmittedSession,
    npc: oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
) -> TestResult {
    let actor = admitted.runtime_actor.ok_or("actual Vescu customer")?;
    // The source's questState<1 branch is the untouched known missing value -1.
    let frames =
        super::super::grizzly_npc::wire::talk_appearance(owner, admitted, npc, &[(1, "hi")])
            .await?;
    let sober = conversations(&frames)?;
    assert!(
        sober.iter().any(|reply| reply.open.is_none()
            && reply
                .lines
                .iter()
                .any(|line| line == "Oh, two t-trolls. Hellooo, wittle twolls. <hicks>")),
        "actual sober Vescu greeting must taunt and refuse focus before the outfit quest"
    );
    assert!(
        !sober.iter().any(|reply| reply.open.is_some()),
        "sober greeting cannot open a source interaction"
    );
    install_drunk(owner, actor, admitted.game_session_id).await?;
    let frames =
        super::super::grizzly_npc::wire::talk_appearance(owner, admitted, npc, &[(2, "hello")])
            .await?;
    assert!(
        conversations(&frames)?
            .iter()
            .any(|reply| reply.open.is_some()
                && reply
                    .lines
                    .iter()
                    .any(|line| line
                        == "Hey t-there, you look like someone who enjoys a good {booze}.")),
        "actual current stored Drunk qualifies the source greeting"
    );
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(61))
        .ok_or("expired native Drunk clock")?;
    let frames =
        super::super::grizzly_npc::wire::talk_appearance(owner, admitted, npc, &[(3, "hi")])
            .await?;
    assert!(
        conversations(&frames)?
            .iter()
            .any(|reply| reply.open.is_none()
                && reply.lines.iter().any(|line| line.contains("two t-trolls"))),
        "expired stored Drunk cannot qualify a later fresh greeting"
    );
    let started:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key='oteryn:quest-progress/quest/u7_8/assassin_outfits/assassin_base_outfit' AND value>=1)")
        .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert!(
        !started,
        "greeting never fabricates earned Assassin progress"
    );
    Ok(())
}
async fn install_drunk(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    session: crate::foundation::GameSessionId,
) -> TestResult {
    use oteryn_gameplay_domain::domain::condition::{ConditionValues, StatusKind};
    use oteryn_gameplay_foundation::foundation::condition::{
        ApplicationFacts, ConditionDefinition,
    };
    use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};
    let definition = ConditionDefinition::new(
        "vescu.qualifier.drunk",
        1,
        ConditionValues::TimedStatus {
            kind: StatusKind::Drunk,
            duration_ms: 60_000,
        },
    )
    .ok_or("native Drunk definition")?;
    let mut runtime = owner.runtime.lock().await;
    let stamp = runtime.issue_owner_work().map_err(debug)?;
    let fence = runtime.owner_fence().map_err(debug)?;
    let mut states = owner.states.spell_states.lock().await;
    let speed = u16::try_from(
        states
            .get(&runtime, actor, session)
            .ok_or("actual owned Vescu state")?
            .owned_base_speed(),
    )?;
    let root = GameplayDecisionRoot::from_bytes([73; 32]);
    let facts = ApplicationFacts {
        now: owner.owner_now().get(),
        base_speed: speed,
        mana_shield_capacity: 0,
        target_reentry_protected: false,
        source_reentry_protected: false,
        target_is_player: true,
        decision_root: &root,
        occurrence: DecisionOccurrenceId::from_bytes(actor.placement_identity()),
    };
    assert!(
        states.install_owned_player_self_use_test_condition(
            &runtime,
            fence,
            stamp,
            actor,
            session,
            &definition,
            &facts
        ),
        "controlled prerequisite uses the existing current-owner condition CAS"
    );
    assert!(
        states
            .get(&runtime, actor, session)
            .ok_or("actual conditioned customer")?
            .owned_conditions()
            .has_status(StatusKind::Drunk, facts.now)
    );
    Ok(())
}
