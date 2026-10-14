//! BUNDLE-GATES-1: the inventory of every `qualified_room` gate of the gameplay seam.
//!
//! `node/serve.rs` hands the seam `qualified_room: None` on a bundle World and `combat_room`, the
//! entry room's bundle view (BUNDLE-COMBAT-ENABLE-1, #2025). A seam read of `combat_room` is a
//! gate open on a bundle World; a read of `qualified_room` is a gate closed there. Every read in
//! this module's sources is listed below: an open gate names the merged PR that opened it, a closed
//! gate names the plan batch that opens it. A read that is added, removed or switched without an
//! update of this table fails `every_seam_room_read_is_inventoried`.

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
}

struct Read {
    gate: &'static str,
    file: &'static str,
    field: &'static str,
    count: usize,
    state: Gate,
}

const fn read(
    gate: &'static str,
    file: &'static str,
    field: &'static str,
    count: usize,
    state: Gate,
) -> Read {
    Read {
        gate,
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
        "mod.rs",
        "self.combat_room",
        1,
        COMBAT_ENABLE,
    ),
    read(
        "auto-attack drain",
        "attack.rs",
        "self.combat_room",
        1,
        COMBAT_ENABLE,
    ),
    read(
        "native combat cast",
        "native_combat_cast.rs",
        "self.combat_room",
        1,
        COMBAT_ENABLE,
    ),
    read(
        "native combat cast commit",
        "native_combat_cast.rs",
        "owner.combat_room",
        1,
        COMBAT_ENABLE,
    ),
    read(
        "ordinary combat timer continuation",
        "spell_timer_callbacks.rs",
        "owner.combat_room",
        1,
        COMBAT_ENABLE,
    ),
    read(
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
        "parameter_cast.rs",
        "self.qualified_room",
        1,
        Gate::Closed {
            owners: &["MAGIC-SUMMON-1"],
        },
    ),
    read(
        "parameter cast commit: house access, vertical move",
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
        "world_item_cast.rs",
        "self.qualified_room",
        1,
        Gate::Closed {
            owners: &["MAGIC-SUMMON-1", "MAGIC-ITEMS-1"],
        },
    ),
    read(
        "world item cast commit",
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
        "field_step_ingress.rs",
        "self.qualified_room",
        1,
        Gate::Closed {
            owners: &["LOOP-WORLD-1"],
        },
    ),
    read(
        "field step ingress commit",
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
        "mod.rs",
        "self.qualified_room",
        3,
        Gate::SourceWorldBranch,
    ),
    read(
        "seam owners",
        "mod.rs",
        "owners.qualified_room",
        1,
        Gate::Plumbing,
    ),
    read(
        "seam owners",
        "mod.rs",
        "owners.combat_room",
        1,
        Gate::Plumbing,
    ),
];

const FIELDS: &[&str] = &[
    "self.qualified_room",
    "owner.qualified_room",
    "owners.qualified_room",
    "self.combat_room",
    "owner.combat_room",
    "owners.combat_room",
];

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn compact(text: &str) -> String {
    text.split_whitespace().collect()
}

#[allow(clippy::expect_used)]
fn module_reads() -> BTreeMap<(String, &'static str), usize> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/gameplay_transport");
    let mut reads = BTreeMap::new();
    for entry in std::fs::read_dir(&dir).expect("gameplay_transport sources") {
        let path = entry.expect("source entry").path();
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .expect("source name")
            .to_owned();
        if !name.ends_with(".rs") || name.ends_with("_tests.rs") {
            continue;
        }
        let source = compact(&std::fs::read_to_string(&path).expect("source text"));
        for field in FIELDS {
            let count = source.matches(field).count();
            if count > 0 {
                reads.insert((name.clone(), *field), count);
            }
        }
    }
    reads
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
            .entry((read.file.to_owned(), read.field))
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
    for read in INVENTORY {
        match read.state {
            Gate::Open { batch, pr } => {
                assert!(
                    row.contains(&format!("#{pr}")) && row.contains(batch),
                    "the plan row does not record that {batch} #{pr} opened `{}`",
                    read.gate
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
                    assert!(
                        row.contains(owner),
                        "the plan row does not name `{owner}` for `{}`",
                        read.gate
                    );
                }
            }
            Gate::SourceWorldBranch | Gate::Plumbing => {}
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
        compact(&serve).contains("qualified_room:bundle.is_none().then_some(&qualified_room),"),
        "serve.rs no longer withholds `qualified_room` from a bundle World: every closed gate \
         would open at once without its owner"
    );
}

#[test]
#[allow(clippy::expect_used)]
fn parameter_room_opens_only_the_locate_message() {
    let source = compact(
        &std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("src/gameplay_transport/parameter_cast.rs"),
        )
        .expect("parameter_cast.rs"),
    );
    assert!(
        source.contains("matchkey{\"locate_message\"=>combat,_=>qualified,}"),
        "`parameter_room` changed which parameter keys read the combat room: record the opened \
         keys in INVENTORY"
    );
}
