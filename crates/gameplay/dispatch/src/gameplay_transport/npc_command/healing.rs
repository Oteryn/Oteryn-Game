//! Free source NPC healing under the existing Channel vitals and canonical condition owners.
use super::*;
use crate::content::npc_catalogue::healing::{HealingProfile, profile};
use crate::domain::condition::{ConditionType, DotElement, StatusKind};

pub(super) fn apply(
    admission: &super::super::ComposedFreshAdmission<'_, '_, '_>,
    runtime: &mut ChannelRuntimeV1,
    actor: ExactActorRef,
    command: super::super::connection::UseCommand,
    intent: &NpcTalkIntent,
) -> Option<NpcTalkOutcome> {
    let session = command.game_session_id;
    let (_, _, key) = admission.states.npc_conversations.current_npc(
        &admission.states.chat,
        runtime,
        actor,
        session,
        intent,
    )?;
    let profile = profile(&key)?;
    let text = intent.text.trim_matches(' ');
    let greeting = matches!(
        profile,
        HealingProfile::RookGuard | HealingProfile::RookPriest
    ) && ["hi", "hello", "hail"]
        .iter()
        .any(|word| text.eq_ignore_ascii_case(word));
    let healing = if profile == HealingProfile::Full {
        text.eq_ignore_ascii_case("healing")
    } else {
        text.eq_ignore_ascii_case("heal") || text.eq_ignore_ascii_case("healing")
    };
    if !greeting && !healing {
        return None;
    }
    let mut sessions = admission.states.npc_conversations.sessions.lock().ok()?;
    let conversation = sessions.get_mut(&session)?;
    if conversation
        .healing_command
        .is_some_and(|previous| command.command_id <= previous)
    {
        return Some(NpcConversations::bank_reply(
            intent,
            "I have already attended to you.".into(),
        ));
    }
    let mut states = admission.states.spell_states.try_lock().ok()?;
    if states.has_pending_spell_commit(actor, session)
        || runtime.assert_actor_spell_unreserved(actor).is_err()
        || runtime
            .player_control_facts(actor, session)
            .ok()?
            .control_loss
            .is_some()
    {
        return None;
    }
    let before = states.get(runtime, actor, session)?;
    let vitals = before.vitals();
    if vitals.health == 0 || states.is_dead(actor) {
        return None;
    }
    let now = admission.owner_now();
    let store = before.owned_conditions();
    if !store.accepts_time(now.get()) {
        return None;
    }
    let fire = ConditionType::DamageOverTime(DotElement::Fire);
    let poison = ConditionType::DamageOverTime(DotElement::Poison);
    let energy = ConditionType::DamageOverTime(DotElement::Energy);
    let has = |kind| {
        store.instances().iter().any(|instance| {
            instance.definition().condition_type() == kind
                && instance
                    .remaining_duration_ms(now.get())
                    .is_some_and(|remaining| remaining != 0)
        })
    };
    if greeting && vitals.health >= 65 && !has(fire) && !has(poison) {
        return None;
    }
    let mut cures = Vec::new();
    let mut target = vitals.health;
    let line;
    if profile == HealingProfile::Full {
        if vitals.health < vitals.max_health {
            target = vitals.max_health;
            cures.extend([
                fire,
                poison,
                energy,
                ConditionType::DamageOverTime(DotElement::Bleeding),
                ConditionType::Status(StatusKind::Drunk),
                ConditionType::DamageOverTime(DotElement::Drown),
                ConditionType::DamageOverTime(DotElement::Cursed),
                ConditionType::DamageOverTime(DotElement::Freezing),
                ConditionType::DamageOverTime(DotElement::Dazzled),
                ConditionType::Paralysis,
            ]);
            line = "May the Three-Fold Path restore your strength.";
        } else {
            line = "You aren't looking really bad. I can only help in cases of real emergencies.";
        }
    } else if greeting {
        target = vitals.health.max(65);
        cures.extend([fire, poison]);
        line = "Let me heal your wounds.";
    } else if (profile != HealingProfile::PoisonGuard && has(fire))
        || has(poison)
        || (profile == HealingProfile::Standard && has(energy))
    {
        let kind = if profile != HealingProfile::PoisonGuard && has(fire) {
            fire
        } else if has(poison) {
            poison
        } else {
            energy
        };
        cures.push(kind);
        if profile != HealingProfile::Standard {
            target = vitals.health.max(65);
        }
        line = if kind == fire {
            "You are burning. Let me quench those flames."
        } else if kind == poison {
            "You are poisoned. I will help you."
        } else {
            "You are electrified, my child. Let me help you to stop trembling."
        };
    } else {
        let threshold = match profile {
            HealingProfile::Standard => 40,
            HealingProfile::RookPriest => 185,
            _ => 65,
        };
        if vitals.health < threshold && vitals.health < vitals.max_health {
            target = threshold;
            line = "You are hurt. I will heal your wounds.";
        } else {
            line = "You aren't looking that bad. I can only help in cases of real emergencies.";
        }
    }
    let amount = u64::from(target.min(vitals.max_health).saturating_sub(vitals.health));
    let mut conditions = store.clone();
    for kind in cures {
        conditions.remove_type(kind);
    }
    let conditions_changed = &conditions != store;
    let (mut next, gained) = before.after_health_gain(amount)?;
    if conditions_changed {
        next.apply_batch_conditions(store, &conditions).ok()?;
        if gained == 0 {
            next.advance_batch_revision().ok()?;
        }
    }
    if (gained != 0 || conditions_changed) && !states.commit(runtime, actor, session, next) {
        return None;
    }
    conversation.healing_command = Some(command.command_id);
    conversation.premium = None;
    conversation.bank = None;
    conversation.exchange = None;
    Some(NpcConversations::bank_reply(intent, line.into()))
}
