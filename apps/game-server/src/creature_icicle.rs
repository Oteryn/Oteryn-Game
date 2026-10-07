//! Crystal00ce Icicle: raw Dragon Egg HP callback + literal-string target clear.
//! SOURCE_CPP_RAW_ADDHEALTH; PROJECT_COMPLETE_CURRENT_CENSUS_BATCH; no fake Combat facts.
use crate::ai_think::profile_schedule::{ProfileAbilityProposal, ScheduleList};
use crate::content::{
    ProjectV2AuthoringProfileData as Data, ProjectV2DefinitionRef as Ref, ProjectV2Draft,
};
use crate::creature_auto_attack::AutoAttackOwner;
use crate::foundation::owner_timer::SemanticTimeMicros;
use crate::foundation::{
    ChannelRuntimeV1, ExactActorRef, GameSessionId, OwnerDamageResult, RuntimeScopeRefV1,
    RuntimeWorkStamp, ScopeRuntimeFence,
};
use sha2::{Digest, Sha256};
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum IcicleError {
    Source,
    Fence,
    Actor,
    MissingFacts,
    Replay,
    Capacity,
    Carrier(crate::foundation::CarrierError),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IcicleOutcome {
    pub(crate) cleared_target: bool,
    pub(crate) eggs: Vec<(ExactActorRef, OwnerDamageResult)>,
}
#[derive(Clone)]
pub(crate) struct IcicleSource {
    pub(crate) ability: Ref,
    pub(crate) index: usize,
    content: [u8; 32],
    fingerprint: [u8; 32],
}
const CASTER: &str = "oteryn:creature.icicle";
const EGG: &str = "oteryn:creature.dragon_egg";
const ABILITY: &str = "oteryn:ability.creature.icicle.optional-field-core-defenses-1";
impl IcicleSource {
    pub(crate) fn qualify(
        d: &ProjectV2Draft,
        caster: &Ref,
        index: usize,
        content: [u8; 32],
    ) -> Result<Self, IcicleError> {
        if caster.family != crate::content::ProjectV2Family::Creature
            || caster.key != CASTER
            || caster.revision != "definition-r1"
            || index != 0
        {
            return Err(IcicleError::Source);
        }
        let expected: serde_json::Value =
            serde_json::from_str(include_str!("icicle_source_fixture.json"))
                .map_err(|_| IcicleError::Source)?;
        let ps =
            serde_json::to_value(&d.state.authoring_profiles).map_err(|_| IcicleError::Source)?;
        let rs = serde_json::to_value(&d.core.records).map_err(|_| IcicleError::Source)?;
        for p in expected["profiles"].as_array().ok_or(IcicleError::Source)? {
            if !ps.as_array().is_some_and(|v| {
                v.contains(p) && v.iter().filter(|x| x["target"] == p["target"]).count() == 1
            }) {
                return Err(IcicleError::Source);
            }
        }
        for r in expected["records"].as_array().ok_or(IcicleError::Source)? {
            if !rs.as_array().is_some_and(|v| {
                v.contains(r) && v.iter().filter(|x| x["identity"] == r["identity"]).count() == 1
            }) {
                return Err(IcicleError::Source);
            }
        }
        let behavior = d
            .state
            .authoring_profiles
            .iter()
            .find_map(|p| match &p.data {
                Data::Behavior(b) if p.target.key == "oteryn:behavior.creature.icicle" => Some(b),
                _ => None,
            })
            .ok_or(IcicleError::Source)?;
        let entry = behavior.defenses.get(index).ok_or(IcicleError::Source)?;
        if entry.ability.key != ABILITY
            || entry.ability.revision != "definition-r1"
            || entry.interval_ms != 2000
            || entry.chance_ppm != 600000
        {
            return Err(IcicleError::Source);
        }
        // Source body qualification is private to this constructor; changed closure cannot route raw HP.
        let fingerprint = Sha256::digest(
            serde_json::to_vec(&(content, &expected, entry)).map_err(|_| IcicleError::Source)?,
        )
        .into();
        Ok(Self {
            ability: entry.ability.clone(),
            index,
            content,
            fingerprint,
        })
    }
}
/// Independently current real Combat/map eligibility for this non-aggressive callback.
/// None refuses the complete cast, including its AI clear; do not replace with roster membership.
pub(crate) trait IcicleWorldReader {
    fn icicle_callback_allowed(
        &mut self,
        r: &ChannelRuntimeV1,
        caster: ExactActorRef,
        target: ExactActorRef,
        session: Option<GameSessionId>,
        stamp: RuntimeWorkStamp,
    ) -> Option<bool>;
}
#[derive(Clone)]
struct Memo {
    actor: ExactActorRef,
    activation: u64,
    sequence: u64,
    at: SemanticTimeMicros,
    proof: [u8; 32],
    out: IcicleOutcome,
}
#[derive(Default)]
pub(crate) struct IcicleOwner {
    memos: Vec<Memo>,
}
fn hex(v: &[u8]) -> String {
    v.iter().map(|b| format!("{b:02x}")).collect()
}
impl IcicleOwner {
    // Keep execute ABI explicit: current runtime owner, independent fence/stamp, exact actor/session/source and occurrence/policy facts are separate admission inputs.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn execute(
        &mut self,
        r: &mut ChannelRuntimeV1,
        fence: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        source: &IcicleSource,
        p: &ProfileAbilityProposal,
        ai: &mut AutoAttackOwner,
        reader: &mut impl IcicleWorldReader,
        now: SemanticTimeMicros,
    ) -> Result<IcicleOutcome, IcicleError> {
        let b = r.binding();
        if !fence.is_current_for_scope(
            RuntimeScopeRefV1::channel(b.world_id(), b.channel_id()),
            b.scope_generation(),
        ) || !fence.accepts_stamp(stamp)
        {
            return Err(IcicleError::Fence);
        }
        if source.content != r.content_pin().server_artifact_digest()
            || !r.contains_live_creature(p.issuer)
            || r.current_live_creature_identity(p.issuer)
                .map_err(IcicleError::Carrier)?
                != CASTER.as_bytes()
        {
            return Err(IcicleError::Actor);
        }
        if r.native_summon_count(p.issuer, None) != 0 {
            return Err(IcicleError::Source);
        } // Source also clears owned summons; unexpected children require their owner handoff.
        if p.list != ScheduleList::Defence
            || p.ability != source.ability
            || p.entry_index != source.index
            || p.range_tiles != 0
            || p.magnitude.is_some()
            || p.target != p.issuer
        {
            return Err(IcicleError::Source);
        }
        let atom = format!("actor:{}", hex(&p.issuer.placement_identity()));
        if p.intent.proposal_source() != crate::ability::ProposalSource::Ai
            || p.intent.actor() != atom
            || p.intent.candidate_count() != 1
            || p.intent.resolved_targets().len() != 1
            || p.intent.resolved_targets()[0].as_str() != atom
        {
            return Err(IcicleError::Source);
        }
        let prefix = format!("ai-profile:{}:", hex(&p.issuer.placement_identity()));
        let tail = p
            .occurrence
            .id()
            .as_str()
            .strip_prefix(&prefix)
            .ok_or(IcicleError::Replay)?;
        let (seq, rest) = tail.split_once(':').ok_or(IcicleError::Replay)?;
        if rest != format!("defence:{}", source.index) {
            return Err(IcicleError::Replay);
        }
        let sequence = seq.parse::<u64>().map_err(|_| IcicleError::Replay)?;
        let proof: [u8; 32] = Sha256::digest(
            serde_json::to_vec(&(source.fingerprint, format!("{p:?}")))
                .map_err(|_| IcicleError::Replay)?,
        )
        .into();
        let activation = r.content_pin().activation_sequence();
        self.memos
            .retain(|m| r.contains_live_creature(m.actor) && m.activation == activation);
        let prior = self.memos.iter().position(|m| m.actor == p.issuer);
        if let Some(i) = prior {
            let m = &self.memos[i];
            if sequence < m.sequence || now < m.at {
                return Err(IcicleError::Replay);
            }
            if sequence == m.sequence {
                return if m.proof == proof {
                    Ok(m.out.clone())
                } else {
                    Err(IcicleError::Replay)
                };
            }
        }
        if prior.is_none() {
            if self.memos.len() >= 64 {
                return Err(IcicleError::Capacity);
            }
            self.memos
                .try_reserve(1)
                .map_err(|_| IcicleError::Capacity)?;
        }
        let origin = r
            .read_actor_position(p.issuer)
            .map_err(IcicleError::Carrier)?;
        if origin.context() != r.pinned_movement_context() {
            return Err(IcicleError::Actor);
        }
        let o = origin.position();
        let census = r.positioned_actor_census().map_err(IcicleError::Carrier)?;
        let mut callbacks = Vec::new();
        callbacks
            .try_reserve(64)
            .map_err(|_| IcicleError::Capacity)?;
        let mut spectator_egg = false;
        for (actor, position, session) in census {
            let at = position.position();
            let x = i64::from(at.x) - i64::from(o.x);
            let y = i64::from(at.y) - i64::from(o.y);
            if at.floor != o.floor || x.abs() > 3 || y.abs() > 3 {
                continue;
            }
            let egg = session.is_none()
                && r.current_live_creature_identity(actor)
                    .map_err(IcicleError::Carrier)?
                    == EGG.as_bytes();
            spectator_egg |= egg;
            let radius_width = [1, 2, 3, 3, 3, 2, 1][(y + 3) as usize];
            if x.abs() > radius_width {
                continue;
            }
            let allowed = reader
                .icicle_callback_allowed(r, p.issuer, actor, session, stamp)
                .ok_or(IcicleError::MissingFacts)?;
            if allowed {
                if callbacks.len() == 64 {
                    return Err(IcicleError::Capacity);
                }
                callbacks.push((actor, egg));
            }
        }
        let clear = spectator_egg && !callbacks.is_empty();
        let mut eggs = Vec::new();
        eggs.try_reserve(callbacks.len())
            .map_err(|_| IcicleError::Capacity)?;
        for (actor, egg) in callbacks {
            if egg && clear {
                eggs.push(actor)
            }
        }
        let b = r.binding();
        if !fence.is_current_for_scope(
            RuntimeScopeRefV1::channel(b.world_id(), b.channel_id()),
            b.scope_generation(),
        ) || !fence.accepts_stamp(stamp)
        {
            return Err(IcicleError::Fence);
        }
        // Every output and memo allocation precedes native HP publication.
        let occurrence = p.occurrence.id().as_str().as_bytes();
        let mut native_binding = Vec::new();
        native_binding
            .try_reserve(
                occurrence
                    .len()
                    .checked_add(33)
                    .ok_or(IcicleError::Capacity)?,
            )
            .map_err(|_| IcicleError::Capacity)?;
        native_binding.extend_from_slice(occurrence);
        native_binding.push(0);
        native_binding.extend_from_slice(&proof);
        let (out, memo) = r
            .borrow_exact_actor_commit()
            .commit_icicle_egg_raw_batch(&eggs, occurrence, &native_binding, |receipts| {
                let mut out = IcicleOutcome {
                    cleared_target: clear,
                    eggs: Vec::new(),
                };
                let mut retained = IcicleOutcome {
                    cleared_target: clear,
                    eggs: Vec::new(),
                };
                out.eggs
                    .try_reserve(eggs.len())
                    .map_err(|_| crate::foundation::CarrierError::AllocationFailed)?;
                retained
                    .eggs
                    .try_reserve(eggs.len())
                    .map_err(|_| crate::foundation::CarrierError::AllocationFailed)?;
                for (actor, receipt) in eggs.iter().copied().zip(receipts.iter().copied()) {
                    out.eggs.push((actor, receipt));
                    retained.eggs.push((actor, receipt));
                }
                let memo = Memo {
                    actor: p.issuer,
                    activation,
                    sequence,
                    at: now,
                    proof,
                    out: retained,
                };
                Ok((out, memo))
            })
            .map_err(IcicleError::Carrier)?;
        if clear {
            ai.clear_target(p.issuer)
        } // Infallible same-owner publication; no fallible calls after HP.
        if let Some(i) = prior {
            self.memos[i] = memo
        } else {
            self.memos.push(memo)
        }
        Ok(out)
    }
}

#[cfg(test)]
#[path = "creature_icicle_tests.rs"]
mod tests;
