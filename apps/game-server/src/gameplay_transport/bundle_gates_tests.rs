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
    field: &'static str,
    count: usize,
    state: Gate,
}

const fn read(
    gate: &'static str,
    plan: &'static str,
    file: &'static str,
    field: &'static str,
    count: usize,
    state: Gate,
) -> Read {
    Read {
        gate,
        plan,
        file,
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
        "self.combat_room",
        1,
        COMBAT_ENABLE,
    ),
    read(
        "auto-attack drain",
        "auto-attack drain",
        "attack.rs",
        "self.combat_room",
        1,
        COMBAT_ENABLE,
    ),
    read(
        "native combat cast",
        "native combat cast",
        "native_combat_cast.rs",
        "self.combat_room",
        1,
        COMBAT_ENABLE,
    ),
    read(
        "native combat cast commit",
        "native combat cast",
        "native_combat_cast.rs",
        "owner.combat_room",
        1,
        COMBAT_ENABLE,
    ),
    read(
        "ordinary combat timer continuation",
        "timer continuation",
        "spell_timer_callbacks.rs",
        "owner.combat_room",
        1,
        COMBAT_ENABLE,
    ),
    read(
        "monster AI",
        "monster AI",
        "monster_ai_cycle.rs",
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
        "self.qualified_room",
        1,
        Gate::Closed {
            owners: &["MAP-CUTOVER-1c"],
        },
    ),
    // The door overlay, step blocking and `use_object` serve a bundle World from `bundle_map`.
    read(
        "source World door overlay, step blocking and use",
        "door overlay, step blocking, USE",
        "mod.rs",
        "self.qualified_room",
        3,
        Gate::SourceWorldBranch,
    ),
    read(
        "seam owners",
        "",
        "mod.rs",
        "owners.qualified_room",
        1,
        Gate::Plumbing,
    ),
    read(
        "seam owners",
        "",
        "mod.rs",
        "owners.combat_room",
        1,
        Gate::Plumbing,
    ),
    // The two fields of `GameplaySeamOwners` and of the composed owner, and the owner initializer.
    read(
        "seam field declarations and initializer",
        "",
        "mod.rs",
        "qualified_room",
        3,
        Gate::Declaration,
    ),
    read(
        "seam field declarations and initializer",
        "",
        "mod.rs",
        "combat_room",
        3,
        Gate::Declaration,
    ),
    // The qualification harness seats its fixture World's room in both fields.
    read(
        "qualification harness seam",
        "",
        "qualification.rs",
        "qualified_room",
        1,
        Gate::Declaration,
    ),
    read(
        "qualification harness seam",
        "",
        "qualification.rs",
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

#[allow(clippy::expect_used)]
fn module_reads() -> BTreeMap<(String, String), usize> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/gameplay_transport");
    let mut reads = BTreeMap::new();
    // The whole module tree: a gate moved into a nested module is still counted.
    let mut pending = vec![dir.clone()];
    let mut sources = Vec::new();
    while let Some(next) = pending.pop() {
        for entry in std::fs::read_dir(&next).expect("gameplay_transport sources") {
            let path = entry.expect("source entry").path();
            if path.is_dir() {
                pending.push(path);
            } else {
                sources.push(path);
            }
        }
    }
    for path in sources {
        let name = path
            .strip_prefix(&dir)
            .ok()
            .and_then(|name| name.to_str())
            .expect("source name")
            .replace('\\', "/");
        if !name.ends_with(".rs") || name.ends_with("_tests.rs") {
            continue;
        }
        let source = code_only(&std::fs::read_to_string(&path).expect("source text"));
        for field in FIELDS {
            for mention in mentions(&source, field) {
                *reads.entry((name.clone(), mention)).or_insert(0) += 1;
            }
        }
    }
    reads
}

/// Each whole-word mention of `field` in code: `<receiver>.<field>` for a field access (`<expr>`
/// when the receiver is not a plain name), the bare `<field>` for any other form. A method call
/// `.<field>(` is another item's method, not the seam field.
fn mentions(source: &str, field: &str) -> Vec<String> {
    let word = |c: char| c.is_alphanumeric() || c == '_';
    let mut found = Vec::new();
    for (at, _) in source.match_indices(field) {
        let before = &source[..at];
        let after = &source[at + field.len()..];
        if before.ends_with(word) || after.starts_with(word) {
            continue;
        }
        let Some(receiver) = before.trim_end().strip_suffix('.') else {
            found.push(field.to_owned());
            continue;
        };
        if after.trim_start().starts_with('(') {
            continue;
        }
        let receiver = receiver.trim_end();
        let name = &receiver[receiver.trim_end_matches(word).len()..];
        let name = if name.is_empty() { "<expr>" } else { name };
        found.push(format!("{name}.{field}"));
    }
    found
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
            .entry((read.file.to_owned(), read.field.to_owned()))
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
