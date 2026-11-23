//! WORLD-BUNDLE-CI-1: the `derive-identity` and `pin-check` modes against a copy of the real
//! World Project, in a scratch git repository (the `inputs_digest` reads the git index).

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use oteryn_world_bundle_compiler::bundle::{self, Family, Manifest, TerrainKind};
use serde_json::{Value, json};

const BIN: &str = env!("CARGO_BIN_EXE_oteryn-world-bundle-compiler");
const SLUG: &str = "oteryn";
const TOPOLOGY: &str = "deploy/synology-game/preprod-topology.toml";

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn git(dir: &Path, args: &[&str]) -> Output {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["-c", "user.name=t", "-c", "user.email=t@example.test"])
        .args(args)
        .output()
        .expect("git runs");
    assert!(out.status.success(), "git {args:?}: {out:?}");
    out
}

/// A scratch repository holding the tracked compiler inputs and refresh targets.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("wb-pin-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("scratch");
        let root = repository_root();
        let listing = git(
            &root,
            &[
                "ls-files",
                "-z",
                "--",
                "content/world",
                "content/cyclopedia",
                "content/encounters/world-changes/layers.json",
                "content/encounters/world-changes/runtime.json",
                "content/assets/files",
                "imports/cipsoft-staticdata/map/subareas",
                "content/houses",
                "content/creatures/definitions",
                "content/items/definitions",
                "content/npcs/definitions",
                "content/quests/definitions",
                "content/dialogues/definitions",
                "content/services",
                "tools/world-bundle-compiler",
                "tools/content-schema/area-authoring/samples/crystal-world-towns-00ce02a5.json",
                "crates/world-bundle",
                "crates/client-assets/src/world.rs",
                "crates/gameplay/content/src/content/activity_static_surface_pin.rs",
                "Cargo.toml",
                "Cargo.lock",
                "rust-toolchain.toml",
                TOPOLOGY,
                ".cargo",
                "vendor",
            ],
        )
        .stdout;
        for path in listing.split(|b| *b == 0).filter(|p| !p.is_empty()) {
            let path = String::from_utf8(path.to_vec()).expect("utf-8 path");
            let to = dir.join(&path);
            fs::create_dir_all(to.parent().expect("parent")).expect("dir");
            fs::copy(root.join(&path), &to).expect("copy");
        }
        git(&dir, &["init", "-q"]);
        git(&dir, &["add", "-A"]);
        git(&dir, &["commit", "-q", "-m", "tree"]);
        Self(dir)
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.0.join(relative)
    }

    fn json(&self, relative: &str) -> Value {
        serde_json::from_slice(&fs::read(self.path(relative)).expect("read")).expect("json")
    }

    fn write_json(&self, relative: &str, value: &Value) {
        fs::write(
            self.path(relative),
            serde_json::to_string_pretty(value).expect("json") + "\n",
        )
        .expect("write");
        git(&self.0, &["add", "-A"]);
    }

    fn pin_file(&self) -> String {
        format!("content/world/pins/{SLUG}.json")
    }

    fn derive(&self) -> Output {
        Command::new(BIN)
            .args(["derive-identity"])
            .arg(&self.0)
            .arg(SLUG)
            .output()
            .expect("compiler runs")
    }

    fn check(&self) -> Output {
        Command::new(BIN)
            .args(["pin-check"])
            .arg(&self.0)
            .arg(self.path("out"))
            .output()
            .expect("compiler runs")
    }

    fn refresh(&self) -> Output {
        Command::new(BIN)
            .arg("pin-refresh")
            .arg(&self.0)
            .output()
            .expect("compiler runs")
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

fn word_after(text: &str, marker: &str) -> String {
    text.split(marker)
        .nth(1)
        .and_then(|rest| rest.split_whitespace().next())
        .unwrap_or_default()
        .trim_end_matches([',', '.', ';'])
        .to_owned()
}

fn pin(digest: &str) -> Value {
    let mut reviewed: Value =
        serde_json::from_slice(include_bytes!("../../../content/world/pins/oteryn.json"))
            .expect("reviewed pin");
    reviewed["digest"] = json!(digest);
    reviewed
}

#[test]
fn pin_check_rejects_v1_with_a_refresh_instruction_and_removed_v2_fields() {
    let dir = Scratch::new("schema");
    let mut legacy = pin(&"0".repeat(64));
    legacy["schema"] = json!("OTERYN_WORLD_PIN/v1");
    legacy["inputs_digest"] = json!("0".repeat(64));
    dir.write_json(&dir.pin_file(), &legacy);
    let out = dir.check();
    assert!(!out.status.success());
    assert!(stderr(&out).contains("schema is not OTERYN_WORLD_PIN/v2"));
    assert!(stderr(&out).contains("pin-refresh"));

    legacy["schema"] = json!("OTERYN_WORLD_PIN/v2");
    dir.write_json(&dir.pin_file(), &legacy);
    let out = dir.check();
    assert!(!out.status.success());
    assert!(stderr(&out).contains("unknown field `inputs_digest`"));
}

#[test]
fn derive_identity_takes_each_field_from_its_source_and_refuses_bad_sources() {
    let dir = Scratch::new("identity");
    let out = dir.derive();
    assert!(out.status.success(), "{}", stderr(&out));
    let identity: Value = serde_json::from_slice(&out.stdout).expect("identity json");
    let project = dir.json("content/world/project.json");
    let manifest = dir.json("content/world/manifest.json");
    let lock = dir.json("content/world/content.lock.json");
    assert_eq!(identity["project_format_version"], project["schema"]);
    assert_eq!(identity["content_revision"], project["project_revision"]);
    assert_eq!(
        identity["content_lock_digest"],
        project["content_lock_sha256"]
    );
    assert_eq!(
        identity["world_schema_version"],
        manifest["semantic_schema_version"]
    );
    assert_eq!(
        identity["required_capabilities"],
        manifest["required_features"]
    );
    assert_eq!(identity["required_capabilities"], json!([]));
    assert_eq!(
        identity["min_runtime_version"],
        bundle::RUNTIME_VERSION.to_string()
    );
    assert_eq!(
        identity["ruleset_compatibility"],
        dir.json(&format!("content/world/pins/{SLUG}.ruleset.json"))["ruleset_compatibility"]
    );
    let mut summary = lock["revision_digest_token"].as_str().unwrap().to_owned();
    for entry in lock["entries"].as_array().unwrap() {
        summary += &format!("/{}", entry["package_provenance_digest"].as_str().unwrap());
    }
    assert_eq!(identity["provenance_summary"], json!(summary));
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        fs::read_to_string(dir.path(&format!("content/world/pins/{SLUG}.identity.json"))).unwrap()
    );

    // A content_lock_sha256 that is not the SHA-256 of content.lock.json is refused.
    let mut wrong = project.clone();
    wrong["content_lock_sha256"] = json!("0".repeat(64));
    dir.write_json("content/world/project.json", &wrong);
    assert!(!dir.derive().status.success());
    dir.write_json("content/world/project.json", &project);

    // A capability this node does not implement is refused.
    let mut feature = manifest.clone();
    feature["required_features"] = json!(["oteryn:feature/x"]);
    dir.write_json("content/world/manifest.json", &feature);
    let out = dir.derive();
    assert!(!out.status.success() && stderr(&out).contains("oteryn:feature/x"));
    dir.write_json("content/world/manifest.json", &manifest);

    // A missing or malformed source is an error, never a chosen value.
    let mut missing = project.clone();
    missing.as_object_mut().unwrap().remove("project_revision");
    dir.write_json("content/world/project.json", &missing);
    assert!(!dir.derive().status.success());
    dir.write_json("content/world/project.json", &project);
    assert!(dir.derive().status.success());
}

