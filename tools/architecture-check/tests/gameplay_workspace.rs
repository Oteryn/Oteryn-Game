//! Exercise the real workspace command, including policy parsing and exit status.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

struct Workspace(PathBuf);

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn write(root: &Path, path: &str, contents: &str) -> Result<(), String> {
    let path = root.join(path);
    let parent = path
        .parent()
        .ok_or_else(|| "missing fixture parent".to_owned())?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    fs::write(path, contents).map_err(|error| error.to_string())
}

fn run(root: &Path) -> Result<Output, String> {
    Command::new(env!("CARGO_BIN_EXE_oteryn-architecture-check"))
        .arg("workspace")
        .arg(root)
        .output()
        .map_err(|error| error.to_string())
}

#[test]
fn gameplay_workspace_command_enforces_layers_names_and_boss_modes() -> Result<(), String> {
    let workspace = Workspace(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(format!(".gameplay-workspace-test-{}", std::process::id())),
    );
    let root = &workspace.0;
    write(
        root,
        "Cargo.toml",
        "[workspace]\nmembers = [\"apps/game-server\", \"crates/gameplay/entities\", \"crates/gameplay/foundation\"]\nresolver = \"3\"\n",
    )?;
    for (name, directory, dependencies, source) in [
        (
            "app",
            "apps/game-server",
            "carrier = { path = \"../../crates/gameplay/entities\" }",
            "",
        ),
        (
            "carrier",
            "crates/gameplay/entities",
            "kernel = { path = \"../foundation\" }",
            "fn welter() {}",
        ),
        ("kernel", "crates/gameplay/foundation", "", ""),
    ] {
        write(
            root,
            &format!("{directory}/Cargo.toml"),
            &format!(
                "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n[dependencies]\n{dependencies}\n"
            ),
        )?;
        write(root, &format!("{directory}/src/lib.rs"), source)?;
    }
    fs::create_dir_all(root.join("apps/game-server/tests")).map_err(|error| error.to_string())?;
    let policy = r#"schema_version = 1
members = ["app","carrier","kernel"]
paths = ["apps/game-server","crates/gameplay/entities","crates/gameplay/foundation"]
production = ["app","carrier","kernel"]
production_roots = ["app"]
synthetic = []
test = []
tool = []
forbidden_package_fragments = []
[gameplay_layers]
l2 = ["carrier","kernel"]
[gameplay_same_layer_edges]
carrier = ["kernel"]
[gameplay_rules]
forbidden_normal_dependencies = ["sqlx","tokio"]
forbidden_name_fragments = ["persistence"]
boss_identifier_paths = ["crates/gameplay/entities","crates/gameplay/foundation"]
boss_identifier_fragments = ["welter"]
boss_identifier_mode = "warn"
[edges]
app = ["carrier"]
carrier = ["kernel"]
kernel = []
"#;
    write(root, "workspace-boundaries.toml", policy)?;
    let warning = run(root)?;
    let stderr = String::from_utf8_lossy(&warning.stderr);
    assert!(warning.status.success(), "{stderr}");
    assert!(
        stderr.contains("WARN:") && stderr.contains("welter"),
        "{stderr}"
    );
    assert!(String::from_utf8_lossy(&warning.stdout).contains("workspace-boundaries: PASS"));

    write(
        root,
        "workspace-boundaries.toml",
        &policy.replace("= \"warn\"", "= \"error\""),
    )?;
    let error = run(root)?;
    assert!(!error.status.success());
    assert!(String::from_utf8_lossy(&error.stderr).contains("boss-specific identifier welter"));

    // Removing the explicit peer exception must fail even though [edges] still matches Cargo.
    write(
        root,
        "workspace-boundaries.toml",
        &policy.replacen("carrier = [\"kernel\"]\n", "", 1),
    )?;
    let peer = run(root)?;
    assert!(!peer.status.success());
    assert!(String::from_utf8_lossy(&peer.stderr).contains("gameplay layer order"));

    // A lower-layer dependency still fails if an L3 purity rule explicitly forbids it.
    let pure_policy = policy
        .replace(
            "l2 = [\"carrier\",\"kernel\"]",
            "l2 = [\"kernel\"]\nl3 = [\"carrier\"]",
        )
        .replacen("carrier = [\"kernel\"]\n", "", 1)
        .replace("[\"sqlx\",\"tokio\"]", "[\"sqlx\",\"tokio\",\"kernel\"]");
    write(root, "workspace-boundaries.toml", &pure_policy)?;
    let purity = run(root)?;
    assert!(!purity.status.success());
    assert!(String::from_utf8_lossy(&purity.stderr).contains("L3 purity: carrier -> kernel"));

    write(root, "workspace-boundaries.toml", policy)?;
    fs::create_dir_all(root.join("crates/gameplay/foundation/old-persistence"))
        .map_err(|error| error.to_string())?;
    let directory = run(root)?;
    assert!(!directory.status.success());
    assert!(String::from_utf8_lossy(&directory.stderr).contains("forbidden gameplay directory"));
    Ok(())
}
