#![allow(clippy::expect_used)]
/// Revision-advancing durable writers and the only non-test source files allowed to call
/// them. Every other writer reaches them through a [`RevisionSlot`].
const SEQUENCED_WRITERS: [(&str, &str); 15] = [
    (
        ".commit_cyclopedia(",
        "durability/character_revision_sequencer/cyclopedia.rs",
    ),
    (
        ".commit_character_experience(",
        "durability/character_revision_sequencer.rs",
    ),
    (
        ".commit_character_death(",
        "durability/character_revision_sequencer.rs",
    ),
    (
        ".commit_bestiary_kill(",
        "durability/character_revision_sequencer.rs",
    ),
    (
        ".commit_charm_command(",
        "durability/character_revision_sequencer.rs",
    ),
    (
        ".commit_character_monk_state_save(",
        "durability/character_revision_sequencer.rs",
    ),
    (
        ".commit_character_build(",
        "durability/character_revision_sequencer.rs",
    ),
    (
        ".commit_character_proficiency(",
        "durability/character_revision_sequencer.rs",
    ),
    (
        ".commit_character_quest_transition(",
        "durability/character_revision_sequencer.rs",
    ),
    (
        ".commit_character_proficiency_modification(",
        "durability/character_revision_sequencer.rs",
    ),
    (
        ".commit_character_quest_experience(",
        "durability/character_revision_sequencer.rs",
    ),
    (
        ".commit_character_wheel(",
        "durability/character_revision_sequencer.rs",
    ),
    (
        ".reset_character_wheel(",
        "durability/character_revision_sequencer.rs",
    ),
    // Revision-advancing fees run inside their sequenced Character source.
    // ITEM-TIMED-1 is the accepted item-only exception under the same locked root.
    (
        "burn_fee_in_transaction(",
        "durability/charm_state.rs|durability/npc_purchase.rs|durability/character_cyclopedia.rs|durability/item_timed_runtime/repair.rs",
    ),
    (
        ".commit_npc_purchase(",
        "durability/character_revision_sequencer/npc_purchase.rs",
    ),
];

/// The non-test part of a source file: from the first `#[cfg(test)]` that, after any other
/// attributes, opens a `mod`, the rest is cut off.
fn production_source(source: &str) -> &str {
    let mut offset = 0;
    let mut lines = source.lines();
    while let Some(line) = lines.next() {
        if line.trim() == "#[cfg(test)]" {
            let mut ahead = lines.clone().map(str::trim);
            if ahead
                .find(|next| !next.starts_with("#["))
                .is_some_and(|next| next.starts_with("mod "))
            {
                return &source[..offset];
            }
        }
        offset += line.len() + 1;
    }
    source
}

fn bypasses(path: &str, source: &str) -> Vec<String> {
    let source = production_source(source);
    SEQUENCED_WRITERS
        .iter()
        .filter(|(call, allowed)| {
            !allowed.split('|').any(|allowed| path.ends_with(allowed))
                && source.lines().any(|line| {
                    let code = line.split("//").next().unwrap_or_default();
                    code.contains(call) && !code.contains("fn ")
                })
        })
        .map(|(call, _)| format!("{path}: {call}"))
        .collect()
}

fn sources(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("source directory") {
        let path = entry.expect("source entry").path();
        if path.is_dir() {
            sources(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs")
            && !path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name == "tests.rs" || name.ends_with("_tests.rs"))
        {
            out.push(path);
        }
    }
}

#[test]
fn no_production_writer_advances_a_revision_outside_the_sequencer() {
    let root = std::path::Path::new(if env!("CARGO_PKG_NAME") == "oteryn-game-server" {
        env!("CARGO_MANIFEST_DIR")
    } else {
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../../apps/game-server")
    })
    .join("src");
    let mut files = Vec::new();
    sources(&root, &mut files);
    for package in ["foundation", "content", "durability"] {
        let extracted = root.join(format!("../../../crates/gameplay/{package}/src"));
        let before = files.len();
        sources(&extracted, &mut files);
        assert!(files.len() > before, "the scan must see {package} sources");
    }
    assert!(
        files.len() > 100,
        "the scan must see the game-server sources"
    );
    let mut found = Vec::new();
    for file in files {
        let source = std::fs::read_to_string(&file).expect("source file");
        let path = file.to_string_lossy().replace('\\', "/");
        found.extend(bypasses(&path, &source));
    }
    assert!(found.is_empty(), "unsequenced revision writers: {found:?}");
}

#[test]
fn the_structural_gate_catches_a_bypass_writer() {
    let bypass = "async fn award(root: &DurabilityRoot) {\n    \
                      let _ = root\n        .commit_character_experience(a, n, f, r)\n        .await;\n}\n";
    assert_eq!(
        bypasses("src/combat/new_writer.rs", bypass),
        ["src/combat/new_writer.rs: .commit_character_experience("]
    );
    let fee = "burn_fee_in_transaction(&mut tx, &fence, &request).await?;";
    assert_eq!(bypasses("src/durability/item_transfer.rs", fee).len(), 1);
    assert!(bypasses("src/durability/charm_state.rs", fee).is_empty());
    // Test modules and definitions are not writers.
    let tested = "pub async fn commit_charm_command(\n#[cfg(test)]\n#[allow(clippy::expect_used)]\nmod tests { x.commit_charm_command(); }";
    assert!(bypasses("src/durability/charm_state.rs", tested).is_empty());
    assert!(bypasses("src/gameplay_transport/charm_native.rs", tested).is_empty());
}
