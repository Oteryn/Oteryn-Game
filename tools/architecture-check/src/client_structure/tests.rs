#[path = "../../tests/support/client.rs"]
mod support;

use super::super::client_fragments;
use serde_json::json;
use std::fs;
use support::Fixture;

#[test]
fn settings_unknown_keys_round_trip_and_generated_entries_equal_fragments() -> Result<(), String> {
    let f = Fixture::structure()?;
    f.write(
        "crates/client-ui/src/features/0057-cyclopedia-map/settings.rs",
        "fn settings() {}",
    )?;
    let mut entries = f.current()?;
    let envelope = json!({"map.zoom":2,"future-option":{"nested":[null,true,3,"value"]},"player_rebinds":{"a":"Ctrl+K","b":"Ctrl+K"}});
    entries.push(json!({"mechanic":"0057-cyclopedia-map","kind":"settings-pages","id":"map-settings","status":"supported","entry":"settings.rs::settings","key":"map.zoom","setting_type":"integer","version":1,"settings_envelope":envelope}));
    f.entries(&entries)?;
    let (discovered, coverage) = client_fragments::scan(&f.0)?;
    let (rust, report) = client_fragments::render(&discovered, &coverage)?;
    let syntax = syn::parse_file(&rust).map_err(|e| e.to_string())?;
    let Some(syn::Item::Const(item)) = syntax.items.first() else {
        return Err("generated descriptors missing".into());
    };
    let syn::Expr::Lit(expr) = item.expr.as_ref() else {
        return Err("generated descriptor literal".into());
    };
    let syn::Lit::Str(literal) = &expr.lit else {
        return Err("generated descriptor JSON".into());
    };
    let round_trip: serde_json::Value =
        serde_json::from_str(&literal.value()).map_err(|e| e.to_string())?;
    assert_eq!(round_trip, json!(discovered));
    let settings = round_trip
        .as_array()
        .ok_or("descriptors")?
        .iter()
        .find(|e| e["id"] == "map-settings")
        .ok_or("settings descriptor")?;
    assert_eq!(settings["settings_envelope"], envelope);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&report).map_err(|e| e.to_string())?,
        json!(coverage)
    );
    // Recreating files in reverse order cannot affect generated output.
    entries.reverse();
    f.entries(&entries)?;
    let (discovered, coverage) = client_fragments::scan(&f.0)?;
    assert_eq!(
        client_fragments::render(&discovered, &coverage)?,
        (rust, report)
    );
    Ok(())
}

#[test]
fn synthetic_command_and_mechanic_extend_only_additive_sources() -> Result<(), String> {
    let f = Fixture::structure()?;
    let original = fs::read(
        f.0.join("crates/client-ui/src/features/0057-cyclopedia-map/fixtures/registrations.json"),
    )
    .map_err(|e| e.to_string())?;
    f.json(
        "crates/protocol-oteryn/capabilities/1000-synthetic-v1.json",
        &json!({"id":1000,"name":"SYNTHETIC_V1","commands":[1000]}),
    )?;
    f.json(
        "docs/contracts/protocol-oteryn/v1/registry/command_types/1000-synthetic.json",
        &json!({"id":1000,"capability":1000}),
    )?;
    let feature = "crates/client-ui/src/features/1000-synthetic";
    f.write(&format!("{feature}/protocol.rs"), "fn handle() {}")?;
    let entry = json!({"mechanic":"1000-synthetic","kind":"handlers","id":"synthetic-handler","status":"supported","commands":[1000],"capability":1000,"direction":"client_to_server","entry":"protocol.rs::handle","not_applicable":{"state":"mapping only","view":"mapping only","input":"mapping only","settings":"mapping only"}});
    f.json(&format!("{feature}/registrations/handlers.json"), &entry)?;
    f.json(
        &format!("{feature}/fixtures/registrations.json"),
        &json!([entry]),
    )?;
    let (entries, coverage) = client_fragments::scan(&f.0)?;
    let (rust, _) = client_fragments::render(&entries, &coverage)?;
    assert!(rust.contains("synthetic-handler"));
    assert!(
        coverage
            .iter()
            .any(|e| e["mechanic"] == "1000-synthetic" && e["status"] == "supported")
    );
    assert_eq!(
        original,
        fs::read(
            f.0.join(
                "crates/client-ui/src/features/0057-cyclopedia-map/fixtures/registrations.json"
            )
        )
        .map_err(|e| e.to_string())?
    );
    let mut bad = entry;
    bad["direction"] = json!("server_to_client");
    f.json(&format!("{feature}/registrations/handlers.json"), &bad)?;
    assert!(client_fragments::scan(&f.0).is_err());
    Ok(())
}
