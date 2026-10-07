//! SPELL-LOCK-2 key-33 pin: every PostgreSQL advisory lock on key 33 (the scope item lock)
//! and every production Rust writer of a key-33-triggered table is enumerated here, and
//! each Rust writer holds a lane-derived proof. A new key-33 site fails this test until it
//! is reviewed against the spell lane and added to the pinned inventory.

#![allow(clippy::expect_used, clippy::panic)]

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

const KEY33_TABLES: [&str; 3] = [
    "game_item_ground_locations",
    "game_item_corpse_container_entries",
    "game_native_map_scope_adoptions",
];

/// SQL functions that take `pg_advisory_xact_lock(..., 33)` and the table their trigger guards.
const SQL_KEY33_FUNCTIONS: [(&str, &str); 3] = [
    ("game_item_ground_owner_lock", "game_item_ground_locations"),
    (
        "game_spell_corpse_entry_owner_lock",
        "game_item_corpse_container_entries",
    ),
    (
        "game_native_map_adoption_stamp",
        "game_native_map_scope_adoptions",
    ),
];

/// Production Rust functions that take the key-33 lock or write a key-33 table.
const RUST_KEY33_SITES: [(&str, &str); 12] = [
    ("src/durability/item_decay_retire.rs", "apply_retire"),
    (
        "src/durability/item_mint.rs",
        "insert_mint_with_corpse_attribution",
    ),
    ("src/durability/item_mint.rs", "insert_corpse_loot_mint"),
    ("src/durability/item_transfer.rs", "apply_transfer"),
    ("src/durability/map_item_mint.rs", "apply_mint"),
    (
        "src/durability/native_map_items.rs",
        "initialize_in_transaction",
    ),
    ("src/durability/native_map_items.rs", "adopt_current_ground"),
    (
        "src/durability/spell_item_temporal.rs",
        "drain_due_items_in_transaction",
    ),
    (
        "src/durability/spell_item_temporal.rs",
        "drain_due_field_chains_in_transaction",
    ),
    (
        "src/durability/spell_item_transaction.rs",
        "assert_spell_item_scope_with_recovery",
    ),
    (
        "src/durability/spell_item_transaction.rs",
        "expire_due_spell_items_bounded",
    ),
    (
        "src/durability/spell_item_transaction.rs",
        "assert_spell_item_authority_with_recovery",
    ),
];

/// Lane-derived proofs: a permit or window directly, or a scope/item authority that is only
/// minted by the permit-taking `assert_spell_item_*_with_recovery` functions.
const LANE_PROOFS: [&str; 4] = [
    "SpellLanePermit",
    "SpellCommitWindow",
    "SpellItemScopeAuthority",
    "SpellItemAuthority",
];

/// Private helpers without a proof parameter, pinned to the proof-holding caller in the same file.
const DELEGATED: [(&str, &str, &str); 1] = [(
    "src/durability/native_map_items.rs",
    "adopt_current_ground",
    "initialize_in_transaction",
)];

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn collapse(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn is_ident(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn find_all(text: &str, needle: &str) -> Vec<usize> {
    text.match_indices(needle)
        .map(|(at, _)| at)
        .filter(|at| {
            let end = at + needle.len();
            !needle.ends_with(is_ident_char) || end >= text.len() || !is_ident(text.as_bytes()[end])
        })
        .collect()
}

fn is_ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

fn key33_lock_sites(text: &str) -> Vec<usize> {
    let mut sites = find_all(text, "'hex'),33)");
    sites.extend(find_all(text, "'hex'), 33)"));
    sites
}

fn rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("source dir") {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            if path.file_name().is_some_and(|name| name != "tests") {
                rs_files(&path, out);
            }
        } else if path.extension().is_some_and(|ext| ext == "rs")
            && !path.to_string_lossy().ends_with("_tests.rs")
        {
            out.push(path);
        }
    }
}

/// Production body of a source file: everything before its trailing `#[cfg(test)] mod`.
fn production(text: &str) -> &str {
    let mut search = 0;
    while let Some(found) = text[search..].find("#[cfg(test)]") {
        let at = search + found;
        let rest = text[at + "#[cfg(test)]".len()..].trim_start();
        let rest = rest
            .strip_prefix("#[allow(")
            .and_then(|tail| tail.split_once(")]").map(|(_, after)| after.trim_start()))
            .unwrap_or(rest);
        if rest.starts_with("mod ") || rest.starts_with("pub(crate) mod ") {
            return &text[..at];
        }
        search = at + 1;
    }
    text
}

