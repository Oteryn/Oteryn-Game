#[path = "support/client.rs"]
mod support;

use oteryn_architecture_check::client_structure;
use serde_json::json;
use std::process::Command;
use support::Fixture;

fn rejected(fixture: &Fixture, expected: &str) -> Result<(), String> {
    let error = client_structure::validate(&fixture.0)
        .err()
        .ok_or("bad fixture passed")?;
    assert!(error.contains(expected), "expected {expected}: {error}");
    Ok(())
}

#[test]
fn structure_command_checks_real_tree_and_reports_every_mechanic() -> Result<(), String> {
    let output = Command::new(env!("CARGO_BIN_EXE_oteryn-architecture-check"))
        .args(["client-structure", "../.."])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .map_err(|e| e.to_string())?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("0057-cyclopedia-map") && stdout.contains("unsupported"));
    Ok(())
}

#[test]
fn new_files_and_existing_ratchets_enforce_lines_and_bytes() -> Result<(), String> {
    let f = Fixture::structure()?;
    let path = "apps/client/src/new.rs";
    f.write(path, &"// line\n".repeat(601))?;
    rejected(&f, "exceeds 600")?;
    f.json(
        "tools/architecture-check/client-size-baseline.json",
        &json!({path:{"lines":601,"bytes":4808}}),
    )?;
    client_structure::validate(&f.0)?;
    f.write(path, &"// larger line\n".repeat(601))?;
    rejected(&f, "ratchet grew")?;
    f.write(path, &"// line\n".repeat(602))?;
    rejected(&f, "ratchet grew")?;
    f.write(path, "// shrunk\n")?;
    client_structure::validate(&f.0)?;
    std::fs::remove_file(f.0.join(path)).map_err(|e| e.to_string())?;
    client_structure::validate(&f.0)?;
    Ok(())
}

#[test]
fn shell_test_fragment_limits_and_generated_output_are_enforced() -> Result<(), String> {
    for (path, lines, expected) in [
        ("apps/client/src/lib.rs", 301, "exceeds 300"),
        (
            "crates/client-runtime/src/features/0057-cyclopedia-map/mod.rs",
            301,
            "exceeds 300",
        ),
        ("crates/client-ui/tests/view.rs", 1001, "exceeds 1000"),
        ("crates/client-ui/src/generated.rs", 1, "OUT_DIR"),
        (
            "crates/client-ui/src/features/0057-cyclopedia-map/registrations/huge.json",
            201,
            "exceeds 200",
        ),
    ] {
        let f = Fixture::structure()?;
        f.write(path, &"\n".repeat(lines))?;
        rejected(&f, expected)?;
    }
    Ok(())
}

#[test]
fn central_catalogues_and_event_matches_fail_outside_features() -> Result<(), String> {
    for source in [
        "const PANELS: &[u8] = &[];",
        "static DEFAULT_KEYBINDINGS: &[u8] = &[];",
        "fn route(e: ServerEvent) { match e { ServerEvent::Chat => (), _ => () } }",
    ] {
        let f = Fixture::structure()?;
        f.write("apps/client/src/catalogue.rs", source)?;
        rejected(&f, "central client registration")?;
    }
    Ok(())
}

#[test]
fn duplicate_unknown_misnamed_missing_and_unsupported_growth_fail() -> Result<(), String> {
    for (case, expected) in [
        ("duplicate", "duplicate registration"),
        ("duplicate-command", "duplicate command handler"),
        ("unknown", "unknown fragment field"),
        ("mismatch", "mechanic/directory"),
        ("missing", "missing/extra command"),
        ("reason", "reason"),
        ("schema", "unexpected value"),
        ("growth", "coverage grew"),
        ("stale", "stale unsupported baseline"),
        ("golden", "golden differs"),
    ] {
        let f = Fixture::structure()?;
        let mut entries = f.current()?;
        match case {
            "duplicate" => entries.push(entries[0].clone()),
            "duplicate-command" => {
                let mut other = entries[0].clone();
                other["id"] = json!("other-handler");
                entries.push(other);
            }
            "unknown" => entries[0]["typo"] = json!(true),
            "mismatch" => entries[0]["mechanic"] = json!("0057-cyclopedia-map-v1"),
            "missing" => entries[0]["commands"] = json!([72]),
            "reason" => entries[0]["reason"] = json!(""),
            "schema" => entries[0]["kind"] = json!("settings"),
            "growth" => f.json(
                "tools/architecture-check/client-unsupported-baseline.json",
                &json!([]),
            )?,
            "stale" => f.json(
                "tools/architecture-check/client-unsupported-baseline.json",
                &json!(["0057-cyclopedia-map", "1000-stale"]),
            )?,
            "golden" => entries[0]["reason"] = json!("changed"),
            _ => return Err("unknown fixture".into()),
        }
        if case == "golden" {
            f.json(
                "crates/client-ui/src/features/0057-cyclopedia-map/registrations/0000.json",
                &entries[0],
            )?;
        } else {
            f.entries(&entries)?;
        }
        rejected(&f, expected)?;
    }
    Ok(())
}

