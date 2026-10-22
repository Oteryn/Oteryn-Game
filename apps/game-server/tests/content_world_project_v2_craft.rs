#![allow(clippy::expect_used)]

use oteryn_game_server::content::*;
use serde_json::{Value, json};

#[path = "support/content_world_project_v2_fixture.rs"]
mod content_world_project_v2_fixture;
use content_world_project_v2_fixture::*;

#[path = "content_world_project_v2_craft/digging_tests.rs"]
mod digging_tests;

// Reuse the existing v2 core/limits and the real repository Item records. This is a
// content-compiler fixture, not another gameplay dispatcher or a materializability admission.
fn item_conversion_candidate(rows: Vec<Value>) -> (ProjectV2Draft, ProjectEvidenceLimits) {
    let mut draft = ProjectV2Draft {
        core: core(),
        state: ProjectV2State::default(),
    };
    let mut keys = std::collections::BTreeSet::new();
    let mut pending: Vec<_> = rows.iter().collect();
    while let Some(value) = pending.pop() {
        match value {
            Value::Object(object) => {
                if object.get("family") == Some(&json!("Item")) {
                    keys.insert(object["key"].as_str().expect("item key").to_owned());
                }
                pending.extend(object.values());
            }
            Value::Array(array) => pending.extend(array),
            _ => {}
        }
    }
    static SOURCE: std::sync::OnceLock<Value> = std::sync::OnceLock::new();
    let source = SOURCE.get_or_init(|| {
        serde_json::from_str(include_str!(
            "../../../content/world/definitions/reference.json"
        ))
        .expect("reference source")
    });
    draft.core.records.extend(
        source["records"]
            .as_array()
            .expect("reference records")
            .iter()
            .filter(|r| keys.contains(r["identity"]["key"].as_str().expect("key")))
            .map(|r| serde_json::from_value(r.clone()).expect("real Item record")),
    );
    draft.state.declarations.extend(
        rows.into_iter()
            .map(|r| serde_json::from_value(r).expect("typed conversion")),
    );
    let wide = ProjectEvidenceLimits {
        max_document_bytes: 4 << 20,
        max_total_bytes: 8 << 20,
        max_decoded_fields: 1 << 20,
        max_string_bytes: 4 << 20,
        max_reference_records: 1024,
        ..limits()
    };
    (draft, wide)
}

#[test]
fn item_conversion_committed_family_round_trips_through_content_dispatch() {
    let catalogue: Value =
        serde_json::from_str(include_str!("../../../content/craft/recipes.json"))
            .expect("catalogue");
    let rows = catalogue["records"].as_array().expect("recipes").clone();
    assert_eq!(rows.len(), 125);
    let mechanical = rows
        .iter()
        .find(|r| r["identity"]["key"] == "oteryn:item-conversion.corpse-mechanical")
        .expect("mechanical corpse");
    let alternatives = mechanical["recipe"]["outputs"]["alternatives"]
        .as_array()
        .expect("alternatives");
    assert_eq!(alternatives.len(), 19);
    assert_eq!(
        alternatives
            .iter()
            .map(|a| a["weight"].as_u64().expect("weight"))
            .sum::<u64>(),
        3000
    );
    let (draft, wide) = item_conversion_candidate(rows);
    let documents = CanonicalProjectDocuments::from_v2_draft(draft, wide)
        .expect("compile accepted source rows");
    let parsed = ProjectSnapshot::new(documents.documents().clone(), wide)
        .expect("snapshot")
        .parse(wide)
        .expect("dispatch v2 parser");
    assert_eq!(parsed.v2().expect("v2").declarations.len(), 125);
    assert_eq!(
        parsed
            .canonical_documents(wide)
            .expect("canonical rewrite")
            .documents(),
        documents.documents()
    );
    assert!(
        parsed
            .lower_reference_source()
            .expect("reference projection")
            .definitions
            .iter()
            .all(|d| !d
                .definition
                .key()
                .as_str()
                .starts_with("oteryn:item-conversion."))
    );
}

