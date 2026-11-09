//! Current Channel-only advisory preview; original161 remains the sole allocation writer.
use super::{ComposedFreshAdmission, connection::AdmittedSession};
use oteryn_gameplay_progression::{
    wheel::{WheelAllocation, WheelDomain, WheelRuleset, WheelSlots},
    wheel_gem_data::WheelGemData,
};
use oteryn_protocol_oteryn::wheel_presentation as wire;
#[path = "wheel_presentation/catalogue.rs"]
mod catalogue;
#[path = "wheel_presentation/effects.rs"]
mod effects;

pub(super) async fn execute(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    a: AdmittedSession,
    r: wire::Request,
) -> Option<Vec<u8>> {
    if !a.continuity.selected_capabilities.contains(148)
    {
        return None;
    }
    let fence = a.item_fence?;
    if !matches!(
        fence.runtime_scope,
        crate::foundation::RuntimeScopeRefV1::Channel { .. }
    ) {
        return None;
    }
    {
        let runtime = owner.runtime.lock().await;
        if !super::battle_instinct_status::actor_is_current(&runtime, a) {
            return None;
        }
    }
    let rules = owner.wheel_ruleset.as_ref()?;
    let data = WheelGemData::embedded().ok()?;
    if rules.revision() != data.revision() {
        return None;
    }
    let read = owner
        .root
        .read_wheel_presentation(
            owner.character,
            owner.holder,
            fence,
            std::sync::Arc::clone(rules),
            owner.premium_coordinator?,
        )
        .await
        .ok()?;
    if read.allocation.wheel_revision != r.wheel_revision
        || read.atelier.as_ref().map_or(0, |a| a.revision) != r.atelier_revision
    {
        return None;
    }
    let actor = a.runtime_actor?;
    let removal = super::wheel_allocation::removal_permitted(owner, actor, a.game_session_id).await;
    let slots = WheelSlots::new(r.points).ok()?;
    let draft_result = read.validate_draft(rules, &slots, &r.vessels, removal);
    let eligible = oteryn_gameplay_progression::wheel::eligibility(
        rules,
        &read.vocation,
        read.level,
        read.promoted,
        read.premium_permitted,
    )
    .is_ok()
        && read.allocation.current;
    let current = effects::configured(
        &data,
        rules,
        &read.allocation,
        effects::Facts {
            vocation: &read.vocation,
            level: read.level,
            promoted: read.promoted,
            premium: read.premium_permitted,
        },
        read.atelier.as_ref(),
    )?;
    let draft = if draft_result.is_ok() {
        let mut allocation = read.allocation.clone();
        allocation.slots = slots;
        let mut atelier = read.atelier.clone();
        if let Some(a) = &mut atelier {
            for gem in a.gems.values_mut() {
                gem.vessel = r.vessels.contains(&Some(gem.id.to_be_bytes()));
            }
        }
        effects::configured(
            &data,
            rules,
            &allocation,
            effects::Facts {
                vocation: &read.vocation,
                level: read.level,
                promoted: read.promoted,
                premium: read.premium_permitted,
            },
            atelier.as_ref(),
        )?
    } else {
        Vec::new()
    };
    let (nodes, revelations) = catalogue::project(&data, rules, &read.vocation)?;
    let may_change;
    {
        let runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        may_change = runtime.assert_actor_spell_unreserved(actor).is_ok()
            && !states.has_pending_spell_commit(actor, a.game_session_id)
            && states
                .get(&runtime, actor, a.game_session_id)
                .is_some_and(|s| s.health > 0);
    }
    let after = owner.current_quest_fence(a.game_session_id).await.ok()??;
    if after.item_fence() != fence {
        return None;
    }

    {
        let runtime = owner.runtime.lock().await;
        if !super::battle_instinct_status::actor_is_current(&runtime, a) {
            return None;
        }
    }
    wire::Reply {
        request: r,
        catalogue: format!("{}@{}", data.revision(), data.source_candidate_sha256()),
        vocation: read.vocation,
        available: read.budget.available,
        unlock_points: read.budget.unlock_points,
        removal_permitted: removal,
        eligible: eligible && may_change,
        draft_valid: draft_result.is_ok() && may_change,
        reason: if !may_change {
            "The current actor cannot change the Wheel yet".into()
        } else {
            draft_result.err().map_or(String::new(), refusal_text)
        },
        nodes,
        revelations,
        current,
        draft,
    }
    .encode()
}
macro_rules! authority_methods {
    () => {
        fn wheel_presentation_available(&self) -> bool {
            false
        }
        fn wheel_presentation_query(
            &self,
            _a: $crate::gameplay_transport::connection::AdmittedSession,
            _r: oteryn_protocol_oteryn::wheel_presentation::Request,
        ) -> impl std::future::Future<Output = Option<Vec<u8>>> {
            async { None }
        }
    };
}
pub(super) use authority_methods;
macro_rules! composed_methods {
    () => {
        fn wheel_presentation_available(&self) -> bool {
            self.wheel_ruleset.as_ref().is_some_and(|r| {
                self.premium_coordinator.is_some()
                    && oteryn_gameplay_progression::wheel_gem_data::WheelGemData::embedded()
                        .is_ok_and(|d| d.revision() == r.revision())
            })
        }
        async fn wheel_presentation_query(
            &self,
            a: connection::AdmittedSession,
            r: oteryn_protocol_oteryn::wheel_presentation::Request,
        ) -> Option<Vec<u8>> {
            wheel_presentation::execute(self, a, r).await
        }
    };
}
pub(super) use composed_methods;

fn refusal_text(r: oteryn_gameplay_progression::wheel::WheelRefusal) -> String {
    use oteryn_gameplay_progression::wheel::{WheelIneligibility as E, WheelRefusal as R};
    match r {
        R::Exhausted => "Wait for the existing action interval",
        R::NotEligible(E::NoVocation) => "A supported vocation is required",
        R::NotEligible(E::Level) => "The character has not reached the required level",
        R::NotEligible(E::NotPromoted) => "Promotion is required",
        R::NotEligible(E::NotPremium) => "Current Premium permission is required",
        R::OverCapacity => "A slot exceeds its current capacity",
        R::OverPoints => "The draft exceeds the current point budget",
        R::NotAdjacent => "The draft needs a full open unlock link or more ring-unlock points",
        R::RemovalNotAtTemple => "Removing points is currently blocked by the combat-lock owner",
        R::RulesetNotCurrent => "The Wheel or selected vessels do not match the current ruleset",
        R::StaleRevision => "The committed Wheel changed; refresh",
        R::NoChange => "The draft has no changes",
    }
    .into()
}