#[test]
fn per_mechanic_unsupported_baseline_files_count_once() -> Result<(), String> {
    let f = Fixture::structure()?;
    let file = "tools/architecture-check/client-unsupported-baseline.d/0057-cyclopedia-map.json";
    f.json(file, &json!({"mechanic":"0057-cyclopedia-map"}))?;
    rejected(&f, "duplicate unsupported baseline entry")?;
    f.json(
        "tools/architecture-check/client-unsupported-baseline.json",
        &json!([]),
    )?;
    client_structure::validate(&f.0)?;
    f.json(file, &json!({"mechanic":"1000-other"}))?;
    rejected(&f, "unsupported baseline file content")?;
    f.write(
        "tools/architecture-check/client-unsupported-baseline.d/0057-cyclopedia-map.txt",
        "",
    )?;
    rejected(&f, "unsupported baseline file")?;
    Ok(())
}

#[test]
fn feature_layout_and_versioned_directory_names_fail() -> Result<(), String> {
    let f = Fixture::structure()?;
    f.write(
        "crates/client-ui/src/features/0057-cyclopedia-map/catalog.rs",
        "",
    )?;
    rejected(&f, "invalid feature concern")?;
    std::fs::remove_file(f.0.join("crates/client-ui/src/features/0057-cyclopedia-map/catalog.rs"))
        .map_err(|e| e.to_string())?;
    let mut entries = f.current()?;
    entries[0]["not_applicable"]
        .as_object_mut()
        .ok_or("not applicable")?
        .remove("state");
    f.entries(&entries)?;
    rejected(&f, "missing concern state")?;
    f.write(
        "crates/client-domain/src/features/0057-cyclopedia-map-v1/mod.rs",
        "",
    )?;
    rejected(&f, "invalid feature directory")?;
    let f = Fixture::structure()?;
    f.write(
        "crates/client-ui/src/components/registrations/stray.json",
        "{}",
    )?;
    rejected(&f, "misplaced client registrations")?;
    Ok(())
}

#[test]
fn default_conflicts_fail_but_explicit_precedence_disambiguates() -> Result<(), String> {
    let f = Fixture::structure()?;
    f.write(
        "crates/client-ui/src/features/0057-cyclopedia-map/input.rs",
        "fn bind() {}",
    )?;
    let mut entries = f.current()?;
    for (id, chord) in [("a", "Ctrl+K"), ("b", "k+CTRL")] {
        entries.push(json!({"mechanic":"0057-cyclopedia-map","kind":"keybindings","id":id,"status":"supported","context":"game","chord":chord,"entry":"input.rs::bind"}));
    }
    f.entries(&entries)?;
    rejected(&f, "default keybinding conflict")?;
    entries[2]["precedence"] = json!(1);
    f.entries(&entries)?;
    client_structure::validate(&f.0)?;
    entries[2]["context"] = json!("menu");
    entries[2]
        .as_object_mut()
        .ok_or("binding")?
        .remove("precedence");
    f.entries(&entries)?;
    client_structure::validate(&f.0)?;
    Ok(())
}

#[test]
fn supported_entry_symbols_asset_digests_and_parents_are_checked() -> Result<(), String> {
    for (entry, error) in [
        (
            json!({"kind":"panels","entry":"view.rs::absent"}),
            "missing Rust entry",
        ),
        (
            json!({"kind":"panels","entry":"../escape.rs::render"}),
            "non-relative",
        ),
        (
            json!({"kind":"assets","asset":"view.rs","digest":"bad"}),
            "digest mismatch",
        ),
        (
            json!({"kind":"cyclopedia-pages","entry":"view.rs::render","parent":"absent"}),
            "unknown page/panel parent",
        ),
        (
            json!({"kind":"settings-pages","entry":"view.rs::render","key":"map.zoom","setting_type":"bad","version":1,"settings_envelope":{}}),
            "invalid setting",
        ),
    ] {
        let f = Fixture::structure()?;
        f.write(
            "crates/client-ui/src/features/0057-cyclopedia-map/view.rs",
            "fn render() {}",
        )?;
        let mut entries = f.current()?;
        let mut entry = entry;
        for (k, v) in [
            ("mechanic", "0057-cyclopedia-map"),
            ("id", "additional"),
            ("status", "supported"),
        ] {
            entry[k] = json!(v);
        }
        entries.push(entry);
        f.entries(&entries)?;
        rejected(&f, error)?;
    }
    Ok(())
}
