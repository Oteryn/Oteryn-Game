use super::*;
use crate::quest::npc_rules::{NpcInventorySnapshot, NpcItemKey};

const DIALOGUE: &str = include_str!(
    "../../../../../../../content/dialogues/quest-rules/quest-batch3-a-hot_cuisine.json"
);

fn catalogue() -> Result<NpcRuleCatalog, NpcRuleError> {
    NpcRuleCatalog::from_json_documents(&[DIALOGUE])
}

fn tracks(program: &NpcRuleProgram) -> BTreeMap<String, i64> {
    program
        .document
        .tracks
        .iter()
        .map(|track| (track.key.clone(), track.initial))
        .collect()
}

fn ingredients(recipe: &Recipe) -> NpcInventorySnapshot {
    let mut inventory = NpcInventorySnapshot::default();
    for ingredient in &recipe.ingredients {
        if let NpcRuleEffect::TakeItem {
            item,
            count,
            subtype,
            ..
        } = ingredient
        {
            inventory.items.insert(
                NpcItemKey {
                    item: item.clone(),
                    subtype: *subtype,
                },
                *count,
            );
        }
    }
    inventory
}

fn input<'a>(
    tracks: &'a BTreeMap<String, i64>,
    inventory: &'a NpcInventorySnapshot,
    outfits: &'a BTreeMap<u16, u8>,
    topic: i32,
) -> NpcRuleInput<'a> {
    NpcRuleInput {
        npc: NPC,
        trigger: NpcRuleTrigger::Say,
        message: "yes",
        topic,
        tracks,
        inventory,
        now_epoch_seconds: 1_785_542_400,
        calendar_month: 8,
        level: 100,
        vocation: "oteryn:vocation.knight",
        sex: 0,
        outfits,
    }
}

fn calendar() -> HotCuisineCalendar {
    HotCuisineCalendar {
        current: YearMonth {
            year: 2026,
            month: 8,
        },
        last: [None; 15],
    }
}

#[test]
fn wrong_fluid_subtype_does_not_consume_or_progress() -> Result<(), NpcRuleError> {
    let hot = HotCuisineProgram::load()?;
    let catalogue = catalogue()?;
    let generic = catalogue
        .program(QUEST)
        .ok_or_else(|| NpcRuleError::Invalid("test program".into()))?;
    let mut progress = tracks(generic);
    progress.insert(hot.quest_start_track.clone(), 1);
    progress.insert(hot.current_dish_track.clone(), 1);
    let mut carried = ingredients(&hot.recipes[0]);
    carried.items.remove(&NpcItemKey {
        item: "oteryn:item.tibia.i2874".into(),
        subtype: Some(3),
    });
    carried.items.insert(
        NpcItemKey {
            item: "oteryn:item.tibia.i2874".into(),
            subtype: Some(15),
        },
        2,
    );
    let outfits = BTreeMap::new();
    let input = input(&progress, &carried, &outfits, 5);
    let plan = hot
        .plan(generic, &input, &calendar())?
        .ok_or_else(|| NpcRuleError::Invalid("test cook plan".into()))?;
    assert_eq!(plan.inventory_before, plan.inventory_after);
    assert!(plan.track_changes.is_empty());
    assert_eq!(plan.reply, hot.missing_ingredients_reply);
    assert_eq!(plan.next_topic, 0);
    Ok(())
}

#[test]
fn full_recipe_exchange_and_progress_are_one_plan() -> Result<(), NpcRuleError> {
    let hot = HotCuisineProgram::load()?;
    let catalogue = catalogue()?;
    let generic = catalogue
        .program(QUEST)
        .ok_or_else(|| NpcRuleError::Invalid("test program".into()))?;
    let mut progress = tracks(generic);
    progress.insert(hot.quest_start_track.clone(), 1);
    progress.insert(hot.current_dish_track.clone(), 14);
    let carried = ingredients(&hot.recipes[13]);
    assert_eq!(
        carried.items.get(&NpcItemKey {
            item: "oteryn:item.tibia.i3606".into(),
            subtype: None
        }),
        Some(&40)
    );
    assert_eq!(
        carried.items.get(&NpcItemKey {
            item: "oteryn:item.tibia.i5096".into(),
            subtype: None
        }),
        Some(&20)
    );
    let outfits = BTreeMap::new();
    let input = input(&progress, &carried, &outfits, 31);
    let plan = hot
        .plan(generic, &input, &calendar())?
        .ok_or_else(|| NpcRuleError::Invalid("test cook plan".into()))?;
    assert_eq!(
        plan.inventory_after.items.get(&NpcItemKey {
            item: "oteryn:item.tibia.i11588".into(),
            subtype: None
        }),
        Some(&1)
    );
    assert!(
        plan.track_changes
            .iter()
            .any(|change| change.key == hot.recipes[13].cook_count_track && change.after == 1)
    );
    assert!(
        plan.track_changes
            .iter()
            .any(|change| change.key == hot.quest_log_track && change.after == 16)
    );
    assert_eq!(plan.program_digest, hot.program_digest(generic.digest));
    Ok(())
}

#[test]
fn sauce_completion_grants_one_bonus_use_without_inventing_order_requirement()
-> Result<(), NpcRuleError> {
    let hot = HotCuisineProgram::load()?;
    let catalogue = catalogue()?;
    let generic = catalogue
        .program(QUEST)
        .ok_or_else(|| NpcRuleError::Invalid("test program".into()))?;
    let mut progress = tracks(generic);
    progress.insert(hot.quest_start_track.clone(), 1);
    progress.insert(hot.current_dish_track.clone(), 15);
    let carried = ingredients(&hot.recipes[14]);
    let outfits = BTreeMap::new();
    let first_input = input(&progress, &carried, &outfits, 33);
    let first = hot
        .plan(generic, &first_input, &calendar())?
        .ok_or_else(|| NpcRuleError::Invalid("test first cook".into()))?;
    for change in &first.track_changes {
        progress.insert(change.key.clone(), change.after);
    }
    assert_eq!(progress.get(&hot.quest_start_track), Some(&2));
    assert_eq!(progress.get(&hot.bonus_year_track), Some(&2026));
    assert_eq!(progress.get(&hot.recipes[0].cook_count_track), Some(&0));
    let mut same_period = calendar();
    same_period.last[14] = Some(same_period.current);
    let second_input = input(&progress, &carried, &outfits, 33);
    let second = hot
        .plan(generic, &second_input, &same_period)?
        .ok_or_else(|| NpcRuleError::Invalid("test bonus cook".into()))?;
    for change in &second.track_changes {
        progress.insert(change.key.clone(), change.after);
    }
    assert_eq!(progress.get(&hot.recipes[14].bonus_used_track), Some(&1));
    let third_input = input(&progress, &carried, &outfits, 33);
    let third = hot
        .plan(generic, &third_input, &same_period)?
        .ok_or_else(|| NpcRuleError::Invalid("test third cook".into()))?;
    assert_eq!(third.reply, hot.already_cooked_reply);
    assert!(third.track_changes.is_empty());
    assert_eq!(third.inventory_before, third.inventory_after);
    Ok(())
}
