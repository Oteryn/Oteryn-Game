//! BUNDLE-GATES-1: the inventory of every `qualified_room` gate of the gameplay seam.
//!
//! `node/serve.rs` hands the seam `qualified_room: None` on a bundle World and `combat_room`, the
//! entry room's bundle view (BUNDLE-COMBAT-ENABLE-1, #2025). A seam read of `combat_room` is a
//! gate open on a bundle World; a read of `qualified_room` is a gate closed there. Every read in
//! this module's sources is listed below: an open gate names the merged PR that opened it, a closed
//! gate names the plan batch that opens it. Every mention of either field in the module's code is
//! counted, whatever its receiver (`x.combat_room`, `(*self).combat_room`) or form (a declaration,
//! initializer or destructuring pattern); one that is added, removed or switched without an update
//! of this table fails `every_seam_room_read_is_inventoried`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const PLAN: &str = "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md";

enum Gate {
    /// Open on a bundle World: reads `combat_room`, opened by a merged PR.
    Open { batch: &'static str, pr: u32 },
    /// Closed on a bundle World: reads `qualified_room`; each owner is a plan batch.
    Closed { owners: &'static [&'static str] },
    /// Reads `qualified_room` only to select the fixture source World's path; a bundle World is
    /// served by its own bundle branch at the same site, so nothing is closed there.
    SourceWorldBranch,
    /// Moves the two fields from `GameplaySeamOwners` into the composed owner.
    Plumbing,
    /// Declares or sets the field without reading it: a struct field or an initializer.
    Declaration,
}

struct Read {
    gate: &'static str,
    /// The gate's phrase in the BUNDLE-GATES-1 plan row; empty for plumbing.
    plan: &'static str,
    file: &'static str,
    /// The function that reads the field; `<item>` outside any function.
    function: &'static str,
    field: &'static str,
    count: usize,
    state: Gate,
}

const fn read(
    gate: &'static str,
    plan: &'static str,
    file: &'static str,
    function: &'static str,
    field: &'static str,
    count: usize,
    state: Gate,
) -> Read {
    Read {
        gate,
        plan,
        file,
        function,
        field,
        count,
        state,
    }
}

const COMBAT_ENABLE: Gate = Gate::Open {
    batch: "BUNDLE-COMBAT-ENABLE-1",
    pr: 2025,
};

