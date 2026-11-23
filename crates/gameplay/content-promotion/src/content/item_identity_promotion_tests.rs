use super::*;

fn draft(keys: &[&str]) -> ProjectV2Draft {
    ProjectV2Draft {
        core: super::super::ProjectDraft {
            project_revision: "test-r1".into(),
            package_key: "oteryn:content.test".into(),
            semantic_schema_version: "reference-schema-v1".into(),
            licensing_metadata: "test".into(),
            world_id: "0123456789ab70cd8ef0123456789abc".into(),
            coordinate_frame: "global-target-2026-09-27".into(),
            records: keys
                .iter()
                .map(|key| ProjectReferenceRecord::Item {
                    identity: super::super::DefinitionIdentityDocument {
                        family: "Item".into(),
                        key: (*key).into(),
                        revision: "definition-r1".into(),
                    },
                    client_projection: ProjectionDocument::ClientSafe,
                    materializable: false,
                    stack_class: ItemStackDocument::Unknown,
                    semantics: Default::default(),
                })
                .collect(),
            imports: vec![],
            metadata: vec![],
        },
        state: Default::default(),
    }
}

#[test]
fn late_dangling_reference_preserves_entire_draft() -> Result<(), Box<dyn std::error::Error>> {
    let old = "oteryn:item.old";
    let mut draft = draft(&[old]);
    draft.state.editor = serde_json::from_value(serde_json::json!([{
        "target": {"family":"Item", "key":"oteryn:item.missing", "revision":"definition-r1"},
        "display_name":"test", "description":"", "categories":[], "notes":[], "aliases":[], "tags":[]
    }]))?;
    let before = draft.clone();
    let table = ItemKeyAliasTable {
        current: [(
            old.into(),
            RetiredItemKey::Alias {
                target: "oteryn:item.tibia.i100".into(),
            },
        )]
        .into(),
    };
    let source_ids = [(old.into(), 100)].into();
    assert!(matches!(
        apply_tibia_id_key_rule(&mut draft, &table, &source_ids),
        Err(ItemIdentityError::DanglingReference(_))
    ));
    assert_eq!(draft, before);
    Ok(())
}

#[test]
fn unmapped_item_preserves_entire_draft() {
    let mut draft = draft(&["unmapped"]);
    let before = draft.clone();
    let table = ItemKeyAliasTable {
        current: BTreeMap::new(),
    };
    assert!(matches!(
        apply_tibia_id_key_rule(&mut draft, &table, &BTreeMap::new()),
        Err(ItemIdentityError::RuleMismatch(_))
    ));
    assert_eq!(draft, before);
}
#[test]
fn physical_promotion_loads_the_committed_packet() -> Result<(), String> {
    use super::super::item_physical_promotion::{
        ITEM_PHYSICAL_PACKET, apply_item_physical_promotion_v1,
    };

    let packet: serde_json::Value =
        serde_json::from_slice(ITEM_PHYSICAL_PACKET).map_err(|error| error.to_string())?;
    let keys = packet["promotions"]
        .as_array()
        .ok_or("missing physical promotions")?
        .iter()
        .map(|row| row["item_key"].as_str().ok_or("missing physical item key"))
        .collect::<Result<std::collections::BTreeSet<_>, _>>()?;
    let mut draft = draft(&keys.into_iter().collect::<Vec<_>>());

    let applied = apply_item_physical_promotion_v1(&mut draft)?;
    assert_eq!(applied.changed, applied.fields);
    Ok(())
}

#[test]
fn public_promotions_reject_duplicate_native_keys_before_mutation() {
    let mut failures = Vec::new();
    macro_rules! check {
        ($module:ident, $function:ident) => {{
            let mut draft = draft(&["oteryn:item.tibia.i100", "oteryn:item.tibia.i100"]);
            let before = draft.clone();
            let error = super::super::$module::$function(&mut draft)
                .err()
                .map(|e| e.to_string());
            if !error
                .as_deref()
                .is_some_and(|e| e.contains("duplicate native Item key"))
            {
                failures.push(format!("{}: {error:?}", stringify!($function)));
            }
            assert_eq!(draft, before);
        }};
    }
    check!(item_timed_promotion, apply_item_timed_promotion_v1);
    check!(item_bed_promotion, apply_item_bed_promotion_v1);
    check!(item_stats_promotion, apply_item_stats_promotion_v2);
    check!(item_capacity_promotion, apply_item_capacity_promotion_v1);
    check!(item_physical_promotion, apply_item_physical_promotion_v1);
    check!(item_movable_promotion, apply_item_movable_promotion_v1);
    check!(
        item_stack_false_promotion,
        apply_item_stack_false_promotion_v1
    );
    check!(
        item_stack_default_promotion,
        apply_item_stack_default_promotion_v1
    );
    check!(
        item_stack_historical_promotion,
        apply_item_stack_historical_promotion_v1
    );
    check!(
        item_consumption_promotion,
        apply_item_consumption_promotion_v1
    );
    check!(item_admission, apply_item_admission_v1);
    check!(item_admission, apply_item_admission_v2);
    check!(item_admission, apply_item_admission_v2_cheese);
    check!(item_admission, apply_item_admission_v3);
    assert!(failures.is_empty(), "{failures:#?}");
}
#[test]
fn complete_packets_reject_duplicate_targets() -> Result<(), Box<dyn std::error::Error>> {
    macro_rules! check {
        ($module:ident, $packet:ident, $function:ident) => {{
            let packet: Value = serde_json::from_slice(super::super::$module::$packet)?;
            let mut keys = packet["promotions"]
                .as_array()
                .ok_or("promotion rows")?
                .iter()
                .map(|row| row["item_key"].as_str().ok_or("item key"))
                .collect::<Result<BTreeSet<_>, _>>()?;
            for row in packet["promotions"].as_array().ok_or("promotion rows")? {
                if let Some(target) = row["typed_value"]["value"].as_str()
                    && target.starts_with(TIBIA_ITEM_KEY_PREFIX)
                {
                    keys.insert(target);
                }
            }
            let mut draft = draft(&keys.into_iter().collect::<Vec<_>>());
            let applied = super::super::$module::$function(&mut draft);
            assert!(applied.is_ok(), "{applied:?}");
            draft.core.records.push(draft.core.records[0].clone());
            let before = draft.clone();
            let error = super::super::$module::$function(&mut draft)
                .err()
                .map(|e| e.to_string());
            assert!(
                error
                    .as_deref()
                    .is_some_and(|e| e.contains("duplicate native Item key")),
                "{error:?}"
            );
            assert_eq!(draft, before);
        }};
    }
    check!(
        item_timed_promotion,
        ITEM_TIMED_PROMOTION_V1_PACKET,
        apply_item_timed_promotion_v1
    );
    check!(
        item_bed_promotion,
        ITEM_BED_PROMOTION_V1_PACKET,
        apply_item_bed_promotion_v1
    );
    Ok(())
}
