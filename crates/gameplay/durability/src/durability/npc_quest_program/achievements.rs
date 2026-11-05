//! Account achievements retain their existing in-transaction grant owner.
use super::*;
use crate::durability::account_achievement::{
    AchievementCatalogueLookup, AchievementGrantRequest, AchievementSourceEvent,
    FencedGrantingCharacter, record_achievement_grant,
};
use oteryn_gameplay_quests::quest::npc_rules::NpcRuleEffect;

pub(super) fn prepare(
    context: &NpcProgramContext,
    request: &QuestTransitionRequest,
) -> Result<BTreeMap<String, (String, AchievementGrantRequest)>, CharacterProgressionError> {
    let source = catalogue_source()?
        .program(&context.quest)
        .ok_or(CharacterProgressionError::InvalidInput)?;
    let mut names = std::collections::BTreeSet::new();
    for key in &context.selected_rule_keys {
        let rule = source
            .document
            .rules
            .iter()
            .find(|rule| &rule.key == key)
            .ok_or(CharacterProgressionError::InvalidInput)?;
        for effect in &rule.effects {
            if let NpcRuleEffect::Achievement { name } = effect {
                names.insert(name);
            }
        }
    }
    if names.is_empty() {
        return Ok(BTreeMap::new());
    }
    let entries = oteryn_gameplay_progression::achievement_catalogue::parse_shards(
        &oteryn_gameplay_progression::achievement_catalogue::SHARDS,
    )
    .map_err(|_| CharacterProgressionError::InvalidInput)?;
    let bytes = serde_json::to_vec(context).map_err(|_| CharacterProgressionError::InvalidInput)?;
    let (cause, ordinal) = request.cause.key();
    let mut prepared = BTreeMap::new();
    for name in names {
        let mut matches = entries.iter().filter(|(_, entry)| &entry.name == name);
        let (key, entry) = matches
            .next()
            .ok_or(CharacterProgressionError::InvalidInput)?;
        if matches.next().is_some() {
            return Err(CharacterProgressionError::InvalidInput);
        }
        let event = Sha256::new()
            .chain_update(b"npc-program-achievement/v1")
            .chain_update(&bytes)
            .chain_update(cause)
            .chain_update(ordinal.to_be_bytes())
            .chain_update(key.as_bytes())
            .finalize()
            .to_vec();
        prepared.insert(
            key.clone(),
            (
                name.clone(),
                AchievementGrantRequest {
                    achievement_key: key.clone(),
                    catalogue: if entry.retired {
                        AchievementCatalogueLookup::Retired
                    } else {
                        AchievementCatalogueLookup::Earnable {
                            revision: entry.revision.clone(),
                        }
                    },
                    source: AchievementSourceEvent {
                        kind: "oteryn:quest-npc-program".into(),
                        event_id: event,
                    },
                },
            ),
        );
    }
    Ok(prepared)
}

pub(super) async fn earn(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: crate::domain::CharacterId,
    plan: &NpcRulePlan,
    prepared: &BTreeMap<String, (String, AchievementGrantRequest)>,
) -> Result<(), DurabilityError> {
    for effect in &plan.effects {
        if let NpcRuleEffect::Achievement { name } = effect
            && !prepared
                .values()
                .any(|(prepared_name, _)| prepared_name == name)
        {
            return Err(DurabilityError::InvalidStoredState);
        }
    }
    // Consistent account-key order across simultaneous Characters on one account.
    for (name, grant) in prepared.values() {
        if !plan.effects.iter().any(|effect| matches!(effect, NpcRuleEffect::Achievement { name: selected } if selected == name)) {
            return Err(DurabilityError::InvalidStoredState);
        }
        let token = FencedGrantingCharacter::in_transaction(tx, character).await?;
        record_achievement_grant(tx, token, grant)
            .await?
            .map_err(|_| DurabilityError::InvalidStoredState)?;
    }
    Ok(())
}