const INVENTORY: &[Read] = &[
    read(
        "attack target",
        "player attack target",
        "mod.rs",
        "attack_target",
        "self.combat_room",
        1,
        COMBAT_ENABLE,
    ),
    read(
        "auto-attack drain",
        "auto-attack drain",
        "attack.rs",
        "drain_auto_attacks",
        "self.combat_room",
        1,
        COMBAT_ENABLE,
    ),
    read(
        "native combat cast",
        "native combat cast",
        "native_combat_cast.rs",
        "cast_native_combat_inner",
        "self.combat_room",
        1,
        COMBAT_ENABLE,
    ),
    read(
        "native combat cast commit",
        "native combat cast",
        "native_combat_cast.rs",
        "cast_native_combat_inner",
        "owner.combat_room",
        1,
        COMBAT_ENABLE,
    ),
    read(
        "ordinary combat timer continuation",
        "timer continuation",
        "spell_timer_callbacks.rs",
        "drain_native_ai_timers",
        "owner.combat_room",
        1,
        COMBAT_ENABLE,
    ),
    read(
        "monster AI",
        "monster AI",
        "monster_ai_cycle.rs",
        "drain_monster_melee",
        "self.combat_room",
        1,
        Gate::Open {
            batch: "LOOP-MONSTER-1",
            pr: 2036,
        },
    ),
    read(
        "periodic player damage",
        "periodic player damage",
        "spell_periodic.rs",
        "tick_spell_periodic",
        "self.combat_room",
        1,
        Gate::Open {
            batch: "MAGIC-COND-1",
            pr: 2086,
        },
    ),
    // `parameter_room`: `locate_message` reads the combat room, every other key the qualified room
    // (checked by `parameter_room_opens_only_the_locate_message`).
    read(
        "parameter cast: locate message",
        "`locate_message` parameter cast",
        "parameter_cast.rs",
        "cast_parameters_inner",
        "self.combat_room",
        1,
        Gate::Open {
            batch: "MAGIC-CAST-1",
            pr: 2112,
        },
    ),
    read(
        "parameter cast commit: locate message",
        "`locate_message` parameter cast",
        "parameter_cast.rs",
        "cast_parameters_inner",
        "owner.combat_room",
        1,
        Gate::Open {
            batch: "MAGIC-CAST-1",
            pr: 2112,
        },
    ),
    // `house_access` and `vertical_move`: SPELL-RT-WORLD-1 of the MAGIC-SUMMON-1 graph.
    read(
        "parameter cast: house access, vertical move",
        "`house_access` and `vertical_move` parameter casts",
        "parameter_cast.rs",
        "cast_parameters_inner",
        "self.qualified_room",
        1,
        Gate::Closed {
            owners: &["MAGIC-SUMMON-1"],
        },
    ),
    read(
        "parameter cast commit: house access, vertical move",
        "`house_access` and `vertical_move` parameter casts",
        "parameter_cast.rs",
        "cast_parameters_inner",
        "owner.qualified_room",
        1,
        Gate::Closed {
            owners: &["MAGIC-SUMMON-1"],
        },
    ),
    // `random_item_grant`: SPELL-RT-WORLD-1 (MAGIC-SUMMON-1 graph); `conjure`: MAGIC-ITEMS-1.
    read(
        "world item cast",
        "world item casts",
        "world_item_cast.rs",
        "cast_world_items_inner",
        "self.qualified_room",
        1,
        Gate::Closed {
            owners: &["MAGIC-SUMMON-1", "MAGIC-ITEMS-1"],
        },
    ),
    read(
        "world item cast commit",
        "world item casts",
        "world_item_cast.rs",
        "cast_world_items_inner",
        "owner.qualified_room",
        1,
        Gate::Closed {
            owners: &["MAGIC-SUMMON-1", "MAGIC-ITEMS-1"],
        },
    ),
    // Field ingress is the WORLDINT-ADMIT-1 child of LOOP-WORLD-1 (plan row MAGIC-SUMMON-1).
    read(
        "field step ingress",
        "field step ingress",
        "field_step_ingress.rs",
        "step_with_field_ingress",
        "self.qualified_room",
        1,
        Gate::Closed {
            owners: &["LOOP-WORLD-1"],
        },
    ),
    read(
        "field step ingress commit",
        "field step ingress",
        "field_step_ingress.rs",
        "step_with_field_ingress",
        "owner.qualified_room",
        1,
        Gate::Closed {
            owners: &["LOOP-WORLD-1"],
        },
    ),
    // The durable custody of the map's Items; on a bundle World that is its Ground persistence.
    read(
        "source map initialization",
        "source map initialization",
        "source_item_cycle.rs",
        "ensure_source_map_initialized",
        "self.qualified_room",
        1,
        Gate::Closed {
            owners: &["MAP-CUTOVER-1c"],
        },
    ),
    // The door overlay, step blocking and `use_object` serve a bundle World from `bundle_map`.
    read(
        "source World door overlay",
        "door overlay, step blocking, USE",
        "mod.rs",
        "observe_world_object_overlay",
        "self.qualified_room",
        1,
        Gate::SourceWorldBranch,
    ),
    read(
        "source World step blocking",
        "door overlay, step blocking, USE",
        "mod.rs",
        "paced_step",
        "self.qualified_room",
        1,
        Gate::SourceWorldBranch,
    ),
    read(
        "source World use",
        "door overlay, step blocking, USE",
        "mod.rs",
        "use_object",
        "self.qualified_room",
        1,
        Gate::SourceWorldBranch,
    ),
    read(
        "seam owners",
        "",
        "mod.rs",
        "serve_gameplay",
        "owners.qualified_room",
        1,
        Gate::Plumbing,
    ),
    read(
        "seam owners",
        "",
        "mod.rs",
        "serve_gameplay",
        "owners.combat_room",
        1,
        Gate::Plumbing,
    ),
    // The two fields of `GameplaySeamOwners` and of the composed owner, and the owner initializer.
    read(
        "seam field declarations",
        "",
        "mod.rs",
        "<item>",
        "qualified_room",
        2,
        Gate::Declaration,
    ),
    read(
        "seam owner initializer",
        "",
        "mod.rs",
        "serve_gameplay",
        "qualified_room",
        1,
        Gate::Declaration,
    ),
    read(
        "seam field declarations",
        "",
        "mod.rs",
        "<item>",
        "combat_room",
        2,
        Gate::Declaration,
    ),
    read(
        "seam owner initializer",
        "",
        "mod.rs",
        "serve_gameplay",
        "combat_room",
        1,
        Gate::Declaration,
    ),
];

