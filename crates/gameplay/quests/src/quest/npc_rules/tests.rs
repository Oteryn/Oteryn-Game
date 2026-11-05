use super::*;

#[path = "../../../build_npc_programs.rs"]
mod build_npc_programs;

#[test]
fn every_registered_source_program_passes_the_closed_schema() -> Result<(), NpcRuleError> {
    NpcRuleCatalog::load_embedded().map(|_| ())
}

#[test]
fn crystal_msg_contains_rejects_any_word_prefixed_occurrence() -> Result<(), NpcRuleError> {
    assert!(keyword::matches(
        "yes please",
        "yes",
        NpcKeywordMode::MsgContains
    )?);
    assert!(!keyword::matches(
        "eyes yes",
        "yes",
        NpcKeywordMode::MsgContains
    )?);
    assert!(keyword::matches(
        "password",
        "password*",
        NpcKeywordMode::MsgContains
    )?);
    assert!(keyword::matches(
        "passwor",
        "password*",
        NpcKeywordMode::MsgContains
    )?);
    assert!(!keyword::matches(
        "xpassword password",
        "password*",
        NpcKeywordMode::MsgContains
    )?);
    assert!(keyword::matches(
        "_yes",
        "yes",
        NpcKeywordMode::MsgContains
    )?);
    assert!(!keyword::matches("yes!", "yes", NpcKeywordMode::Exact)?);
    assert!(keyword::matches("YES", "yes", NpcKeywordMode::Exact)?);
    Ok(())
}

#[test]
fn hero_greeting_plans_the_source_access_write() -> Result<(), NpcRuleError> {
    let catalogue =
        NpcRuleCatalog::from_json_documents(&[include_str!("test-fixtures/hero_greeting.json")])?;
    let inventory = NpcInventorySnapshot::default();
    let tracks = BTreeMap::new();
    let outfits = BTreeMap::new();
    let input = NpcRuleInput {
        npc: "oteryn:npc.strange_pipe",
        trigger: NpcRuleTrigger::Greet,
        message: "hi",
        topic: 0,
        tracks: &tracks,
        inventory: &inventory,
        now_epoch_seconds: 1_700_000_000,
        calendar_month: 11,
        level: 100,
        vocation: "oteryn:vocation.knight",
        sex: 0,
        outfits: &outfits,
    };
    let plan = catalogue
        .plan(&input)?
        .ok_or_else(|| NpcRuleError::Invalid("missing greeting plan".into()))?;
    assert_eq!(plan.track_changes.len(), 1);
    assert_eq!(plan.track_changes[0].before, -1);
    assert_eq!(plan.track_changes[0].after, 1);
    assert!(plan.reply.is_empty());
    assert!(tracks.is_empty());
    Ok(())
}

struct Facts {
    tracks: BTreeMap<String, i64>,
    inventory: NpcInventorySnapshot,
    outfits: BTreeMap<u16, u8>,
}
impl Facts {
    fn empty() -> Self {
        Self {
            tracks: BTreeMap::new(),
            inventory: NpcInventorySnapshot::default(),
            outfits: BTreeMap::new(),
        }
    }
    fn input<'a>(&'a self, npc: &'a str, message: &'a str, topic: i32) -> NpcRuleInput<'a> {
        NpcRuleInput {
            npc,
            trigger: NpcRuleTrigger::Say,
            message,
            topic,
            tracks: &self.tracks,
            inventory: &self.inventory,
            now_epoch_seconds: 1_700_000_000,
            calendar_month: 11,
            level: 100,
            vocation: "oteryn:vocation.knight",
            sex: 0,
            outfits: &self.outfits,
        }
    }
}

fn fixture_rule(key: &str, effects: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "key": format!("oteryn:quest-npc/test/{key}"),
        "quest":"oteryn:quest.test_npc", "npc":"oteryn:npc.test", "trigger":"say",
        "keywords":["mission"], "guards":[], "reply":["Source reply."], "effects":effects,
        "sources":[format!("{SOURCE_PIN}data/npclib/npc.lua:13")]
    })
}

fn fixture_document(rules: Vec<serde_json::Value>) -> serde_json::Value {
    serde_json::json!({
        "schema":1, "quest":"oteryn:quest.test_npc",
        "tracks":[{"key":"oteryn:quest-progress/test/value", "quest":"oteryn:quest.test_npc",
          "initial":-1, "min":-1, "max":2147483647,
          "sources":[format!("{SOURCE_PIN}data/npclib/npc.lua:13")]}],
        "rules":rules
    })
}