#[test]
fn item_conversion_limits_and_malformed_operands_fail_at_the_real_compiler() {
    let catalogue: Value =
        serde_json::from_str(include_str!("../../../content/craft/recipes.json"))
            .expect("catalogue");
    let rows = catalogue["records"].as_array().expect("rows");
    let recipe = |name: &str| {
        rows.iter()
            .find(|r| r["identity"]["key"] == format!("oteryn:item-conversion.{name}"))
            .expect("recipe")
            .clone()
    };
    let mut maximum = recipe("corpse-mechanical");
    let alternatives = maximum["recipe"]["outputs"]["alternatives"]
        .as_array_mut()
        .expect("alternatives");
    alternatives.resize(24, alternatives[0].clone());
    let (draft, wide) = item_conversion_candidate(vec![maximum.clone()]);
    CanonicalProjectDocuments::from_v2_draft(draft, wide).expect("24 accepted");
    let extra = maximum["recipe"]["outputs"]["alternatives"][0].clone();
    maximum["recipe"]["outputs"]["alternatives"]
        .as_array_mut()
        .expect("alternatives")
        .push(extra);
    let (draft, wide) = item_conversion_candidate(vec![maximum]);
    assert!(
        CanonicalProjectDocuments::from_v2_draft(draft, wide).is_err(),
        "25 rejected"
    );

    let cases = [
        (
            "missing target",
            "flour-2874-1",
            "/recipe/target/kind",
            json!("none"),
        ),
        (
            "wrong input",
            "flour-2874-1",
            "/recipe/input_item/key",
            json!("oteryn:item.tibia.i2874"),
        ),
        (
            "cross family",
            "flour-2874-1",
            "/recipe/operands/0/item_id/family",
            json!("Terrain"),
        ),
        (
            "unresolved",
            "flour-2874-1",
            "/recipe/operands/0/item_id/key",
            json!("oteryn:item.tibia.i999999"),
        ),
        (
            "zero count",
            "flour-2874-1",
            "/recipe/operands/0/count",
            json!(0),
        ),
        (
            "count 101",
            "flour-2874-1",
            "/recipe/operands/0/count",
            json!(101),
        ),
        (
            "vessel max stack",
            "flour-2874-1",
            "/recipe/operands/1/count",
            json!(2),
        ),
        (
            "missing fluid",
            "flour-2874-1",
            "/recipe/operands/1/sub_type",
            Value::Null,
        ),
        (
            "missing lifecycle",
            "flour-2874-1",
            "/recipe/operands/1/lifecycle",
            Value::Null,
        ),
        (
            "missing output subtype",
            "flour-2874-1",
            "/recipe/operands/1/output_sub_type",
            Value::Null,
        ),
        (
            "ingredient not consumed",
            "fish-3483-4597",
            "/recipe/operands/1/effect",
            json!("KEEP"),
        ),
        (
            "bait always",
            "fish-3483-4597",
            "/recipe/operands/1/consume_on",
            json!("ALWAYS"),
        ),
        (
            "missing bait policy",
            "fish-3483-4597",
            "/recipe/operands/1/consume_on",
            Value::Null,
        ),
        (
            "no capacity check",
            "fish-3483-4597",
            "/recipe/capacity_check",
            json!(false),
        ),
        (
            "missing cooldown",
            "fish-3483-4597",
            "/recipe/tile_cooldown_ms",
            Value::Null,
        ),
        (
            "cooldown zero",
            "fish-3483-4597",
            "/recipe/tile_cooldown_ms",
            json!(0),
        ),
        (
            "cooldown over max",
            "fish-3483-4597",
            "/recipe/tile_cooldown_ms",
            json!(3600001),
        ),
        (
            "wrong curve",
            "fish-3483-4597",
            "/recipe/chance_curve/per_skill",
            json!(598),
        ),
        (
            "non fishing cooldown",
            "fish-3483-4597",
            "/recipe/skill",
            Value::Null,
        ),
        (
            "output zero count",
            "cookies-2535",
            "/recipe/outputs/entries/0/count",
            json!(0),
        ),
        (
            "output count 101",
            "cookies-2535",
            "/recipe/outputs/entries/0/count",
            json!(101),
        ),
        (
            "transform output max_stack",
            "flour-2874-1",
            "/recipe/operands/1/count",
            json!(2),
        ),
        (
            "zero weight",
            "corpse-mechanical",
            "/recipe/outputs/alternatives/0/weight",
            json!(0),
        ),
        (
            "source substitution",
            "corpse-mechanical",
            "/recipe/source",
            recipe("dirty-water")["recipe"]["source"].clone(),
        ),
    ];
    for (name, row, pointer, value) in cases {
        let mut row = recipe(row);
        *row.pointer_mut(pointer).expect("existing field") = value;
        let (draft, wide) = item_conversion_candidate(vec![row]);
        assert!(
            CanonicalProjectDocuments::from_v2_draft(draft, wide).is_err(),
            "{name}"
        );
    }
    let row = recipe("corpse-mechanical");
    let (draft, wide) = item_conversion_candidate(vec![row.clone(), row]);
    assert!(
        CanonicalProjectDocuments::from_v2_draft(draft, wide).is_err(),
        "duplicate identity"
    );
}

