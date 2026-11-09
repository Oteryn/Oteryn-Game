use std::collections::{BTreeMap, BTreeSet};

use super::{
    MAX_NPC_EFFECTS, MAX_NPC_TRACK_CHANGES, NpcInventorySnapshot, NpcItemKey, NpcKeywordMode,
    NpcRule, NpcRuleCatalog, NpcRuleEffect, NpcRuleError, NpcRuleGuard, NpcRuleInput, NpcRulePlan,
    NpcRuleProgram, NpcTrackChange, NpcTrackDefinition, keyword,
};

struct Staged<'a> {
    definitions: BTreeMap<&'a str, &'a NpcTrackDefinition>,
    original: &'a BTreeMap<String, i64>,
    tracks: BTreeMap<String, i64>,
    observed: BTreeMap<String, i64>,
    changed: BTreeSet<String>,
    inventory: NpcInventorySnapshot,
    outfits: BTreeMap<u16, u8>,
    topic: i32,
}

impl<'a> Staged<'a> {
    fn new(program: &'a NpcRuleProgram, input: &'a NpcRuleInput<'_>) -> Self {
        Self {
            definitions: program
                .document
                .tracks
                .iter()
                .map(|track| (track.key.as_str(), track))
                .collect(),
            original: input.tracks,
            tracks: input.tracks.clone(),
            observed: BTreeMap::new(),
            changed: BTreeSet::new(),
            inventory: input.inventory.clone(),
            outfits: input.outfits.clone(),
            topic: input.topic,
        }
    }

    fn track(&mut self, key: &str) -> Result<i64, NpcRuleError> {
        let definition = self.definitions.get(key);
        let original = self
            .original
            .get(key)
            .copied()
            .or_else(|| definition.map(|track| track.initial))
            .ok_or_else(|| NpcRuleError::MissingTrack(key.to_owned()))?;
        // A source's absent sentinel is a read default, not permission to store
        // that value outside the native owner's range. Explicit rows stay strict.
        if self.original.contains_key(key)
            && definition.is_some_and(|track| !(track.min..=track.max).contains(&original))
        {
            return Err(NpcRuleError::Invalid(format!(
                "track outside bounds: {key}"
            )));
        }
        self.observed.entry(key.to_owned()).or_insert(original);
        Ok(self.tracks.get(key).copied().unwrap_or(original))
    }

    fn set_track(&mut self, key: &str, value: i64) -> Result<(), NpcRuleError> {
        self.track(key)?;
        let definition = self
            .definitions
            .get(key)
            .ok_or_else(|| NpcRuleError::MissingTrack(key.to_owned()))?;
        if !(definition.min..=definition.max).contains(&value) {
            return Err(NpcRuleError::Invalid(format!(
                "write outside bounds: {key}"
            )));
        }
        self.tracks.insert(key.to_owned(), value);
        self.changed.insert(key.to_owned());
        if self.changed.len() > MAX_NPC_TRACK_CHANGES {
            return Err(NpcRuleError::LimitExceeded);
        }
        Ok(())
    }

    fn guard(
        &mut self,
        guard: &NpcRuleGuard,
        input: &NpcRuleInput<'_>,
    ) -> Result<bool, NpcRuleError> {
        Ok(match guard {
            NpcRuleGuard::Storage {
                track,
                op,
                value,
                maximum,
            } => op.holds(self.track(track)?, *value, *maximum),
            NpcRuleGuard::StorageTime {
                track,
                op,
                offset_seconds,
            } => {
                let threshold = input
                    .now_epoch_seconds
                    .checked_add(*offset_seconds)
                    .ok_or_else(|| NpcRuleError::Arithmetic("clock threshold".into()))?;
                op.holds(self.track(track)?, threshold, None)
            }
            NpcRuleGuard::Item {
                item,
                count,
                subtype,
                op,
            } => {
                let actual = i64::try_from(item_count(&self.inventory, item, *subtype)?)
                    .map_err(|_| NpcRuleError::Arithmetic("item count".into()))?;
                op.holds(actual, i64::from(*count), None)
            }
            NpcRuleGuard::Money {
                amount,
                include_bank,
            } => money_available(&self.inventory, *include_bank)? >= *amount,
            NpcRuleGuard::Equipped { slot, item } => {
                self.inventory.equipped.get(slot) == Some(item)
            }
            NpcRuleGuard::Level { op, value } => op.holds(i64::from(input.level), *value, None),
            NpcRuleGuard::CalendarMonth { value } => input.calendar_month == *value,
            NpcRuleGuard::Vocation { value } => input.vocation == value,
            NpcRuleGuard::Sex { value } => input.sex == *value,
            NpcRuleGuard::Outfit { look_type, addon } => self
                .outfits
                .get(look_type)
                .is_some_and(|owned| *owned & *addon == *addon),
        })
    }

