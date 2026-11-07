use super::*;
#[derive(Debug)]
pub(super) enum PlanError {
    Blocked(&'static str),
    Invalid,
}
pub(super) struct Prepared {
    pub source_key: &'static str,
    pub transition: &'static source::Transition,
    pub changes: Vec<rules::TrackChange>,
    pub quest_changes: Vec<rules::TrackChange>,
    pub costs: Vec<super::super::super::carried_cost::NpcItemCost>,
    pub grants: Vec<source::Grant>,
    pub achievements: Vec<String>,
    pub progress: Vec<(String, u32)>,
    pub magic_effect: Option<u16>,
    pub advance_notices: Vec<String>,
    pub hazard_selection: Option<u16>,
    pub primal_prize: bool,
    pub stampor: bool,
    pub close: bool,
    pub paid: bool,
    pub royal_kit: bool,
    pub mint_output: bool,
}
pub(super) fn achievement(name: &str) -> String {
    let mut key = String::from("oteryn:achievement/");
    let mut separator = false;
    for c in name.bytes() {
        if c.is_ascii_alphanumeric() {
            key.push(char::from(c.to_ascii_lowercase()));
            separator = false
        } else if !separator {
            key.push('_');
            separator = true
        }
    }
    key.trim_end_matches('_').to_owned()
}
pub(super) fn prepare(
    npc: &'static source::Npc,
    transition: &'static source::Transition,
    state: &Snapshot,
    native: &NativeGameplayState,
    effects: &NpcSourceEffects,
) -> Result<Prepared, PlanError> {
    let document = source::embedded().map_err(|_| PlanError::Invalid)?;
    let mut plan = Prepared {
        source_key: &transition.source_key,
        transition,
        changes: vec![],
        quest_changes: vec![],
        costs: vec![],
        grants: transition.grants.clone(),
        achievements: vec![],
        progress: vec![],
        magic_effect: None,
        advance_notices: Vec::new(),
        hazard_selection: None,
        primal_prize: false,
        stampor: false,
        close: false,
        paid: false,
        royal_kit: false,
        mint_output: false,
    };
    for effect in &transition.effects {
        match effect {
            source::Effect::SetTrack { storage, value } => {
                if !npc.owned_tracks.contains(storage) && effects.quest_catalogue.is_none() {
                    return Err(PlanError::Blocked("foreign_quest_state"));
                }
                let binding = document.binding(storage).ok_or(PlanError::Invalid)?;
                let before = state.track(storage).map_err(|_| PlanError::Invalid)?;
                let after = value.value(state).map_err(|_| PlanError::Invalid)?;
                let target = if npc.owned_tracks.contains(storage) {
                    &mut plan.changes
                } else {
                    &mut plan.quest_changes
                };
                target.push(rules::TrackChange {
                    key: binding.key.clone(),
                    quest: binding.quest.clone(),
                    before,
                    after,
                });
            }
            source::Effect::ConsumeItem { item, count } => {
                if *count == 0 {
                    return Err(PlanError::Invalid);
                }
                let policy = native
                    .item_policy_for_source_id(*item)
                    .ok_or(PlanError::Invalid)?;
                let record = policy.record();
                if !record.admission.materializable||!record.admission.legal_destinations.contains(&oteryn_gameplay_content::content::native_gameplay::NativeItemDestination::CharacterInventory){return Err(PlanError::Invalid)}
                plan.costs
                    .push(super::super::super::carried_cost::NpcItemCost {
                        definition: super::super::super::item_mint::TypedDefinitionRef {
                            family: record.production_definition.family.clone(),
                            production_key: record.production_definition.production_key.clone(),
                            revision_ref: record.production_definition.revision_ref.clone(),
                        },
                        count: *count,
                    });
            }
            source::Effect::GrantLookAddon { .. } => {} // exact source targets were linked and will be rechecked against native content
            source::Effect::MagicEffect { id } => {
                if plan.magic_effect.replace(*id).is_some() {
                    return Err(PlanError::Invalid);
                }
            }
            source::Effect::Achievement { name } => plan.achievements.push(achievement(name)),
            source::Effect::AchievementProgress { name, target } => {
                plan.progress.push((achievement(name), *target))
            }
            source::Effect::CloseConversation => plan.close = true,
            source::Effect::ConsumeMoney { gold } | source::Effect::ConsumeMoneyBank { gold } => {
                let Some((price, cash_minimum)) = source::paid::terms(&npc.npc, &transition.id)
                else {
                    return Err(PlanError::Blocked("paid_source"));
                };
                let cash_only = matches!(effect, source::Effect::ConsumeMoney { .. });
                if plan.paid
                    || *gold != price
                    || cash_only != (cash_minimum == price)
                    || effects.payment.is_none()
                {
                    return Err(PlanError::Blocked("paid_source"));
                }
                plan.paid = true;
            }
            source::Effect::MintItem { item, count } => {
                let expected = crate::durability::reward_claim_mint::npc_source::output_id(
                    &transition.source_key,
                );
                if expected != Some(*item) || *count != 1 || plan.mint_output {
                    return Err(PlanError::Blocked("source_item_output"));
                }
                plan.mint_output = true;
            }
            source::Effect::MintStoreDecorationKit {
                source_target,
                source_action,
                requires_free_store_inbox_entry,
            } => {
                if !plan.paid
                    || transition.id != "golden-buy-armor"
                    || *source_target != 31510
                    || source_action.is_some()
                    || !requires_free_store_inbox_entry
                {
                    return Err(PlanError::Blocked("store_decoration_kit"));
                }
                plan.royal_kit = true;
            }
            source::Effect::ConditionDamage {
                condition,
                delayed,
                ticks,
                interval_ms,
                damage_per_tick,
            } => {
                if npc.npc != "oteryn:npc.ajax"
                    || transition.source_key != "oteryn:npc-cosmetic/ajax/barbarian-fire"
                    || condition != "FIRE"
                    || !delayed
                    || *ticks != 10
                    || *interval_ms != 2000
                    || *damage_per_tick != 10
                {
                    return Err(PlanError::Invalid);
                }
                if effects.actor_effect.is_none() {
                    return Err(PlanError::Blocked("native_condition"));
                }
            }
            source::Effect::ConsumeOwnerPoints { .. }
            | source::Effect::SpikeUpperKillAccept
            | source::Effect::SpikeUpperKillReport => {
                return Err(PlanError::Blocked("spike_source"));
            }
            source::Effect::SetHazardLevel { owner, selection } => {
                if npc.npc != "oteryn:npc.gnomadness"
                    || !matches!(
                        transition.source_key.as_str(),
                        "oteryn:npc-cosmetic/gnomadness/ripptor-prize"
                            | "oteryn:npc-cosmetic/gnomadness/ripptor-select"
                    )
                    || owner != "hazard.gnomprona-gardens"
                    || selection != "hazard_level"
                {
                    return Err(PlanError::Blocked("source_owner_reward"));
                }
                let value = state.selected.ok_or(PlanError::Invalid)?;
                if !state
                    .hazard_can_select(owner, value)
                    .map_err(|_| PlanError::Invalid)?
                {
                    return Err(PlanError::Invalid);
                }
                let value = u16::try_from(value).map_err(|_| PlanError::Invalid)?;
                if plan.hazard_selection.replace(value).is_some() {
                    return Err(PlanError::Invalid);
                }
            }
            source::Effect::GrantMount { id } => {
                if npc.npc == "oteryn:npc.chondur"
                    && transition.source_key == "oteryn:npc-cosmetic/chondur/stampor-spirit"
                    && *id == 11
                {
                    let exact_costs = [(12312, 50), (12314, 30), (12313, 100)];
                    let actual: Vec<_> = transition
                        .effects
                        .iter()
                        .filter_map(|effect| {
                            if let source::Effect::ConsumeItem { item, count } = effect {
                                Some((*item, *count))
                            } else {
                                None
                            }
                        })
                        .collect();
                    if actual != exact_costs || plan.stampor {
                        return Err(PlanError::Invalid);
                    }
                    plan.stampor = true;
                    continue;
                }
                if npc.npc != "oteryn:npc.gnomadness"
                    || transition.source_key != "oteryn:npc-cosmetic/gnomadness/ripptor-prize"
                    || *id != 202
                {
                    return Err(PlanError::Blocked("source_owner_reward"));
                }
                plan.primal_prize = true;
            }
            source::Effect::SetKv { scope, key, value } => {
                if !plan.primal_prize
                    || scope != "primal-ordeal"
                    || key != "received-prize"
                    || !value
                {
                    return Err(PlanError::Blocked("source_owner_reward"));
                }
            }
            source::Effect::TextMessage { class, text } => {
                let expected = match transition.source_key.as_str() {
                    "oteryn:npc-cosmetic/jack_springer/inquisition-hand" => {
                        "Congratulations! You received the Hand of the Inquisition Outfit."
                    }
                    "oteryn:npc-cosmetic/gnomadness/ripptor-prize" => {
                        "Congratulations you received the Noxious Ripptor mount."
                    }
                    _ => return Err(PlanError::Blocked("source_notice")),
                };
                if class != "EVENT_ADVANCE" || text != expected || !plan.advance_notices.is_empty()
                {
                    return Err(PlanError::Invalid);
                }
                plan.advance_notices.push(text.clone());
            }
        }
    }
    if plan.primal_prize
        && (plan.hazard_selection.is_none_or(|selected| selected < 6)
            || state.primal.is_none_or(|facts| facts.received_prize)
            || !plan.mint_output)
    {
        return Err(PlanError::Invalid);
    }
    if plan.changes.len() > 8 || plan.grants.len() > 2 {
        return Err(PlanError::Invalid);
    }
    Ok(plan)
}
