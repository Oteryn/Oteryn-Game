use std::collections::{BTreeMap, BTreeSet};

use super::{
    MAX_NPC_EFFECTS, NpcComparison, NpcRuleCatalog, NpcRuleEffect, NpcRuleError, NpcRuleGuard,
    NpcRuleTrigger, SOURCE_PIN, keyword,
};
use crate::quest::valid_quest_key;

fn invalid(detail: impl Into<String>) -> NpcRuleError {
    NpcRuleError::Invalid(detail.into())
}

fn sources(sources: &[String], quest: &str) -> bool {
    !sources.is_empty()
        && sources.iter().all(|source| {
            source
                .strip_prefix(SOURCE_PIN)
                .or_else(|| {
                    source
                        .strip_prefix("crystal:9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d:")
                        .filter(|reference| {
                            quest == "oteryn:quest.marlin_trophy_quest"
                                && reference.starts_with("data-global/npc/pemaret.lua:")
                        })
                })
                .or_else(|| super::hidden_threats::reference(quest, source))
                .is_some_and(|reference| {
                    reference
                        .rsplit_once(':')
                        .is_some_and(|(path, line)| !path.is_empty() && source_lines(line))
                })
        })
}

fn source_lines(line: &str) -> bool {
    if let Some((first, last)) = line.split_once('-') {
        return first
            .parse::<u32>()
            .is_ok_and(|first| first > 0 && last.parse::<u32>().is_ok_and(|last| last >= first));
    }
    line.parse::<u32>().is_ok_and(|number| number > 0)
}

fn comparison(op: NpcComparison, value: i64, maximum: Option<i64>) -> bool {
    match op {
        NpcComparison::Between => maximum.is_some_and(|high| value <= high),
        _ => maximum.is_none(),
    }
}