#[test]
fn inputs_digest_is_stable_across_squash_and_pin_edits_and_moves_with_inputs() {
    let dir = Scratch::new("inputs");
    // A production pin fails before the build and reports the diagnostic input digest.
    let mut wrong = pin(&"0".repeat(64));
    wrong["production"] = json!(true);
    dir.write_json(&dir.pin_file(), &wrong);
    let out = dir.check();
    assert!(!out.status.success());
    let tree = word_after(&stderr(&out), "inputs_digest ");
    assert_eq!(tree.len(), 64, "{}", stderr(&out));
    let stale = |dir: &Scratch| word_after(&stderr(&dir.check()), "inputs_digest ");

    // A pin file edit does not move it; neither do a commit or a squash into one commit.
    let mut edited = wrong.clone();
    edited["entry_start"]["x"] = json!(1);
    dir.write_json(&dir.pin_file(), &edited);
    assert_eq!(stale(&dir), tree);
    git(&dir.0, &["commit", "-qam", "pin edit"]);
    git(&dir.0, &["reset", "-q", "--soft", "HEAD~1"]);
    git(&dir.0, &["commit", "-qm", "squashed"]);
    assert_eq!(stale(&dir), tree);

    // One compiler input moves it: a content file, the ruleset file, an added file.
    let ruleset = format!("content/world/pins/{SLUG}.ruleset.json");
    let before = fs::read(dir.path(&ruleset)).unwrap();
    dir.write_json(
        &ruleset,
        &json!({"ruleset_compatibility": ["oteryn:ruleset/entry-r2"]}),
    );
    let moved = stale(&dir);
    assert_ne!(moved, tree);
    fs::write(dir.path(&ruleset), before).unwrap();
    git(&dir.0, &["add", "-A"]);
    assert_eq!(stale(&dir), tree);
    fs::write(dir.path("tools/world-bundle-compiler/NOTE.txt"), "x").unwrap();
    git(&dir.0, &["add", "-A"]);
    assert_ne!(stale(&dir), tree);
    // A file outside the inputs does not.
    git(
        &dir.0,
        &["rm", "-qf", "tools/world-bundle-compiler/NOTE.txt"],
    );
    fs::write(dir.path("README.md"), "x").unwrap();
    git(&dir.0, &["add", "-A"]);
    assert_eq!(stale(&dir), tree);
}

