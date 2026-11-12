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

#[test]
fn core_extension_requires_exact_contract_command_binding_and_test() -> Result<(), String> {
    let f = Fixture::structure()?;
    f.json(
        "crates/protocol-oteryn/capabilities/0371-diagonal-movement-v1.json",
        &json!({"id":371,"name":"DIAGONAL_MOVEMENT_V1","commands":[]}),
    )?;
    let feature = "crates/client-ui/src/features/0371-diagonal-movement";
    f.write(&format!("{feature}/protocol.rs"), "fn handle() {}")?;
    f.write(&format!("{feature}/tests.rs"), "fn proves_profile() {}")?;
    let contract =
        "docs/contracts/protocol-oteryn/v1/registry/capabilities/0371-diagonal-movement-v1.json";
    let descriptor = "docs/contracts/protocol-oteryn/v1/registry/command_types/0001-world-actor-step-intent.json";
    let entry = json!({"mechanic":"0371-diagonal-movement","kind":"handlers","id":"diagonal-handler",
        "status":"supported","commands":[],"capability":371,"direction":"client_to_server",
        "shared_commands":[1],"entry":"protocol.rs::handle","test":"tests.rs::proves_profile",
        "not_applicable":{"state":"core command","view":"core command","input":"core command","settings":"core command"}});
    let valid = json!({"id":371,"extends_core_commands":[1]});
    f.json(contract, &valid)?;
    f.json(descriptor, &json!({"id":1}))?;
    f.json(&format!("{feature}/registrations/handlers.json"), &entry)?;
    f.json(
        &format!("{feature}/fixtures/registrations.json"),
        &json!([entry]),
    )?;
    client_fragments::scan(&f.0)?;
    for (mutation, expected) in [
        "core extension capability mismatch",
        "core extension differs from contract",
        "core extension differs from contract",
        "extension target is not a core command",
        "missing Rust entry symbol tests.rs::missing",
        "core extension differs from contract",
    ]
    .into_iter()
    .enumerate()
    {
        f.json(contract, &valid)?;
        f.json(descriptor, &json!({"id":1}))?;
        let mut changed = entry.clone();
        match mutation {
            0 => changed["capability"] = json!(370),
            1 => f.json(contract, &json!({"id":371}))?,
            2 => changed["shared_commands"] = json!([2]),
            3 => f.json(descriptor, &json!({"id":1,"capability":13}))?,
            4 => changed["test"] = json!("tests.rs::missing"),
            5 => {
                changed["shared_commands"] = json!([1, 1]);
                f.json(contract, &json!({"id":371,"extends_core_commands":[1,1]}))?;
            }
            _ => return Err("mutation".into()),
        }
        f.json(&format!("{feature}/registrations/handlers.json"), &changed)?;
        f.json(
            &format!("{feature}/fixtures/registrations.json"),
            &json!([changed]),
        )?;
        let error = client_fragments::scan(&f.0)
            .err()
            .ok_or(format!("invalid extension {mutation} passed"))?;
        assert!(
            error.contains(expected),
            "invalid extension {mutation}: {error}"
        );
    }
    Ok(())
}

