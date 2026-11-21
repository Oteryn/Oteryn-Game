//! Native-only closed self-heals use the existing physical heal owner and replay ledger.
use crate::content::native_gameplay::NativeGameplayState;
use crate::creature_auto_attack::ProfileAbilityProposal;
use crate::foundation::{
    CarrierError, ChannelRuntimeV1, CreatureSelfHealLedger, OwnerDamageResult,
};
use oteryn_gameplay_encounters::self_heal_qualification::QualifiedCreatureSelfHeal;
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};
use sha2::{Digest, Sha256};

pub(super) fn execute(
    runtime: &mut ChannelRuntimeV1,
    ledger: &mut CreatureSelfHealLedger,
    content: &mut Option<[u8; 32]>,
    native: &NativeGameplayState,
    proposals: &[ProfileAbilityProposal],
) {
    if runtime.owner_fence().is_err()
        || runtime.content_pin().server_artifact_digest() != native.source_digest()
    {
        return;
    }
    if *content != Some(native.source_digest()) {
        *ledger = CreatureSelfHealLedger::default();
        *content = Some(native.source_digest());
    }
    for proposal in proposals {
        let _ = apply(runtime, ledger, native, proposal);
    }
}

pub(in crate::gameplay_transport) fn apply(
    runtime: &mut ChannelRuntimeV1,
    ledger: &mut CreatureSelfHealLedger,
    native: &NativeGameplayState,
    proposal: &ProfileAbilityProposal,
) -> Result<OwnerDamageResult, CarrierError> {
    use crate::ai_think::profile_schedule::ScheduleList;
    if proposal.list != ScheduleList::Defence
        || proposal.target != proposal.issuer
        || runtime.current_live_creature_identity(proposal.issuer)? != b"oteryn:creature.fugue"
    {
        return Err(CarrierError::CreatureTargetMismatch);
    }
    let row = native
        .creature_profiles()
        .records
        .iter()
        .find(|row| row.profile.target.key == "oteryn:creature.fugue")
        .ok_or(CarrierError::PlanConflict)?;
    let crate::content::ProjectV2AuthoringProfileData::Behavior(behavior) = &row
        .behavior
        .as_ref()
        .ok_or(CarrierError::PlanConflict)?
        .data
    else {
        return Err(CarrierError::PlanConflict);
    };
    if behavior
        .defenses
        .get(proposal.entry_index)
        .is_none_or(|entry| entry.ability != proposal.ability)
    {
        return Err(CarrierError::PlanConflict);
    }
    let id = proposal.occurrence.id().as_str();
    let mut parts = id.rsplitn(4, ':');
    if parts.next().and_then(|v| v.parse::<usize>().ok()) != Some(proposal.entry_index)
        || parts.next() != Some("defence")
    {
        return Err(CarrierError::PlanConflict);
    }
    let sequence = parts
        .next()
        .and_then(|v| v.parse::<u64>().ok())
        .ok_or(CarrierError::PlanConflict)?;
    let actor = proposal
        .issuer
        .placement_identity()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    if parts.next() != Some(format!("ai-profile:{actor}").as_str()) {
        return Err(CarrierError::PlanConflict);
    }
    let source = QualifiedCreatureSelfHeal::qualify_current_native(
        runtime,
        native,
        &row.profile.target.key,
        &proposal.ability.key,
    )
    .map_err(|_| CarrierError::PlanConflict)?;
    if source.removes_paralysis() {
        return Err(CarrierError::PlanConflict);
    }
    let registration = source.register(runtime)?;
    let digest = Sha256::digest(id.as_bytes());
    let mut occurrence = [0; 16];
    occurrence.copy_from_slice(&digest[..16]);
    let roll = deterministic_decision_u64(
        &GameplayDecisionRoot::from_bytes(native.source_digest()),
        DecisionOccurrenceId::from_bytes(occurrence),
        "CREATURE_SELF_HEAL",
        0,
    )
    .map_err(|_| CarrierError::PlanConflict)?;
    let (minimum, maximum) = source.magnitude_range();
    let draw = minimum + roll % (maximum - minimum + 1);
    runtime.commit_creature_self_heal(
        ledger,
        &registration,
        proposal.issuer,
        sequence,
        i64::try_from(draw).map_err(|_| CarrierError::InvalidDamage)?,
    )
}