    /// `false` means an optional source removeItem failed and made no change.
    fn effect(&mut self, effect: &NpcRuleEffect, now: i64) -> Result<bool, NpcRuleError> {
        match effect {
            NpcRuleEffect::BigfootMatchmakerTarget | NpcRuleEffect::BigfootSoulMelody => {
                return Err(NpcRuleError::MissingNativeResolution(
                    "unresolved native effect".into(),
                ));
            }
            NpcRuleEffect::Set { track, value } => self.set_track(track, *value)?,
            NpcRuleEffect::Add { track, value } => {
                let result = self
                    .track(track)?
                    .checked_add(*value)
                    .ok_or_else(|| NpcRuleError::Arithmetic(track.to_owned()))?;
                self.set_track(track, result)?;
            }
            NpcRuleEffect::SetNowPlus { track, value } => {
                let result = now
                    .checked_add(*value)
                    .ok_or_else(|| NpcRuleError::Arithmetic(track.to_owned()))?;
                self.set_track(track, result)?;
            }
            NpcRuleEffect::SetTopic { value } => self.topic = *value,
            NpcRuleEffect::TakeItem {
                item,
                count,
                subtype,
                optional,
            } => {
                if item_count(&self.inventory, item, *subtype)? < u64::from(*count) {
                    if *optional {
                        return Ok(false);
                    }
                    return Err(NpcRuleError::MissingItems(item.to_owned()));
                }
                take_items(&mut self.inventory, item, *subtype, *count)?;
            }
            NpcRuleEffect::GiveItem {
                item,
                count,
                subtype,
            } => {
                let key = NpcItemKey {
                    item: item.to_owned(),
                    subtype: *subtype,
                };
                let entry = self.inventory.items.entry(key).or_default();
                *entry = entry
                    .checked_add(*count)
                    .ok_or_else(|| NpcRuleError::Arithmetic(item.to_owned()))?;
            }
            NpcRuleEffect::TakeMoney {
                amount,
                include_bank,
            } => {
                if money_available(&self.inventory, *include_bank)? < *amount {
                    return Err(NpcRuleError::MissingMoney);
                }
                // Crystal data/libs/functions/player.lua:201-227 spends the
                // inventory first, then debits only the remainder from the bank.
                let carried = self.inventory.money.min(*amount);
                self.inventory.money -= carried;
                self.inventory.bank -= amount - carried;
            }
            NpcRuleEffect::GiveMoney {
                amount,
                include_bank,
            } => {
                let destination = if *include_bank {
                    &mut self.inventory.bank
                } else {
                    &mut self.inventory.money
                };
                *destination = destination
                    .checked_add(*amount)
                    .ok_or_else(|| NpcRuleError::Arithmetic("money".into()))?;
            }
            NpcRuleEffect::Outfit { look_type, addon }
            | NpcRuleEffect::OutfitAddon { look_type, addon } => {
                *self.outfits.entry(*look_type).or_default() |= *addon;
            }
            NpcRuleEffect::Experience { .. }
            | NpcRuleEffect::Achievement { .. }
            | NpcRuleEffect::Teleport { .. }
            | NpcRuleEffect::MagicEffect { .. }
            | NpcRuleEffect::MapMark { .. }
            | NpcRuleEffect::EndConversation { .. } => {}
        }
        Ok(true)
    }
}