#[test]
fn item_conversion_timed_operands_are_keep_only_and_outputs_do_not_add_timed_state() {
    let catalogue: Value =
        serde_json::from_str(include_str!("../../../content/craft/recipes.json"))
            .expect("catalogue");
    let rows = catalogue["records"].as_array().expect("rows");
    for (name, accepted) in [
        ("fish-3483-4597", true),
        ("cookies-2535", false),
        ("flour-2874-1", false),
    ] {
        let row = rows
            .iter()
            .find(|r| r["identity"]["key"] == format!("oteryn:item-conversion.{name}"))
            .expect("recipe")
            .clone();
        let input = row["recipe"]["input_item"]["key"]
            .as_str()
            .expect("input")
            .to_owned();
        let (mut draft, wide) = item_conversion_candidate(vec![row]);
        for record in &mut draft.core.records {
            if let ProjectReferenceRecord::Item {
                identity,
                semantics,
                ..
            } = record
                && identity.key == input
            {
                semantics.charges = ReferenceItemField::Known(ReferenceItemCharges {
                    count: ReferenceItemField::Known(5),
                });
            }
        }
        let result = CanonicalProjectDocuments::from_v2_draft(draft, wide);
        assert_eq!(result.is_ok(), accepted, "{name}: {result:?}");
        if name == "cookies-2535" {
            assert!(matches!(
                result,
                Err(ProjectError::InvalidProject(
                    "item_conversion timed operand must KEEP"
                ))
            ));
        }
    }
    // The real mechanical table includes the charged amulet 9303. Its admission is data-only;
    // the v2 document format creates no instance and has nowhere to write timed state.
    let row = rows
        .iter()
        .find(|r| r["identity"]["key"] == "oteryn:item-conversion.corpse-mechanical")
        .expect("mechanical")
        .clone();
    let (draft, wide) = item_conversion_candidate(vec![row]);
    let documents = CanonicalProjectDocuments::from_v2_draft(draft, wide).expect("timed output");
    assert!(documents.documents().keys().all(|k| !k.contains("timed")));
}