fn fixture_catalog(document: &serde_json::Value) -> Result<NpcRuleCatalog, NpcRuleError> {
    let text =
        serde_json::to_string(document).map_err(|error| NpcRuleError::Json(error.to_string()))?;
    NpcRuleCatalog::from_json_documents(&[&text])
}

#[test]
fn continuation_reads_staged_tracks_topics_and_items() -> Result<(), NpcRuleError> {
    let mut first = fixture_rule(
        "first",
        serde_json::json!([
            {"kind":"set","track":"oteryn:quest-progress/test/value","value":3},
            {"kind":"set_topic","value":2},
            {"kind":"give_item","item":"oteryn:item.tibia.i140","count":1}
        ]),
    );
    first["continue_matching"] = true.into();
    let mut second = fixture_rule(
        "second",
        serde_json::json!([
            {"kind":"add","track":"oteryn:quest-progress/test/value","value":1},
            {"kind":"take_item","item":"oteryn:item.tibia.i140","count":1}
        ]),
    );
    second["topics"] = serde_json::json!([2]);
    second["guards"] = serde_json::json!([
        {"kind":"storage","track":"oteryn:quest-progress/test/value","op":"eq","value":3},
        {"kind":"item","item":"oteryn:item.tibia.i140","count":1}
    ]);
    let catalog = fixture_catalog(&fixture_document(vec![first, second]))?;
    let facts = Facts::empty();
    let plan = catalog
        .plan(&facts.input("oteryn:npc.test", "mission", 0))?
        .ok_or_else(|| NpcRuleError::Invalid("missing continued plan".into()))?;
    assert_eq!(plan.selected_rule_keys.len(), 2);
    assert_eq!(
        plan.track_changes,
        vec![NpcTrackChange {
            key: "oteryn:quest-progress/test/value".into(),
            before: -1,
            after: 4,
        }]
    );
    assert_eq!(plan.next_topic, 2);
    assert!(plan.inventory_after.items.values().all(|count| *count == 0));
    assert_eq!(
        plan.observed_tracks.get("oteryn:quest-progress/test/value"),
        Some(&-1)
    );
    assert!(facts.tracks.is_empty());
    assert!(facts.inventory.items.is_empty());
    Ok(())
}

#[test]
fn failed_later_consumption_produces_no_partial_plan() -> Result<(), NpcRuleError> {
    let rule = fixture_rule(
        "failure",
        serde_json::json!([
            {"kind":"set","track":"oteryn:quest-progress/test/value","value":1},
            {"kind":"take_item","item":"oteryn:item.tibia.i140","count":2}
        ]),
    );
    let catalog = fixture_catalog(&fixture_document(vec![rule]))?;
    let facts = Facts::empty();
    assert!(matches!(
        catalog.plan(&facts.input("oteryn:npc.test", "mission", 0)),
        Err(NpcRuleError::MissingItems(_))
    ));
    assert!(facts.tracks.is_empty());
    assert!(facts.inventory.items.is_empty());
    Ok(())
}

#[test]
fn ignored_remove_item_result_preserves_the_source_reward() -> Result<(), NpcRuleError> {
    let rule = fixture_rule(
        "optional",
        serde_json::json!([
            {"kind":"take_item","item":"oteryn:item.tibia.i140","count":1,"optional":true},
            {"kind":"give_item","item":"oteryn:item.tibia.i141","count":1},
            {"kind":"set","track":"oteryn:quest-progress/test/value","value":1}
        ]),
    );
    let catalog = fixture_catalog(&fixture_document(vec![rule]))?;
    let facts = Facts::empty();
    let plan = catalog
        .plan(&facts.input("oteryn:npc.test", "mission", 0))?
        .ok_or_else(|| NpcRuleError::Invalid("missing optional-removal plan".into()))?;
    assert_eq!(plan.effects.len(), 2);
    assert_eq!(
        plan.inventory_after.items.get(&NpcItemKey {
            item: "oteryn:item.tibia.i141".into(),
            subtype: None
        }),
        Some(&1)
    );
    Ok(())
}

