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

#[test]
fn familiar_history_marks_the_window_before_any_fallible_reconciliation() {
    let raw =
        fs::read_to_string(root().join("src/durability/character_familiar.rs")).expect("source");
    // The whole file: `character_familiar.rs` keeps a test module above its writer.
    let text = collapse(&raw).replace(' ', "");
    let start = text
        .find("asyncfncommit_familiar_spell_inner<")
        .expect("the familiar writer");
    let body = &text[start..];
    let pair = body
        .find("PendingFamiliarStateOutcome::AlreadyCommitted(value),")
        .expect("the historical pair arm");
    let mark = body
        .find("window.mark_already_committed();")
        .expect("history marks the window");
    assert!(
        mark > pair,
        "the mark follows the matched historical receipts"
    );
    for later in [
        "prepare_character_build_with_recovery(",
        "letsame_transaction:bool",
        "historical_outcome()",
        "commit_semantic_transaction(tx,deadline).await?;returnOk(Ok(FamiliarSpellCommit::Reconciled",
    ] {
        let at = body[pair..]
            .find(later)
            .unwrap_or_else(|| panic!("the familiar writer runs {later}"))
            + pair;
        assert!(mark < at, "the window is marked before {later}");
    }
}

/// Fix round 3 (#1907): an item outcome known to be historical opens and marks the commit
/// window before the parameter-result and training follow-ups, so their failure parks the
/// attempt and keeps the lane fenced instead of returning it to the marker.
#[test]
fn guarded_cast_writers_mark_history_before_any_fallible_follow_up() {
    for (file, outcome, follow_ups) in [
        (
            "src/gameplay_transport/native_combat_cast.rs",
            "letitems=matchitem_tx::apply_spell_items_in_transaction_guarded(",
            &[
                "write_parameter_result_in_transaction(",
                "prepare_character_build_in_transaction(",
                "reconcile_spell_owner_commit_in_transaction(",
            ][..],
        ),
        (
            "src/gameplay_transport/world_item_cast.rs",
            "letoutcome=matchitems::apply_spell_items_in_transaction_guarded(",
            &[
                "prepare_character_build_in_transaction(",
                "reconcile_spell_owner_commit_in_transaction(",
            ][..],
        ),
        (
            "src/gameplay_transport/parameter_cast.rs",
            "letoutcome=matchitems::apply_spell_items_in_transaction_guarded(",
            &[
                "write_parameter_result_in_transaction(",
                "prepare_character_build_in_transaction(",
                "reconcile_spell_owner_commit_in_transaction(",
            ][..],
        ),
    ] {
        let raw = fs::read_to_string(root().join(file)).expect("source");
        let text = collapse(&raw).replace(' ', "");
        let at = text
            .find(outcome)
            .unwrap_or_else(|| panic!("{file} applies its items"));
        let body = &text[at..];
        let mark = body
            .find("ifhistorical{window.mark_already_committed();}")
            .unwrap_or_else(|| panic!("{file} marks a historical outcome"));
        let open = body
            .find("open_commit_window(")
            .unwrap_or_else(|| panic!("{file} opens its window"));
        assert!(open < mark, "{file} marks the window it opened");
        for later in follow_ups {
            let found = body
                .find(later)
                .unwrap_or_else(|| panic!("{file} runs {later}"));
            assert!(mark < found, "{file} marks history before {later}");
        }
        assert_eq!(
            body.matches("window.mark_already_committed();").count(),
            1,
            "{file} marks history once"
        );
        assert!(
            body.contains("if!historical&&letOk(attempt)=window.reclaim_uncommitted()"),
            "{file} reclaims only a new write's follow-up failure"
        );
    }
}

#[test]
fn every_cast_writer_runs_its_pass_under_a_cancel_safe_guard() {
    for file in [
        "src/gameplay_transport/native_combat_cast.rs",
        "src/gameplay_transport/world_item_cast.rs",
        "src/gameplay_transport/parameter_cast.rs",
        "src/gameplay_transport/familiar_cast.rs",
    ] {
        let raw = fs::read_to_string(root().join(file)).expect("source");
        let text = collapse(&raw).replace(' ', "");
        let empty = text
            .find("attempt:None,")
            .unwrap_or_else(|| panic!("{file} pushes an empty marker"));
        let guard = text
            .find("SpellWriterPass::new(")
            .unwrap_or_else(|| panic!("{file} guards its writer pass"));
        assert!(
            empty < guard && !text[empty..guard].contains(".await"),
            "{file} guards the pass before any await after the marker"
        );
        let finishes = find_all(&text, "writer.finish();");
        assert!(!finishes.is_empty(), "{file} finishes its pass");
        for at in finishes {
            let rest = &text[at..];
            let settle = rest
                .find("PendingSpellMarker::settle(")
                .unwrap_or_else(|| panic!("{file} settles a finished pass"));
            assert!(
                !rest[..settle].contains(".await"),
                "{file} settles its marker with no await after finishing the pass"
            );
        }
    }
}

#[test]
fn no_cast_writer_awaits_after_its_window_gives_up_the_attempt() {
    for file in [
        "src/gameplay_transport/native_combat_cast.rs",
        "src/gameplay_transport/world_item_cast.rs",
        "src/gameplay_transport/parameter_cast.rs",
        "src/gameplay_transport/familiar_cast.rs",
    ] {
        let raw = fs::read_to_string(root().join(file)).expect("source");
        let text = collapse(&raw).replace(' ', "");
        let installs = find_all(&text, "window.install();");
        assert!(!installs.is_empty(), "{file} installs its attempt");
        for at in installs {
            let rest = &text[at..];
            let end = ["Ok(", "(SpellCastOutcome{"]
                .iter()
                .filter_map(|done| rest.find(done))
                .min()
                .unwrap_or_else(|| panic!("{file} returns after installing"));
            assert!(
                !rest[..end].contains(".await"),
                "{file} awaits nothing once the window gave up the attempt"
            );
        }
    }
    let raw = fs::read_to_string(root().join("src/gameplay_transport/native_combat_cast.rs"))
        .expect("source");
    let text = collapse(&raw).replace(' ', "");
    let lock = text
        .find("letattack=owner.attack.lock().await;")
        .expect("native takes the attack guard");
    let install = text
        .find("letmutattempt=window.install();")
        .expect("native installs");
    let record = text
        .find("owner.record_spell_kills_locked(&attack,&mutruntime,std::iter::once(&batch));")
        .expect("native records its kills under the guard");
    assert!(
        lock < install && install < record,
        "native takes the attack guard before the window gives up the attempt"
    );
}
