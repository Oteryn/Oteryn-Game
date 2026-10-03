//! Exact native filesystem loader qualification; no materialization or gameplay execution.
use oteryn_game_server::content::*;
use serde_json::{Value, json};
use oteryn_game_server::content::item_identity::{ItemKeyAliasTable, RetiredItemKey, is_canonical_item_key};
use std::collections::BTreeMap;
use std::path::Path;

fn limits() -> ProjectEvidenceLimits {
    ProjectEvidenceLimits { max_documents: 11, max_document_bytes: 96_000_000,
        max_total_bytes: 160_000_000, max_json_depth: 24, max_decoded_fields: 2_120_000,
        max_string_bytes: 43_000_000, max_locator_bytes: 160, max_locator_segments: 8,
        max_reference_records: CW2_B1_FULL_ITEM_FAMILY_COUNT + 24933 + 2220, max_import_records: 12, max_reimport_states: 104 }
}
fn keyed(values: &[Value], profile: bool) -> BTreeMap<(String,String,String), Value> {
    let mut index = BTreeMap::new();
    for value in values {
        let identity = &value[if profile {"target"} else {"identity"}];
        let family = identity["family"].as_str().or(value["kind"].as_str()).expect("typed identity family");
        let key = (family.to_owned(), identity["key"].as_str().expect("key").to_owned(), identity["revision"].as_str().expect("revision").to_owned());
        assert!(index.insert(key, value.clone()).is_none(), "duplicate typed native identity");
    }
    index
}
fn normalize_item_refs(value: &mut Value, aliases: &ItemKeyAliasTable) {
    match value {
        Value::Object(object) => {
            if object.get("family").and_then(Value::as_str)==Some("Item") {
                let key=object.get("key").and_then(Value::as_str).expect("typed Item key");
                if !is_canonical_item_key(key) {
                    let target=match aliases.resolve(key) { Some(RetiredItemKey::Alias{target})=>target.clone(), _=>panic!("unqualified or retired Item key {}",key) };
                    object.insert("key".into(),Value::String(target));
                }
            }
            for child in object.values_mut() {normalize_item_refs(child,aliases);}
        }
        Value::Array(rows) => for row in rows {normalize_item_refs(row,aliases);},
        _=>{}
    }
}
fn exact_subset(mut expected: Vec<Value>, actual: Vec<Value>, profile: bool, aliases: &ItemKeyAliasTable) -> usize {
    for value in &mut expected {normalize_item_refs(value,aliases);}
    let expected_index=keyed(&expected,profile); let actual_index=keyed(&actual,profile);
    for (identity,value) in &expected_index { assert_eq!(actual_index.get(identity),Some(value),"native loader changed/missed staged identity {:?}",identity); }
    expected_index.len()
}
fn values<T: serde::Serialize>(rows: &[T]) -> Vec<Value> { rows.iter().map(|r|serde_json::to_value(r).expect("typed native JSON")).collect() }
fn main() -> Result<(),Box<dyn std::error::Error>> {
    let args:Vec<String>=std::env::args().collect(); assert_eq!(args.len(),4,"world root, exact native stage and verified Item alias table required");
    let aliases=ItemKeyAliasTable::parse(&std::fs::read(&args[3])?)?;
    let world=Path::new(&args[1]); let stage:Value=serde_json::from_slice(&std::fs::read(&args[2])?)?;
    assert_eq!(stage["counts"]["creatures"],1763); assert_eq!(stage["counts"]["encounters"],104); assert_eq!(stage["counts"]["documents"],1609);
    let filesystem=ProjectFilesystemLimits{project:limits(),max_entries_per_directory_scan:32,max_total_directory_entries_scanned:201};
    let loaded=capture_world_project(world.parent().ok_or("world parent")?,world.file_name().ok_or("world basename")?,filesystem)?;
    let first=loaded.migrate_to_v2();
    let canonical=loaded.canonical_documents(limits())?;
    let document_count=canonical.documents().len();
    let snapshot=canonical.into_snapshot(limits())?;
    let roundtrip=snapshot.parse(limits())?.migrate_to_v2();
    assert_eq!(first,roundtrip,"native canonical filesystem-loader round trip changed draft");
    let staged_records:Vec<ProjectReferenceRecord>=serde_json::from_value(stage["records"].clone())?;
    let staged_profiles:Vec<ProjectV2AuthoringProfile>=serde_json::from_value(stage["authoring_profiles"].clone())?;
    let staged_declarations:Vec<ProjectV2Declaration>=serde_json::from_value(stage["declarations"].clone())?;
    let staged_bindings:Vec<ProjectV2SourceIdentityBinding>=serde_json::from_value(stage["source_identity_bindings"].clone())?;
    let record_cases=exact_subset(values(&staged_records),values(&roundtrip.core.records),false,&aliases);
    let profile_cases=exact_subset(values(&staged_profiles),values(&roundtrip.state.authoring_profiles),true,&aliases);
    let declaration_cases=exact_subset(values(&staged_declarations),values(&roundtrip.state.declarations),false,&aliases);
    let binding_index: BTreeMap<String,Value>=roundtrip.state.source_identity_bindings.iter().map(|r|{let v=serde_json::to_value(r).expect("binding");(v.to_string(),v)}).collect();
    // Existing importer keeps the Crystal commit while disambiguating its Creature batch from NPCs.
    let mut crystal_revisions=0;
    for binding in &staged_bindings {
        let mut v=serde_json::to_value(binding)?;
        if v["source_key"]=="oteryn:source.crystalserver" {
            assert_eq!(v["source_revision"],"00ce02a57ca5a12e48f32a3476e37471167e4c3f");
            assert_eq!(v["target"]["family"],"Creature");
            assert_eq!(v["identity_namespace"],"crystalserver/monster-file");
            v["source_revision"]=json!("crystalserver-creature-1530:00ce02a57ca5a12e48f32a3476e37471167e4c3f");
            crystal_revisions+=1;
        }
        assert_eq!(binding_index.get(&v.to_string()),Some(&v),"exact staged source binding missing");
    }
    assert_eq!(crystal_revisions,96);
    let creatures=roundtrip.core.records.iter().filter(|r|matches!(r,ProjectReferenceRecord::Creature{..})).count();
    let encounters=roundtrip.state.declarations.iter().filter(|r|matches!(r,ProjectV2Declaration::Encounter{..})).count();
    let documents=roundtrip.state.declarations.iter().filter(|r|matches!(r,ProjectV2Declaration::Document{..})).count();
    assert_eq!((creatures,encounters,documents),(1763,104,1609));
    let declared=keyed(&values(&roundtrip.state.declarations),false);
    let mut encyclopedia_links=0;
    for profile in &roundtrip.state.authoring_profiles {
        let p=serde_json::to_value(profile)?;
        if p["data"]["kind"]!="Creature" {continue;}
        let r=&p["data"]["profile"]["details"]["encyclopedia_document"];
        if r.is_null(){continue;}
        assert_eq!(r["family"],"Document");
        let id=("Document".to_owned(),r["key"].as_str().ok_or("doc key")?.to_owned(),r["revision"].as_str().ok_or("doc revision")?.to_owned());
        assert!(declared.contains_key(&id),"exact encyclopedia declaration unresolved");encyclopedia_links+=1;
    }
    assert_eq!(encyclopedia_links,1609);
    // Two meaningful negative controls exercise this current canonical/parser boundary.
    let mut corrupted=snapshot.documents().clone();let root=corrupted.get_mut("project.json").ok_or("root document")?;root.push(b'x');
    assert!(ProjectSnapshot::new(corrupted,limits())?.parse(limits()).is_err(),"corrupt root JSON accepted");
    let mut missing=roundtrip.clone();let index=missing.state.declarations.iter().position(|r|matches!(r,ProjectV2Declaration::Document{..})).ok_or("Document")?;
    missing.state.declarations.remove(index);
    assert!(CanonicalProjectDocuments::from_v2_draft(missing,limits()).is_err(),"missing referenced Document accepted");
    println!("{}",json!({"scope":"CURRENT_NATIVE_FILESYSTEM_PROJECT_LOADER_CANONICAL_ROUNDTRIP_AND_EXACT_STAGED_LINKS","creatures":creatures,"encounters":encounters,"documents":documents,"encyclopedia_links":encyclopedia_links,"protected_item_alias_normalization_verified":true,"existing_crystal_batch_revision_normalizations":crystal_revisions,"exact_staged_records":record_cases,"exact_staged_profiles":profile_cases,"exact_staged_declarations":declaration_cases,"exact_staged_source_bindings":staged_bindings.len(),"canonical_file_documents":document_count,"project_limits_match_actual_materializer":true,"filesystem_limits_match_existing_repository_loader_test":true,"negative_controls":2,"gameplay_execution_tested":false,"live_server_started":false}));
    Ok(())
}