const FIELDS: &[&str] = &["qualified_room", "combat_room"];

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn compact(text: &str) -> String {
    text.split_whitespace().collect()
}

/// The source without comments, so only code is counted. A string literal (raw too) becomes
/// `"<hex of its text>"`: its value can still be matched, but text inside it is no code. A char
/// literal becomes `' '`; a lifetime is kept as written.
fn code_only(source: &str) -> String {
    let chars: Vec<char> = source.chars().collect();
    let mut code = String::with_capacity(source.len());
    let at = |index: usize| chars.get(index).copied();
    let literal = |text: &[char]| format!("\"{}\"", hex(&text.iter().collect::<String>()));
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '/' if at(i + 1) == Some('/') => {
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
            }
            '/' if at(i + 1) == Some('*') => {
                let mut depth = 0usize;
                while i < chars.len() {
                    if chars[i] == '/' && at(i + 1) == Some('*') {
                        depth += 1;
                        i += 2;
                    } else if chars[i] == '*' && at(i + 1) == Some('/') {
                        depth -= 1;
                        i += 2;
                        if depth == 0 {
                            break;
                        }
                    } else {
                        i += 1;
                    }
                }
                code.push(' ');
            }
            'r' if (at(i + 1) == Some('"') || at(i + 1) == Some('#'))
                && !chars[..i]
                    .last()
                    .is_some_and(|c| c.is_alphanumeric() || *c == '_') =>
            {
                let hashes = chars[i + 1..].iter().take_while(|c| **c == '#').count();
                if at(i + 1 + hashes) != Some('"') {
                    code.push('r');
                    i += 1;
                    continue;
                }
                i += 2 + hashes;
                let start = i;
                while i < chars.len()
                    && !(chars[i] == '"'
                        && chars[i + 1..].iter().take(hashes).all(|c| *c == '#')
                        && chars.len() >= i + 1 + hashes)
                {
                    i += 1;
                }
                code.push_str(&literal(&chars[start..i.min(chars.len())]));
                i += 1 + hashes;
            }
            '"' => {
                i += 1;
                let start = i;
                while i < chars.len() && chars[i] != '"' {
                    i += if chars[i] == '\\' { 2 } else { 1 };
                }
                code.push_str(&literal(&chars[start..i.min(chars.len())]));
                i += 1;
            }
            '\'' if at(i + 1) == Some('\\') => {
                i += 2;
                while i < chars.len() && chars[i] != '\'' {
                    i += 1;
                }
                i += 1;
                code.push_str("' '");
            }
            '\'' if at(i + 2) == Some('\'') => {
                i += 3;
                code.push_str("' '");
            }
            c => {
                code.push(c);
                i += 1;
            }
        }
    }
    code
}

fn hex(text: &str) -> String {
    text.bytes().map(|byte| format!("{byte:02x}")).collect()
}

/// The source as compacted code: no comments or whitespace, string literals as hex.
fn code(source: &str) -> String {
    compact(&code_only(source))
}

