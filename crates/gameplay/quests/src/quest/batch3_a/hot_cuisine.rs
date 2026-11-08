//! Hot Cuisine's calendar and fifteen atomic ingredient exchanges.
//!
//! The ordinary conversation is data in `content/dialogues/quest-rules`. This
//! planner handles only the donor's `yes` confirmation for a selected dish. It
//! delegates inventory and staged-write evaluation to the shared NPC planner.

use std::collections::BTreeMap;

use serde::Deserialize;
use sha2::{Digest, Sha256};

#[path = "hot_cuisine/calendar.rs"]
mod calendar;
use crate::quest::npc_rules::{
    NpcComparison, NpcKeywordMode, NpcRule, NpcRuleCatalog, NpcRuleEffect, NpcRuleError,
    NpcRuleGuard, NpcRuleInput, NpcRulePlan, NpcRuleProgram, NpcRuleTrigger,
};
use calendar::{YearMonth, can_cook, spends_bonus};

pub const QUEST: &str = "oteryn:quest.hot_cuisine_quest";
pub const NPC: &str = "oteryn:npc.jean_pierre";
pub const EMBEDDED: &str = include_str!("../../../../../../content/quests/hot-cuisine-batch3.json");

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Recipe {
    index: u8,
    dish_number: u8,
    topic: i32,
    ingredients: Vec<NpcRuleEffect>,
    reward: NpcRuleEffect,
    last_track: String,
    cook_count_track: String,
    bonus_used_track: String,
    next_dish: i64,
    quest_log: i64,
    opens_cookbook_door: bool,
    completes_menu: bool,
    reply: Vec<String>,
    sources: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cookbook {
    pub item: String,
    pub text: String,
    pub sha256: String,
    pub sources: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HotCuisineProgram {
    schema: u32,
    quest: String,
    calendar: String,
    recipes: Vec<Recipe>,
    quest_start_track: String,
    current_dish_track: String,
    quest_log_track: String,
    cookbook_door_track: String,
    bonus_year_track: String,
    missing_ingredients_reply: Vec<String>,
    already_cooked_reply: Vec<String>,
    sources: Vec<String>,
    pub books: Vec<Cookbook>,
}

/// All dates use UTC, per coordinator ANSWER:
/// https://github.com/Oteryn/Oteryn-Game/pull/2843#issuecomment-6589517205.
/// The clock and saved dates come from one immutable snapshot. Physical exchanges
/// remain disabled until the separate NPC exchange owner is approved.
#[derive(Clone, Debug)]
pub struct HotCuisineCalendar {
    pub current: YearMonth,
    pub last: [Option<YearMonth>; 15],
}

impl HotCuisineProgram {
    pub fn load() -> Result<Self, NpcRuleError> {
        let program: Self = serde_json::from_str(EMBEDDED)
            .map_err(|error| NpcRuleError::Json(error.to_string()))?;
        if program.schema != 1
            || program.quest != QUEST
            || program.calendar != "UTC"
            || program.recipes.len() != 15
            || program.sources.is_empty()
        {
            return Err(NpcRuleError::Invalid("Hot Cuisine document".into()));
        }
        for (index, recipe) in program.recipes.iter().enumerate() {
            if usize::from(recipe.index) != index + 1
                || usize::from(recipe.dish_number) != index + 2
                || recipe.topic != i32::from(recipe.index) * 2 + 3
                || recipe.ingredients.is_empty()
                || recipe.reply.is_empty()
                || recipe.sources.is_empty()
                || recipe.ingredients.iter().any(|ingredient| {
                    !matches!(ingredient, NpcRuleEffect::TakeItem { count, optional: false, .. } if *count > 0)
                })
                || !matches!(recipe.reward, NpcRuleEffect::GiveItem { count: 1, .. })
            {
                return Err(NpcRuleError::Invalid(format!(
                    "Hot Cuisine recipe {}",
                    recipe.index
                )));
            }
        }
        Ok(program)
    }

    /// The durable owner uses this same digest when revalidating a native plan.
    #[must_use]
    pub fn program_digest(&self, generic_digest: [u8; 32]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(generic_digest);
        hasher.update(EMBEDDED.as_bytes());
        hasher.finalize().into()
    }

    /// M17: decode both current and saved dates from the owner's UTC epoch.
    /// M42's production cooking dispatch uses this instead of a caller-made year.
    pub fn calendar_snapshot_utc(&self, input: &NpcRuleInput<'_>) -> Result<HotCuisineCalendar, NpcRuleError> {
        self.calendar_snapshot(input, |seconds| {
            crate::quest::npc_rules::NpcUtcCalendar::from_epoch(seconds).ok()
                .map(|date| YearMonth { year: date.year, month: date.month })
        })
    }

    pub fn calendar_snapshot(
        &self,
        input: &NpcRuleInput<'_>,
        decode: impl Fn(i64) -> Option<YearMonth>,
    ) -> Result<HotCuisineCalendar, NpcRuleError> {
        let current = decode(input.now_epoch_seconds)
            .filter(|date| date.is_valid())
            .ok_or_else(|| NpcRuleError::Invalid("Hot Cuisine world calendar".into()))?;
        let mut last = [None; 15];
        for (index, recipe) in self.recipes.iter().enumerate() {
            let timestamp = input.tracks.get(&recipe.last_track).copied().unwrap_or(-1);
            if timestamp != -1 {
                last[index] = Some(
                    decode(timestamp)
                        .filter(|date| date.is_valid())
                        .ok_or_else(|| {
                            NpcRuleError::Invalid("Hot Cuisine saved calendar".into())
                        })?,
                );
            }
        }
        Ok(HotCuisineCalendar { current, last })
    }

    /// Prepare one source confirmation. `None` lets the ordinary data-driven
    /// dialogue handle apprenticeship, menus and dish selection.
    pub fn plan(
        &self,
        generic: &NpcRuleProgram,
        input: &NpcRuleInput<'_>,
        calendar: &HotCuisineCalendar,
    ) -> Result<Option<NpcRulePlan>, NpcRuleError> {
        if input.npc != NPC || input.trigger != NpcRuleTrigger::Say
            || input.level < 35 || !input.premium_current {
            return Ok(None);
        }
        let Some(recipe) = self
            .recipes
            .iter()
            .find(|recipe| recipe.topic == input.topic)
        else {
            return Ok(None);
        };
        // Crystal 201189f data-global/npc/jean_pierre.lua:154-157,687:
        // the annual bonus belongs to the transaction's UTC year, not a
        // caller-supplied calendar projection with an otherwise matching month.
        let current = input.utc_calendar()?;
        if generic.document.quest != QUEST
            || !calendar.current.is_valid()
            || calendar.current.month != input.calendar_month
            || calendar.current.year != current.year
            || calendar.current.month != current.month
        {
            return Err(NpcRuleError::Invalid("Hot Cuisine snapshot".into()));
        }
        let mut observed = BTreeMap::new();
        let mut read = |key: &str| -> Result<i64, NpcRuleError> {
            let definition = generic
                .document
                .tracks
                .iter()
                .find(|track| track.key == key)
                .ok_or_else(|| NpcRuleError::MissingTrack(key.to_owned()))?;
            let value = input.tracks.get(key).copied().unwrap_or(definition.initial);
            if !(definition.min..=definition.max).contains(&value) {
                return Err(NpcRuleError::Invalid(format!("Hot Cuisine track: {key}")));
            }
            observed.insert(key.to_owned(), value);
            Ok(value)
        };
        let last_timestamp = read(&recipe.last_track)?;
        let bonus_year = read(&self.bonus_year_track)?;
        let bonus_used = read(&recipe.bonus_used_track)? > 0;
        let last_date = calendar.last[usize::from(recipe.index - 1)];
        if (last_timestamp == -1) != last_date.is_none() {
            return Err(NpcRuleError::Invalid(
                "Hot Cuisine timestamp/date mismatch".into(),
            ));
        }
        let bonus_year = i32::try_from(bonus_year).ok().filter(|year| *year >= 0);
        let permitted = can_cook(calendar.current, last_date, bonus_year, bonus_used);
        let mut effects = Vec::new();
        let mut inventory_guards = Vec::new();
        if permitted {
            let current_dish = read(&self.current_dish_track)?;
            let ingredient_recipe = self
                .recipes
                .iter()
                .find(|candidate| i64::from(candidate.index) == current_dish)
                .ok_or_else(|| NpcRuleError::Invalid("Hot Cuisine ingredient selection".into()))?;
            for ingredient in &ingredient_recipe.ingredients {
                if let NpcRuleEffect::TakeItem {
                    item,
                    count,
                    subtype,
                    ..
                } = ingredient
                {
                    inventory_guards.push(NpcRuleGuard::Item {
                        item: item.clone(),
                        count: *count,
                        subtype: *subtype,
                        op: NpcComparison::Ge,
                    });
                    effects.push(ingredient.clone());
                }
            }
            let first_completion = read(&self.quest_start_track)? < 2;
            let cooked_before = read(&recipe.cook_count_track)?;
            let cooked_after = cooked_before
                .checked_add(1)
                .ok_or_else(|| NpcRuleError::Arithmetic("Hot Cuisine cook count".into()))?;
            let mut all_twice = true;
            for candidate in &self.recipes {
                let count = if candidate.index == recipe.index {
                    cooked_after
                } else {
                    read(&candidate.cook_count_track)?
                };
                all_twice &= count >= 2;
            }
            effects.push(set(&recipe.last_track, input.now_epoch_seconds));
            effects.push(set(&recipe.cook_count_track, cooked_after));
            if spends_bonus(calendar.current, last_date) {
                effects.push(set(&recipe.bonus_used_track, 1));
            }
            if all_twice {
                effects.push(NpcRuleEffect::Achievement {
                    name: "Culinary Master".into(),
                });
            }
            effects.push(set(&self.current_dish_track, recipe.next_dish));
            if first_completion {
                effects.push(set(&self.quest_log_track, recipe.quest_log));
                if recipe.completes_menu {
                    effects.push(set(&self.quest_start_track, 2));
                    effects.push(set(
                        &self.bonus_year_track,
                        i64::from(calendar.current.year),
                    ));
                }
            }
            if recipe.opens_cookbook_door {
                effects.push(set(&self.cookbook_door_track, 1));
            }
            effects.push(recipe.reward.clone());
        }
        // These equality observations fence every value that selected a native
        // branch, including the fourteen other Culinary Master counters.
        let guards: Vec<_> = observed
            .into_iter()
            .map(|(track, value)| NpcRuleGuard::Storage {
                track,
                op: NpcComparison::Eq,
                value,
                maximum: None,
            })
            .collect();
        let mut document = generic.document.clone();
        let mut selected = native_rule(recipe, guards.clone());
        if permitted {
            selected.guards.extend(inventory_guards);
            selected.effects = effects;
            selected.effects.push(NpcRuleEffect::SetTopic { value: 0 });
            selected.reply = recipe.reply.clone();
            selected.inventory_failure_reply = self.missing_ingredients_reply.clone();
            let mut missing = native_rule(recipe, guards);
            missing.key.push_str("-missing-ingredients");
            missing.reply = self.missing_ingredients_reply.clone();
            missing.effects = vec![NpcRuleEffect::SetTopic { value: 0 }];
            document.rules = vec![selected, missing];
        } else {
            selected.key.push_str("-already-cooked");
            selected.reply = self.already_cooked_reply.clone();
            selected.effects = vec![NpcRuleEffect::SetTopic { value: 0 }];
            document.rules = vec![selected];
        }
        let json = serde_json::to_string(&document)
            .map_err(|error| NpcRuleError::Json(error.to_string()))?;
        let catalogue = NpcRuleCatalog::from_json_documents(&[&json])?;
        let mut plan = catalogue.plan(input)?;
        if let Some(plan) = &mut plan {
            plan.program_digest = self.program_digest(generic.digest);
        }
        Ok(plan)
    }
}

fn set(track: &str, value: i64) -> NpcRuleEffect {
    NpcRuleEffect::Set {
        track: track.to_owned(),
        value,
    }
}

fn native_rule(recipe: &Recipe, guards: Vec<NpcRuleGuard>) -> NpcRule {
    NpcRule {
        key: format!(
            "oteryn:quest-npc/hot_cuisine_quest/jean_pierre/native-cook-{}",
            recipe.index
        ),
        quest: QUEST.into(),
        npc: NPC.into(),
        trigger: NpcRuleTrigger::Say,
        keywords: vec!["yes".into()],
        match_player_name: false,
        exclude_keywords: ["cook", "recipe", "menu", "apprentice"]
            .map(String::from)
            .to_vec(),
        keyword_mode: NpcKeywordMode::MsgContains,
        any_message: false,
        topics: Some(vec![recipe.topic]),
        guards,
        reply: Vec::new(),
        effects: Vec::new(),
        continue_matching: false,
        inventory_failure_reply: Vec::new(),
        completes: false,
        sources: recipe.sources.clone(),
    }
}

#[cfg(test)]
#[path = "hot_cuisine/tests.rs"]
mod tests;