fn item_count(
    inventory: &NpcInventorySnapshot,
    item: &str,
    subtype: Option<u16>,
) -> Result<u64, NpcRuleError> {
    inventory
        .items
        .iter()
        .filter(|(key, _)| {
            key.item == item && subtype.is_none_or(|wanted| key.subtype == Some(wanted))
        })
        .try_fold(0_u64, |sum, (_, count)| {
            sum.checked_add(u64::from(*count))
                .ok_or_else(|| NpcRuleError::Arithmetic(item.to_owned()))
        })
}

fn take_items(
    inventory: &mut NpcInventorySnapshot,
    item: &str,
    subtype: Option<u16>,
    count: u32,
) -> Result<(), NpcRuleError> {
    let mut remaining = count;
    for (key, available) in &mut inventory.items {
        if key.item != item || subtype.is_some_and(|wanted| key.subtype != Some(wanted)) {
            continue;
        }
        let used = remaining.min(*available);
        *available -= used;
        remaining -= used;
        if remaining == 0 {
            return Ok(());
        }
    }
    Err(NpcRuleError::MissingItems(item.to_owned()))
}

fn money_available(
    inventory: &NpcInventorySnapshot,
    include_bank: bool,
) -> Result<u64, NpcRuleError> {
    inventory
        .money
        .checked_add(if include_bank { inventory.bank } else { 0 })
        .ok_or_else(|| NpcRuleError::Arithmetic("money balance".into()))
}

fn words_match(rule: &NpcRule, input: &NpcRuleInput<'_>) -> Result<bool, NpcRuleError> {
    for word in &rule.exclude_keywords {
        if keyword::matches(input.message, word, NpcKeywordMode::MsgContains)? {
            return Ok(false);
        }
    }
    if rule.any_message || rule.keywords.is_empty() {
        return Ok(true);
    }
    for word in &rule.keywords {
        if keyword::matches(input.message, word, rule.keyword_mode)? {
            return Ok(true);
        }
    }
    Ok(false)
}