/// Every seam room mention of the module tree's production code, keyed by file, enclosing
/// function and mention. Test code is left out: a module declared `#[cfg(test)] mod x;` (with or
/// without `#[path]`), a `*_tests.rs` file, and an inline `#[cfg(test)]` module or function.
#[allow(clippy::expect_used)]
fn module_reads() -> BTreeMap<(String, String, String), usize> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/gameplay_transport");
    // The whole module tree: a gate moved into a nested module is still counted.
    let mut pending = vec![dir.clone()];
    let mut sources = BTreeMap::new();
    while let Some(next) = pending.pop() {
        for entry in std::fs::read_dir(&next).expect("gameplay_transport sources") {
            let path = entry.expect("source entry").path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                let source = std::fs::read_to_string(&path).expect("source text");
                sources.insert(path, without_test_items(&code_only(&source)));
            }
        }
    }
    let mut test_files = Vec::new();
    for (path, (_, declared)) in &sources {
        let parent = path.parent().expect("source directory");
        let children = if path.file_name().is_some_and(|name| name == "mod.rs") {
            parent.to_path_buf()
        } else {
            parent.join(path.file_stem().expect("source stem"))
        };
        for (module, file) in declared {
            match file {
                Some(file) => test_files.push(parent.join(file)),
                None => {
                    test_files.push(children.join(format!("{module}.rs")));
                    test_files.push(children.join(module).join("mod.rs"));
                }
            }
        }
    }
    let test_files: Vec<PathBuf> = test_files
        .iter()
        .filter_map(|file| file.canonicalize().ok())
        .collect();
    let mut reads = BTreeMap::new();
    for (path, (code, _)) in &sources {
        // A test module is declared `#[cfg(test)]`, or named `*_tests.rs` when it is declared
        // inside another test module.
        let test = path
            .to_str()
            .is_some_and(|path| path.ends_with("_tests.rs"))
            || test_files.contains(&path.canonicalize().expect("source path"));
        if test {
            continue;
        }
        let name = path
            .strip_prefix(&dir)
            .ok()
            .and_then(|name| name.to_str())
            .expect("source name")
            .replace('\\', "/");
        for field in FIELDS {
            for (function, mention) in mentions(code, field) {
                *reads.entry((name.clone(), function, mention)).or_insert(0) += 1;
            }
        }
    }
    reads
}

/// The code without its `#[cfg(test)]` modules and functions, and the test modules it declares
/// out of line (`#[cfg(test)] mod x;`), each with its `#[path]` when it has one. `code` is
/// `code_only` text, so every brace is code.
fn without_test_items(code: &str) -> (String, Vec<(String, Option<String>)>) {
    const TEST: &str = "#[cfg(test)]";
    let word = |c: char| c.is_alphanumeric() || c == '_';
    let mut kept = String::with_capacity(code.len());
    let mut declared = Vec::new();
    let mut rest = code;
    while let Some(at) = rest.find(TEST) {
        kept.push_str(&rest[..at]);
        let mut item = rest[at + TEST.len()..].trim_start();
        let mut file = None;
        while item.starts_with("#[") {
            let close = item.find(']').map_or(item.len(), |close| close + 1);
            let attribute = compact(&item[..close]);
            if let Some(path) = attribute
                .strip_prefix("#[path=\"")
                .and_then(|path| path.strip_suffix("\"]"))
            {
                file = Some(unhex(path));
            }
            item = item[close..].trim_start();
        }
        let head = item.trim_start_matches("pub(crate)").trim_start();
        let head = head.trim_start_matches("pub(super)").trim_start();
        let head = head.trim_start_matches("pub").trim_start();
        let head = head.trim_start_matches("async").trim_start();
        let module = head.strip_prefix("mod ").map(str::trim_start);
        let body = item.find(['{', ';']);
        match (module, body) {
            (Some(module), Some(end)) if item[end..].starts_with(';') => {
                let name: String = module.chars().take_while(|c| word(*c)).collect();
                declared.push((name, file));
                rest = &item[end + 1..];
            }
            (Some(_), Some(open)) => rest = &item[block_end(item, open)..],
            (None, Some(open)) if head.starts_with("fn ") && item[open..].starts_with('{') => {
                rest = &item[block_end(item, open)..];
            }
            _ => {
                // An attribute on a statement or a field: kept.
                kept.push_str(TEST);
                rest = &rest[at + TEST.len()..];
            }
        }
    }
    kept.push_str(rest);
    (kept, declared)
}