#[test]
fn fluid_consumption_requires_the_exact_subtype() -> Result<(), NpcRuleError> {
    let mut rule = fixture_rule(
        "fluid",
        serde_json::json!([
            {"kind":"take_item","item":"oteryn:item.tibia.i2874","count":1,"subtype":13}
        ]),
    );
    rule["guards"] = serde_json::json!([{"kind":"item","item":"oteryn:item.tibia.i2874","count":1,"subtype":13}]);
    let catalog = fixture_catalog(&fixture_document(vec![rule]))?;
    let mut facts = Facts::empty();
    facts.inventory.items.insert(
        NpcItemKey {
            item: "oteryn:item.tibia.i2874".into(),
            subtype: Some(1),
        },
        5,
    );
    assert!(
        catalog
            .plan(&facts.input("oteryn:npc.test", "mission", 0))?
            .is_none()
    );
    facts.inventory.items.insert(
        NpcItemKey {
            item: "oteryn:item.tibia.i2874".into(),
            subtype: Some(13),
        },
        1,
    );
    let plan = catalog
        .plan(&facts.input("oteryn:npc.test", "mission", 0))?
        .ok_or_else(|| NpcRuleError::Invalid("missing fluid plan".into()))?;
    assert_eq!(
        plan.inventory_after.items.get(&NpcItemKey {
            item: "oteryn:item.tibia.i2874".into(),
            subtype: Some(1)
        }),
        Some(&5)
    );
    assert_eq!(
        plan.inventory_after.items.get(&NpcItemKey {
            item: "oteryn:item.tibia.i2874".into(),
            subtype: Some(13)
        }),
        Some(&0)
    );
    Ok(())
}

#[test]
fn money_bank_consumption_matches_crystal_inventory_first() -> Result<(), NpcRuleError> {
    let rule = fixture_rule(
        "money",
        serde_json::json!([
            {"kind":"take_money","amount":200,"include_bank":true}
        ]),
    );
    let catalog = fixture_catalog(&fixture_document(vec![rule]))?;
    let mut facts = Facts::empty();
    facts.inventory.money = 75;
    facts.inventory.bank = 150;
    let plan = catalog
        .plan(&facts.input("oteryn:npc.test", "mission", 0))?
        .ok_or_else(|| NpcRuleError::Invalid("missing money plan".into()))?;
    assert_eq!(plan.inventory_after.money, 0);
    assert_eq!(plan.inventory_after.bank, 25);
    assert_eq!(facts.inventory.money, 75);
    assert_eq!(facts.inventory.bank, 150);
    Ok(())
}

#[test]
fn absent_foreign_track_prevents_fallback_reward() -> Result<(), NpcRuleError> {
    let mut first = fixture_rule("foreign", serde_json::json!([]));
    first["guards"] = serde_json::json!([{"kind":"storage","track":"oteryn:quest-progress/other/value","op":"ge","value":1}]);
    let second = fixture_rule(
        "fallback",
        serde_json::json!([{"kind":"give_money","amount":500}]),
    );
    let catalog = fixture_catalog(&fixture_document(vec![first, second]))?;
    let facts = Facts::empty();
    assert!(matches!(
        catalog.plan(&facts.input("oteryn:npc.test", "mission", 0)),
        Err(NpcRuleError::MissingTrack(_))
    ));
    Ok(())
}

#[test]
fn outer_keyword_barrier_blocks_later_yes() -> Result<(), NpcRuleError> {
    let mut rule = fixture_rule("yes", serde_json::json!([]));
    rule["keywords"] = serde_json::json!(["yes"]);
    rule["exclude_keywords"] = serde_json::json!(["mission"]);
    let catalog = fixture_catalog(&fixture_document(vec![rule]))?;
    let facts = Facts::empty();
    assert!(
        catalog
            .plan(&facts.input("oteryn:npc.test", "mission yes", 0))?
            .is_none()
    );
    assert!(
        catalog
            .plan(&facts.input("oteryn:npc.test", "yes", 0))?
            .is_some()
    );
    Ok(())
}