pub(super) fn plan(
    catalogue: &NpcRuleCatalog,
    input: &NpcRuleInput<'_>,
    resolutions: &BTreeMap<String, Vec<NpcRuleEffect>>,
    storage_only: bool,
) -> Result<Option<NpcRulePlan>, NpcRuleError> {
    if input.topic < 0 || input.message.len() > 4096 || input.inventory.items.len() > 4096 {
        return Err(NpcRuleError::Invalid("invalid NPC input".into()));
    }
    for program in &catalogue.programs {
        if !program
            .document
            .rules
            .iter()
            .any(|rule| rule.npc == input.npc && rule.trigger == input.trigger)
        {
            continue;
        }
        let mut state = Staged::new(program, input);
        let mut selected_rule_keys = Vec::new();
        let mut reply = Vec::new();
        let mut effects = Vec::new();
        let mut inventory_failure_reply = Vec::new();
        for rule in &program.document.rules {
            if rule.npc != input.npc
                || rule.trigger != input.trigger
                || rule
                    .topics
                    .as_ref()
                    .is_some_and(|topics| !topics.contains(&state.topic))
                || !words_match(rule, input)?
            {
                continue;
            }
            let mut eligible = true;
            for guard in &rule.guards {
                if storage_only
                    && !matches!(
                        guard,
                        NpcRuleGuard::Storage { .. }
                            | NpcRuleGuard::StorageTime { .. }
                            | NpcRuleGuard::CalendarMonth { .. }
                            | NpcRuleGuard::Level { .. }
                    )
                {
                    return Err(NpcRuleError::MissingNativeResolution(rule.key.clone()));
                }
                if !state.guard(guard, input)? {
                    eligible = false;
                    break;
                }
            }
            if !eligible {
                continue;
            }
            for effect in &rule.effects {
                if storage_only
                    && !matches!(
                        effect,
                        NpcRuleEffect::Set { .. }
                            | NpcRuleEffect::Add { .. }
                            | NpcRuleEffect::SetNowPlus { .. }
                            | NpcRuleEffect::SetTopic { .. }
                            | NpcRuleEffect::EndConversation { .. }
                            | NpcRuleEffect::Experience { .. }
                    )
                {
                    return Err(NpcRuleError::MissingNativeResolution(rule.key.clone()));
                }
                let concrete = match effect {
                    NpcRuleEffect::BigfootMatchmakerTarget | NpcRuleEffect::BigfootSoulMelody => {
                        resolve_native(effect, &rule.key, resolutions)?
                    }
                    _ => std::slice::from_ref(effect),
                };
                for effect in concrete {
                    if state.effect(effect, input.now_epoch_seconds)? {
                        effects.push(effect.clone());
                        if effects.len() > MAX_NPC_EFFECTS {
                            return Err(NpcRuleError::LimitExceeded);
                        }
                    }
                }
            }
            selected_rule_keys.push(rule.key.clone());
            reply.extend_from_slice(&rule.reply);
            if reply.len() > 32 {
                return Err(NpcRuleError::LimitExceeded);
            }
            if !rule.inventory_failure_reply.is_empty() {
                inventory_failure_reply.clone_from(&rule.inventory_failure_reply);
            }
            if !rule.continue_matching {
                break;
            }
        }
        if selected_rule_keys.is_empty() {
            continue;
        }
        let mut track_changes = Vec::with_capacity(state.changed.len());
        for key in &state.changed {
            let before = state
                .observed
                .get(key)
                .copied()
                .ok_or_else(|| NpcRuleError::MissingTrack(key.clone()))?;
            let after = state
                .tracks
                .get(key)
                .copied()
                .ok_or_else(|| NpcRuleError::MissingTrack(key.clone()))?;
            track_changes.push(NpcTrackChange {
                key: key.clone(),
                before,
                after,
            });
        }
        return Ok(Some(NpcRulePlan {
            quest: program.document.quest.clone(),
            program_digest: program.digest,
            selected_rule_keys,
            reply,
            next_topic: state.topic,
            observed_tracks: state.observed,
            track_changes,
            effects,
            inventory_before: input.inventory.clone(),
            inventory_after: state.inventory,
            inventory_failure_reply,
        }));
    }
    Ok(None)
}

fn resolve_native<'a>(
    effect: &NpcRuleEffect,
    rule: &str,
    resolutions: &'a BTreeMap<String, Vec<NpcRuleEffect>>,
) -> Result<&'a [NpcRuleEffect], NpcRuleError> {
    let resolved = resolutions
        .get(rule)
        .ok_or_else(|| NpcRuleError::MissingNativeResolution(rule.to_owned()))?;
    let prefix = "oteryn:quest-progress/quest/u9_60/bigfoots_burden/";
    // Crystal gnomeral.lua:85 and gnomelvis.lua:75-77. These are the only
    // accepted writes; the server RNG owner cannot inject a different effect.
    let valid = match effect {
        NpcRuleEffect::BigfootMatchmakerTarget => matches!(resolved.as_slice(),
            [NpcRuleEffect::Set { track, value }]
            if track == &format!("{prefix}matchmaker_id_needed") && (15809..=15815).contains(value)),
        NpcRuleEffect::BigfootSoulMelody => {
            resolved.len() == 7
                && resolved.iter().enumerate().all(|(index, effect)| {
                    matches!(effect, NpcRuleEffect::Set { track, value }
                if track == &format!("{prefix}melody_tone{}", index + 1) && (1..=4).contains(value))
                })
        }
        _ => false,
    };
    if !valid {
        return Err(NpcRuleError::Invalid(format!(
            "invalid native resolution: {rule}"
        )));
    }
    Ok(resolved)
}