#[test]
fn item_conversion_transaction_shape_boundaries_and_read_only_map_targets() {
    let catalogue: Value =
        serde_json::from_str(include_str!("../../../content/craft/recipes.json"))
            .expect("catalogue");
    let rows = catalogue["records"].as_array().expect("rows");
    let mut row = rows
        .iter()
        .find(|r| r["identity"]["key"] == "oteryn:item-conversion.flour-2874-1")
        .expect("flour")
        .clone();
    let ingredient = |id| {
        json!({"role":"INGREDIENT", "item_id":reference(ProjectV2Family::Item, &format!("oteryn:item.tibia.i{id}")),
                                "count":if id == 953 { 1 } else { 100 }, "effect":"CONSUME", "consume_on":"ALWAYS"})
    };
    row["recipe"]["operands"]
        .as_array_mut()
        .expect("operands")
        .extend([ingredient(3492), ingredient(953)]);
    row["recipe"]["operands"][0]["count"] = json!(100);
    let entry =
        json!({"item_id":reference(ProjectV2Family::Item, "oteryn:item.tibia.i8199"), "count":100});
    row["recipe"]["outputs"] =
        json!({"kind":"fixed", "entries":[entry.clone(), entry.clone(), entry.clone()]});
    let (draft, wide) = item_conversion_candidate(vec![row.clone()]);
    CanonicalProjectDocuments::from_v2_draft(draft, wide)
        .expect("4 operands, 2 ingredients, 3 outputs and count 100 accepted");
    let (mut conflicted, wide) = item_conversion_candidate(vec![row.clone()]);
    for record in &mut conflicted.core.records {
        if let ProjectReferenceRecord::Item {
            identity,
            stack_class,
            semantics,
            ..
        } = record
            && identity.key == "oteryn:item.tibia.i3603"
        {
            *stack_class = ItemStackDocument::StackCapable;
            semantics.stack = ReferenceItemField::Known(ReferenceItemStack {
                stackable: ReferenceItemField::Known(true),
                stack_max: ReferenceItemField::Conflict,
            });
        }
    }
    assert!(
        CanonicalProjectDocuments::from_v2_draft(conflicted, wide).is_err(),
        "a conflicting definition maximum cannot fall back to source evidence"
    );
    let mut too_many_outputs = row.clone();
    too_many_outputs["recipe"]["outputs"]["entries"]
        .as_array_mut()
        .expect("entries")
        .push(entry);
    let (draft, wide) = item_conversion_candidate(vec![too_many_outputs]);
    assert!(
        CanonicalProjectDocuments::from_v2_draft(draft, wide).is_err(),
        "4 fixed outputs rejected"
    );
    row["recipe"]["operands"]
        .as_array_mut()
        .expect("operands")
        .push(ingredient(3578));
    let (draft, wide) = item_conversion_candidate(vec![row]);
    assert!(
        CanonicalProjectDocuments::from_v2_draft(draft, wide).is_err(),
        "5 operands and 3 ingredients rejected"
    );
    let mut corpse = rows
        .iter()
        .find(|r| r["identity"]["key"] == "oteryn:item-conversion.corpse-mechanical")
        .expect("corpse")
        .clone();
    corpse["recipe"]["target"]["definition"]["key"] = json!("oteryn:item.tibia.i2535");
    let (draft, wide) = item_conversion_candidate(vec![corpse]);
    assert!(
        CanonicalProjectDocuments::from_v2_draft(draft, wide).is_err(),
        "an oven cannot be retired as a corpse"
    );
}

#[test]
fn item_conversion_closed_operand_schema_rejects_extra_quantity_and_lifecycle_fields_on_import() {
    let catalogue: Value =
        serde_json::from_str(include_str!("../../../content/craft/recipes.json"))
            .expect("catalogue");
    let row = catalogue["records"]
        .as_array()
        .expect("rows")
        .iter()
        .find(|r| r["identity"]["key"] == "oteryn:item-conversion.flour-2874-1")
        .expect("flour")
        .clone();
    let (draft, wide) = item_conversion_candidate(vec![row]);
    let documents = CanonicalProjectDocuments::from_v2_draft(draft, wide).expect("valid base");
    for field in ["output_count", "instance_quantity", "runtime_instance"] {
        let mut altered = documents.documents().clone();
        let mut declarations: Value =
            serde_json::from_slice(&altered["definitions/declarations.json"])
                .expect("declarations");
        declarations["records"][0]["recipe"]["operands"][1][field] = json!(2);
        let bytes = canonical(&declarations);
        let mut manifest: Value =
            serde_json::from_slice(&altered["manifest.json"]).expect("manifest");
        let entry = manifest["documents"]
            .as_array_mut()
            .expect("documents")
            .iter_mut()
            .find(|r| r["locator"] == "definitions/declarations.json")
            .expect("declarations inventory");
        entry["byte_length"] = json!(bytes.len());
        entry["sha256"] = json!(world_project_sha256(&bytes));
        altered.insert("definitions/declarations.json".into(), bytes);
        rebind_manifest_and_lock(&mut altered, &manifest);
        assert!(
            ProjectSnapshot::new(altered, wide)
                .expect("snapshot")
                .parse(wide)
                .is_err(),
            "unknown operand field {field}"
        );
    }
}