/// PIN-SCOPE-1: the root `Cargo.toml` and `Cargo.lock` count only in what can reach a build of
/// the World crates.
#[test]
fn inputs_digest_takes_the_cargo_inputs_of_the_world_crates_only() {
    let dir = Scratch::new("cargo");
    let mut production = pin(&"0".repeat(64));
    production["production"] = json!(true);
    dir.write_json(&dir.pin_file(), &production);
    let stale = |dir: &Scratch| {
        let out = dir.check();
        assert!(!out.status.success());
        let tree = word_after(&stderr(&out), "inputs_digest ");
        assert_eq!(tree.len(), 64, "{}", stderr(&out));
        tree
    };
    let tree = stale(&dir);
    let edit = |path: &str, from: &str, to: &str| {
        let text = fs::read_to_string(dir.path(path)).unwrap();
        assert_eq!(text.matches(from).count(), 1, "{path}: {from}");
        assert_ne!(from, to, "{path}: {from}");
        fs::write(dir.path(path), text.replacen(from, to, 1)).unwrap();
        git(&dir.0, &["add", "-A"]);
    };
    let restore = || {
        git(
            &dir.0,
            &["checkout", "-q", "HEAD", "--", "Cargo.toml", "Cargo.lock"],
        )
    };
    // Read the current versions and checksums from the copied workspace. Dependency updates
    // must not invalidate the mutations before the digest comparison can run.
    let lock = fs::read_to_string(dir.path("Cargo.lock")).unwrap();
    let lock_field = |name: &str, field: &str| {
        let package = format!("name = \"{name}\"\n");
        lock.split("\n[[package]]\n")
            .find(|entry| entry.starts_with(&package))
            .expect("locked package")
            .lines()
            .find(|line| line.starts_with(field))
            .expect("locked package field")
    };
    let manifest = fs::read_to_string(dir.path("Cargo.toml")).unwrap();
    let workspace_version = |prefix: &str| {
        let value = manifest
            .lines()
            .find(|line| line.starts_with(prefix))
            .expect("workspace dependency");
        let end = value[prefix.len()..].find('"').expect("version end");
        &value[..prefix.len() + end + 1]
    };

    // An unrelated workspace member, its new lock package, an unrelated dependency bump in the
    // lock and in [workspace.dependencies], and a manifest comment do not move it.
    edit(
        "Cargo.toml",
        "    \"tools/world-bundle-compiler\",\n",
        "    \"tools/world-bundle-compiler\",\n    \"tools/unrelated\", # new crate\n",
    );
    edit(
        "Cargo.lock",
        "[[package]]\nname = \"rcgen\"\n",
        "[[package]]\nname = \"oteryn-unrelated\"\nversion = \"0.1.0\"\ndependencies = [\n \"rcgen\",\n \"serde\",\n]\n\n\
         [[package]]\nname = \"rcgen\"\n",
    );
    edit(
        "Cargo.lock",
        lock_field("rcgen", "checksum = "),
        &format!("checksum = \"{}\"", "9".repeat(64)),
    );
    edit(
        "Cargo.toml",
        workspace_version("rcgen = { version = \"="),
        "rcgen = { version = \"=999.0.0\"",
    );
    edit(
        "Cargo.toml",
        "[workspace.package]\n",
        "# The package defaults.\n[workspace.package]\n",
    );
    assert_eq!(stale(&dir), tree);
    restore();
    assert_eq!(stale(&dir), tree);

    // The checksum or version of a package in the closure, a World workspace dependency and a
    // profile each move it.
    let zstd_lock_version = format!("name = \"zstd\"\n{}", lock_field("zstd", "version = "));
    for (path, from, to) in [
        (
            "Cargo.lock",
            lock_field("zstd", "checksum = "),
            format!("checksum = \"{}\"", "0".repeat(64)),
        ),
        (
            "Cargo.lock",
            zstd_lock_version.as_str(),
            "name = \"zstd\"\nversion = \"999.0.0\"".to_owned(),
        ),
        (
            "Cargo.toml",
            workspace_version("zstd = { version = \"="),
            "zstd = { version = \"=999.0.0\"".to_owned(),
        ),
        (
            "Cargo.toml",
            "[workspace.lints.rust]\n",
            "[profile.release]\nopt-level = 2\n\n[workspace.lints.rust]\n".to_owned(),
        ),
    ] {
        edit(path, from, &to);
        assert_ne!(stale(&dir), tree, "{path}: {to}");
        restore();
    }
    assert_eq!(stale(&dir), tree);

    // A lock whose World crates reach another path package is refused.
    edit(
        "Cargo.lock",
        "name = \"oteryn-world-bundle-compiler\"\nversion = \"0.1.0\"\ndependencies = [\n",
        "name = \"oteryn-world-bundle-compiler\"\nversion = \"0.1.0\"\ndependencies = [\n \"oteryn-foundation\",\n",
    );
    let out = dir.check();
    assert!(
        !out.status.success() && stderr(&out).contains("oteryn-foundation"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn compile_regions_refuses_mismatched_placement_checksums_and_lists() {
    const INDEX: &str = "content/world/placements/index.json";
    const REGION: &str = "content/world/placements/region-z07-x126-y126.b3";
    let dir = Scratch::new("placement-checksums");
    let compile = || {
        Command::new(BIN)
            .arg("compile-regions")
            .arg(&dir.0)
            .arg(dir.path("content/world/pins/oteryn.identity.json"))
            .arg(dir.path("world.otwb"))
            .arg("126,126")
            .output()
            .expect("compiler runs")
    };
    let out = compile();
    assert!(out.status.success(), "{}", stderr(&out));

    let index = dir.json(INDEX);
    let at = index["regions"]
        .as_array()
        .unwrap()
        .iter()
        .position(|entry| entry["path"] == REGION)
        .expect("reviewed region");
    let mut wrong = index.clone();
    wrong["regions"][at]["sha256"] = json!("0".repeat(64));
    dir.write_json(INDEX, &wrong);
    let out = compile();
    assert!(
        !out.status.success() && stderr(&out).contains(&format!("{REGION}: sha256")),
        "status {}: {}",
        out.status,
        stderr(&out)
    );

    let mut missing = index.clone();
    missing["regions"].as_array_mut().unwrap().remove(at);
    let mut duplicate = index.clone();
    duplicate["shards"]
        .as_array_mut()
        .unwrap()
        .push(json!(REGION));
    let mut extra = index.clone();
    extra["shards"]
        .as_array_mut()
        .unwrap()
        .retain(|path| path != REGION);
    for wrong in [missing, duplicate, extra] {
        dir.write_json(INDEX, &wrong);
        let out = compile();
        assert!(
            !out.status.success()
                && stderr(&out).contains("shards and regions")
                && stderr(&out).contains(REGION),
            "status {}: {}",
            out.status,
            stderr(&out)
        );
    }
}

#[test]
fn pin_refresh_refuses_invalid_topology_before_writing() {
    let dir = Scratch::new("invalid-topology");
    let pin_path = dir.pin_file();
    let unset_digest = pin("");
    dir.write_json(&pin_path, &unset_digest);
    let topology = fs::read_to_string(dir.path(TOPOLOGY)).unwrap();
    let identity = dir.path("content/world/pins/oteryn.identity.json");
    let original_identity = fs::read(&identity).unwrap();
    // A refresh normally regenerates this file. Its bytes must survive validation failures.
    let stale_identity = [original_identity.as_slice(), b"\n"].concat();
    fs::write(&identity, &stale_identity).unwrap();
    for invalid in [
        format!("{topology}\ninvalid = [\n"),
        topology.replace("start_x = ", "incorrect_x = "),
        topology.replace("production = false", "production = \"false\""),
    ] {
        fs::write(dir.path(TOPOLOGY), &invalid).unwrap();
        let out = dir.refresh();
        assert!(
            !out.status.success() && stderr(&out).contains(TOPOLOGY),
            "status {}: {}",
            out.status,
            stderr(&out)
        );
        assert_eq!(dir.json(&pin_path), unset_digest);
        assert_eq!(fs::read(&identity).unwrap(), stale_identity);
        assert_eq!(fs::read_to_string(dir.path(TOPOLOGY)).unwrap(), invalid);
    }
}

// Use a release build for this regression, which compiles the complete World repeatedly.
#[test]
fn pin_check_accepts_the_reviewed_pin_and_refuses_every_drift() {
    let dir = Scratch::new("check");
    let good = dir.json(&dir.pin_file());
    let first = pin(&"0".repeat(64));
    dir.write_json(&dir.pin_file(), &first);
    let out = dir.check();
    assert!(!out.status.success());
    let digest = word_after(&stderr(&out), "is not the tree's ");
    assert_eq!(digest.len(), 64, "{}", stderr(&out));
    assert_eq!(good["digest"], digest);

    // The reviewed pin passes (each run builds twice and compares); the artifact name
    // carries the digest.
    dir.write_json(&dir.pin_file(), &good);

    // Refresh migrates v1 and its signed floor while preserving the bundle identity and entry cell.
    let mut legacy = good.clone();
    legacy["schema"] = json!("OTERYN_WORLD_PIN/v1");
    legacy["digest"] = json!("");
    legacy["inputs_digest"] = json!("0".repeat(64));
    let start = &good["entry_start"];
    let floor = start["floor"].as_i64().unwrap();
    legacy["entry_start"]["floor"] = json!(-floor);
    dir.write_json(&dir.pin_file(), &legacy);
    // The manifest can be stale independently of the old pin. A matching digest in an
    // unrelated field must be preserved along with the rest of the document.
    let topology = fs::read_to_string(dir.path(TOPOLOGY)).unwrap();
    let previous_digest = "a".repeat(64);
    let note = format!("\n# Previous bundle for operator reference: {previous_digest}\n");
    let stale_topology = topology
        .replace(&digest, &previous_digest)
        .replace("production = false", "production = true")
        .replace(&format!("start_x = {}", start["x"]), "start_x = 32369")
        .replace(&format!("start_y = {}", start["y"]), "start_y = 32238")
        .replace(
            &format!("start_floor = {floor}"),
            &format!("start_floor = {}", -floor),
        );
    fs::write(dir.path(TOPOLOGY), stale_topology + &note).unwrap();
    let refresh = dir.refresh();
    assert!(refresh.status.success(), "{}", stderr(&refresh));
    assert_eq!(dir.json(&dir.pin_file()), good);
    assert_eq!(
        fs::read_to_string(dir.path(TOPOLOGY)).unwrap(),
        topology + &note
    );
    let report: Value = serde_json::from_slice(&refresh.stdout).unwrap();
    assert_eq!(report["pins"][0]["digest"], digest);

    // Compiler-only input changes are reported; they do not stale an identical bundle pin.
    fs::write(dir.path("tools/world-bundle-compiler/NOTE.txt"), "x").unwrap();
    git(&dir.0, &["add", "-A"]);
    let out = dir.check();
    assert!(out.status.success(), "{}", stderr(&out));
    let checked: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_ne!(checked["inputs_digest"], report["inputs_digest"]);
    assert_eq!(checked["pins"][0]["digest"], digest);
    assert_eq!(dir.json(&dir.pin_file()), good);
    let name = fs::read_to_string(dir.path("out/artifact-name.txt")).unwrap();
    assert_eq!(name.trim(), format!("world-bundle-{digest}"));
    let written = fs::read(dir.path(&format!("out/{SLUG}-{digest}.otwb"))).unwrap();

    // A production pin, a v1 pin, a v2 pin with a removed field and a stale revision fail.
    let mut production = good.clone();
    production["production"] = json!(true);
    let mut legacy = good.clone();
    legacy["schema"] = json!("OTERYN_WORLD_PIN/v1");
    legacy["inputs_digest"] = json!("1".repeat(64));
    let mut removed = good.clone();
    removed["inputs_digest"] = json!("1".repeat(64));
    let mut revision = good.clone();
    revision["content_revision"] = json!("other");
    for bad in [production, legacy, removed, revision] {
        dir.write_json(&dir.pin_file(), &bad);
        assert!(!dir.check().status.success(), "{bad}");
    }

    // entry_start outside the World bounds fails; so does a cell without walkable ground,
    // one without a tile and one whose walkable ground holds a plain Item (no Terrain record),
    // all read from the bundle that was just built. (10228, 10032, 0), the former start, holds
    // the plain Item i1788: the node refuses it (E2012) because a plain Item without a served
    // `blocks_movement` blocks.
    let mut cells = vec![
        (5, 5, 0),
        (32369, 32238, -7),
        (10228, 10032, 99),
        (34000, 33000, 0),
        (10228, 10032, 0),
    ];
    let (mut blocked, mut open, mut item) = (None, None, None);
    let visited = bundle::visit(&written, |manifest: &Manifest, sector| {
        for tile in &sector.tiles {
            let terrains: Vec<_> = tile
                .items
                .iter()
                .filter(|item| item.depth == 0)
                .map(|item| manifest.palette[item.palette as usize].terrain)
                .collect();
            let ground = terrains
                .iter()
                .flatten()
                .find(|t| t.kind == TerrainKind::Ground);
            let cell = (tile.x, tile.y, -sector.floor);
            if blocked.is_none()
                && (terrains
                    .iter()
                    .flatten()
                    .any(|t| t.kind == TerrainKind::Wall)
                    || ground.is_some_and(|t| t.walkable == Some(false)))
            {
                blocked = Some(cell);
            }
            if open.is_none() && tile.items.is_empty() {
                open = Some(cell);
            }
            if item.is_none()
                && ground.is_some_and(|t| t.walkable == Some(true))
                && tile.items.iter().any(|entry| {
                    entry.depth == 0
                        && manifest.palette[entry.palette as usize].family == Family::Item
                        && manifest.palette[entry.palette as usize].terrain.is_none()
                })
            {
                item = Some(cell);
            }
        }
        Ok(())
    })
    .expect("bundle");
    let extent = visited.manifest.world;
    cells.extend(blocked);
    cells.extend(open);
    cells.extend(item);
    assert!(blocked.is_some(), "the World holds a blocked cell");
    assert!(
        item.is_some(),
        "the World holds a plain Item on walkable ground"
    );
    for (x, y, floor) in cells {
        let mut bad = good.clone();
        bad["entry_start"] = json!({"x": x, "y": y, "floor": floor});
        dir.write_json(&dir.pin_file(), &bad);
        let out = dir.check();
        assert!(
            !out.status.success() && stderr(&out).contains("entry_start"),
            "{x} {y} {floor} (inside bounds: {}): {}",
            extent.contains(x, y, -floor),
            stderr(&out)
        );
    }
    dir.write_json(&dir.pin_file(), &good);

    // A hand-edited identity field fails, even when the pin follows it.
    let identity = format!("content/world/pins/{SLUG}.identity.json");
    let mut edited = dir.json(&identity);
    edited["provenance_summary"] = json!("lock:hand-edited");
    dir.write_json(&identity, &edited);
    let out = dir.check();
    assert!(!out.status.success() && stderr(&out).contains("identity file"));
}

/// BUNDLE-BUILD-1: the preproduction topology manifest's `[world_bundle]` table is exactly the
/// committed World pin, and its readiness `map_revision` rule names the pinned digest.
#[test]
fn preprod_manifest_world_bundle_equals_the_pin() {
    let root = repository_root();
    let pin: Value = serde_json::from_slice(
        &fs::read(root.join("content/world/pins/oteryn.json")).expect("pin"),
    )
    .expect("pin json");
    let manifest = fs::read_to_string(root.join("deploy/synology-game/preprod-topology.toml"))
        .expect("manifest");
    let table: Vec<&str> = manifest
        .lines()
        .skip_while(|line| *line != "[world_bundle]")
        .skip(1)
        .take_while(|line| !line.starts_with('['))
        .collect();
    let get = |key: &str| -> String {
        let hits: Vec<&str> = table
            .iter()
            .filter_map(|line| line.strip_prefix(key)?.strip_prefix(" = "))
            .collect();
        assert_eq!(hits.len(), 1, "[world_bundle] {key}");
        hits[0].trim_matches('"').to_owned()
    };
    for key in [
        "digest",
        "project_format_version",
        "world_schema_version",
        "content_revision",
    ] {
        assert_eq!(get(key), pin[key].as_str().expect(key), "{key}");
    }
    for (key, value) in [
        ("start_x", &pin["entry_start"]["x"]),
        ("start_y", &pin["entry_start"]["y"]),
        ("start_floor", &pin["entry_start"]["floor"]),
    ] {
        assert_eq!(get(key), value.to_string(), "{key}");
    }
    assert_eq!(get("production"), "false");
    assert_eq!(pin["production"], json!(false));
}
