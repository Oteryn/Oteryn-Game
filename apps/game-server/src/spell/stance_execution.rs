//! One actor-owned retained durable stance intent. Historical receipts prove a
//! particular writer result; the Channel independently proves current ownership.
use super::cast::{PaidNativeCast, PlayerSpellState};
use super::native_actor_states::{StancePlan, StandardStance};
use super::{Execution, SpellDefinition};
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::durability::character_stance::{
    CommittedCharacterStance, StanceChangeOccurrence, StanceChangeRequest,
};
use crate::foundation::{ExactActorRef, GameSessionId};
use oteryn_protocol_oteryn::actor_spell::SpellCastDisposition;
use oteryn_simulation_determinism::SemanticTimeMicros;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PreparedStance {
    pub(crate) actor: ExactActorRef,
    pub(crate) session: GameSessionId,
    pub(crate) command_id: u64,
    pub(crate) spell_index: std::num::NonZeroU32,
    pub(crate) fence: CurrentCharacterGameplayFence,
    pub(crate) request: StanceChangeRequest,
    before: Box<PlayerSpellState>,
    paid: Box<PlayerSpellState>,
    anchor: super::combat_batch::SpellAnchor,
    plan: StancePlan,
    pub(crate) prepared_at: SemanticTimeMicros,
}
fn reject<T>() -> Result<T, SpellCastDisposition> {
    Err(SpellCastDisposition::Rejected)
}
pub(crate) fn policy_digest(spell: &SpellDefinition) -> Result<[u8; 32], SpellCastDisposition> {
    let Execution::NativeProfile(profile) = &spell.execution else {
        return reject();
    };
    if profile.spell()["execution"]["native_behavior"]["key"] != "stance_toggle" {
        return reject();
    }
    let bytes = serde_json::to_vec(profile.spell()).map_err(|_| SpellCastDisposition::Rejected)?;
    if bytes.len() > 16384 {
        return reject();
    }
    Ok(Sha256::digest(bytes).into())
}
impl PreparedStance {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        before: &PlayerSpellState,
        paid: PaidNativeCast,
        spell: &SpellDefinition,
        actor: ExactActorRef,
        session: GameSessionId,
        command_id: u64,
        spell_index: std::num::NonZeroU32,
        fence: CurrentCharacterGameplayFence,
        content_revision: String,
        policy_revision: String,
        now: SemanticTimeMicros,
    ) -> Result<Self, SpellCastDisposition> {
        if before.pending_stance.is_some()
            || paid.next.pending_stance.is_some()
            || fence.game_session_id != session
            || command_id == 0
            || !paid.next.paid_successor_of(before, &paid.anchor)
        {
            return reject();
        }
        let super::native::Plan::Stance(plan) = paid.plan else {
            return reject();
        };
        if plan.before != before.stance
            || !plan.persist_across_sessions
            || !plan.keep_on_death
            || !plan.debit_mana_and_start_cooldowns
            || plan
                .after
                .is_some_and(|stance| !stance.eligible(before.facts.vocation))
        {
            return reject();
        }
        let digest = policy_digest(spell)?;
        let mut raw: [u8; 16] = Sha256::new()
            .chain_update(b"oteryn:stance-cast:v1")
            .chain_update(actor.placement_identity())
            .chain_update(session.as_bytes())
            .chain_update(command_id.to_be_bytes())
            .chain_update(digest)
            .finalize()[..16]
            .try_into()
            .map_err(|_| SpellCastDisposition::Rejected)?;
        raw[6] = (raw[6] & 15) | 0x70;
        raw[8] = (raw[8] & 63) | 0x80;
        let request = StanceChangeRequest {
            occurrence: StanceChangeOccurrence::from_bytes(raw)
                .map_err(|_| SpellCastDisposition::Rejected)?,
            before: before.stance_key.clone(),
            after: plan.after.map(|stance| stance.key().to_owned()),
            content_revision,
            policy_revision,
            policy_digest: digest,
        };
        if request.before == request.after {
            return reject();
        }
        Ok(Self {
            actor,
            session,
            command_id,
            spell_index,
            fence,
            request,
            before: Box::new(before.clone()),
            paid: Box::new(paid.next),
            anchor: paid.anchor,
            plan: *plan,
            prepared_at: now,
        })
    }
    pub(crate) fn matches_command(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        command_id: u64,
    ) -> bool {
        self.actor == actor && self.session == session && self.command_id == command_id
    }
    pub(crate) fn matches_receipt(&self, receipt: &CommittedCharacterStance) -> bool {
        receipt.matches_request(&self.fence, &self.request)
    }
}
impl PlayerSpellState {
    pub(crate) fn reserve_stance(
        &mut self,
        prepared: PreparedStance,
    ) -> Result<(), SpellCastDisposition> {
        if self.pending_stance.is_some() || self != prepared.before.as_ref() {
            return reject();
        }
        self.pending_stance = Some(Box::new(prepared));
        Ok(())
    }
    /// A conclusively refused writer may retire only this same reservation.
    pub(crate) fn cancel_stance(&mut self, prepared: &PreparedStance) -> bool {
        if self.pending_stance.as_deref() == Some(prepared) {
            self.pending_stance = None;
            true
        } else {
            false
        }
    }
    /// Preserve real HP, conditions, movement and Monk evaluations that changed
    /// during the writer. The exact prepared cost is subtracted from current resources; cooldown or build changes invalidate payment.
    pub(crate) fn commit_stance_receipt(
        &mut self,
        prepared: &PreparedStance,
        receipt: &CommittedCharacterStance,
        spell: &SpellDefinition,
        now: SemanticTimeMicros,
    ) -> Result<(), SpellCastDisposition> {
        if self.pending_stance.as_deref() != Some(prepared)
            || !prepared.matches_receipt(receipt)
            || policy_digest(spell)? != prepared.request.policy_digest
            || now < prepared.prepared_at
            || self.facts != prepared.before.facts
            || self.cooldowns != prepared.before.cooldowns
            || self.stance_key != prepared.before.stance_key
        {
            return reject();
        }
        let after = receipt
            .after()
            .and_then(|key| StandardStance::read(key).ok())
            .filter(|s| s.eligible(self.facts.vocation));
        if after != prepared.plan.after {
            return reject();
        }
        let mut next = self.clone();
        next.mana = self
            .mana
            .checked_sub(prepared.anchor.paid_mana)
            .ok_or(SpellCastDisposition::Rejected)?;
        next.soul = self
            .soul
            .checked_sub(prepared.anchor.paid_soul)
            .ok_or(SpellCastDisposition::Rejected)?;
        // This durable primary commit starts the casting cooldowns now; DB wait
        // time never silently consumes their prepared lifetime.
        next.cooldowns
            .rearm(spell, now)
            .map_err(|_| SpellCastDisposition::Rejected)?;
        next.stance = after;
        next.stance_key = prepared.request.after.clone();
        next.pending_stance = None;
        next.revision = next
            .revision
            .checked_add(1)
            .ok_or(SpellCastDisposition::Rejected)?;
        *self = next;
        Ok(())
    }
}

pub(crate) fn active_stance_from_book(
    key: Option<&str>,
    vocation: super::Vocation,
    book: &super::SpellBook,
) -> Option<StandardStance> {
    let key = key?;
    book.spells
        .iter()
        .enumerate()
        .filter(|(index, _)| !book.inactive_aliases.contains(index))
        .find_map(|(_, spell)| {
            let Execution::NativeProfile(profile) = &spell.execution else {
                return None;
            };
            let native = &profile.spell()["execution"]["native_behavior"];
            if native["key"] != "stance_toggle" || native["parameters"]["stance"] != key {
                return None;
            }
            let plan =
                super::native_actor_states::plan_stance(&native["parameters"], None, vocation)
                    .ok()?;
            plan.after
        })
}
