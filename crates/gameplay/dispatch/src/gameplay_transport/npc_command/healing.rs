//! Free source NPC healing under the existing Channel vitals and canonical condition owners.
use super::*;
use crate::content::npc_catalogue::healing::{HealingProfile, catalogue, profile};
use crate::domain::condition::{ConditionType, DotElement, StatusKind};

pub(super) fn apply(
    admission: &super::super::ComposedFreshAdmission<'_, '_, '_>,
    runtime: &mut ChannelRuntimeV1,
    actor: ExactActorRef,
    command: super::super::connection::UseCommand,
    intent: &NpcTalkIntent,
) -> Option<NpcTalkOutcome> {
    let session = command.game_session_id;
    let context = admission.states.npc_conversations.current_npc(
        &admission.states.chat,
        runtime,
        actor,
        session,
        intent,
    )?;
    let key = &context.2;
    let quest = catalogue().and_then(|catalogue| catalogue.quest_emergency(key));
    #[cfg(test)]
    let quest = admission
        .states
        .npc_conversations
        .fixture_healers
        .get()
        .and_then(|catalogue| catalogue.quest_emergency(key))
        .or(quest);
    let profile = if quest.is_some() {
        HealingProfile::QuestEmergency
    } else {
        profile(key)?
    };
    let text = intent.text.trim_matches(' ');
    let greeting = matches!(
        profile,
        HealingProfile::RookGuard | HealingProfile::RookPriest
    ) && ["hi", "hello", "hail"]
        .iter()
        .any(|word| text.eq_ignore_ascii_case(word));
    let healing = match profile {
        HealingProfile::Full => text.eq_ignore_ascii_case("healing"),
        HealingProfile::QuestEmergency => oteryn_gameplay_npc::greeting::words(text)
            .iter()
            .any(|word| word == "heal"),
        _ => text.eq_ignore_ascii_case("heal") || text.eq_ignore_ascii_case("healing"),
    };
    if !greeting && !healing {
        return None;
    }
    // Crystal 201189f8 data-global/npc/hairycles.lua:223-239.
    if let Some(quest) = quest {
        let eligible = admission
            .states
            .quest_sessions
            .lock()
            .is_ok_and(|sessions| {
                sessions
                    .get(&session)
                    .and_then(|state| state.copy.as_ref())
                    .and_then(|copy| copy.tracks().get(&quest.track))
                    .is_some_and(|value| *value >= quest.minimum)
            });
        if !eligible {
            return Some(NpcConversations::bank_reply(intent, quest.refusal.clone()));
        }
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
    let mut effect = None;
    if let Some(quest) = quest {
        line = if vitals.health < quest.health_floor {
            target = quest.health_floor;
            effect = Some(&quest.health_effect);
            None
        } else if has(fire) {
            cures.push(fire);
            effect = Some(&quest.fire_effect);
            None
        } else if has(poison) {
            cures.push(poison);
            effect = Some(&quest.poison_effect);
            None
        } else {
            Some(quest.refusal.as_str())
        };
    } else if profile == HealingProfile::Full {
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
            line = Some("May the Three-Fold Path restore your strength.");
        } else {
            line = Some(
                "You aren't looking really bad. I can only help in cases of real emergencies.",
            );
        }
    } else if greeting {
        target = vitals.health.max(65);
        cures.extend([fire, poison]);
        line = Some("Let me heal your wounds.");
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
        line = Some(if kind == fire {
            "You are burning. Let me quench those flames."
        } else if kind == poison {
            "You are poisoned. I will help you."
        } else {
            "You are electrified, my child. Let me help you to stop trembling."
        });
    } else {
        let threshold = match profile {
            HealingProfile::Standard => 40,
            HealingProfile::RookPriest => 185,
            _ => 65,
        };
        if vitals.health < threshold && vitals.health < vitals.max_health {
            target = threshold;
            line = Some("You are hurt. I will heal your wounds.");
        } else {
            line =
                Some("You aren't looking that bad. I can only help in cases of real emergencies.");
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
    drop(states);
    drop(sessions);
    if let Some(effect) = effect {
        super::magic_effects::publish(
            admission,
            runtime,
            actor,
            command,
            intent,
            &context,
            &[oteryn_gameplay_quests::quest::npc_magic::Cue {
                effect: effect.clone(),
                target: Default::default(),
            }],
        );
    }
    let mut outcome = NpcConversations::bank_reply(intent, String::new());
    if let Some(conversation) = outcome.conversation.as_mut() {
        conversation.lines = line.into_iter().map(str::to_owned).collect();
    }
    Some(outcome)
}
