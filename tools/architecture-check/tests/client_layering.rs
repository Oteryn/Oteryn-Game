#[path = "support/client.rs"]
mod support;

use oteryn_architecture_check::client_layering;
use serde_json::{Value, json};
use std::process::Command;
use support::Fixture;

fn metadata(source: &str, target: &str, kind: Value) -> Value {
    json!({"workspace_members":["source", "target"],"packages":[
        {"id":"source","name":format!("oteryn-{source}"),"dependencies":[{"name":format!("oteryn-{target}"),"kind":kind}]},
        {"id":"target","name":format!("oteryn-{target}"),"dependencies":[]}
    ]})
}

#[test]
fn forbidden_edge_classes_fail_with_real_cargo_metadata() -> Result<(), String> {
    for (source, target) in [
        ("foundation", "client-domain"),
        ("input-actions", "client-ui"),
        ("protocol-oteryn", "session"),
        ("client-domain", "protocol-oteryn"),
        ("client-domain", "session"),
        ("client-simulation", "client-runtime"),
        ("client-runtime", "client-ui"),
        ("session", "renderer"),
        ("session-tcp", "client-assets"),
        ("renderer", "client-runtime"),
        ("client-assets", "client-ui"),
        ("input-platform", "client-ui"),
        ("platform-client", "client-ui"),
        ("client-ui", "session-tcp"),
        ("client-ui", "protocol-oteryn"),
        ("client-ui", "platform-client"),
    ] {
        let fixture = Fixture::new()?;
        fixture.write("Cargo.toml", &format!("[workspace]\nmembers = [\"crates/{source}\",\"crates/{target}\"]\nresolver = \"3\"\n"))?;
        for name in [source, target] {
            let dependency = if name == source {
                format!(
                    "[dependencies]\nrenamed = {{ package = \"oteryn-{target}\", path = \"../{target}\" }}\n"
                )
            } else {
                String::new()
            };
            fixture.write(&format!("crates/{name}/Cargo.toml"), &format!("[package]\nname = \"oteryn-{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n{dependency}"))?;
            fixture.write(&format!("crates/{name}/src/lib.rs"), "")?;
        }
        let output = Command::new(env!("CARGO"))
            .current_dir(&fixture.0)
            .args([
                "metadata",
                "--offline",
                "--no-deps",
                "--format-version",
                "1",
            ])
            .output()
            .map_err(|e| e.to_string())?;
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let metadata = serde_json::from_slice(&output.stdout).map_err(|e| e.to_string())?;
        let error = client_layering::validate(&fixture.0, &metadata)
            .err()
            .ok_or("forbidden edge passed")?;
        assert!(
            error.contains("client layering"),
            "{source} -> {target}: {error}"
        );
    }
    Ok(())
}

#[test]
fn purity_and_dev_build_edges_cannot_bypass_layering() -> Result<(), String> {
    let fixture = Fixture::structure()?;
    assert_eq!(fixture.current()?.len(), 1);
    for kind in [json!(null), json!("dev"), json!("build")] {
        assert!(
            client_layering::validate(&fixture.0, &metadata("client-ui", "protocol-oteryn", kind))
                .is_err()
        );
    }
    for source in [
        "use std::{net::TcpStream};",
        "use std::fs as storage;",
        "fn run() { let _ = tokio::spawn(async {}); }",
        "use wgpu::*;",
        "use std::os::windows::*;",
        "use oteryn_protocol_oteryn::Frame;",
    ] {
        fixture.write("crates/client-domain/src/lib.rs", source)?;
        let error = client_layering::validate(
            &fixture.0,
            &metadata("client-domain", "foundation", json!(null)),
        )
        .err()
        .ok_or("IO import passed")?;
        assert!(error.contains("purity"), "{error}");
    }
    fixture.write(
        "crates/client-domain/src/lib.rs",
        "#[cfg(test)] mod tests { use std::fs; }",
    )?;
    client_layering::validate(
        &fixture.0,
        &metadata("client-domain", "foundation", json!(null)),
    )?;
    client_layering::validate(
        &fixture.0,
        &metadata("client-simulation", "foundation", json!("dev")),
    )?;
    Ok(())
}

#[test]
fn client_dependency_cycles_are_rejected() -> Result<(), String> {
    let fixture = Fixture::new()?;
    let mut graph = metadata("other-a", "other-b", json!(null));
    graph["packages"][1]["dependencies"] = json!([{"name":"oteryn-other-a","kind":null}]);
    assert!(client_layering::validate(&fixture.0, &graph).is_err());
    Ok(())
}

#[test]
fn workspace_command_runs_client_layering_against_current_metadata() -> Result<(), String> {
    let output = Command::new(env!("CARGO_BIN_EXE_oteryn-architecture-check"))
        .args(["workspace", "../.."])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .map_err(|e| e.to_string())?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("workspace-boundaries: PASS"));
    Ok(())
}