fn unhex(text: &str) -> String {
    let bytes: Vec<u8> = (0..text.len() / 2)
        .filter_map(|index| u8::from_str_radix(&text[index * 2..index * 2 + 2], 16).ok())
        .collect();
    String::from_utf8_lossy(&bytes).into_owned()
}

/// The index just after the brace that closes the block opened at `open`.
fn block_end(code: &str, open: usize) -> usize {
    let mut depth = 0usize;
    for (index, c) in code[open..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return open + index + 1;
                }
            }
            _ => {}
        }
    }
    code.len()
}

/// Each whole-word mention of `field` in code with its enclosing function (`<item>` outside any
/// function): `<receiver>.<field>` for a field access (`<expr>` when the receiver is not a plain
/// name), the bare `<field>` for any other form. A method call `.<field>(` is another item's
/// method, not the seam field.
fn mentions(source: &str, field: &str) -> Vec<(String, String)> {
    let word = |c: char| c.is_alphanumeric() || c == '_';
    let mut found = Vec::new();
    for (at, _) in source.match_indices(field) {
        let before = &source[..at];
        let after = &source[at + field.len()..];
        if before.ends_with(word) || after.starts_with(word) {
            continue;
        }
        let function = enclosing_function(before);
        let Some(receiver) = before.trim_end().strip_suffix('.') else {
            found.push((function, field.to_owned()));
            continue;
        };
        if after.trim_start().starts_with('(') {
            continue;
        }
        let receiver = receiver.trim_end();
        let name = &receiver[receiver.trim_end_matches(word).len()..];
        let name = if name.is_empty() { "<expr>" } else { name };
        found.push((function, format!("{name}.{field}")));
    }
    found
}

/// The innermost function whose body is open at the end of `code`.
fn enclosing_function(code: &str) -> String {
    let word = |c: char| c.is_alphanumeric() || c == '_';
    let mut open: Vec<Option<String>> = Vec::new();
    let mut signature: Option<String> = None;
    let mut nesting = 0usize;
    let mut previous = ' ';
    let mut chars = code.char_indices().peekable();
    while let Some((index, c)) = chars.next() {
        match c {
            'f' if !word(previous)
                && code[index..].starts_with("fn")
                && code[index + 2..].starts_with(char::is_whitespace) =>
            {
                let name: String = code[index + 2..]
                    .trim_start()
                    .chars()
                    .take_while(|c| word(*c))
                    .collect();
                signature = Some(name);
                chars.next();
            }
            '(' | '[' => nesting += 1,
            ')' | ']' => nesting = nesting.saturating_sub(1),
            ';' if nesting == 0 => signature = None,
            '{' => {
                let inherited = open.last().cloned().flatten();
                open.push(signature.take().or(inherited));
            }
            '}' => {
                open.pop();
            }
            _ => {}
        }
        previous = c;
    }
    open.last()
        .cloned()
        .flatten()
        .unwrap_or_else(|| "<item>".to_owned())
}

#[allow(clippy::expect_used)]
fn plan() -> String {
    std::fs::read_to_string(repository_root().join(PLAN)).expect("completion plan")
}

#[test]
fn every_seam_room_read_is_inventoried() {
    let mut expected = BTreeMap::new();
    for read in INVENTORY {
        *expected
            .entry((
                read.file.to_owned(),
                read.function.to_owned(),
                read.field.to_owned(),
            ))
            .or_insert(0) += read.count;
    }
    assert_eq!(
        module_reads(),
        expected,
        "a `qualified_room` or `combat_room` read of the seam changed: record it in INVENTORY \
         (an opened gate names its batch and merged PR, a closed gate its owning batch)"
    );
}