#[test]
fn empty_terminal_rule_preserves_source_first_branch_priority() -> Result<(), NpcRuleError> {
    let mut barrier = fixture_rule("barrier", serde_json::json!([]));
    barrier["reply"] = serde_json::json!([]);
    let award = fixture_rule(
        "award",
        serde_json::json!([{"kind":"give_money","amount":500}]),
    );
    let catalog = fixture_catalog(&fixture_document(vec![barrier, award]))?;
    let facts = Facts::empty();
    let plan = catalog
        .plan(&facts.input("oteryn:npc.test", "mission", 0))?
        .ok_or_else(|| NpcRuleError::Invalid("missing terminal rule".into()))?;
    assert!(plan.effects.is_empty());
    assert!(plan.reply.is_empty());
    assert_eq!(plan.selected_rule_keys.len(), 1);
    Ok(())
}

#[test]
fn source_time_guard_and_cooldown_use_supplied_epoch_seconds() -> Result<(), NpcRuleError> {
    let mut rule = fixture_rule(
        "clock",
        serde_json::json!([
            {"kind":"set_now_plus","track":"oteryn:quest-progress/test/value","value":1800}
        ]),
    );
    rule["guards"] = serde_json::json!([{"kind":"storage_time","track":"oteryn:quest-progress/test/value","op":"le"}]);
    let catalog = fixture_catalog(&fixture_document(vec![rule]))?;
    let mut facts = Facts::empty();
    facts
        .tracks
        .insert("oteryn:quest-progress/test/value".into(), 1_700_000_000);
    let plan = catalog
        .plan(&facts.input("oteryn:npc.test", "mission", 0))?
        .ok_or_else(|| NpcRuleError::Invalid("missing clock plan".into()))?;
    assert_eq!(plan.track_changes[0].after, 1_700_001_800);
    facts
        .tracks
        .insert("oteryn:quest-progress/test/value".into(), 1_700_000_001);
    assert!(
        catalog
            .plan(&facts.input("oteryn:npc.test", "mission", 0))?
            .is_none()
    );
    Ok(())
}

#[test]
fn father_acceptance_refuses_all_eleven_atomic_writes_without_splitting() -> Result<(), NpcRuleError>
{
    let catalog = NpcRuleCatalog::from_json_documents(&[include_str!(
        "test-fixtures/father_acceptance.json"
    )])?;
    let facts = Facts::empty();
    // Coordinator ANSWER preserves the existing eight-write contract and forbids
    // splitting one donor transaction: PR2843#issuecomment-6589517205, point 5.
    assert!(matches!(
        catalog.plan(&facts.input("oteryn:npc.tereban", "yes", 1)),
        Err(NpcRuleError::LimitExceeded)
    ));
    assert!(facts.tracks.is_empty());
    assert_eq!(facts.inventory, NpcInventorySnapshot::default());
    Ok(())
}

#[test]
fn catalogue_rejects_unknown_schema_fields_and_foreign_writes() -> Result<(), NpcRuleError> {
    let rule = fixture_rule("known", serde_json::json!([]));
    let mut document = fixture_document(vec![rule]);
    document["schema"] = 2.into();
    assert!(fixture_catalog(&document).is_err());
    document["schema"] = 1.into();
    document["unknown_executable"] = true.into();
    assert!(fixture_catalog(&document).is_err());
    let foreign = fixture_rule(
        "foreign-write",
        serde_json::json!([
            {"kind":"set","track":"oteryn:quest-progress/other/value","value":1}
        ]),
    );
    assert!(fixture_catalog(&fixture_document(vec![foreign])).is_err());
    Ok(())
}

#[test]
fn program_digest_changes_with_the_actual_reply_and_not_whitespace() -> Result<(), NpcRuleError> {
    let mut document = fixture_document(vec![fixture_rule("digest", serde_json::json!([]))]);
    let first = fixture_catalog(&document)?;
    let pretty = serde_json::to_string_pretty(&document)
        .map_err(|error| NpcRuleError::Json(error.to_string()))?;
    let second = NpcRuleCatalog::from_json_documents(&[&pretty])?;
    assert_eq!(first.programs()[0].digest, second.programs()[0].digest);
    document["rules"][0]["reply"] = serde_json::json!(["Changed source reply."]);
    let changed = fixture_catalog(&document)?;
    assert_ne!(first.programs()[0].digest, changed.programs()[0].digest);
    Ok(())
}

