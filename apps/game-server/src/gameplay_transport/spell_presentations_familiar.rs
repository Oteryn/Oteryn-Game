//! Actual familiar defense cues share the Channel outbox. They have no player
//! cast command; a sealed physical defense receipt is their sole commit proof.
use super::*;
use crate::foundation::{CompanionSnapshot, FamiliarDefenseReceipt, FamiliarSelfHealDefense};
#[derive(Debug)]
pub(crate) struct PreparedFamiliarDefenseCue {
    actor: ExactActorRef,
    position: MovementLocalPosition,
    content: [u8; 32],
    profile: [u8; 32],
    ordinal: u64,
    event: Presentation,
    cause: std::sync::Arc<PresentationCause>,
}
impl SpellPresentationOwner {
    pub(crate) fn prepare_familiar_defense_before_draw(
        &mut self,
        runtime: &ChannelRuntimeV1,
        active: &NativeGameplayState,
        policy: &FamiliarSelfHealDefense,
        snapshot: &CompanionSnapshot,
    ) -> Result<PreparedFamiliarDefenseCue, Error> {
        if active.source_digest() != self.content
            || active
                .familiar_defenses()
                .and_then(|set| set.defense(&policy.creature_key, &policy.creature_revision))
                != Some(policy)
            || policy.effect != "CONST_ME_MAGIC_GREEN"
            || runtime
                .companion_snapshot(snapshot.actor)
                .map_err(|_| Error::StaleOwner)?
                != *snapshot
        {
            return Err(Error::UnqualifiedSource);
        }
        self.reserve_before_draw(runtime, 1)?;
        let ordinal = snapshot
            .state
            .familiar_defense
            .as_ref()
            .map_or(0, |clock| clock.ordinal)
            .checked_add(1)
            .ok_or(Error::Capacity)?;
        let binding = "appearance:effect/magic_green";
        let cue = resolve_source_cue(binding).ok_or(Error::UnqualifiedSource)?;
        Ok(PreparedFamiliarDefenseCue {
            actor: snapshot.actor,
            position: snapshot.position,
            content: self.content,
            profile: policy.profile_digest,
            ordinal,
            cause: std::sync::Arc::new(PresentationCause::FamiliarDefense {
                actor: snapshot.actor,
                origin: snapshot.position,
                profile: policy.profile_digest,
                ordinal,
            }),
            event: Presentation {
                source_binding: binding.into(),
                cue,
                actor: Some(snapshot.actor),
                position: snapshot.position,
            },
        })
    }
    /// Prevalidated and consumed only in the same uninterrupted owner turn.
    /// Failed source chance advances its real clock but produces no effect.
    pub(crate) fn install_familiar_defense_preflighted(
        &mut self,
        prepared: PreparedFamiliarDefenseCue,
        receipt: &FamiliarDefenseReceipt,
    ) {
        assert_eq!(prepared.actor, receipt.actor());
        assert_eq!(prepared.position, receipt.position());
        assert_eq!(prepared.content, receipt.content_digest());
        assert_eq!(prepared.profile, receipt.profile_digest());
        assert_eq!(prepared.ordinal, receipt.ordinal());
        if receipt.effect().is_some() {
            assert_eq!(receipt.effect(), Some("CONST_ME_MAGIC_GREEN"));
            assert!(self.pending.len() < self.pending.capacity());
            self.install_committed_events(prepared.cause, std::iter::once(prepared.event));
        } else {
            // A genuine applied source defense decision still advances owner order;
            // its failed chance emits no presentation event.
            self.install_committed_events(prepared.cause, std::iter::empty());
        }
    }
}
