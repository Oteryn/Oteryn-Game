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

/// A scratch repository holding every tracked compiler input of this tree.
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
                "content/assets/files",
                "imports/cipsoft-staticdata/map/subareas",
                "content/houses",
                "content/creatures",
                "content/items",
                "content/npcs/definitions",
                "content/dialogues/definitions",
                "content/services",
                "tools/world-bundle-compiler",
                "crates/world-bundle",
                "Cargo.toml",
                "Cargo.lock",
                "rust-toolchain.toml",
                "deploy/synology-game/preprod-topology.toml",
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

/// The reviewed revision, read from the checked-in identity file so a content bump needs no edit here.
fn reviewed_revision() -> String {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/world/pins/oteryn.identity.json");
    let identity: Value =
        serde_json::from_slice(&fs::read(path).expect("identity file")).expect("identity json");
    identity["content_revision"]
        .as_str()
        .expect("revision")
        .to_owned()
}

fn pin(digest: &str) -> Value {
    json!({
        "schema": "OTERYN_WORLD_PIN/v2",
        "digest": digest,
        "project_format_version": "OTERYN_WORLD_PROJECT_ROOT/v2",
        "world_schema_version": "reference-schema-v1",
        "content_revision": reviewed_revision(),
        "production": false,
        "entry_start": {"x": 32369, "y": 32238, "floor": -7},
    })
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
        fs::write(dir.path(path), text.replacen(from, to, 1)).unwrap();
        git(&dir.0, &["add", "-A"]);
    };
    let restore = || {
        git(
            &dir.0,
            &["checkout", "-q", "HEAD", "--", "Cargo.toml", "Cargo.lock"],
        )
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
        "8774e05a7d0de114588e6a28fe7e71694b82614ed569d86d8b389dfbc98b8ad8",
        &"9".repeat(64),
    );
    edit(
        "Cargo.toml",
        "rcgen = { version = \"=0.14.10\"",
        "rcgen = { version = \"=0.14.11\"",
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
    for (path, from, to) in [
        (
            "Cargo.lock",
            "db3ae9bf90af6e40197bef767eeefc46ad6397420fe873830bd85e5fbbfb3c70",
            "0".repeat(64),
        ),
        (
            "Cargo.lock",
            "name = \"zstd\"\nversion = \"0.14.3\"",
            "name = \"zstd\"\nversion = \"0.14.4\"".to_owned(),
        ),
        (
            "Cargo.toml",
            "zstd = { version = \"=0.14.3\"",
            "zstd = { version = \"=0.14.4\"".to_owned(),
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

// Each run compiles the whole World twice; in a debug build that takes tens of minutes, so
// it runs with `cargo test --release` (the `world_bundle` job runs the real pin-check in release).
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "compiles the whole World; run with --release"
)]
fn pin_check_accepts_the_reviewed_pin_and_refuses_every_drift() {
    let dir = Scratch::new("check");
    let first = pin(&"0".repeat(64));
    dir.write_json(&dir.pin_file(), &first);
    let out = dir.check();
    assert!(!out.status.success());
    let digest = word_after(&stderr(&out), "is not the tree's ");
    assert_eq!(digest.len(), 64, "{}", stderr(&out));

    // The reviewed pin passes (each run builds twice and compares); the artifact name
    // carries the digest.
    let good = pin(&digest);
    dir.write_json(&dir.pin_file(), &good);

    // Refresh migrates v1 without changing bundle identity or the reviewed entry cell.
    let mut legacy = good.clone();
    legacy["schema"] = json!("OTERYN_WORLD_PIN/v1");
    legacy["inputs_digest"] = json!("0".repeat(64));
    dir.write_json(&dir.pin_file(), &legacy);
    let refresh = Command::new(BIN)
        .arg("pin-refresh")
        .arg(&dir.0)
        .output()
        .expect("compiler runs");
    assert!(refresh.status.success(), "{}", stderr(&refresh));
    assert_eq!(dir.json(&dir.pin_file()), good);
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
            let cell = (tile.x, tile.y, sector.floor);
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
            extent.contains(x, y, floor),
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