/// Name and signature (up to the body brace) of the last `fn` before `at`.
fn enclosing_fn(text: &str, at: usize) -> (String, String) {
    let start = find_all(&text[..at], "fn ")
        .into_iter()
        .rev()
        .find(|&fn_at| fn_at == 0 || !is_ident(text.as_bytes()[fn_at - 1]))
        .expect("enclosing fn");
    let name: String = text[start + 3..]
        .chars()
        .take_while(|&c| is_ident_char(c))
        .collect();
    let brace = text[start..].find('{').expect("fn body") + start;
    (name, text[start..brace].to_owned())
}

fn signature_of(text: &str, name: &str) -> String {
    let needle = format!("fn {name}(");
    let at = text.find(&needle).expect("pinned fn present");
    enclosing_fn(text, at + needle.len()).1
}

#[test]
fn sql_key33_lock_functions_and_triggers_are_pinned() {
    let mut found = BTreeSet::new();
    let mut sql = String::new();
    let mut paths: Vec<_> = fs::read_dir(root().join("migrations"))
        .expect("migrations")
        .map(|entry| entry.expect("migration").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "sql"))
        .collect();
    paths.sort();
    for path in paths {
        let text = collapse(&fs::read_to_string(&path).expect("migration text"));
        for at in key33_lock_sites(&text) {
            let start = text[..at]
                .rfind("FUNCTION ")
                .expect("key-33 lock outside a function");
            let name: String = text[start + "FUNCTION ".len()..]
                .chars()
                .take_while(|&c| is_ident_char(c))
                .collect();
            found.insert(name);
        }
        sql.push_str(&text);
        sql.push(' ');
    }
    let pinned: BTreeSet<String> = SQL_KEY33_FUNCTIONS
        .iter()
        .map(|(name, _)| (*name).to_owned())
        .collect();
    assert_eq!(
        found, pinned,
        "SQL key-33 lock functions changed; review against the spell lane"
    );
    for (function, table) in SQL_KEY33_FUNCTIONS {
        let trigger = format!("CREATE TRIGGER {function} BEFORE INSERT");
        let at = sql.find(&trigger).expect("key-33 trigger present");
        let statement = &sql[at..at + sql[at..].find(';').expect("trigger end")];
        assert!(
            statement.contains(&format!(" ON {table} ")),
            "{function} guards {table}"
        );
        assert!(statement.contains(&format!("EXECUTE FUNCTION {function}()")));
    }
}

#[test]
fn rust_key33_writers_are_pinned_and_hold_a_lane_proof() {
    let mut files = Vec::new();
    rs_files(&root().join("src"), &mut files);
    let mut found = BTreeSet::new();
    for path in files {
        let relative = path
            .strip_prefix(root())
            .expect("under manifest")
            .to_string_lossy()
            .replace('\\', "/");
        let raw = fs::read_to_string(&path).expect("source text");
        let text = collapse(production(&raw));
        let mut sites = key33_lock_sites(&text);
        for table in KEY33_TABLES {
            for verb in ["INSERT INTO", "DELETE FROM", "UPDATE"] {
                sites.extend(find_all(&text, &format!("{verb} {table}")));
            }
        }
        for at in sites {
            let (name, signature) = enclosing_fn(&text, at);
            found.insert((relative.clone(), name.clone()));
            let delegated = DELEGATED
                .iter()
                .find(|(file, helper, _)| *file == relative && *helper == name);
            let proof_holder = match delegated {
                Some((_, _, caller)) => {
                    let caller_signature = signature_of(&text, caller);
                    assert!(
                        text.matches(&format!("{name}(")).count() == 2,
                        "{relative}::{name} must have exactly one caller ({caller})"
                    );
                    caller_signature
                }
                None => signature,
            };
            assert!(
                LANE_PROOFS.iter().any(|proof| proof_holder.contains(proof)),
                "{relative}::{name} writes a key-33 table without a lane-derived proof"
            );
        }
    }
    let pinned: BTreeSet<(String, String)> = RUST_KEY33_SITES
        .iter()
        .map(|(file, name)| ((*file).to_owned(), (*name).to_owned()))
        .collect();
    assert_eq!(
        found, pinned,
        "Rust key-33 sites changed; review against the spell lane"
    );
}