#[test]
fn native_bigfoot_resolution_is_required_and_bounded_before_plan() -> Result<(), NpcRuleError> {
    let rule = fixture_rule(
        "native",
        serde_json::json!([{"kind":"bigfoot_matchmaker_target"}]),
    );
    let mut document = fixture_document(vec![rule]);
    let track = "oteryn:quest-progress/quest/u9_60/bigfoots_burden/matchmaker_id_needed";
    document["quest"] = "oteryn:quest.bigfoot_s_burden_quest".into();
    document["tracks"][0]["quest"] = "oteryn:quest.bigfoot_s_burden_quest".into();
    document["rules"][0]["quest"] = "oteryn:quest.bigfoot_s_burden_quest".into();
    document["tracks"][0]["key"] = track.into();
    let catalog = fixture_catalog(&document)?;
    let facts = Facts::empty();
    let input = facts.input("oteryn:npc.test", "mission", 0);
    assert!(matches!(
        catalog.plan(&input),
        Err(NpcRuleError::MissingNativeResolution(_))
    ));
    let mut resolutions = BTreeMap::new();
    resolutions.insert(
        "oteryn:quest-npc/test/native".into(),
        vec![NpcRuleEffect::Set {
            track: track.into(),
            value: 15815,
        }],
    );
    let plan = catalog
        .plan_with_native(&input, &resolutions)?
        .ok_or_else(|| NpcRuleError::Invalid("missing resolved native plan".into()))?;
    assert_eq!(plan.track_changes[0].after, 15815);
    assert!(matches!(
        plan.effects.as_slice(),
        [NpcRuleEffect::Set { value: 15815, .. }]
    ));
    resolutions.insert(
        "oteryn:quest-npc/test/native".into(),
        vec![NpcRuleEffect::Set {
            track: track.into(),
            value: 15816,
        }],
    );
    assert!(catalog.plan_with_native(&input, &resolutions).is_err());
    Ok(())
}

#[test]
fn pinned_source_ranges_are_valid_but_inverted_ranges_are_rejected() -> Result<(), NpcRuleError> {
    let mut document = fixture_document(vec![fixture_rule("range", serde_json::json!([]))]);
    document["rules"][0]["sources"] =
        serde_json::json!([format!("{SOURCE_PIN}data/npclib/npc.lua:13-20")]);
    fixture_catalog(&document)?;
    document["rules"][0]["sources"] =
        serde_json::json!([format!("{SOURCE_PIN}data/npclib/npc.lua:20-13")]);
    assert!(fixture_catalog(&document).is_err());
    Ok(())
}

#[test]
fn unqualified_hazard_receipt_and_mount_contracts_are_rejected() {
    let mut guard = fixture_rule("hazard", serde_json::json!([]));
    guard["guards"] = serde_json::json!([{"kind":"hazard_receipt","minimum":6}]);
    assert!(fixture_catalog(&fixture_document(vec![guard])).is_err());
    let effect = fixture_rule("mount", serde_json::json!([{"kind":"mount","mount":202}]));
    assert!(fixture_catalog(&fixture_document(vec![effect])).is_err());
}

#[test]
fn dialogue_admission_refuses_an_unavailable_guard_before_fallback() -> Result<(), NpcRuleError> {
    let catalogue = NpcRuleCatalog::load_embedded()?;
    let facts = Facts::empty();
    // Crystal tereban_functions.lua: the source checks a real sinew item before
    // its missing-item reply. An unavailable inventory owner is not zero items.
    let input = facts.input("oteryn:npc.tereban", "yes", 7201);
    assert!(matches!(
        catalogue.plan_dialogue(&input),
        Err(NpcRuleError::MissingRuntimeOwner(_))
    ));
    Ok(())
}

#[test]
fn dialogue_admission_keeps_a_pure_topic_branch_and_refuses_storage_writes()
-> Result<(), NpcRuleError> {
    let catalogue = NpcRuleCatalog::load_embedded()?;
    let facts = Facts::empty();
    let offer = catalogue
        .plan_dialogue(&facts.input("oteryn:npc.tereban", "mission", 0))?
        .ok_or_else(|| NpcRuleError::Invalid("missing offer".into()))?;
    assert_eq!(offer.next_topic, 1);
    assert!(offer.track_changes.is_empty());
    assert!(
        offer
            .reply
            .iter()
            .any(|line| line.starts_with("I have four sons"))
    );
    assert!(matches!(
        catalogue.plan_dialogue(&facts.input("oteryn:npc.tereban", "yes", 1)),
        Err(NpcRuleError::MissingRuntimeOwner(_))
    ));
    Ok(())
}

