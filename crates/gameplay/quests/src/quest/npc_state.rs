//! Bind source-only state branches to the existing fenced QuestState writer.
use super::npc_rules::{NpcComparison, NpcRule, NpcRuleCatalog, NpcRuleEffect, NpcRuleGuard};
use super::{QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTransition};
use std::collections::BTreeMap;
use std::sync::OnceLock;

pub fn load() -> Result<&'static NpcRuleCatalog, &'static str> {
    static BOOK: OnceLock<Result<NpcRuleCatalog, String>> = OnceLock::new();
    BOOK.get_or_init(|| {
        NpcRuleCatalog::from_json_documents(&[include_str!(
            "../../../../../content/dialogues/quest-rules/quest-batch3-a-fathers_burden.json"
        )])
        .map_err(|error| error.to_string())
    })
    .as_ref()
    .map_err(String::as_str)
}

#[must_use]
pub fn transition_key(rule: &str) -> String {
    rule.replacen("oteryn:quest-npc/", "oteryn:quest-transition/state/", 1)
}

/// Every predicate travels into the durable evaluator, including tracks only read
/// by a guard. A rule with an unsupported dependency receives no executable key.
fn transition(rule: &NpcRule) -> Option<QuestTransition> {
    let mut effects = BTreeMap::new();
    let mut writes = false;
    for guard in &rule.guards {
        let NpcRuleGuard::Storage {
            track,
            op,
            value,
            maximum,
        } = guard
        else {
            return None;
        };
        let from = match op {
            NpcComparison::Eq => QuestComparison::Eq(*value),
            NpcComparison::Ne => QuestComparison::Ne(*value),
            NpcComparison::Lt => QuestComparison::Lt(*value),
            NpcComparison::Le => QuestComparison::Le(*value),
            NpcComparison::Gt => QuestComparison::Gt(*value),
            NpcComparison::Ge => QuestComparison::Ge(*value),
            NpcComparison::Between => QuestComparison::Between(*value, (*maximum)?),
        };
        // Multiple predicates cannot be silently reduced to one comparison.
        if effects
            .insert(
                track.clone(),
                QuestEffect {
                    track: track.clone(),
                    from,
                    effect: QuestEffectKind::Add(0),
                },
            )
            .is_some()
        {
            return None;
        }
    }
    for effect in &rule.effects {
        let (track, effect) = match effect {
            NpcRuleEffect::Set { track, value } => (track, QuestEffectKind::Set(*value)),
            NpcRuleEffect::Add { track, value } => (track, QuestEffectKind::Add(*value)),
            NpcRuleEffect::SetTopic { .. } => continue,
            _ => return None,
        };
        writes = true;
        let entry = effects.entry(track.clone()).or_insert_with(|| QuestEffect {
            track: track.clone(),
            from: QuestComparison::Any,
            effect: QuestEffectKind::Add(0),
        });
        // Combining sequential writes requires preserving their intermediate checks.
        if entry.effect != QuestEffectKind::Add(0) {
            return None;
        }
        entry.effect = effect;
    }
    writes.then(|| QuestTransition {
        key: transition_key(&rule.key),
        quest: rule.quest.clone(),
        effects: effects.into_values().collect(),
        completes: false,
        experience: None,
    })
}

pub(super) fn extend(catalogue: QuestStateCatalogue) -> Result<QuestStateCatalogue, String> {
    let book = load()?;
    let mut transitions: Vec<_> = catalogue.transitions.values().cloned().collect();
    for program in book.programs() {
        for rule in &program.document.rules {
            if let Some(transition) = transition(rule) {
                transitions.push(transition);
            }
        }
    }
    // Keep the existing track owner, initial values and bounds. Source plans are
    // selections; only this admitted catalogue can authorize durable values.
    QuestStateCatalogue::new(
        catalogue.content_revision,
        catalogue.tracks.into_values().collect(),
        transitions,
    )
    .map_err(|error| format!("{error:?}"))
}
