//! Current fenced shaping observations through the production owner and real PostgreSQL.
use super::{TestResult, fixture::debug};
use crate::durability::character_proficiency::ProficiencyDefinitions;
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::foundation::{ExactActorRef, GameSessionId};
use crate::gameplay_transport::weapon_proficiency::ProductionProficiencyCatalogue;
use crate::gameplay_transport::{
    ComposedFreshAdmission,
    connection::{AdmittedSession, FirstEntryOutcome, FreshAdmissionAuthority},
};
use oteryn_protocol_oteryn::{self as protocol, weapon_proficiency_state::State};
use tokio::io::AsyncWriteExt;
#[test]
fn weapon_proficiency_state_real_wire_track_revision_slots_and_fences() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| super::proficiency_modification::state_scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "proficiency state scenario panicked")?
        .map_err(Into::into)
}
pub(super) fn admission(
    mut admitted: AdmittedSession,
    fence: CurrentCharacterItemFence,
) -> TestResult<AdmittedSession> {
    admitted.item_fence = Some(fence);
    admitted.first_entry = FirstEntryOutcome::Positioned;
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,
            &[4, 6, 12, 14, 195, 448],
        )
        .ok_or("state capabilities")?;
    Ok(admitted)
}
pub(super) async fn observe(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    definition: u32,
) -> TestResult<State> {
    let bytes =
        FreshAdmissionAuthority::weapon_proficiency_state_query(owner, admitted, definition)
            .await
            .ok_or("state observation")?;
    State::decode(&bytes).ok_or_else(|| "state decode".into())
}
pub(super) async fn assert_fences(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    definition: u32,
) -> TestResult {
    let fence = admitted.item_fence.ok_or("item fence")?;
    let mut missing = admitted;
    missing.item_fence = None;
    assert!(
        FreshAdmissionAuthority::weapon_proficiency_state_query(owner, missing, definition)
            .await
            .is_none()
    );
    for replacement in [
        CurrentCharacterItemFence {
            character_lease_generation: fence.character_lease_generation + 1,
            ..fence
        },
        CurrentCharacterItemFence {
            scope_ownership_generation: crate::foundation::ScopeOwnershipGeneration::new(
                fence.scope_ownership_generation.get() + 1,
            )
            .map_err(debug)?,
            ..fence
        },
    ] {
        let mut stale = admitted;
        stale.item_fence = Some(replacement);
        assert!(
            FreshAdmissionAuthority::weapon_proficiency_state_query(owner, stale, definition)
                .await
                .is_none()
        );
    }
    let mut unselected = admitted;
    unselected.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,
            &[4, 6, 12, 14, 195],
        )
        .ok_or("old capabilities")?;
    assert!(
        FreshAdmissionAuthority::weapon_proficiency_state_query(owner, unselected, definition)
            .await
            .is_none()
    );
    assert!(
        FreshAdmissionAuthority::weapon_proficiency_state_query(owner, admitted, 0)
            .await
            .is_none()
    );
    Ok(())
}
pub(super) async fn advance_other_track(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
    catalogue: &ProductionProficiencyCatalogue,
    selected: &str,
) -> TestResult {
    let other = catalogue
        .weapon_keys()
        .find(|key| {
            *key != selected
                && catalogue.empty_track(key).is_some()
                && catalogue
                    .resolve_weapon(key)
                    .is_some_and(|definition| definition.canonical_item_key != selected)
        })
        .ok_or("another weapon")?;
    {
        let mut states = owner
            .states
            .proficiencies
            .lock()
            .map_err(|_| "proficiency states")?;
        let state = states.entry(session).or_default();
        let mut track = catalogue.empty_track(other).ok_or("other track")?;
        track.progress = 1;
        state.install_track(other, track);
        state.pending_progress.insert(other.into(), 1);
        state.dirty_since = Some(0);
    }
    assert_eq!(
        owner.save_weapon_proficiency(actor, session, true).await,
        crate::gameplay_transport::actor_spell::TrainingSave::Saved
    );
    Ok(())
}
pub(super) async fn wire_query(
    client: &mut tokio::io::DuplexStream,
    definition: u32,
) -> TestResult<State> {
    let frame = protocol::encode_client_command(
        1,
        &protocol::ClientCommandValue {
            command_id: 1,
            command_type: 732,
            payload: &definition.to_be_bytes(),
        },
    )
    .map_err(debug)?;
    client.write_u32(u32::try_from(frame.len())?).await?;
    client.write_all(&frame).await?;
    loop {
        let frame = crate::gameplay_transport::tcp_tls::read_frame(client).await?;
        let envelope = protocol::decode_wire_envelope(&frame).map_err(debug)?;
        if envelope.message_type() == protocol::MessageType::CommandResult {
            let result = protocol::decode_command_result(envelope.payload()).map_err(debug)?;
            assert_eq!(result.status, protocol::CommandStatus::Accepted);
            return State::decode(result.payload).ok_or_else(|| "wire state decode".into());
        }
    }
}