pub(super) fn catalogue(catalogue: &NpcRuleCatalog) -> Result<(), NpcRuleError> {
    let mut quests = BTreeSet::new();
    let mut keys = BTreeSet::new();
    for program in &catalogue.programs {
        let document = &program.document;
        if document.schema != 1
            || !valid_quest_key(&document.quest)
            || !quests.insert(document.program_key.as_ref().unwrap_or(&document.quest))
            || document
                .program_key
                .as_ref()
                .is_some_and(|key| !valid_quest_key(key))
            || document.rules.len() > 4096
            || document.tracks.len() > 4096
        {
            return Err(invalid("invalid NPC program schema, identity or size"));
        }
        if document.quest == "oteryn:quest.marlin_trophy_quest"
            && (document.source_files.len() != 1
                || document.source_files.iter().any(|file| {
                    file.revision != "9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d"
                        || file.path != "data-global/npc/pemaret.lua"
                        || file.sha256
                            != "7860edb917cf36e43f2f675b9a2f9b8bbbc0821051ef20e4a9443cf04649489c"
                }))
        {
            return Err(invalid("Pemaret source custody"));
        }
        if document.program_key.as_deref() == Some("oteryn:quest-npc-program.draconia")
            && (document.quest != "oteryn:quest.draconia_quest"
                || document.source_files.len() != 1
                || document.source_files.iter().any(|file| {
                    file.revision != "201189f871d23597b82caf88fd611454a2ddefb8"
                        || file.path != "data-global/npc/an_old_dragon_lord.lua"
                        || file.sha256
                            != "85016cc08856f49d94c507f43bc84f54801d3a2844946ae391015a0d8c84a052"
                }))
        {
            return Err(invalid("Draconia source custody"));
        }
        // Crystal 201189f data-global/npc/tamoril.lua:45-111: the calendar gate
        // and all unconditional acceptance resets belong to this exact source.
        if document.program_key.as_deref() == Some("oteryn:quest-npc-program.first-dragon-calendar")
            && (document.quest != "oteryn:quest.the_first_dragon_quest"
                || document.source_files.len() != 1
                || document.source_files.iter().any(|file| {
                    file.revision != "201189f871d23597b82caf88fd611454a2ddefb8"
                        || file.path != "data-global/npc/tamoril.lua"
                        || file.sha256
                            != "d5d55a74c2bb9ade413c23442924b46ee6e5b05c9daf73ed8259e67f04576969"
                }))
        {
            return Err(invalid("Tamoril source custody"));
        }
        if !super::hidden_threats::custody(document) {
            return Err(invalid("Corym source custody"));
        }
        let mut tracks = BTreeMap::new();
        for track in &document.tracks {
            if !valid_quest_key(&track.key)
                || track.quest != document.quest
                || !(track.min..=track.max).contains(&track.initial)
                || !sources(&track.sources, &document.quest)
                || tracks.insert(&track.key, track).is_some()
            {
                return Err(invalid(format!("invalid NPC track: {}", track.key)));
            }
        }
        for file in &document.source_files {
            if file.path.is_empty()
                || (file.revision != "201189f871d23597b82caf88fd611454a2ddefb8"
                    && !super::hidden_threats::file_is_pinned(
                        &document.quest,
                        &file.path,
                        &file.revision,
                        &file.sha256,
                    )
                    && !(document.quest == "oteryn:quest.marlin_trophy_quest"
                        && file.revision == "9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d"
                        && file.path == "data-global/npc/pemaret.lua"
                        && file.sha256
                            == "7860edb917cf36e43f2f675b9a2f9b8bbbc0821051ef20e4a9443cf04649489c"))
                || file.sha256.len() != 64
                || !file.sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
            {
                return Err(invalid("invalid source file custody"));
            }
        }
        for rule in &document.rules {
            if !valid_quest_key(&rule.key)
                || !keys.insert(&rule.key)
                || rule.quest != document.quest
                || !valid_quest_key(&rule.npc)
                || !rule.npc.starts_with("oteryn:npc.")
                || !sources(&rule.sources, &document.quest)
                || rule.effects.len() > MAX_NPC_EFFECTS
                || rule.guards.len() > 128
                || rule.topics.as_ref().is_some_and(|topics| {
                    topics.is_empty() || topics.iter().any(|topic| *topic < 0)
                })
                || (rule.trigger == NpcRuleTrigger::Say
                    && rule.keywords.is_empty()
                    && !rule.any_message
                    && !rule.match_player_name)
                || (rule.match_player_name
                    && (rule.trigger != NpcRuleTrigger::Say
                        || !rule.keywords.is_empty()
                        || rule.any_message))
                || rule.reply.len() > 32
                || rule
                    .reply
                    .iter()
                    .any(|line| line.is_empty() || line.len() > 2048 || line.contains('\0'))
                || rule
                    .inventory_failure_reply
                    .iter()
                    .any(|line| line.is_empty())
            {
                return Err(invalid(format!("invalid NPC rule: {}", rule.key)));
            }
            for word in &rule.keywords {
                keyword::validate(word, rule.keyword_mode)?;
            }
            // Exclusions represent the donor's preceding MsgContains branches,
            // independently of this rule's exact/substring selector.
            for word in &rule.exclude_keywords {
                keyword::validate(word, super::NpcKeywordMode::MsgContains)?;
            }
            for guard in &rule.guards {
                let valid = match guard {
                    NpcRuleGuard::Storage {
                        track,
                        op,
                        value,
                        maximum,
                    } => valid_quest_key(track) && comparison(*op, *value, *maximum),
                    NpcRuleGuard::StorageTime { track, op, .. } => {
                        valid_quest_key(track) && *op != NpcComparison::Between
                    }
                    NpcRuleGuard::Item {
                        item, count, op, ..
                    } => valid_quest_key(item) && *count > 0 && *op != NpcComparison::Between,
                    NpcRuleGuard::Money { amount, .. } => *amount > 0,
                    NpcRuleGuard::Equipped { slot, item } => {
                        valid_quest_key(item)
                            && [
                                "head", "armor", "legs", "feet", "left", "right", "necklace",
                                "ring", "ammo", "backpack",
                            ]
                            .contains(&slot.as_str())
                    }
                    NpcRuleGuard::Level { op, value } => {
                        *value >= 0 && *op != NpcComparison::Between
                    }
                    NpcRuleGuard::CalendarMonth { value } => (1..=12).contains(value),
                    NpcRuleGuard::CalendarWindow {
                        start_month,
                        start_day,
                        end_month,
                        end_day,
                        ..
                    } => {
                        super::calendar::valid_date(*start_month, *start_day)
                            && super::calendar::valid_date(*end_month, *end_day)
                    }
                    NpcRuleGuard::Vocation { value } => valid_quest_key(value),
                    NpcRuleGuard::Premium { .. } => true,
                    NpcRuleGuard::Sex { value } => *value <= 1,
                    NpcRuleGuard::Outfit { look_type, addon } => *look_type > 0 && *addon <= 3,
                };
                if !valid {
                    return Err(invalid(format!("invalid guard in {}", rule.key)));
                }
            }
            for effect in &rule.effects {
                let valid = match effect {
                    NpcRuleEffect::BigfootMatchmakerTarget => {
                        document.quest == "oteryn:quest.bigfoot_s_burden_quest"
                            && tracks.keys().any(|key| key.as_str() == "oteryn:quest-progress/quest/u9_60/bigfoots_burden/matchmaker_id_needed")
                    }
                    NpcRuleEffect::BigfootSoulMelody => {
                        document.quest == "oteryn:quest.bigfoot_s_burden_quest"
                            && (1..=7).all(|tone| tracks.contains_key(&format!("oteryn:quest-progress/quest/u9_60/bigfoots_burden/melody_tone{tone}")))
                    }
                    NpcRuleEffect::Set { track, value } => {
                        tracks.get(track).is_some_and(|definition| {
                            (definition.min..=definition.max).contains(value)
                        })
                    }
                    NpcRuleEffect::Add { track, .. } => tracks.contains_key(track),
                    NpcRuleEffect::SetNowPlus { track, value } => {
                        tracks.contains_key(track) && *value >= 0
                    }
                    NpcRuleEffect::SetTopic { value } => *value >= 0,
                    NpcRuleEffect::TakeItem { item, count, .. }
                    | NpcRuleEffect::GiveItem { item, count, .. } => {
                        valid_quest_key(item) && *count > 0
                    }
                    NpcRuleEffect::TakeMoney { amount, .. }
                    | NpcRuleEffect::GiveMoney { amount, .. } => *amount > 0,
                    NpcRuleEffect::Experience { amount } => *amount > 0 && *amount <= 100_000_000,
                    NpcRuleEffect::Achievement { name } => !name.is_empty() && name.len() <= 256,
                    NpcRuleEffect::Outfit { look_type, addon }
                    | NpcRuleEffect::OutfitAddon { look_type, addon } => {
                        *look_type > 0 && *addon <= 3
                    }
                    NpcRuleEffect::Teleport { position }
                    | NpcRuleEffect::MapMark { position, .. } => {
                        (0..=65535).contains(&position.x)
                            && (0..=65535).contains(&position.y)
                            && (-15..=0).contains(&position.floor)
                    }
                    NpcRuleEffect::MagicEffect { effect } => effect.starts_with("CONST_ME_"),
                    NpcRuleEffect::EndConversation { delay_ms } => *delay_ms <= 60_000,
                };
                if !valid {
                    return Err(invalid(format!(
                        "invalid or foreign effect in {}",
                        rule.key
                    )));
                }
            }
        }
    }
    Ok(())
}
