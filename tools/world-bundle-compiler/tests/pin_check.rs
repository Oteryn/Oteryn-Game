//! WORLD-BUNDLE-CI-1: the `derive-identity` and `pin-check` modes against a copy of the real
//! World Project, in a scratch git repository (the `inputs_digest` reads the git index).

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use oteryn_world_bundle_compiler::bundle::{self, Manifest, TerrainKind};
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
                "content/houses",
                "content/creatures",
                "content/items",
                "tools/world-bundle-compiler",
                "crates/world-bundle",
                "Cargo.toml",
                "Cargo.lock",
                "rust-toolchain.toml",
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
        .trim_end_matches([',', '.'])
        .to_owned()
}

fn pin(inputs: &str, digest: &str) -> Value {
    json!({
        "schema": "OTERYN_WORLD_PIN/v1",
        "digest": digest,
        "project_format_version": "OTERYN_WORLD_PROJECT_ROOT/v2",
        "world_schema_version": "reference-schema-v1",
        "content_revision": "g4-npc-wave-a-r9",
        "production": false,
        "entry_start": {"x": 10228, "y": 10032, "floor": 0},
        "inputs_digest": inputs,
    })
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
    assert_eq!(identity["min_runtime_version"], "1");
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
    // The first run names the tree's inputs_digest in its refusal.
    let wrong = pin(&"0".repeat(64), &"0".repeat(64));
    dir.write_json(&dir.pin_file(), &wrong);
    let out = dir.check();
    assert!(!out.status.success());
    let tree = word_after(&stderr(&out), "the tree's ");
    assert_eq!(tree.len(), 64, "{}", stderr(&out));
    let stale = |dir: &Scratch| word_after(&stderr(&dir.check()), "the tree's ");

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

// Each run compiles the whole World twice; in a debug build that takes tens of minutes, so
// it runs with `cargo test --release` (the `world_bundle` job runs the real pin-check in release).
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "compiles the whole World; run with --release"
)]
fn pin_check_accepts_the_reviewed_pin_and_refuses_every_drift() {
    let dir = Scratch::new("check");
    let first = pin(&"0".repeat(64), &"0".repeat(64));
    dir.write_json(&dir.pin_file(), &first);
    let tree = word_after(&stderr(&dir.check()), "the tree's ");
    dir.write_json(&dir.pin_file(), &pin(&tree, &"0".repeat(64)));
    let out = dir.check();
    assert!(!out.status.success());
    let digest = word_after(&stderr(&out), "is not the tree's ");
    assert_eq!(digest.len(), 64, "{}", stderr(&out));

    // The reviewed pin passes (each run builds twice and compares); the artifact name
    // carries the digest.
    let good = pin(&tree, &digest);
    dir.write_json(&dir.pin_file(), &good);
    let out = dir.check();
    assert!(out.status.success(), "{}", stderr(&out));
    let name = fs::read_to_string(dir.path("out/artifact-name.txt")).unwrap();
    assert_eq!(name.trim(), format!("world-bundle-{digest}"));
    let written = fs::read(dir.path(&format!("out/{SLUG}-{digest}.otwb"))).unwrap();

    // A production pin, a stale inputs_digest and a stale revision each fail.
    let mut production = good.clone();
    production["production"] = json!(true);
    let mut stale = good.clone();
    stale["inputs_digest"] = json!("1".repeat(64));
    let mut revision = good.clone();
    revision["content_revision"] = json!("other");
    for bad in [production, stale, revision] {
        dir.write_json(&dir.pin_file(), &bad);
        assert!(!dir.check().status.success(), "{bad}");
    }

    // entry_start outside the World bounds fails; so does a cell without walkable ground
    // and one without a tile, both read from the bundle that was just built.
    let mut cells = vec![(5, 5, 0), (10228, 10032, 99), (34000, 33000, 0)];
    let (mut blocked, mut open) = (None, None);
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
        }
        Ok(())
    })
    .expect("bundle");
    let extent = visited.manifest.world;
    cells.extend(blocked);
    cells.extend(open);
    assert!(blocked.is_some(), "the World holds a blocked cell");
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
    let tree = word_after(&stderr(&dir.check()), "the tree's ");
    dir.write_json(&dir.pin_file(), &pin(&tree, &digest));
    let out = dir.check();
    assert!(!out.status.success() && stderr(&out).contains("identity file"));
}