#[test]
fn pushed_handler_requires_reducer_window_and_test() -> Result<(), String> {
    for missing in ["reducer", "window", "test"] {
        let f = Fixture::structure()?;
        f.json(
            "crates/protocol-oteryn/capabilities/1000-push-v1.json",
            &json!({"id":1000,"name":"PUSH_V1","commands":[]}),
        )?;
        let feature = "crates/client-ui/src/features/1000-push";
        f.write(&format!("{feature}/protocol.rs"), "fn reduce() {}")?;
        f.write(
            &format!("{feature}/tests/state.rs"),
            "#[test] fn pushed_state_is_visible() {}",
        )?;
        f.write(&format!("{feature}/view.rs"), "fn panel() {}")?;
        let mut handler = json!({"mechanic":"1000-push","kind":"handlers","id":"push-handler","status":"supported","commands":[],"capability":1000,"direction":"server_to_client","entry":"protocol.rs::reduce","push_messages":["Push"],"reducer":"protocol.rs::reduce","test":"tests/state.rs::pushed_state_is_visible","not_applicable":{"state":"stored by reducer","input":"push only","settings":"server owned"}});
        if missing == "reducer" {
            handler.as_object_mut().ok_or("handler")?.remove("reducer");
        }
        if missing == "test" {
            handler.as_object_mut().ok_or("handler")?.remove("test");
        }
        let mut entries = vec![handler];
        if missing != "window" {
            entries.push(json!({"mechanic":"1000-push","kind":"panels","id":"push-panel","status":"supported","entry":"view.rs::panel","not_applicable":{"state":"stored by reducer","protocol":"push handler","input":"push only","settings":"server owned"}}));
        }
        let registrations = f.0.join(format!("{feature}/registrations"));
        fs::create_dir_all(&registrations).map_err(|e| e.to_string())?;
        for (index, entry) in entries.iter().enumerate() {
            f.json(&format!("{feature}/registrations/{index}.json"), entry)?;
        }
        entries.sort_by_key(|entry| entry["id"].as_str().unwrap_or_default().to_owned());
        f.json(
            &format!("{feature}/fixtures/registrations.json"),
            &json!(entries),
        )?;
        assert!(client_fragments::scan(&f.0).is_err(), "missing {missing}");
    }
    Ok(())
}

#[test]
fn session_transport_handler_requires_actual_async_session_symbol_and_preserves_coverage()
-> Result<(), String> {
    let f = Fixture::structure()?;
    f.json(
        "crates/protocol-oteryn/capabilities/1000-transport-v1.json",
        &json!({"id":1000,"name":"TRANSPORT_V1","commands":[1000]}),
    )?;
    f.json(
        "docs/contracts/protocol-oteryn/v1/registry/command_types/1000-transport.json",
        &json!({"id":1000,"capability":1000}),
    )?;
    let feature = "crates/client-ui/src/features/1000-transport";
    let mut entry = json!({"mechanic":"1000-transport","kind":"handlers","id":"transport-handler",
        "status":"supported","commands":[1000],"capability":1000,"direction":"client_to_server",
        "entry_owner":"session","entry":"activity_handoff.rs::acknowledge",
        "not_applicable":{"state":"Session owns transfer","protocol":"Session transport entry",
            "view":"Native receipt presentation is separate","input":"Automatic transport acknowledgement",
            "settings":"Mandatory transport sequencing"}});
    f.json(&format!("{feature}/registrations/handler.json"), &entry)?;
    f.json(
        &format!("{feature}/fixtures/registrations.json"),
        &json!([entry]),
    )?;
    let source = "impl<S> Session<S> { async fn acknowledge(&mut self) {} }";
    f.write("crates/session/src/activity_handoff.rs", source)?;
    let (_, coverage) = client_fragments::scan(&f.0)?;
    assert!(
        coverage
            .iter()
            .any(|row| row["mechanic"] == "1000-transport" && row["status"] == "supported")
    );
    for invalid in [
        "impl<S> Session<S> { async fn other(&mut self) {} }",
        "impl OtherOwner { async fn acknowledge(&mut self) {} }",
        "impl<S> Session<S> { fn acknowledge(&mut self) {} }",
        "async fn acknowledge() {}",
    ] {
        f.write("crates/session/src/activity_handoff.rs", invalid)?;
        assert!(
            client_fragments::scan(&f.0).is_err(),
            "wrong transport symbol: {invalid}"
        );
    }
    f.write("crates/session/src/activity_handoff.rs", source)?;
    for invalid in ["missing.rs::acknowledge", "../outside.rs::acknowledge"] {
        entry["entry"] = json!(invalid);
        f.json(&format!("{feature}/registrations/handler.json"), &entry)?;
        f.json(
            &format!("{feature}/fixtures/registrations.json"),
            &json!([entry]),
        )?;
        assert!(
            client_fragments::scan(&f.0).is_err(),
            "wrong transport path: {invalid}"
        );
    }
    entry["entry"] = json!("activity_handoff.rs::acknowledge");
    entry["commands"] = json!([]);
    f.json(&format!("{feature}/registrations/handler.json"), &entry)?;
    f.json(
        &format!("{feature}/fixtures/registrations.json"),
        &json!([entry]),
    )?;
    assert!(
        client_fragments::scan(&f.0).is_err(),
        "transport entry cannot remove command coverage"
    );
    Ok(())
}
