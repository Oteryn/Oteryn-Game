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
        player_name: None,
        topic: 0,
        tracks: &tracks,
        inventory: &inventory,
        now_epoch_seconds: 1_700_000_000,
        calendar_month: 11,
        premium_current: false,
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

#[test]
fn lai_gives_read_only_hints_for_each_fragment_and_the_finished_statue() -> Result<(), NpcRuleError>
{
    let catalogue = NpcRuleCatalog::runtime()?;
    let mut facts = Facts::empty();
    let track = "oteryn:quest-progress/chosen/mystery-of-the-valley/stage";
    for (stage, hint) in [
        (1, "ice and snow"),
        (2, "Bamboo"),
        (3, "large river"),
        (4, "Kha'zeel"),
        (5, "Kha'zeel"),
        (6, "reconstructed"),
    ] {
        facts.tracks.insert(track.into(), stage);
        let mut input = facts.input("oteryn:npc.lai", "fragment of a statue", 0);
        input.premium_current = true;
        let plan = catalogue
            .plan_storage(&input)?
            .ok_or_else(|| NpcRuleError::Invalid(format!("missing Lai hint at stage {stage}")))?;
        assert!(
            plan.reply.iter().any(|line| line.contains(hint)),
            "{stage}: {:?}",
            plan.reply
        );
        assert!(plan.track_changes.is_empty());
        assert!(plan.effects.is_empty());
    }
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
            player_name: None,
            topic,
            tracks: &self.tracks,
            inventory: &self.inventory,
            now_epoch_seconds: 1_700_000_000,
            calendar_month: 11,
            premium_current: false,
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
fn m04_explicit_terminal_npc_rule_is_admitted() -> Result<(), NpcRuleError> {
    let mut terminal = fixture_rule(
        "terminal",
        serde_json::json!([
            {"kind":"set","track":"oteryn:quest-progress/test/value","value":1}
        ]),
    );
    terminal["completes"] = serde_json::json!(true);
    let catalog = fixture_catalog(&fixture_document(vec![terminal]))?;
    let facts = Facts::empty();
    let plan = catalog
        .plan_storage(&facts.input("oteryn:npc.test", "mission", 0))?
        .ok_or_else(|| NpcRuleError::Invalid("terminal plan".into()))?;
    assert!(plan.completes);
    let plain = fixture_catalog(&fixture_document(vec![fixture_rule(
        "plain",
        serde_json::json!([
            {"kind":"set","track":"oteryn:quest-progress/test/value","value":1}
        ]),
    )]))?;
    assert!(
        !plain
            .plan_storage(&facts.input("oteryn:npc.test", "mission", 0))?
            .ok_or_else(|| NpcRuleError::Invalid("plain plan".into()))?
            .completes
    );
    Ok(())
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

// The accepted source cohorts from both parents retain these exact predecessors.
// Draconia has no historical definition; Stag and schema2 have separate owners.
const EXPECTED_LEGACY_QUESTS: [&str; 29] = [
    "oteryn:quest.a_father_s_burden_quest",
    "oteryn:quest.a_pirates_tail",
    "oteryn:quest.bigfoot_s_burden_quest",
    "oteryn:quest.blood_brothers_quest",
    "oteryn:quest.cults_of_tibia_quest",
    "oteryn:quest.dangerous_depths_quest",
    "oteryn:quest.demon_oak",
    "oteryn:quest.elemental_spheres_quest",
    "oteryn:quest.forgotten_knowledge_quest",
    "oteryn:quest.hero_of_rathleton_quest",
    "oteryn:quest.hot_cuisine_quest",
    "oteryn:quest.in_service_of_yalahar_quest",
    "oteryn:quest.killing_in_the_name_of_quest",
    "oteryn:quest.kilmaresh_quest",
    "oteryn:quest.marlin_trophy_quest",
    "oteryn:quest.outfit_and_addon_quests",
    "oteryn:quest.spike_task",
    "oteryn:quest.the_ape_city_quest",
    "oteryn:quest.the_beginning_quest",
    "oteryn:quest.the_djinn_war_efreet_faction",
    "oteryn:quest.the_djinn_war_marid_faction",
    "oteryn:quest.the_dream_courts_quest",
    "oteryn:quest.the_first_dragon_quest",
    "oteryn:quest.the_hidden_city_of_beregar_quest",
    "oteryn:quest.the_inquisition_quest",
    "oteryn:quest.the_paradox_tower_quest",
    "oteryn:quest.the_postman_missions_quest",
    "oteryn:quest.the_white_raven_monastery_quest",
    "oteryn:quest.tibia_tales",
    "oteryn:quest.tower_defence_quest",
    "oteryn:quest.wrath_of_the_emperor_quest",
    "oteryn:quest.troll_sabotage_quest",
];

#[test]
fn npc_only_track_owners_keep_exact_existing_quest_predecessors() -> Result<(), String> {
    let text = include_str!("../../../../../../content/quests/missions/quest-state.json");
    let old = super::super::loader::parse_quest_state(text, "npc-legacy")
        .map_err(|error| format!("{error:?}"))?;
    let document: serde_json::Value =
        serde_json::from_str(text).map_err(|error| error.to_string())?;
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
    let runtime = NpcRuleCatalog::runtime().map_err(|error| error.to_string())?;
    for (quest, program_key) in [
        ("oteryn:quest.elemental_spheres_quest", None),
        (
            "oteryn:quest.outfit_and_addon_quests",
            Some("oteryn:quest-npc-program.citizen-source-sagas"),
        ),
    ] {
        let program = runtime
            .programs()
            .iter()
            .find(|program| {
                program.document.quest == quest
                    && program.document.program_key.as_deref() == program_key
            })
            .ok_or("accepted NPC owner")?;
        assert!(additive_legacy_owner(program));
        assert!(
            program
                .document
                .tracks
                .iter()
                .all(|track| !bindings.contains_key(&track.key))
        );
        let old_hash = old.catalogue().definition_hash(quest).ok_or("old hash")?;
        let defaults = old
            .catalogue()
            .tracks
            .values()
            .filter(|track| track.quest == quest)
            .map(|track| (track.key.clone(), track.initial))
            .collect::<BTreeMap<_, _>>();
        assert_ne!(active.definition_hash(quest), Some(old_hash));
        assert_eq!(legacy_defaults(&active, quest, &old_hash), Some(&defaults));
        let mut foreign_hash = old_hash;
        foreign_hash[0] ^= 1;
        assert!(legacy_defaults(&active, quest, &foreign_hash).is_none());

        let mut foreign_key = program.clone();
        foreign_key.document.program_key = Some("oteryn:quest-npc-program.foreign".into());
        assert!(!additive_legacy_owner(&foreign_key));
        let mut foreign_schema = program.clone();
        foreign_schema.document.schema = 2;
        assert!(!additive_legacy_owner(&foreign_schema));
        for foreign_quest in [
            "oteryn:quest.foreign",
            "oteryn:quest.draconia_quest",
            "oteryn:quest.the_order_of_the_stag_quest",
        ] {
            let mut foreign_owner = program.clone();
            foreign_owner.document.quest = foreign_quest.into();
            assert!(!additive_legacy_owner(&foreign_owner));
        }
        let mut crossed_owner = program.clone();
        crossed_owner.document.quest = if program_key.is_none() {
            "oteryn:quest.outfit_and_addon_quests"
        } else {
            "oteryn:quest.elemental_spheres_quest"
        }
        .into();
        assert!(!additive_legacy_owner(&crossed_owner));
    }
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
        "oteryn:quest.blood_brothers_quest",
        "oteryn:quest.draconia_quest",
        "oteryn:quest.forgotten_knowledge_quest",
        "oteryn:quest.elemental_spheres_quest",
        "oteryn:quest.hero_of_rathleton_quest",
        "oteryn:quest.hot_cuisine_quest",
        "oteryn:quest.in_service_of_yalahar_quest",
        "oteryn:quest.marlin_trophy_quest",
        "oteryn:quest.outfit_and_addon_quests",
        "oteryn:quest.killing_in_the_name_of_quest",
        "oteryn:quest.kilmaresh_quest",
        "oteryn:quest.the_ape_city_quest",
        "oteryn:quest.the_beginning_quest",
        "oteryn:quest.the_djinn_war_efreet_faction",
        "oteryn:quest.the_djinn_war_marid_faction",
        "oteryn:quest.the_dream_courts_quest",
        "oteryn:quest.the_first_dragon_quest",
        "oteryn:quest.the_hidden_city_of_beregar_quest",
        "oteryn:quest.the_inquisition_quest",
        "oteryn:quest.the_order_of_the_stag_quest",
        "oteryn:quest.the_paradox_tower_quest",
        "oteryn:quest.the_postman_missions_quest",
        "oteryn:quest.the_white_raven_monastery_quest",
        "oteryn:quest.spike_task",
        "oteryn:quest.tibia_tales",
        "oteryn:quest.tower_defence_quest",
        "oteryn:quest.wrath_of_the_emperor_quest",
        "oteryn:quest.troll_sabotage_quest",
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
    let dangerous_depths_programmes: Vec<_> = runtime
        .programs()
        .iter()
        .filter(|program| program.document.quest == questpc2_gnomus_source::QUEST)
        .map(|program| program.document.program_key.as_deref())
        .collect();
    assert_eq!(dangerous_depths_programmes.len(), 3);
    assert_eq!(
        dangerous_depths_programmes
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>(),
        std::collections::BTreeSet::from([
            None,
            Some(questpc2_gnomus_source::PROGRAM),
            Some("oteryn:quest-npc-program.dangerous-depths-subterranean-life"),
        ])
    );
    // The two Dangerous Depths supplements and the schema2 Mystery add three owners.
    assert_eq!(runtime.programs().len(), expected_programmes.len() + 3);
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
        expected_legacy,
        std::collections::BTreeSet::from(EXPECTED_LEGACY_QUESTS)
    );
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
    // M04 adds seven -1 defaults: three per Djinn faction and one monastery diary.
    // Paradox adds three source-backed Riddler tracks, also starting absent at -1.
    // Dream Courts, Tamoril, Gnomus, Julius and Citizen scopes remain registered.
    assert_eq!(defaults, BTreeMap::from([(-1, 214), (0, 55)]));
    let postman = runtime
        .program("oteryn:quest.the_postman_missions_quest")
        .ok_or("Postman programme")?;
    assert_eq!(postman.document.tracks.len(), 18);
    assert!(
        postman
            .document
            .tracks
            .iter()
            .all(|track| track.initial == 0)
    );
    let paradox = NpcRuleCatalog::embedded()
        .map_err(|error| error.to_string())?
        .program("oteryn:quest.the_paradox_tower_quest")
        .ok_or("Paradox source programme")?;
    for (suffix, line) in [
        ("the_feared_hugo", 78),
        ("favorite_colour", 109),
        ("mathemagics", 129),
    ] {
        let key = format!("oteryn:quest-progress/quest/u7_24/the_paradox_tower/{suffix}");
        let track = paradox
            .document
            .tracks
            .iter()
            .find(|track| track.key == key)
            .ok_or("Paradox Riddler track")?;
        assert_eq!(track.quest, "oteryn:quest.the_paradox_tower_quest");
        assert_eq!(
            (track.initial, track.min, track.max),
            (-1, -1, i64::from(i32::MAX))
        );
        assert_eq!(
            track.sources,
            vec![format!(
                "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/riddler.lua:{line}"
            )]
        );
    }
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
    let mut expected_programmes: Vec<_> = EXPECTED_LEGACY_QUESTS
        .into_iter()
        .map(|quest| {
            let program = match quest {
                questpc2_gerimor_source::QUEST => Some(questpc2_gerimor_source::PROGRAM),
                questpc2_oldrak_source::QUEST => Some(questpc2_oldrak_source::PROGRAM),
                questpc2_albinius_source::QUEST => Some(questpc2_albinius_source::PROGRAM),
                questpc2_henricus_source::QUEST => Some(questpc2_henricus_source::PROGRAM),
                "oteryn:quest.kilmaresh_quest" => {
                    Some("oteryn:quest-npc-program.m41_kilmaresh_finale")
                }
                "oteryn:quest.outfit_and_addon_quests" => {
                    Some("oteryn:quest-npc-program.citizen-source-sagas")
                }
                "oteryn:quest.the_first_dragon_quest" => {
                    Some("oteryn:quest-npc-program.first-dragon-calendar")
                }
                "oteryn:quest.the_postman_missions_quest" => {
                    Some("oteryn:quest-npc-program.postman")
                }
                "oteryn:quest.tibia_tales" => Some("oteryn:quest-npc-program.hidden_threats"),
                _ => None,
            };
            (quest, program)
        })
        .collect();
    expected_programmes.extend([
        (
            "oteryn:quest.draconia_quest",
            Some("oteryn:quest-npc-program.draconia"),
        ),
        ("oteryn:quest.the_order_of_the_stag_quest", None),
        (
            questpc2_gnomus_source::QUEST,
            Some(questpc2_gnomus_source::PROGRAM),
        ),
    ]);
    let programmes: Vec<_> = runtime
        .programs()
        .iter()
        .filter(|program| program.document.schema == 1)
        .map(|program| {
            (
                program.document.quest.as_str(),
                program.document.program_key.as_deref(),
            )
        })
        .collect();
    assert_eq!(programmes.len(), expected_programmes.len());
    assert_eq!(
        programmes
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>(),
        expected_programmes.into_iter().collect()
    );
    // Canonical main adds the exact source cohort checked above; schema2
    // Mystery has no fabricated legacy definition or absent-value mapping.
    assert_eq!(
        active
            .npc_legacy
            .keys()
            .map(String::as_str)
            .collect::<std::collections::BTreeSet<_>>(),
        std::collections::BTreeSet::from(EXPECTED_LEGACY_QUESTS)
    );
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
    assert_eq!(defaults, BTreeMap::from([(-1, 214), (0, 55)]));
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
    assert_eq!(
        loaded
            .catalogue()
            .npc_legacy
            .keys()
            .map(String::as_str)
            .collect::<std::collections::BTreeSet<_>>(),
        std::collections::BTreeSet::from(EXPECTED_LEGACY_QUESTS)
    );
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

#[test]
fn durable_outfit_exchange_uses_owned_snapshot_and_keeps_unknown_effects_closed()
-> Result<(), NpcRuleError> {
    let mut rule = fixture_rule(
        "outfit",
        serde_json::json!([
            {"kind":"take_item","item":"oteryn:item.tibia.i5890","count":1},
            {"kind":"outfit_addon","look_type":128,"addon":1},
            {"kind":"set","track":"oteryn:quest-progress/test/value","value":1}
        ]),
    );
    rule["guards"] = serde_json::json!([{"kind":"outfit","look_type":128,"addon":0}]);
    let catalogue = fixture_catalog(&fixture_document(vec![rule.clone()]))?;
    let mut facts = Facts::empty();
    facts.inventory.items.insert(
        NpcItemKey {
            item: "oteryn:item.tibia.i5890".into(),
            subtype: None,
        },
        1,
    );
    assert!(
        catalogue
            .plan_item_exchange(&facts.input("oteryn:npc.test", "mission", 0))?
            .is_none()
    );
    facts.outfits.insert(128, 0);
    let plan = catalogue
        .plan_item_exchange(&facts.input("oteryn:npc.test", "mission", 0))?
        .ok_or_else(|| NpcRuleError::Invalid("missing outfit hand-in".into()))?;
    assert_eq!(plan.track_changes.len(), 1);
    assert!(plan.effects.iter().any(|effect| matches!(
        effect,
        NpcRuleEffect::OutfitAddon {
            look_type: 128,
            addon: 1
        }
    )));
    rule["effects"]
        .as_array_mut()
        .ok_or_else(|| NpcRuleError::Invalid("effects".into()))?
        .push(serde_json::json!({"kind":"give_money","amount":1}));
    assert!(
        fixture_catalog(&fixture_document(vec![rule]))?
            .plan_item_exchange(&facts.input("oteryn:npc.test", "mission", 0))
            .is_err()
    );
    Ok(())
}

#[test]
fn native_yalahar_outfit_branch_has_a_durable_source_plan() -> Result<(), NpcRuleError> {
    let mut facts = Facts::empty();
    facts.tracks.insert(
        "oteryn:quest-progress/quest/u8_4/in_service_of_yalahar/questline".into(),
        52,
    );
    let plan = NpcRuleCatalog::runtime()?
        .plan_outfit_exchange(&facts.input("oteryn:npc.palimuth", "mission", 0))?
        .ok_or_else(|| NpcRuleError::Invalid("missing native outfit branch".into()))?;
    assert_eq!(
        plan.selected_rule_keys,
        vec!["oteryn:quest-npc/in_service_of_yalahar_quest/palimuth/conversation-244-27-1-1"]
    );
    assert_eq!(plan.track_changes.len(), 3);
    assert_eq!(
        plan.effects
            .iter()
            .filter(|effect| matches!(effect, NpcRuleEffect::Outfit { .. }))
            .count(),
        2
    );
    Ok(())
}

#[test]
fn live_eligibility_guards_refuse_below_level_wrong_vocation_and_expired_premium()
-> Result<(), NpcRuleError> {
    let mut rule = fixture_rule(
        "live_eligibility",
        serde_json::json!([
            {"kind":"set","track":"oteryn:quest-progress/test/value","value":1}
        ]),
    );
    rule["guards"] = serde_json::json!([
        {"kind":"level","op":"ge","value":80},
        {"kind":"vocation","value":"oteryn:vocation.knight"},
        {"kind":"premium","value":true}
    ]);
    let catalog = fixture_catalog(&fixture_document(vec![rule]))?;
    let facts = Facts::empty();
    for (level, vocation, premium, accepted) in [
        (79, "oteryn:vocation.knight", true, false),
        (80, "oteryn:vocation.druid", true, false),
        (80, "oteryn:vocation.knight", false, false),
        (80, "oteryn:vocation.knight", true, true),
    ] {
        let mut input = facts.input("oteryn:npc.test", "mission", 0);
        input.level = level;
        input.vocation = vocation;
        input.premium_current = premium;
        assert_eq!(catalog.plan_storage(&input)?.is_some(), accepted);
    }
    Ok(())
}

#[test]
fn elemental_spheres_live_eligibility_selects_each_source_vocation_reward()
-> Result<(), NpcRuleError> {
    let catalog = NpcRuleCatalog::runtime()?;
    let track = "oteryn:quest-progress/elemental_spheres/quest_line";
    for (vocation, reward) in [
        ("sorcerer", 8039),
        ("druid", 8041),
        ("paladin", 8025),
        ("knight", 8055),
    ] {
        let mut facts = Facts::empty();
        facts.tracks.insert(track.into(), 2);
        facts.inventory.items.insert(
            NpcItemKey {
                item: "oteryn:item.tibia.i954".into(),
                subtype: None,
            },
            1,
        );
        let key = format!("oteryn:vocation.{vocation}");
        let mut input = facts.input("oteryn:npc.arkulius", "mission", 0);
        input.vocation = &key;
        input.level = 80;
        input.premium_current = true;
        let plan = catalog
            .plan_item_exchange(&input)?
            .ok_or_else(|| NpcRuleError::Invalid("missing sphere reward".into()))?;
        assert!(plan.effects.iter().any(|effect| matches!(effect,
            NpcRuleEffect::GiveItem {item,count:1,..} if item == &format!("oteryn:item.tibia.i{reward}"))));
        assert!(plan.track_changes.iter().any(|change| change.after == 3));
        input.premium_current = false;
        assert!(catalog.plan_item_exchange(&input)?.is_none());
    }
    Ok(())
}

#[test]
fn m54_source_completion_does_not_complete_shared_canonical_quest() -> Result<(), NpcRuleError> {
    let marker = "oteryn:quest-progress/source-completion/test_npc/rook";
    let rule = fixture_rule(
        "rook",
        serde_json::json!([
            {"kind":"complete_source", "track":marker}
        ]),
    );
    let mut document = fixture_document(vec![rule]);
    document["tracks"][0]["key"] = marker.into();
    document["tracks"][0]["initial"] = 0.into();
    document["tracks"][0]["min"] = 0.into();
    document["tracks"][0]["max"] = 1.into();
    let catalog = fixture_catalog(&document)?;
    let mut facts = Facts::empty();
    facts.tracks.insert(
        "oteryn:quest-progress/source-completion/test_npc/mainland".into(),
        1,
    );
    let plan = catalog
        .plan_storage(&facts.input("oteryn:npc.test", "mission", 0))?
        .ok_or_else(|| NpcRuleError::Invalid("source completion plan".into()))?;
    assert!(!plan.completes);
    assert_eq!(
        plan.track_changes,
        vec![NpcTrackChange {
            key: marker.into(),
            before: 0,
            after: 1
        }]
    );
    facts.tracks.insert(marker.into(), 1);
    assert!(
        catalog
            .plan_storage(&facts.input("oteryn:npc.test", "mission", 0))?
            .is_none()
    );
    document["rules"][0]["completes"] = true.into();
    assert!(fixture_catalog(&document).is_err());
    document["rules"][0]["completes"] = false.into();
    document["rules"][0]["effects"][0]["track"] = "oteryn:quest-progress/test/value".into();
    assert!(fixture_catalog(&document).is_err());
    Ok(())
}

#[test]
fn m54_citizen_source_aliases_require_both_local_sagas() -> Result<(), NpcRuleError> {
    let runtime = NpcRuleCatalog::runtime()?;
    let loaded = super::super::loader::load_embedded_quest_state("m54-source-sagas")
        .map_err(|e| NpcRuleError::Invalid(format!("{e:?}")))?;
    let native = loaded.catalogue();
    let prefix = "oteryn:quest-progress/quest/u7_8/";
    let mut facts = Facts::empty();
    for (track, value) in [
        ("citizen_outfits/addon_backpack", 3),
        ("citizen_outfits/addon_hat", 2),
        ("citizen_outfits_rook/addon_backpack_rook", 0),
        ("citizen_outfits_rook/addon_hat_rook", 0),
    ] {
        facts.tracks.insert(format!("{prefix}{track}"), value);
    }
    for npc in ["oteryn:npc.lubo", "oteryn:npc.hanna", "oteryn:npc.amber"] {
        let admitted = runtime
            .admitted_for_npc(npc, native)?
            .ok_or_else(|| NpcRuleError::Invalid(format!("missing Citizen alias {npc}")))?;
        let plan = admitted.plan_storage(&facts.input(npc, "addon", 0))?;
        if npc == "oteryn:npc.amber" {
            assert!(plan.is_none());
        } else {
            let plan = plan.ok_or_else(|| NpcRuleError::Invalid("mainland proof".into()))?;
            assert!(!plan.completes);
            assert_eq!(plan.quest, "oteryn:quest.outfit_and_addon_quests");
            assert_eq!(
                plan.track_changes[0].key,
                "oteryn:quest-progress/source-completion/outfit_and_addon_quests/citizen-mainland"
            );
        }
    }
    facts.tracks.insert(
        format!("{prefix}citizen_outfits_rook/addon_backpack_rook"),
        3,
    );
    let rook = runtime
        .admitted_for_npc("oteryn:npc.amber", native)?
        .ok_or_else(|| NpcRuleError::Invalid("Rook source".into()))?;
    assert!(
        rook.plan_storage(&facts.input("oteryn:npc.amber", "addon", 0))?
            .is_none()
    );
    facts
        .tracks
        .insert(format!("{prefix}citizen_outfits_rook/addon_hat_rook"), 2);
    let plan = rook
        .plan_storage(&facts.input("oteryn:npc.amber", "addon", 0))?
        .ok_or_else(|| NpcRuleError::Invalid("Rook proof".into()))?;
    assert!(!plan.completes);
    assert_eq!(
        plan.track_changes[0].key,
        "oteryn:quest-progress/source-completion/outfit_and_addon_quests/citizen-rook"
    );
    Ok(())
}

#[test]
fn measurements_offer_reopens_at_the_stored_deadline() -> Result<(), NpcRuleError> {
    let catalogue = NpcRuleCatalog::runtime()?;
    let mut facts = Facts::empty();
    let prefix = "oteryn:quest-progress/quest/u11_50/dangerous_depths/gnomes/";
    facts.tracks.insert(format!("{prefix}measurements"), 2);
    facts
        .tracks
        .insert(format!("{prefix}time_task_measurements"), 1_700_000_000);
    let mut input = facts.input("oteryn:npc.gnomus", "measurements", 0);
    input.now_epoch_seconds -= 1;
    let cooldown = catalogue
        .plan(&input)?
        .ok_or_else(|| NpcRuleError::Invalid("missing measurements cooldown".into()))?;
    assert_eq!(cooldown.next_topic, 1);
    input.now_epoch_seconds += 1;
    let offer = catalogue
        .plan(&input)?
        .ok_or_else(|| NpcRuleError::Invalid("missing measurements restart".into()))?;
    assert_eq!(offer.next_topic, 2);
    assert!(offer.track_changes.is_empty());
    Ok(())
}

#[test]
fn subterranean_life_accepts_reports_and_restarts() -> Result<(), NpcRuleError> {
    let catalogue = NpcRuleCatalog::runtime()?;
    let mut facts = Facts::empty();
    let prefix = "oteryn:quest-progress/quest/u11_50/dangerous_depths/dwarves/";
    let npc = "oteryn:npc.klom_stonecutter";
    let offer = catalogue
        .plan(&facts.input(npc, "subterraneans", 0))?
        .ok_or_else(|| NpcRuleError::Invalid("missing Subterranean Life offer".into()))?;
    let accept = catalogue
        .plan(&facts.input(npc, "yes", offer.next_topic))?
        .ok_or_else(|| NpcRuleError::Invalid("missing Subterranean Life acceptance".into()))?;
    for change in accept.track_changes {
        facts.tracks.insert(change.key, change.after);
    }
    assert_eq!(facts.tracks.get(&format!("{prefix}subterranean")), Some(&1));
    assert_eq!(facts.tracks.get(&format!("{prefix}organisms")), Some(&0));
    facts.tracks.insert(format!("{prefix}organisms"), 50);
    let report = catalogue
        .plan_inventory(&facts.input(npc, "subterraneans", accept.next_topic))?
        .ok_or_else(|| NpcRuleError::Invalid("missing Subterranean Life report".into()))?;
    assert!(report.effects.iter().any(|effect| matches!(effect,
        NpcRuleEffect::GiveItem { item, count: 1, .. } if item == "oteryn:item.tibia.i27654")));
    for change in report.track_changes {
        facts.tracks.insert(change.key, change.after);
    }
    assert_eq!(facts.tracks.get(&format!("{prefix}subterranean")), Some(&2));
    assert_eq!(facts.tracks.get(&format!("{prefix}points")), Some(&0));
    let mut input = facts.input(npc, "subterraneans", report.next_topic);
    input.now_epoch_seconds += 71_999;
    let cooldown = catalogue
        .plan(&input)?
        .ok_or_else(|| NpcRuleError::Invalid("missing cooldown".into()))?;
    assert_eq!(cooldown.next_topic, 1);
    input.now_epoch_seconds += 1;
    let restart = catalogue
        .plan(&input)?
        .ok_or_else(|| NpcRuleError::Invalid("missing restart".into()))?;
    assert_eq!(restart.next_topic, offer.next_topic);
    Ok(())
}

#[test]
fn spike_tasks_enforce_level_attempt_and_report_cooldown() -> Result<(), NpcRuleError> {
    let catalogue = NpcRuleCatalog::runtime()?;
    let prefix = "oteryn:quest-progress/quest/u10_20/spike_task_quest/";
    for (npc, task, word, report_word, level, goal, xp) in [
        ("gnomilly", "upper_kill", "kill", "killing", 25, 7, 1000),
        ("gnombold", "middle_kill", "kill", "killing", 50, 7, 2000),
        ("gnomargery", "lower_kill", "kill", "kill", 80, 7, 3500),
        (
            "gnomilly",
            "upper_pacifier",
            "pacifiers",
            "pacifiers",
            25,
            7,
            1000,
        ),
        (
            "gnombold",
            "middle_mushroom",
            "fertilise",
            "fertilisation",
            50,
            4,
            2000,
        ),
        (
            "gnombold",
            "middle_charge",
            "charges",
            "charges",
            50,
            3,
            2000,
        ),
    ] {
        let mut facts = Facts::empty();
        let npc = format!("oteryn:npc.{npc}");
        let stage = format!("{prefix}spike_{task}_main");
        let mut request = facts.input(&npc, word, 0);
        request.level = level;
        let offer = catalogue
            .plan(&request)?
            .ok_or_else(|| NpcRuleError::Invalid(format!("missing {task} offer")))?;
        assert_ne!(offer.next_topic, 0);
        request.level -= 1;
        assert!(catalogue.plan(&request)?.is_none());
        let mut confirm = facts.input(&npc, "yes", offer.next_topic);
        confirm.level = level;
        let start = catalogue
            .plan(&confirm)?
            .ok_or_else(|| NpcRuleError::Invalid(format!("missing {task} acceptance")))?;
        assert!(
            start
                .track_changes
                .iter()
                .any(|change| change.key == stage && change.after == 0)
        );
        facts.tracks.insert(stage.clone(), goal);
        let report = catalogue
            .plan(&facts.input(&npc, "report", 0))?
            .ok_or_else(|| NpcRuleError::Invalid("missing report prompt".into()))?;
        let result = catalogue
            .plan(&facts.input(&npc, report_word, report.next_topic))?
            .ok_or_else(|| NpcRuleError::Invalid(format!("missing {task} report")))?;
        assert!(
            result
                .effects
                .contains(&NpcRuleEffect::Experience { amount: xp })
        );
        assert!(
            result
                .track_changes
                .iter()
                .any(|change| change.key == stage && change.after == -1)
        );
        for change in result.track_changes {
            facts.tracks.insert(change.key, change.after);
        }
        let mut request = facts.input(&npc, word, 0);
        request.level = level;
        request.now_epoch_seconds += 72_000;
        assert_eq!(
            catalogue.plan(&request)?.map(|plan| plan.next_topic),
            Some(0)
        );
        request.now_epoch_seconds += 1;
        assert_eq!(
            catalogue.plan(&request)?.map(|plan| plan.next_topic),
            Some(offer.next_topic)
        );
    }
    Ok(())
}

#[test]
fn spike_undercover_counts_each_contact_once_per_attempt() -> Result<(), NpcRuleError> {
    let catalogue = NpcRuleCatalog::runtime()?;
    let mut facts = Facts::empty();
    let stage = "oteryn:quest-progress/quest/u10_20/spike_task_quest/spike_lower_undercover_main";
    facts.tracks.insert(stage.into(), 0);
    for (index, npc) in [
        "oteryn:npc.a_drillworm",
        "oteryn:npc.a_vulcongra",
        "oteryn:npc.a_lost_thrower",
    ]
    .into_iter()
    .enumerate()
    {
        let mut hello = facts.input(npc, "hi", 0);
        hello.trigger = NpcRuleTrigger::Greet;
        let report = catalogue
            .plan(&hello)?
            .ok_or_else(|| NpcRuleError::Invalid("undercover contact missing".into()))?;
        assert!(
            report
                .track_changes
                .iter()
                .any(|change| change.key == stage && change.after == index as i64 + 1)
        );
        for change in report.track_changes {
            facts.tracks.insert(change.key, change.after);
        }
        let mut hello = facts.input(npc, "hi", 0);
        hello.trigger = NpcRuleTrigger::Greet;
        assert!(catalogue.plan(&hello)?.is_none());
    }
    Ok(())
}

#[test]
fn dangerous_depths_offers_reset_the_distinct_survey_flags() -> Result<(), NpcRuleError> {
    let catalogue = NpcRuleCatalog::runtime()?;
    let prefix = "oteryn:quest-progress/quest/u11_50/dangerous_depths/gnomes/";
    let npc = "oteryn:npc.gnomus";
    for (name, counter, source, flags) in [
        (
            "measurements",
            "location_count",
            "gnome_chart_chest",
            &[
                "location_a",
                "location_b",
                "location_c",
                "location_d",
                "location_e",
            ][..],
        ),
        (
            "charting",
            "charting_count",
            "gnome_chart_paper",
            &[
                "old_gate",
                "lost_ruin",
                "the_gaze",
                "outpost",
                "bastion",
                "broken_tower",
            ][..],
        ),
    ] {
        let mut facts = Facts::empty();
        facts.tracks.insert(format!("{prefix}{name}"), 2);
        facts
            .tracks
            .insert(format!("{prefix}time_task_{name}"), 1_700_000_000);
        for flag in flags {
            facts.tracks.insert(format!("{prefix}{flag}"), 1);
        }
        let offer = catalogue
            .plan(&facts.input(npc, name, 0))?
            .ok_or_else(|| NpcRuleError::Invalid(format!("missing {name} offer")))?;
        let accepted = catalogue
            .plan(&facts.input(npc, "yes", offer.next_topic))?
            .ok_or_else(|| NpcRuleError::Invalid(format!("missing {name} acceptance")))?;
        for (key, expected) in [(name, 1), (counter, 0), (source, 1)]
            .into_iter()
            .chain(flags.iter().map(|flag| (*flag, 0)))
        {
            assert!(
                accepted.track_changes.iter().any(
                    |change| change.key == format!("{prefix}{key}") && change.after == expected
                ),
                "{key}"
            );
        }
    }
    Ok(())
}

#[test]
fn gnomus_charting_bonus_plans_two_separate_jar_outputs() -> Result<(), NpcRuleError> {
    let catalogue = NpcRuleCatalog::runtime()?;
    let mut facts = Facts::empty();
    let prefix = "oteryn:quest-progress/quest/u11_50/dangerous_depths/gnomes/";
    facts.tracks.insert(format!("{prefix}charting"), 1);
    facts.tracks.insert(format!("{prefix}charting_count"), 6);
    let report = catalogue
        .plan(&facts.input("oteryn:npc.gnomus", "charting", 0))?
        .ok_or_else(|| NpcRuleError::Invalid("missing charting report".into()))?;
    let gifts: Vec<_> = report
        .effects
        .iter()
        .filter_map(|effect| match effect {
            NpcRuleEffect::GiveItem { item, count, .. } => Some((item.as_str(), *count)),
            _ => None,
        })
        .collect();
    assert_eq!(gifts, [("oteryn:item.tibia.i27654", 1); 2]);
    Ok(())
}

#[test]
fn completed_regional_hunts_explain_the_boss_route_hold() -> Result<(), NpcRuleError> {
    let rules = NpcRuleCatalog::runtime()?;
    let loaded = crate::quest::loader::load_embedded_quest_state("regional-hunt-dialogue")
        .map_err(|error| NpcRuleError::Invalid(format!("{error:?}")))?;
    let mut missing = Vec::new();
    for (npc, hunt, quota) in [
        ("oteryn:npc.budrik", "minotaurs", 5000),
        ("oteryn:npc.malor", "blue-djinns", 500),
        ("oteryn:npc.gabel", "green-djinns", 500),
        ("oteryn:npc.lugri", "necromancers", 4000),
    ] {
        let tracks = BTreeMap::from([
            (
                format!("oteryn:quest-progress/hunting-tasks/{hunt}/phase"),
                1,
            ),
            (
                format!("oteryn:quest-progress/hunting-tasks/{hunt}/count"),
                quota,
            ),
        ]);
        let input = NpcRuleInput {
            npc,
            trigger: NpcRuleTrigger::Say,
            message: "task",
            player_name: None,
            topic: 0,
            tracks: &tracks,
            inventory: &NpcInventorySnapshot::default(),
            now_epoch_seconds: 1_700_000_000,
            calendar_month: 11,
            premium_current: false,
            level: 100,
            vocation: "oteryn:vocation.knight",
            sex: 0,
            outfits: &BTreeMap::new(),
        };
        let admitted = rules
            .admitted_for_npc(npc, loaded.catalogue())?
            .ok_or_else(|| NpcRuleError::Invalid(format!("missing hunt programme: {hunt}")))?;
        for message in ["task", "I finished the task"] {
            let input = NpcRuleInput { message, ..input };
            match admitted.plan_storage(&input)? {
                Some(plan) if plan.reply.iter().any(|line| line.contains("boss route")) => {
                    assert!(plan.track_changes.is_empty(), "{hunt}");
                    assert!(plan.effects.is_empty(), "{hunt}");
                }
                plan => missing.push((hunt, message, plan.map(|p| p.reply))),
            }
        }
    }
    assert!(
        missing.is_empty(),
        "completed report dialogue missing: {missing:?}"
    );
    Ok(())
}

#[test]
fn troll_sabotage_source_topics_award_necklace_and_exchange_it_for_rope() -> Result<(), NpcRuleError>
{
    let catalogue = NpcRuleCatalog::runtime()?;
    let track = "oteryn:quest-progress/quest/u8_2/troll_sabotage_quest/questline";
    let chief = "oteryn:npc.chief_grarkharok";
    let jerom = "oteryn:npc.jerom";
    let mut facts = Facts::empty();
    let plan =
        |facts: &Facts, npc, message, topic| catalogue.plan(&facts.input(npc, message, topic));
    assert!(plan(&facts, chief, "kill", 0)?.is_none());
    facts.tracks.insert(track.into(), 1);
    for (message, old, new) in [("kill", 0, 1), ("lady", 1, 2), ("yes", 2, 3)] {
        let next = plan(&facts, chief, message, old)?
            .ok_or_else(|| NpcRuleError::Invalid(format!("missing Chief topic {message}")))?;
        assert_eq!(next.next_topic, new);
    }
    let necklace = plan(&facts, chief, "Griselda", 3)?
        .ok_or_else(|| NpcRuleError::Invalid("missing necklace plan".into()))?;
    assert_eq!(necklace.track_changes[0].after, 2);
    assert!(necklace.effects.iter().any(|effect| matches!(
        effect, NpcRuleEffect::GiveItem { item, count: 1, subtype: None } if item == "oteryn:item.tibia.i7754"
    )));
    facts.tracks.insert(track.into(), 2);
    assert!(plan(&facts, jerom, "mission", 0)?.is_none());
    facts.inventory.items.insert(
        NpcItemKey {
            item: "oteryn:item.tibia.i7754".into(),
            subtype: None,
        },
        1,
    );
    let rope = plan(&facts, jerom, "quest", 0)?
        .ok_or_else(|| NpcRuleError::Invalid("missing rope plan".into()))?;
    assert_eq!(rope.track_changes[0].after, 3);
    assert!(rope.completes);
    assert!(rope.effects.iter().any(|effect| matches!(
        effect, NpcRuleEffect::GiveItem { item, count: 1, subtype: None } if item == "oteryn:item.tibia.i646"
    )));
    facts.tracks.insert(track.into(), 3);
    assert!(plan(&facts, jerom, "quest", 0)?.is_none());
    Ok(())
}

#[test]
fn ape_city_and_beginning_programs_are_admitted_by_the_production_loader()
-> Result<(), NpcRuleError> {
    let loaded = super::super::loader::load_embedded_quest_state("ape-city-and-beginning")
        .map_err(|error| NpcRuleError::Invalid(format!("{error:?}")))?;
    let source = NpcRuleCatalog::runtime()?;
    for npc in [
        "oteryn:npc.hairycles",
        "oteryn:npc.santiago",
        "oteryn:npc.zirella",
        "oteryn:npc.carlos",
    ] {
        assert!(
            source.admitted_for_npc(npc, loaded.catalogue())?.is_some(),
            "{npc}"
        );
    }
    Ok(())
}

#[test]
fn ape_city_final_report_requires_the_destroyed_monument() -> Result<(), NpcRuleError> {
    let loaded = super::super::loader::load_embedded_quest_state("ape-city-final-report")
        .map_err(|error| NpcRuleError::Invalid(format!("{error:?}")))?;
    let npc = "oteryn:npc.hairycles";
    let source = NpcRuleCatalog::runtime()?
        .admitted_for_npc(npc, loaded.catalogue())?
        .ok_or_else(|| NpcRuleError::Invalid("Hairycles".into()))?;
    let stage = "oteryn:quest-progress/quest/u7_6/the_ape_city/questline";
    let monument = "oteryn:quest-progress/quest/u7_6/the_ape_city/snake_destroyer";
    let mut facts = Facts::empty();
    facts.tracks.insert(stage.into(), 17);
    let pending = source
        .plan_item_exchange(&facts.input(npc, "mission", 0))?
        .ok_or_else(|| NpcRuleError::Invalid("mission reminder".into()))?;
    assert!(pending.track_changes.is_empty());
    assert!(!pending.completes);

    facts.tracks.insert(monument.into(), 1);
    let finished = source
        .plan_item_exchange(&facts.input(npc, "mission", 0))?
        .ok_or_else(|| NpcRuleError::Invalid("mission report".into()))?;
    assert!(finished.completes);
    assert_eq!(finished.track_changes[0].key, stage);
    assert_eq!(finished.track_changes[0].after, 18);
    assert!(finished.effects.contains(&NpcRuleEffect::Achievement {
        name: "Friend of the Apes".into(),
    }));

    facts.tracks.insert(stage.into(), 18);
    let replay = source
        .plan_item_exchange(&facts.input(npc, "mission", 0))?
        .ok_or_else(|| NpcRuleError::Invalid("finished reply".into()))?;
    assert!(!replay.completes);
    assert!(replay.track_changes.is_empty());
    Ok(())
}

#[test]
fn beginning_reports_bind_three_legs_and_the_wood_delivery() -> Result<(), NpcRuleError> {
    let loaded = super::super::loader::load_embedded_quest_state("beginning-reports")
        .map_err(|error| NpcRuleError::Invalid(format!("{error:?}")))?;
    let source = NpcRuleCatalog::runtime()?;
    let npc = "oteryn:npc.santiago";
    let program = source
        .admitted_for_npc(npc, loaded.catalogue())?
        .ok_or_else(|| NpcRuleError::Invalid("Santiago".into()))?;
    let mut facts = Facts::empty();
    facts.tracks.insert(
        "oteryn:quest-progress/quest/u8_2/the_beginning_quest/santiago_npc_greet_storage".into(),
        6,
    );
    let leg = NpcItemKey {
        item: "oteryn:item.tibia.i7882".into(),
        subtype: None,
    };
    for count in [2, 3] {
        facts.inventory.items.insert(leg.clone(), count);
        let mut input = facts.input(npc, "hi", 0);
        input.trigger = NpcRuleTrigger::Greet;
        let plan = program
            .plan_item_exchange(&input)?
            .ok_or_else(|| NpcRuleError::Invalid("Santiago report".into()))?;
        assert_eq!(
            plan.effects
                .contains(&NpcRuleEffect::Experience { amount: 100 }),
            count == 3
        );
        assert_eq!(
            plan.inventory_after.items.get(&leg).copied().unwrap_or(0),
            if count == 3 { 0 } else { count }
        );
    }
    let npc = "oteryn:npc.zirella";
    let program = source
        .admitted_for_npc(npc, loaded.catalogue())?
        .ok_or_else(|| NpcRuleError::Invalid("Zirella".into()))?;
    for value in [6, 7] {
        facts.tracks.insert(
            "oteryn:quest-progress/quest/u8_2/the_beginning_quest/zirella_npc_greet_storage".into(),
            value,
        );
        let plan = program.plan_storage(&facts.input(npc, "yes", 8))?;
        assert_eq!(
            plan.is_some_and(|plan| {
                plan.effects
                    .contains(&NpcRuleEffect::Experience { amount: 50 })
                    && plan.track_changes.iter().all(|change| change.after == 8)
            }),
            value == 7
        );
    }
    Ok(())
}

#[test]
fn beginning_carlos_outfit_branch_completes_the_tutorial() -> Result<(), NpcRuleError> {
    let loaded = super::super::loader::load_embedded_quest_state("beginning-completion")
        .map_err(|error| NpcRuleError::Invalid(format!("{error:?}")))?;
    let npc = "oteryn:npc.carlos";
    let source = NpcRuleCatalog::runtime()?
        .admitted_for_npc(npc, loaded.catalogue())?
        .ok_or_else(|| NpcRuleError::Invalid("Carlos".into()))?;
    let greet = "oteryn:quest-progress/quest/u8_2/the_beginning_quest/carlos_npc_greet_storage";
    let log = "oteryn:quest-progress/quest/u8_2/the_beginning_quest/carlos_quest_log";
    let mut facts = Facts::empty();
    facts.tracks.insert(greet.into(), 1);
    facts.tracks.insert(log.into(), 1);
    assert!(
        source
            .plan_storage(&facts.input(npc, "outfit", 0))?
            .is_none()
    );
    let plan = source
        .plan_storage(&facts.input(npc, "outfit", 1))?
        .ok_or_else(|| NpcRuleError::Invalid("Carlos outfit branch".into()))?;
    assert!(plan.completes);
    assert_eq!(plan.track_changes.len(), 2);
    assert!(
        plan.track_changes
            .iter()
            .any(|change| change.key == greet && change.after == 8)
    );
    assert!(
        plan.track_changes
            .iter()
            .any(|change| change.key == log && change.after == 7)
    );
    Ok(())
}