#[test]
fn dialogue_admission_discards_earlier_continuing_reply_on_later_unowned_effect()
-> Result<(), NpcRuleError> {
    let mut first = fixture_rule("first", serde_json::json!([{"kind":"set_topic","value":2}]));
    first["continue_matching"] = true.into();
    let mut second = fixture_rule(
        "second",
        serde_json::json!([{"kind":"give_money","amount":500}]),
    );
    second["topics"] = serde_json::json!([2]);
    let catalogue = fixture_catalog(&fixture_document(vec![first, second]))?;
    let facts = Facts::empty();
    assert!(matches!(
        catalogue.plan_dialogue(&facts.input("oteryn:npc.test", "mission", 0)),
        Err(NpcRuleError::MissingRuntimeOwner(_))
    ));
    assert!(facts.tracks.is_empty());
    assert_eq!(facts.inventory, NpcInventorySnapshot::default());
    Ok(())
}

#[test]
fn npc_programs_extend_the_ordinary_definition_hash_and_generic_callers_refuse()
-> Result<(), NpcRuleError> {
    let programs = NpcRuleCatalog::load_embedded()?;
    let empty = crate::quest::QuestStateCatalogue::empty("content-1")
        .ok_or_else(|| NpcRuleError::Invalid("empty catalogue".into()))?;
    let merged = programs.extend_quest_state(&empty)?;
    for program in programs.programs() {
        let transition = merged
            .transition(&program_transition_key(program.digest))
            .ok_or_else(|| NpcRuleError::Invalid("missing synthetic transition".into()))?;
        assert_eq!(transition.quest, program.document.quest);
        assert_eq!(
            merged.evaluate(transition, &BTreeMap::new(), 0),
            Err(crate::quest::QuestRefusal::NotSupported)
        );
        assert!(merged.definition_hash(&program.document.quest).is_some());
        assert!(program_transition_key(program.digest).len() <= crate::quest::QUESTSTATE0_RL_06);
    }
    assert_eq!(
        programs.extend_quest_state(&merged)?,
        merged,
        "idempotent exact merge"
    );
    let mut changed = programs.clone();
    changed.programs[0].document.rules[0]
        .reply
        .push("changed source reply".into());
    let source = serde_json::to_string(&changed.programs[0].document)
        .map_err(|error| NpcRuleError::Json(error.to_string()))?;
    let changed = NpcRuleCatalog::from_json_documents(&[&source])?;
    let changed_catalogue = changed.extend_quest_state(&empty)?;
    let quest = &changed.programs[0].document.quest;
    assert_ne!(
        merged.definition_hash(quest),
        changed_catalogue.definition_hash(quest)
    );
    Ok(())
}

#[test]
fn source_extension_retains_wider_bounds_and_refuses_owner_or_initial_conflicts()
-> Result<(), NpcRuleError> {
    let programs = fixture_catalog(&fixture_document(vec![fixture_rule(
        "offer",
        serde_json::json!([]),
    )]))?;
    let source = &programs.programs[0].document.tracks[0];
    let wider = crate::quest::QuestTrack {
        key: source.key.clone(),
        quest: source.quest.clone(),
        initial: source.initial,
        min: source.min.saturating_sub(1),
        max: source.max.saturating_add(1),
    };
    let base = crate::quest::QuestStateCatalogue::new("content-1", vec![wider.clone()], vec![])
        .map_err(|error| NpcRuleError::Invalid(format!("{error:?}")))?;
    let merged = programs.extend_quest_state(&base)?;
    assert_eq!(merged.track(&wider.key), Some(&wider));
    for track in [
        crate::quest::QuestTrack {
            initial: source.initial + 1,
            ..wider.clone()
        },
        crate::quest::QuestTrack {
            quest: "oteryn:quest.other".into(),
            ..wider.clone()
        },
    ] {
        let conflicting = crate::quest::QuestStateCatalogue::new("content-1", vec![track], vec![])
            .map_err(|error| NpcRuleError::Invalid(format!("{error:?}")))?;
        assert!(matches!(
            programs.extend_quest_state(&conflicting),
            Err(NpcRuleError::Invalid(_))
        ));
    }
    Ok(())
}
