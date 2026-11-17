//! NX-8 S13c: a live party query publishes effects of every local party member.
use crate::bestiary_postgres_harness::{TestResult, debug};
use crate::gameplay_transport::{
    ComposedFreshAdmission,
    capabilities::{BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities},
    connection::{AdmittedSession, FirstEntryOutcome, SessionContinuity},
    party_command::SessionPartyView,
};
use oteryn_protocol_oteryn::party::*;

#[test]
fn party_member_effects_real_query_durations_and_legacy_gate_postgres() -> TestResult {
    super::party_buffs::run_party_buffs(None)
}

pub(super) async fn check(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    players: &[(
        crate::foundation::ExactActorRef,
        crate::foundation::GameSessionId,
    )],
) -> TestResult {
    let (actor, session) = players[0];
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(debug)?
        .ok_or("member effects fence")?;
    let mut continuity = SessionContinuity::FRESH;
    continuity.connection_generation = fence.connection_generation.get();
    continuity.selected_capabilities = SelectedCapabilities::select(
        BUNDLE_WORLD_OFFERED_CAPABILITIES,
        &[6, CAPABILITY_PARTY_V1, CAPABILITY_PARTY_MEMBER_EFFECTS_V1],
    )
    .ok_or("member effects selection")?;
    let mut admitted = AdmittedSession {
        game_session_id: session,
        world_id: owner.world_id,
        channel_id: owner.channel_id,
        runtime_actor: Some(actor),
        first_entry: FirstEntryOutcome::Positioned,
        controller: None,
        continuity,
        item_fence: Some(fence.item_fence()),
    };
    let mut view = SessionPartyView::default();
    let bytes = owner
        .party_wire_command(admitted, 501, COMMAND_TYPE_PARTY_QUERY, &[], &mut view)
        .await
        .ok_or("member effects query")?;
    let result = decode_party_result(&bytes).map_err(debug)?;
    assert_eq!(result.disposition, PartyDisposition::Ok);
    assert_eq!(result.members.len(), 3);
    let durations: Vec<_> = result
        .members
        .iter()
        .map(|member| member.party_spell_remaining_ms)
        .collect();
    assert!(durations.iter().all(Option::is_some));
    assert_eq!(
        durations
            .iter()
            .filter(|durations| durations
                .is_some_and(|values| values.iter().any(|ms| *ms > 0 && *ms <= 120_000)))
            .count(),
        2
    );
    assert_eq!(
        durations
            .iter()
            .filter(|durations| **durations == Some([0; 5]))
            .count(),
        1
    );
    {
        let runtime = owner.runtime.lock().await;
        let mut states = owner.states.spell_states.lock().await;
        for (member_actor, member_session) in players {
            let state = states
                .get_mut(&runtime, *member_actor, *member_session)
                .ok_or("member state")?;
            // The actual lifecycle owner removes these conditions on death/logout.
            oteryn_gameplay_spell::spell::actor_conditions::clear_on_lifecycle(state);
        }
    }
    let removed = owner
        .party_wire_command(admitted, 504, COMMAND_TYPE_PARTY_QUERY, &[], &mut view)
        .await
        .ok_or("removed effects query")?;
    assert!(
        decode_party_result(&removed)
            .map_err(debug)?
            .members
            .iter()
            .all(|member| member.party_spell_remaining_ms == Some([0; 5]))
    );
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(BUNDLE_WORLD_OFFERED_CAPABILITIES, &[6, CAPABILITY_PARTY_V1])
            .ok_or("legacy party selection")?;
    let legacy = owner
        .party_wire_command(admitted, 502, COMMAND_TYPE_PARTY_QUERY, &[], &mut view)
        .await
        .ok_or("legacy party query")?;
    assert!(
        decode_party_result(&legacy)
            .map_err(debug)?
            .members
            .iter()
            .all(|member| member.party_spell_remaining_ms.is_none())
    );
    // A current member handle does not bypass the acting session fence.
    admitted
        .item_fence
        .as_mut()
        .ok_or("fence")?
        .character_lease_generation = 0;
    let stale = owner
        .party_wire_command(admitted, 503, COMMAND_TYPE_PARTY_QUERY, &[], &mut view)
        .await
        .ok_or("stale query response")?;
    assert_eq!(
        decode_party_result(&stale).map_err(debug)?.disposition,
        PartyDisposition::Rejected
    );
    Ok(())
}
