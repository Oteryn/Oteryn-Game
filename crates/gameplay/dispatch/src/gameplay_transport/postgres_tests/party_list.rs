//! T172: real admitted party dispatch projects only nearby vitals.
use crate::bestiary_postgres_harness::{TestResult, debug};
use crate::gameplay_transport::{
    ComposedFreshAdmission,
    capabilities::{BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities},
    connection::{AdmittedSession, FirstEntryOutcome, SessionContinuity},
    party_command::SessionPartyView,
};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_protocol_oteryn::party::*;

#[test]
fn party_list_real_dispatch_range_and_legacy_gate_postgres() -> TestResult {
    super::party_buffs::run_party_list()
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
        .ok_or("party list fence")?;
    let mut continuity = SessionContinuity::FRESH;
    continuity.connection_generation = fence.connection_generation.get();
    continuity.selected_capabilities = SelectedCapabilities::select(
        BUNDLE_WORLD_OFFERED_CAPABILITIES,
        &[6, CAPABILITY_PARTY_V1, CAPABILITY_PARTY_LIST_V1],
    )
    .ok_or("party list selection")?;
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
    let summon = {
        let mut runtime = owner.runtime.lock().await;
        owner
            .active_generation
            .ok_or("generation")?
            .native_gameplay()
            .ok_or("native")?
            .install_companion_policies(&mut runtime)
            .map_err(debug)?;
        let mut at = runtime
            .read_actor_position(actor)
            .map_err(debug)?
            .position();
        at.x += 3;
        at.y += 1;
        let prepared = runtime
            .prepare_companion_spawn(
                actor,
                session,
                "oteryn:creature.rat",
                at,
                None,
                None,
                1,
                0,
                true,
            )
            .map_err(debug)?;
        runtime.install_companion_spawn(prepared).map_err(debug)?
    };
    {
        let runtime = owner.runtime.lock().await;
        let mut states = owner.states.spell_states.lock().await;
        let state = states
            .get_mut(&runtime, actor, session)
            .ok_or("player vitals")?;
        let mana = state.vitals().mana;
        state.restore_saved_vitals(1, mana);
    }
    let mut view = SessionPartyView::default();
    let results = super::party_membership::roundtrip(
        owner,
        &mut admitted,
        &[super::party_membership::Step::Query],
    )
    .await?;
    let result = results.first().ok_or("party list result")?;
    assert_eq!(result.disposition, PartyDisposition::Ok);
    assert_eq!(result.members.len(), 3);
    let rows: Vec<_> = result.members.iter().flat_map(|m| &m.list).collect();
    assert_eq!(rows.len(), 4);
    assert!(
        rows.iter()
            .any(|row| row.entity.identity == actor.placement_identity()
                && row.health_percent == Some(1))
    );
    assert_eq!(
        rows.iter()
            .filter(|row| row.summon && row.name == "Rat")
            .count(),
        1
    );
    assert_eq!(
        rows.iter()
            .filter(|row| row.health_percent.is_some())
            .count(),
        3
    );
    assert_eq!(
        rows.iter()
            .filter(|row| row.distance > 30
                && row.health_percent.is_none()
                && row.mana_percent.is_none()
                && row.skull.is_none())
            .count(),
        1
    );
    assert!(
        rows.iter()
            .filter(|row| row.distance <= 30 && !row.summon)
            .all(|row| row.mana_percent.is_some())
    );
    {
        let mut runtime = owner.runtime.lock().await;
        let snapshot = runtime.companion_snapshot(summon).map_err(debug)?;
        runtime
            .despawn_companion(actor, session, &snapshot)
            .map_err(debug)?;
        let mut position = runtime.read_actor_position(players[1].0).map_err(debug)?;
        for _ in 0..29 {
            let mut next = position.position();
            next.x += 1;
            position = runtime
                .borrow_movement_position()
                .commit_cardinal(position, next)
                .map_err(debug)?;
        }
    }
    let boundary = super::party_membership::roundtrip(
        owner,
        &mut admitted,
        &[super::party_membership::Step::Query],
    )
    .await?;
    let boundary = boundary.first().ok_or("boundary result")?;
    assert_eq!(boundary.members.iter().flat_map(|m| &m.list).count(), 3);
    assert!(
        boundary
            .members
            .iter()
            .flat_map(|m| &m.list)
            .any(|row| row.distance == 30
                && row.health_percent.is_some()
                && row.mana_percent.is_some())
    );
    {
        let mut runtime = owner.runtime.lock().await;
        let position = runtime.read_actor_position(players[1].0).map_err(debug)?;
        let mut next = position.position();
        next.x += 1;
        runtime
            .borrow_movement_position()
            .commit_cardinal(position, next)
            .map_err(debug)?;
    }
    let outside = super::party_membership::roundtrip(
        owner,
        &mut admitted,
        &[super::party_membership::Step::Query],
    )
    .await?;
    assert!(
        outside
            .first()
            .ok_or("outside result")?
            .members
            .iter()
            .flat_map(|m| &m.list)
            .any(|row| row.distance == 31
                && row.health_percent.is_none()
                && row.mana_percent.is_none()
                && row.skull.is_none())
    );
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(BUNDLE_WORLD_OFFERED_CAPABILITIES, &[6, CAPABILITY_PARTY_V1])
            .ok_or("legacy selection")?;
    let legacy = owner
        .party_wire_command(admitted, 502, COMMAND_TYPE_PARTY_QUERY, &[], &mut view)
        .await
        .ok_or("legacy query")?;
    assert!(
        decode_party_result(&legacy)
            .map_err(debug)?
            .members
            .iter()
            .all(|member| member.list.is_empty())
    );
    admitted
        .item_fence
        .as_mut()
        .ok_or("fence")?
        .character_lease_generation = 0;
    let stale = owner
        .party_wire_command(admitted, 503, COMMAND_TYPE_PARTY_QUERY, &[], &mut view)
        .await
        .ok_or("stale response")?;
    assert_eq!(
        decode_party_result(&stale).map_err(debug)?.disposition,
        PartyDisposition::Rejected
    );
    Ok(())
}
