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

#[test]
fn inventory_planner_refuses_value_only_rules_and_unowned_character_guards()
-> Result<(), NpcRuleError> {
    let fixture = include_str!("test-fixtures/hero_greeting.json");
    for value_only in [true, false] {
        let mut document: serde_json::Value =
            serde_json::from_str(fixture).map_err(|error| NpcRuleError::Json(error.to_string()))?;
        if value_only {
            document["rules"][0]["effects"] = serde_json::json!([
                {"kind": "give_item", "item": "oteryn:item.tibia.i3031", "count": 1}
            ]);
        } else {
            document["rules"][0]["guards"] = serde_json::json!([
                {"kind": "level", "op": "ge", "value": 0}
            ]);
        }
        let encoded = serde_json::to_string(&document)
            .map_err(|error| NpcRuleError::Json(error.to_string()))?;
        let catalogue = NpcRuleCatalog::from_json_documents(&[&encoded])?;
        let facts = Facts::empty();
        let mut input = facts.input("oteryn:npc.strange_pipe", "hi", 0);
        input.trigger = NpcRuleTrigger::Greet;
        assert!(matches!(
            catalogue.plan_inventory(&input),
            Err(NpcRuleError::MissingNativeResolution(_))
        ));
    }
    Ok(())
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
fn father_acceptance_plans_all_eleven_atomic_writes_without_splitting() -> Result<(), NpcRuleError>
{
    let catalog = NpcRuleCatalog::from_json_documents(&[include_str!(
        "test-fixtures/father_acceptance.json"
    )])?;
    let facts = Facts::empty();
    let plan = catalog
        .plan(&facts.input("oteryn:npc.tereban", "yes", 1))?
        .ok_or_else(|| NpcRuleError::Invalid("missing acceptance".into()))?;
    assert_eq!(plan.track_changes.len(), 11);
    assert!(facts.tracks.is_empty());
    assert_eq!(facts.inventory, NpcInventorySnapshot::default());
    Ok(())
}

#[test]
fn generic_track_capacity_is_sixteen_and_refuses_seventeen() -> Result<(), NpcRuleError> {
    for count in [9, 16, 17] {
        let mut document = fixture_document(vec![]);
        let template = document["tracks"][0].clone();
        let tracks: Vec<_> = (0..count)
            .map(|i| {
                let mut track = template.clone();
                track["key"] = format!("oteryn:quest-progress/test/value-{i}").into();
                track
            })
            .collect();
        let effects: Vec<_> = tracks
            .iter()
            .map(|track| {
                serde_json::json!({
                    "kind":"set", "track":track["key"], "value":1
                })
            })
            .collect();
        document["tracks"] = tracks.into();
        document["rules"] = vec![fixture_rule("capacity", effects.into())].into();
        let catalog = fixture_catalog(&document)?;
        let facts = Facts::empty();
        let result = catalog.plan_storage(&facts.input("oteryn:npc.test", "mission", 0));
        if count == 17 {
            assert!(matches!(result, Err(NpcRuleError::LimitExceeded)));
        } else {
            assert_eq!(
                result?
                    .ok_or_else(|| NpcRuleError::Invalid("missing plan".into()))?
                    .track_changes
                    .len(),
                count
            );
        }
        assert!(facts.tracks.is_empty());
    }
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
fn storage_owner_refuses_inventory_dependent_fallbacks() -> Result<(), NpcRuleError> {
    let mut conditional = fixture_rule("inventory-first", serde_json::json!([]));
    conditional["guards"] = serde_json::json!([
        {"kind":"item","item":"oteryn:item.tibia.i140","count":1}
    ]);
    let fallback = fixture_rule(
        "fallback",
        serde_json::json!([
            {"kind":"set","track":"oteryn:quest-progress/test/value","value":1}
        ]),
    );
    let catalogue = fixture_catalog(&fixture_document(vec![conditional, fallback]))?;
    let facts = Facts::empty();
    assert!(matches!(
        catalogue.plan_storage(&facts.input("oteryn:npc.test", "mission", 0)),
        Err(NpcRuleError::MissingNativeResolution(_))
    ));
    Ok(())
}

#[test]
fn storage_owner_preserves_order_and_rejects_unowned_effects() -> Result<(), NpcRuleError> {
    let mut first = fixture_rule(
        "first",
        serde_json::json!([
            {"kind":"set","track":"oteryn:quest-progress/test/value","value":3}
        ]),
    );
    first["continue_matching"] = true.into();
    let second = fixture_rule(
        "second",
        serde_json::json!([
            {"kind":"add","track":"oteryn:quest-progress/test/value","value":1},
            {"kind":"set_topic","value":2}
        ]),
    );
    let catalogue = fixture_catalog(&fixture_document(vec![first, second]))?;
    let facts = Facts::empty();
    let input = facts.input("oteryn:npc.test", "mission", 0);
    let plan = catalogue
        .plan_storage(&input)?
        .ok_or_else(|| NpcRuleError::Invalid("storage plan".into()))?;
    assert_eq!(plan.track_changes[0].after, 4);
    assert_eq!(plan.next_topic, 2);
    assert_eq!(plan.selected_rule_keys.len(), 2);
    let gift = fixture_rule(
        "gift",
        serde_json::json!([
            {"kind":"give_item","item":"oteryn:item.tibia.i140","count":1}
        ]),
    );
    let catalogue = fixture_catalog(&fixture_document(vec![gift]))?;
    assert!(matches!(
        catalogue.plan_storage(&input),
        Err(NpcRuleError::MissingNativeResolution(_))
    ));
    Ok(())
}

#[test]
fn npc_activation_keeps_exact_legacy_hashes_defaults_and_rejects_incompatible_rollback()
-> Result<(), String> {
    let text = include_str!("../../../../../../content/quests/missions/quest-state.json");
    let old = super::super::loader::parse_quest_state(text, "npc-legacy")
        .map_err(|e| format!("{e:?}"))?;
    let document: serde_json::Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
    let mut bindings = BTreeMap::new();
    for quest in document["quests"].as_array().ok_or("quests")? {
        for track in quest["tracks"].as_array().ok_or("tracks")? {
            bindings.insert(
                track["key"].as_str().ok_or("key")?.to_owned(),
                track["source_key"].as_str().ok_or("source")?.to_owned(),
            );
        }
    }
    let active = extend(old.catalogue().clone(), &bindings)?;
    let runtime = NpcRuleCatalog::runtime().map_err(|e| e.to_string())?;
    // Exact activation closure: Hot Cuisine is intentional for authoritative
    // calendar/dismissal execution, alongside the independently bound programmes.
    let expected_programmes = std::collections::BTreeSet::from([
        "oteryn:quest.a_father_s_burden_quest",
        "oteryn:quest.a_pirates_tail",
        "oteryn:quest.bigfoot_s_burden_quest",
        "oteryn:quest.demon_oak",
        "oteryn:quest.cults_of_tibia_quest",
        "oteryn:quest.dangerous_depths_quest",
        "oteryn:quest.draconia_quest",
        "oteryn:quest.forgotten_knowledge_quest",
        "oteryn:quest.hero_of_rathleton_quest",
        "oteryn:quest.hot_cuisine_quest",
        "oteryn:quest.in_service_of_yalahar_quest",
        "oteryn:quest.marlin_trophy_quest",
        "oteryn:quest.the_hidden_city_of_beregar_quest",
        "oteryn:quest.the_inquisition_quest",
        "oteryn:quest.the_order_of_the_stag_quest",
        "oteryn:quest.tibia_tales",
    ]);
    assert_eq!(
        runtime
            .programs()
            .iter()
            .filter(|program| program.document.schema == 1)
            .map(|p| p.document.quest.as_str())
            .collect::<std::collections::BTreeSet<_>>(),
        expected_programmes
    );
    assert_eq!(runtime.programs().len(), expected_programmes.len() + 1);
    assert_eq!(
        runtime
            .programs()
            .iter()
            .filter(|program| program.document.schema == 2)
            .map(|program| program.document.quest.as_str())
            .collect::<std::collections::BTreeSet<_>>(),
        std::collections::BTreeSet::from(["oteryn:quest.authored.mystery_of_the_valley_quest"])
    );
    assert!(
        !active
            .npc_legacy
            .contains_key("oteryn:quest.authored.mystery_of_the_valley_quest")
    );
    let hot_cuisine = "oteryn:quest.hot_cuisine_quest";
    assert_eq!(
        active.npc_legacy.get(hot_cuisine).map(|(hash, _)| *hash),
        old.catalogue().definition_hash(hot_cuisine)
    );
    assert!(active.npc_legacy.contains_key(hot_cuisine));
    assert_eq!(
        runtime
            .program("oteryn:quest.draconia_quest")
            .ok_or("Dragonlord programme")?
            .document
            .program_key
            .as_deref(),
        Some("oteryn:quest-npc-program.draconia")
    );
    assert!(
        runtime
            .program("oteryn:quest.marlin_trophy_quest")
            .is_some()
    );
    let corym = runtime
        .program("oteryn:quest.tibia_tales")
        .ok_or("Corym programme")?;
    assert_eq!(
        corym.document.program_key.as_deref(),
        Some(hidden_threats::PROGRAM)
    );
    assert!(
        corym
            .document
            .rules
            .iter()
            .all(|rule| rule.npc.starts_with("oteryn:npc.corym_"))
    );
    // The historical frozen catalogue has no Draconia definition to alias.
    let expected_legacy = expected_programmes
        .iter()
        .copied()
        .filter(|quest| {
            !matches!(
                *quest,
                "oteryn:quest.draconia_quest" | "oteryn:quest.the_order_of_the_stag_quest"
            )
        })
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        active
            .npc_legacy
            .keys()
            .map(String::as_str)
            .collect::<std::collections::BTreeSet<_>>(),
        expected_legacy
    );
    assert!(
        !active
            .npc_legacy
            .contains_key("oteryn:quest.draconia_quest")
    );
    let mut defaults = BTreeMap::new();
    for track in NpcRuleCatalog::embedded()
        .map_err(|e| e.to_string())?
        .programs()
        .iter()
        .flat_map(|program| &program.document.tracks)
    {
        *defaults.entry(track.initial).or_insert(0usize) += 1;
    }
    assert_eq!(defaults, BTreeMap::from([(-1, 192), (0, 52)]));
    for program in NpcRuleCatalog::embedded()
        .map_err(|e| e.to_string())?
        .programs()
    {
        let quest = &program.document.quest;
        if let Some((hash, defaults)) = active.npc_legacy.get(quest) {
            assert_eq!(Some(*hash), old.catalogue().definition_hash(quest));
            assert_ne!(Some(*hash), active.definition_hash(quest));
            assert_eq!(legacy_defaults(&active, quest, hash), Some(defaults));
            assert!(legacy_defaults(&active, quest, &[0; 32]).is_none());
            // Old artifacts cannot accept a new pin: rollback requires this verified
            // compatibility adapter, never activation of an incompatible old binary.
            assert!(
                legacy_defaults(
                    old.catalogue(),
                    quest,
                    &active.definition_hash(quest).ok_or("hash")?
                )
                .is_none()
            );
            for track in old
                .catalogue()
                .tracks
                .values()
                .filter(|t| &t.quest == quest)
            {
                assert_eq!(defaults.get(&track.key), Some(&track.initial));
                let new = active.track(&track.key).ok_or("track")?;
                assert_eq!(
                    new.initial, track.initial,
                    "all existing native consumers keep their absent value"
                );
                assert!(new.min <= track.min && new.max >= track.max);
            }
            for transition in old
                .catalogue()
                .transitions
                .values()
                .filter(|t| &t.quest == quest)
            {
                assert_eq!(active.transition(&transition.key), Some(transition));
            }
        } else if runtime.program(quest).is_none() {
            assert_eq!(
                active.definition_hash(quest),
                old.catalogue().definition_hash(quest)
            );
        }
    }
    runtime
        .clone()
        .with_native_bounds(&active)
        .map_err(|e| e.to_string())?;
    let loaded = super::super::loader::load_embedded_quest_state("npc-legacy")
        .map_err(|e| format!("{e:?}"))?;
    let expected_loaded_legacy = expected_legacy;
    assert_eq!(
        loaded
            .catalogue()
            .npc_legacy
            .keys()
            .map(String::as_str)
            .collect::<std::collections::BTreeSet<_>>(),
        expected_loaded_legacy
    );
    // Draconia is a newly introduced programme, not a historical definition.
    // It must never acquire a fabricated legacy hash/default mapping.
    let draconia = "oteryn:quest.draconia_quest";
    assert!(old.catalogue().definition_hash(draconia).is_none());
    assert!(loaded.catalogue().definition_hash(draconia).is_some());
    assert!(!loaded.catalogue().npc_legacy.contains_key(draconia));
    assert!(
        legacy_defaults(
            old.catalogue(),
            draconia,
            &loaded
                .catalogue()
                .definition_hash(draconia)
                .ok_or("new Draconia hash")?
        )
        .is_none()
    );
    Ok(())
}

#[test]
fn storage_owner_plans_exact_father_acceptance_with_shared_cap() -> Result<(), NpcRuleError> {
    let source = NpcRuleCatalog::runtime()?;
    let facts = Facts::empty();
    let input = facts.input("oteryn:npc.tereban", "yes", 1);
    assert_eq!(MAX_NPC_TRACK_CHANGES, 16);
    let plan = source
        .plan_storage(&input)?
        .ok_or_else(|| NpcRuleError::Invalid("father acceptance".into()))?;
    assert_eq!(plan.track_changes.len(), 11);
    assert_eq!(
        plan.selected_rule_keys,
        vec!["oteryn:quest-npc/a_father_s_burden_quest/tereban_functions/mission-accept"]
    );
    assert!(facts.tracks.is_empty());
    Ok(())
}

#[test]
fn native_bounds_preserve_source_absence_without_accepting_out_of_range_rows() -> Result<(), String>
{
    let native = super::super::loader::load_embedded_quest_state("npc-bounds")
        .map_err(|error| format!("{error:?}"))?;
    let source = NpcRuleCatalog::runtime()
        .map_err(|error| error.to_string())?
        .clone()
        .with_native_bounds(native.catalogue())
        .map_err(|error| error.to_string())?;
    let mut facts = Facts::empty();
    let plan = source
        .plan_storage(&facts.input("oteryn:npc.tereban", "yes", 1))
        .map_err(|error| error.to_string())?
        .ok_or("Father acceptance")?;
    assert_eq!(plan.track_changes.len(), 11);
    assert!(plan.track_changes.iter().all(|change| change.before == -1));
    assert_eq!(
        source
            .plan_item_exchange(&facts.input("oteryn:npc.tereban", "yes", 1))
            .map_err(|error| error.to_string())?
            .ok_or("inventory-backed acceptance")?
            .track_changes,
        plan.track_changes
    );
    let quest_log = "oteryn:quest-progress/quest/u8_6/afathers_burden/quest_log";
    for invalid in [-1, 2] {
        facts.tracks.insert(quest_log.into(), invalid);
        assert!(
            matches!(source.plan_storage(&facts.input("oteryn:npc.tereban", "yes", 1)),
            Err(NpcRuleError::Invalid(message)) if message.contains("track outside bounds"))
        );
    }
    Ok(())
}

#[test]
fn shipped_npc_source_does_not_claim_an_unbound_native_catalogue() -> Result<(), String> {
    let source = NpcRuleCatalog::runtime().map_err(|error| error.to_string())?;
    let frozen = super::super::loader::parse_quest_state(
        include_str!("../../../../../../content/quests/missions/quest-state.json"),
        "npc-binding",
    )
    .map_err(|error| format!("{error:?}"))?;
    assert!(
        source
            .admitted_for_npc("oteryn:npc.palimuth", frozen.catalogue())
            .map_err(|error| error.to_string())?
            .is_none()
    );
    let loaded = super::super::loader::load_embedded_quest_state("npc-binding")
        .map_err(|error| format!("{error:?}"))?;
    let bound = source
        .admitted_for_npc("oteryn:npc.tereban", loaded.catalogue())
        .map_err(|error| error.to_string())?
        .ok_or("bound Father programme")?;
    assert_eq!(bound.programs().len(), 1);
    assert_eq!(
        bound.programs()[0].document.quest,
        "oteryn:quest.a_father_s_burden_quest"
    );
    Ok(())
}

#[test]
fn storage_owner_refuses_inventory_dependent_fallbacks_main_source() -> Result<(), NpcRuleError> {
    let mut conditional = fixture_rule("inventory-first", serde_json::json!([]));
    conditional["guards"] = serde_json::json!([
        {"kind":"item","item":"oteryn:item.tibia.i140","count":1}
    ]);
    let fallback = fixture_rule(
        "fallback",
        serde_json::json!([
            {"kind":"set","track":"oteryn:quest-progress/test/value","value":1}
        ]),
    );
    let catalogue = fixture_catalog(&fixture_document(vec![conditional, fallback]))?;
    let facts = Facts::empty();
    assert!(matches!(
        catalogue.plan_storage(&facts.input("oteryn:npc.test", "mission", 0)),
        Err(NpcRuleError::MissingNativeResolution(_))
    ));
    Ok(())
}

#[test]
fn storage_owner_preserves_order_and_rejects_unowned_effects_main_source()
-> Result<(), NpcRuleError> {
    let mut first = fixture_rule(
        "first",
        serde_json::json!([
            {"kind":"set","track":"oteryn:quest-progress/test/value","value":3}
        ]),
    );
    first["continue_matching"] = true.into();
    let second = fixture_rule(
        "second",
        serde_json::json!([
            {"kind":"add","track":"oteryn:quest-progress/test/value","value":1},
            {"kind":"set_topic","value":2}
        ]),
    );
    let catalogue = fixture_catalog(&fixture_document(vec![first, second]))?;
    let facts = Facts::empty();
    let input = facts.input("oteryn:npc.test", "mission", 0);
    let plan = catalogue
        .plan_storage(&input)?
        .ok_or_else(|| NpcRuleError::Invalid("storage plan".into()))?;
    assert_eq!(plan.track_changes[0].after, 4);
    assert_eq!(plan.next_topic, 2);
    assert_eq!(plan.selected_rule_keys.len(), 2);
    let gift = fixture_rule(
        "gift",
        serde_json::json!([
            {"kind":"give_item","item":"oteryn:item.tibia.i140","count":1}
        ]),
    );
    let catalogue = fixture_catalog(&fixture_document(vec![gift]))?;
    assert!(matches!(
        catalogue.plan_storage(&input),
        Err(NpcRuleError::MissingNativeResolution(_))
    ));
    Ok(())
}

#[test]
fn npc_activation_keeps_exact_legacy_hashes_defaults_and_rejects_incompatible_rollback_main_source()
-> Result<(), String> {
    let text = include_str!("../../../../../../content/quests/missions/quest-state.json");
    let old = super::super::loader::parse_quest_state(text, "npc-legacy")
        .map_err(|e| format!("{e:?}"))?;
    let document: serde_json::Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
    let mut bindings = BTreeMap::new();
    for quest in document["quests"].as_array().ok_or("quests")? {
        for track in quest["tracks"].as_array().ok_or("tracks")? {
            bindings.insert(
                track["key"].as_str().ok_or("key")?.to_owned(),
                track["source_key"].as_str().ok_or("source")?.to_owned(),
            );
        }
    }
    let active = extend(old.catalogue().clone(), &bindings)?;
    let runtime = NpcRuleCatalog::runtime().map_err(|e| e.to_string())?;
    assert_eq!(
        runtime
            .programs()
            .iter()
            .filter(|program| program.document.schema == 1)
            .count(),
        11
    );
    // Canonical main adds the exact source cohort checked above; schema2
    // Mystery has no fabricated legacy definition or absent-value mapping.
    assert_eq!(active.npc_legacy.len(), 9);
    // Main adds independently owned programmes; retain the exact original lane set.
    let original = [
        "oteryn:quest.a_father_s_burden_quest",
        "oteryn:quest.a_pirates_tail",
        "oteryn:quest.bigfoot_s_burden_quest",
        "oteryn:quest.hero_of_rathleton_quest",
        "oteryn:quest.hot_cuisine_quest",
        "oteryn:quest.in_service_of_yalahar_quest",
        "oteryn:quest.the_order_of_the_stag_quest",
    ];
    assert_eq!(
        runtime
            .programs()
            .iter()
            .filter(|p| original.contains(&p.document.quest.as_str()))
            .count(),
        7
    );
    assert_eq!(
        active
            .npc_legacy
            .keys()
            .filter(|q| original.contains(&q.as_str()))
            .count(),
        6
    );
    let mut defaults = BTreeMap::new();
    for track in NpcRuleCatalog::embedded()
        .map_err(|e| e.to_string())?
        .programs()
        .iter()
        .flat_map(|program| &program.document.tracks)
    {
        *defaults.entry(track.initial).or_insert(0usize) += 1;
    }
    assert_eq!(defaults, BTreeMap::from([(-1, 192), (0, 52)]));
    for program in NpcRuleCatalog::embedded()
        .map_err(|e| e.to_string())?
        .programs()
    {
        let quest = &program.document.quest;
        if let Some((hash, defaults)) = active.npc_legacy.get(quest) {
            assert_eq!(Some(*hash), old.catalogue().definition_hash(quest));
            assert_ne!(Some(*hash), active.definition_hash(quest));
            assert_eq!(legacy_defaults(&active, quest, hash), Some(defaults));
            assert!(legacy_defaults(&active, quest, &[0; 32]).is_none());
            // Old artifacts cannot accept a new pin: rollback requires this verified
            // compatibility adapter, never activation of an incompatible old binary.
            assert!(
                legacy_defaults(
                    old.catalogue(),
                    quest,
                    &active.definition_hash(quest).ok_or("hash")?
                )
                .is_none()
            );
            for track in old
                .catalogue()
                .tracks
                .values()
                .filter(|t| &t.quest == quest)
            {
                assert_eq!(defaults.get(&track.key), Some(&track.initial));
                let new = active.track(&track.key).ok_or("track")?;
                assert_eq!(
                    new.initial, track.initial,
                    "all existing native consumers keep their absent value"
                );
                assert!(new.min <= track.min && new.max >= track.max);
            }
            for transition in old
                .catalogue()
                .transitions
                .values()
                .filter(|t| &t.quest == quest)
            {
                assert_eq!(active.transition(&transition.key), Some(transition));
            }
        } else if runtime.program(quest).is_none() {
            assert_eq!(
                active.definition_hash(quest),
                old.catalogue().definition_hash(quest)
            );
        }
    }
    runtime
        .clone()
        .with_native_bounds(&active)
        .map_err(|e| e.to_string())?;
    let loaded = super::super::loader::load_embedded_quest_state("npc-legacy")
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(loaded.catalogue().npc_legacy.len(), 9);
    assert!(
        !loaded
            .catalogue()
            .npc_legacy
            .contains_key("oteryn:quest.authored.mystery_of_the_valley_quest")
    );
    assert_eq!(
        loaded
            .catalogue()
            .npc_legacy
            .keys()
            .filter(|quest| original.contains(&quest.as_str()))
            .count(),
        6
    );
    Ok(())
}

#[test]
fn storage_owner_commits_exact_father_acceptance_without_raising_general_cap_main_source()
-> Result<(), NpcRuleError> {
    let source = NpcRuleCatalog::runtime()?;
    let facts = Facts::empty();
    let input = facts.input("oteryn:npc.tereban", "yes", 1);
    assert_eq!(MAX_NPC_TRACK_CHANGES, 8);
    let plan = source
        .plan_storage(&input)?
        .ok_or_else(|| NpcRuleError::Invalid("father acceptance".into()))?;
    assert_eq!(plan.track_changes.len(), 11);
    assert_eq!(
        plan.selected_rule_keys,
        vec!["oteryn:quest-npc/a_father_s_burden_quest/tereban_functions/mission-accept"]
    );
    assert!(facts.tracks.is_empty());
    Ok(())
}

#[test]
fn authored_npc_program_requires_explicit_recipe_provenance() -> Result<(), NpcRuleError> {
    let mut document: serde_json::Value =
        serde_json::from_str(include_str!("test-fixtures/hero_greeting.json"))
            .map_err(|e| NpcRuleError::Json(e.to_string()))?;
    let quest = "oteryn:quest.authored.mystery_of_the_valley_quest";
    let source = "oteryn-authored:chosen-recipe:mystery-of-the-valley-quest:authored-r1";
    document["schema"] = serde_json::json!(2);
    document["quest"] = serde_json::json!(quest);
    for group in ["tracks", "rules"] {
        for row in document[group]
            .as_array_mut()
            .ok_or_else(|| NpcRuleError::Invalid("rows".into()))?
        {
            row["quest"] = serde_json::json!(quest);
            row["sources"] = serde_json::json!([source]);
        }
    }
    let encode = |value: &serde_json::Value| {
        serde_json::to_string(value).map_err(|e| NpcRuleError::Json(e.to_string()))
    };
    NpcRuleCatalog::from_json_documents(&[&encode(&document)?])?;
    for (schema, quest, provenance) in [
        (1, quest, source),
        (2, "oteryn:quest.hero_of_rathleton_quest", source),
        (
            2,
            quest,
            "oteryn-authored:chosen-recipe:unrelated:authored-r1",
        ),
        (
            2,
            quest,
            "https://tibia.fandom.com/wiki/Mystery_of_the_Valley_Quest",
        ),
    ] {
        let mut invalid = document.clone();
        invalid["schema"] = serde_json::json!(schema);
        invalid["quest"] = serde_json::json!(quest);
        invalid["rules"][0]["sources"] = serde_json::json!([provenance]);
        assert!(NpcRuleCatalog::from_json_documents(&[&encode(&invalid)?]).is_err());
    }
    // The original source-only schema remains valid and keeps its digest contract.
    NpcRuleCatalog::from_json_documents(&[include_str!("test-fixtures/hero_greeting.json")])?;
    Ok(())
}

#[test]
fn mystery_access_source_transitions_preserve_their_native_effects() -> Result<(), String> {
    let loaded = crate::quest::loader::load_embedded_quest_state("content-1")
        .map_err(|e| format!("{e:?}"))?;
    for key in [
        "oteryn:quest-transition/quest/u8_0/the_ice_islands/mission06/npc_1",
        "oteryn:quest-transition/quest/u8_54/the_new_frontier/mission08/movement_1",
    ] {
        let transition = loaded
            .catalogue()
            .transition(key)
            .ok_or_else(|| format!("missing {key}"))?;
        assert_eq!(transition.effects.len(), 1);
        if key.contains("mission08") {
            assert_eq!(
                transition.effects[0].effect,
                crate::quest::QuestEffectKind::Computed
            );
        } else {
            assert_eq!(
                transition.effects[0].effect,
                crate::quest::QuestEffectKind::Set(1)
            );
        }
    }
    let access = loaded
        .catalogue()
        .transition("oteryn:quest-transition/quest/u8_54/the_new_frontier/questline/action_2")
        .ok_or("native prison secret-door event")?;
    assert_eq!(
        access.effects[0].from,
        crate::quest::QuestComparison::Lt(23)
    );
    assert_eq!(
        access.effects[0].effect,
        crate::quest::QuestEffectKind::Set(23)
    );
    Ok(())
}