#[test]
fn lane_proof_authorities_are_minted_only_behind_a_permit() {
    let text = collapse(
        &fs::read_to_string(root().join("src/durability/spell_item_transaction.rs"))
            .expect("source"),
    );
    let text = production(&text).to_owned();
    for (literal, minters) in [
        (
            "SpellItemScopeAuthority {",
            &["assert_spell_item_scope_with_recovery", "locked_tile_rows"][..],
        ),
        (
            "SpellItemAuthority {",
            &["assert_spell_item_authority_with_recovery"][..],
        ),
    ] {
        for at in find_all(&text, literal) {
            let before = &text[..at];
            if ["struct ", "impl ", "-> ", "-> &"]
                .iter()
                .any(|prefix| before.ends_with(prefix))
            {
                continue;
            }
            let (name, signature) = enclosing_fn(&text, at);
            assert!(
                minters.contains(&name.as_str()),
                "{literal} minted by {name}"
            );
            assert!(
                LANE_PROOFS.iter().any(|proof| signature.contains(proof)),
                "{name} mints {literal} without a lane-derived proof"
            );
        }
    }
}

/// Fix round 1 (#1907): every committing cast writer reserves its physical batch inside S and
/// turns a receipt-free verdict rejection into a proven rollback that releases its reservations
/// and keeps the rejection, instead of parking or retrying the attempt as unavailable.
#[test]
fn guarded_cast_writers_reserve_in_s_and_release_a_definite_rejection() {
    for (file, refusal, releases) in [
        (
            "src/gameplay_transport/native_combat_cast.rs",
            "Rejected(NATIVE_NEW_WRITE_REFUSED))=>{",
            &[
                "release_definitely_uncommitted_spell_batch(&installation.physical)",
                ".release_definitely_uncommitted(presentation)",
            ][..],
        ),
        (
            "src/gameplay_transport/world_item_cast.rs",
            "Rejected(WORLD_ITEM_NEW_GRANT_REFUSED))=>{",
            &[
                "release_definitely_uncommitted_spell_batch(&attempt.physical)",
                ".release_definitely_uncommitted(&attempt.presentation)",
            ][..],
        ),
        (
            "src/gameplay_transport/parameter_cast.rs",
            "Rejected(PARAMETER_NEW_GRANT_REFUSED))=>{",
            &["release_definitely_uncommitted_spell_batch(&installation.physical)"][..],
        ),
    ] {
        let raw = fs::read_to_string(root().join(file)).expect("source");
        // The whole file: `native_combat_cast.rs` keeps a test module above its writer.
        let text = collapse(&raw).replace(' ', "");
        let refusal = refusal.replace(' ', "");
        let release_guards = text.find("*guards=None;").expect("guards released after S");
        let span = &text[..release_guards];
        assert!(
            span.contains("reserve_spell_batch(") || file.ends_with("native_combat_cast.rs"),
            "{file} reserves its physical batch inside S"
        );
        let at = text
            .find(&refusal)
            .unwrap_or_else(|| panic!("{file} matches its refusal"));
        assert!(at > release_guards, "{file} matches the refusal after S");
        let arm = &text[at..at + text[at..].find("Err(_)=>return").expect("arm end")];
        let rollback = arm
            .find("tx.rollback().await")
            .expect("rollback proves no COMMIT");
        for release in releases {
            let release = release.replace(' ', "");
            let found = arm
                .find(&release)
                .unwrap_or_else(|| panic!("{file} runs {release}"));
            assert!(found > rollback, "{file} releases only after the rollback");
        }
        assert!(arm.contains("*pending=None;"), "{file} drops the attempt");
        assert!(
            arm.contains("SpellCastDisposition::Rejected"),
            "{file} keeps the rejection"
        );
        assert!(
            !arm.contains("open_commit_window"),
            "{file} never parks a rejection"
        );
    }
}