#[test]
fn each_gate_reads_the_room_its_state_says() {
    for read in INVENTORY {
        let combat = read.field.ends_with(".combat_room");
        match read.state {
            Gate::Open { batch, pr } => {
                assert!(combat, "open gate `{}` must read combat_room", read.gate);
                assert!(!batch.is_empty() && pr > 0, "open gate `{}`", read.gate);
            }
            Gate::Closed { owners } => {
                assert!(!combat, "closed gate `{}` reads combat_room", read.gate);
                assert!(
                    !owners.is_empty(),
                    "closed gate `{}` has no owner",
                    read.gate
                );
            }
            Gate::SourceWorldBranch => assert!(!combat, "`{}`", read.gate),
            Gate::Plumbing => assert!(read.field.starts_with("owners."), "`{}`", read.gate),
            Gate::Declaration => assert!(!read.field.contains('.'), "`{}`", read.gate),
        }
    }
}

#[test]
#[allow(clippy::expect_used)]
fn every_gate_is_recorded_in_the_plan_row() {
    let plan = plan();
    let row = plan
        .lines()
        .find(|line| line.starts_with("| BUNDLE-GATES-1 |"))
        .expect("BUNDLE-GATES-1 plan row");
    // Each gate is checked in its own clause of the row: its phrase together with its opener
    // or owners, so one gate cannot drop out behind another that shares them.
    let clauses: Vec<&str> = row
        .split([';', '|'])
        .flat_map(|part| part.split(". "))
        .collect();
    let clause = |read: &Read, names: &[String]| {
        clauses.iter().any(|clause| {
            clause.contains(read.plan) && names.iter().all(|name| clause.contains(name.as_str()))
        })
    };
    for read in INVENTORY {
        match read.state {
            Gate::Open { batch, pr } => {
                assert!(
                    clause(read, &[batch.to_owned(), format!("#{pr}")]),
                    "the plan row has no clause recording that {batch} #{pr} opened `{}` ({})",
                    read.gate,
                    read.plan
                );
            }
            Gate::Closed { owners } => {
                for owner in owners {
                    assert!(
                        plan.lines()
                            .any(|line| line.starts_with(&format!("| {owner} |"))),
                        "closed gate `{}` names `{owner}`, which is not a plan batch",
                        read.gate
                    );
                }
                let names: Vec<String> = owners.iter().map(|owner| (*owner).to_owned()).collect();
                assert!(
                    clause(read, &names),
                    "the plan row has no clause naming {owners:?} for `{}` ({})",
                    read.gate,
                    read.plan
                );
            }
            Gate::SourceWorldBranch => assert!(
                clause(read, &[]),
                "the plan row does not record the source-World branch `{}`",
                read.gate
            ),
            Gate::Plumbing | Gate::Declaration => {}
        }
    }
}

#[test]
#[allow(clippy::expect_used)]
fn a_bundle_world_seam_has_no_qualified_room() {
    let serve =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/node/serve.rs"))
            .expect("serve.rs");
    assert!(
        code(&serve).contains("qualified_room:bundle.is_none().then_some(&qualified_room),"),
        "serve.rs no longer withholds `qualified_room` from a bundle World: every closed gate \
         would open at once without its owner"
    );
}

#[test]
#[allow(clippy::expect_used)]
fn parameter_room_opens_only_the_locate_message() {
    let source = code(
        &std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("src/gameplay_transport/parameter_cast.rs"),
        )
        .expect("parameter_cast.rs"),
    );
    let arms = format!(
        "matchkey{{\"{}\"=>combat,_=>qualified,}}",
        hex("locate_message")
    );
    assert!(
        source.contains(&arms),
        "`parameter_room` changed which parameter keys read the combat room: record the opened \
         keys in INVENTORY"
    );
}
