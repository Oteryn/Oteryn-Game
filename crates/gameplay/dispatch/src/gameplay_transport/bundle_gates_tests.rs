//! BUNDLE-GATES-1: the inventory of every gate of the gameplay seam's rooms.
//!
//! The seam's two rooms are the private fields of `SeamRooms` (`seam_rooms.rs`), so the compiler
//! holds every other module to its gate-named accessors: a room is read only by calling one. An
//! accessor returning `combat` is a gate open on a bundle World; one returning `qualified` is
//! closed there, since `node/serve.rs` hands a bundle World no qualified room. Each gate below
//! names the merged PR that opened it or the plan batch that opens it, and every call of its
//! accessor in the crate's sources. An accessor added, removed or switched, a call added, removed
//! or moved to another file, or `seam_rooms.rs` given any other item, form or attribute fails a
//! test of this module.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use syn::ext::IdentExt as _;
use syn::visit::{self, Visit};

const PLAN: &str = "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md";

enum Gate {
    /// Open on a bundle World: reads `combat`, opened by a merged PR.
    Open { batch: &'static str, pr: u32 },
    /// Closed on a bundle World: reads `qualified`; each owner is a plan batch.
    Closed { owners: &'static [&'static str] },
    /// Reads `qualified` only to select the fixture source World's path; a bundle World is served
    /// by its own bundle branch at the same site, so nothing is closed there.
    SourceWorldBranch,
}

struct Entry {
    gate: &'static str,
    /// The gate's phrase in the BUNDLE-GATES-1 plan row.
    plan: &'static str,
    /// The `SeamRooms` accessor.
    accessor: &'static str,
    /// The parameter key of a keyed accessor's arm (`_` for its default arm); empty otherwise.
    key: &'static str,
    state: Gate,
    /// Every call of the accessor: its file under `src/` and its number of calls there. A keyed
    /// accessor's calls are recorded on its first entry.
    calls: &'static [(&'static str, usize)],
}

const COMBAT_ENABLE: Gate = Gate::Open {
    batch: "BUNDLE-COMBAT-ENABLE-1",
    pr: 2025,
};

const INVENTORY: &[Entry] = &[
    Entry {
        gate: "attack target",
        plan: "player attack target",
        accessor: "gate_attack_target",
        key: "",
        state: COMBAT_ENABLE,
        calls: &[("gameplay_transport/mod.rs", 1)],
    },
    Entry {
        gate: "auto-attack drain",
        plan: "auto-attack drain",
        accessor: "gate_auto_attack",
        key: "",
        state: COMBAT_ENABLE,
        calls: &[("gameplay_transport/attack.rs", 1)],
    },
    Entry {
        gate: "native combat cast and its commit",
        plan: "native combat cast",
        accessor: "gate_native_combat_cast",
        key: "",
        state: COMBAT_ENABLE,
        calls: &[
            ("gameplay_transport/native_combat_cast.rs", 2),
            ("gameplay_transport/rune_use_bridge/revision.rs", 1),
        ],
    },
    Entry {
        gate: "ordinary combat timer continuation",
        plan: "timer continuation",
        accessor: "gate_combat_timer_continuation",
        key: "",
        state: COMBAT_ENABLE,
        calls: &[("gameplay_transport/spell_timer_callbacks.rs", 1)],
    },
    Entry {
        gate: "chase step",
        plan: "chase step",
        accessor: "gate_chase_step",
        key: "",
        state: Gate::Open {
            batch: "LOOP-COMBAT-MODES-1",
            pr: 2113,
        },
        calls: &[("gameplay_transport/attack.rs", 1)],
    },
    Entry {
        gate: "monster AI",
        plan: "monster AI",
        accessor: "gate_monster_ai",
        key: "",
        state: Gate::Open {
            batch: "LOOP-MONSTER-1",
            pr: 2036,
        },
        calls: &[("gameplay_transport/monster_ai_cycle.rs", 1)],
    },
    Entry {
        gate: "periodic player damage",
        plan: "periodic player damage",
        accessor: "gate_periodic_damage",
        key: "",
        state: Gate::Open {
            batch: "MAGIC-COND-1",
            pr: 2086,
        },
        calls: &[("gameplay_transport/spell_periodic.rs", 1)],
    },
    Entry {
        gate: "parameter cast and its commit: locate message",
        plan: "`locate_message` parameter cast",
        accessor: "gate_parameter_cast",
        key: "locate_message",
        state: Gate::Open {
            batch: "MAGIC-CAST-1",
            pr: 2112,
        },
        // The cast and its commit, and the three of `bundle_world_opens_locate_message_only`.
        calls: &[("gameplay_transport/parameter_cast.rs", 5)],
    },
    // `house_access` and `vertical_move`: SPELL-RT-WORLD-1 of the MAGIC-SUMMON-1 graph.
    Entry {
        gate: "parameter cast and its commit: house access, vertical move",
        plan: "`house_access` and `vertical_move` parameter casts",
        accessor: "gate_parameter_cast",
        key: "_",
        state: Gate::Closed {
            owners: &["MAGIC-SUMMON-1"],
        },
        calls: &[],
    },
    // `random_item_grant`: SPELL-RT-WORLD-1 (MAGIC-SUMMON-1 graph); `conjure`: MAGIC-ITEMS-1.
    Entry {
        gate: "world item cast and its commit",
        plan: "world item casts",
        accessor: "gate_world_item_cast",
        key: "",
        state: Gate::Closed {
            owners: &["MAGIC-SUMMON-1", "MAGIC-ITEMS-1"],
        },
        calls: &[("gameplay_transport/world_item_cast.rs", 2)],
    },
    // Field ingress is the WORLDINT-ADMIT-1 child of LOOP-WORLD-1 (plan row MAGIC-SUMMON-1).
    Entry {
        gate: "field step ingress and its commit",
        plan: "field step ingress",
        accessor: "gate_field_step_ingress",
        key: "",
        state: Gate::Closed {
            owners: &["LOOP-WORLD-1"],
        },
        calls: &[("gameplay_transport/field_step_ingress.rs", 2)],
    },
    // The durable custody of the map's Items; on a bundle World that is its Ground persistence.
    Entry {
        gate: "source map initialization",
        plan: "source map initialization",
        accessor: "gate_source_map_initialization",
        key: "",
        state: Gate::Closed {
            owners: &["MAP-CUTOVER-1c"],
        },
        calls: &[("gameplay_transport/source_item_cycle.rs", 1)],
    },
    // The door overlay, step blocking, `use_object` and the chase step's door blocking serve a
    // bundle World from `bundle_map`.
    Entry {
        gate: "source World door overlay, step blocking, USE, chase step blocking",
        plan: "door overlay, step blocking, USE",
        accessor: "gate_source_world",
        key: "",
        state: Gate::SourceWorldBranch,
        calls: &[
            ("gameplay_transport/attack.rs", 1),
            ("gameplay_transport/mod.rs", 2),
            ("gameplay_transport/paced_movement.rs", 1),
        ],
    },
];

fn repository_root() -> PathBuf {
    Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../apps/game-server"
    ))
    .join("../..")
}

#[allow(clippy::expect_used)]
fn parse_source(relative: &str) -> syn::File {
    let package = if relative.starts_with("src/gameplay_transport/") {
        Path::new(env!("CARGO_MANIFEST_DIR"))
    } else {
        Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../apps/game-server"
        ))
    };
    let path = package.join(relative);
    let source = std::fs::read_to_string(&path).expect("production source");
    syn::parse_file(&source).expect("production source parses")
}

#[allow(clippy::expect_used)]
fn plan() -> String {
    std::fs::read_to_string(repository_root().join(PLAN)).expect("completion plan")
}

/// The room an accessor's arm returns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Room {
    Qualified,
    Combat,
}

/// `self.qualified` or `self.combat`.
fn field_room(expr: &syn::Expr) -> Option<Room> {
    let syn::Expr::Field(field) = expr else {
        return None;
    };
    let syn::Member::Named(name) = &field.member else {
        return None;
    };
    let receiver = matches!(&*field.base, syn::Expr::Path(path) if path.path.is_ident("self"));
    match name.to_string().as_str() {
        "qualified" if receiver && field.attrs.is_empty() => Some(Room::Qualified),
        "combat" if receiver && field.attrs.is_empty() => Some(Room::Combat),
        _ => None,
    }
}

/// An accessor's arms, each key with the room it returns: `[("", room)]` for a body that is one
/// field read, or the literal keys and a closing `_` of a `match key { … }` of field reads.
fn accessor_arms(function: &syn::ImplItemFn) -> Option<Vec<(String, Room)>> {
    let [syn::Stmt::Expr(body, None)] = function.block.stmts.as_slice() else {
        return None;
    };
    if let Some(room) = field_room(body) {
        return Some(vec![(String::new(), room)]);
    }
    let syn::Expr::Match(choice) = body else {
        return None;
    };
    let keyed = matches!(&*choice.expr, syn::Expr::Path(path) if path.path.is_ident("key"));
    let (default, keys) = choice.arms.split_last()?;
    if !keyed || !choice.attrs.is_empty() || !matches!(default.pat, syn::Pat::Wild(_)) {
        return None;
    }
    let mut arms = Vec::new();
    for arm in keys {
        let syn::Pat::Lit(syn::ExprLit {
            lit: syn::Lit::Str(key),
            attrs,
        }) = &arm.pat
        else {
            return None;
        };
        if !attrs.is_empty() || !arm.attrs.is_empty() || arm.guard.is_some() {
            return None;
        }
        arms.push((key.value(), field_room(&arm.body)?));
    }
    if !default.attrs.is_empty() || default.guard.is_some() {
        return None;
    }
    arms.push(("_".to_owned(), field_room(&default.body)?));
    Some(arms)
}

/// `Self { qualified, combat }`.
fn sets_both_rooms(function: &syn::ImplItemFn) -> bool {
    let [syn::Stmt::Expr(syn::Expr::Struct(init), None)] = function.block.stmts.as_slice() else {
        return false;
    };
    let fields: Vec<String> = init
        .fields
        .iter()
        .filter(|field| field.colon_token.is_none() && field.attrs.is_empty())
        .filter_map(|field| match &field.member {
            syn::Member::Named(name) => Some(name.to_string()),
            syn::Member::Unnamed(_) => None,
        })
        .collect();
    init.path.is_ident("Self") && init.rest.is_none() && fields == ["qualified", "combat"]
}

/// Every macro invocation and every attribute other than a doc comment or the struct's
/// `#[derive(Clone, Copy)]`, at any depth.
#[derive(Default)]
struct Foreign(Vec<String>);

impl<'ast> Visit<'ast> for Foreign {
    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        self.0.push(format!("macro `{}`", path_name(&mac.path)));
        visit::visit_macro(self, mac);
    }

    fn visit_attribute(&mut self, attr: &'ast syn::Attribute) {
        if attr.path().is_ident("doc") || derives_clone_copy(attr) {
            return;
        }
        self.0
            .push(format!("attribute `{}`", path_name(attr.path())));
    }
}

fn derives_clone_copy(attr: &syn::Attribute) -> bool {
    let mut derived = Vec::new();
    attr.path().is_ident("derive")
        && attr
            .parse_nested_meta(|meta| {
                derived.push(path_name(&meta.path));
                Ok(())
            })
            .is_ok()
        && derived == ["Clone", "Copy"]
}

fn path_name(path: &syn::Path) -> String {
    path.segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect::<Vec<_>>()
        .join("::")
}

/// The arms of every `SeamRooms` accessor, by accessor, after holding `seam_rooms.rs` to its two
/// private fields, their constructor and accessors of the forms `accessor_arms` reads.
#[allow(clippy::expect_used, clippy::panic)]
fn accessors() -> BTreeMap<String, Vec<(String, Room)>> {
    let file = parse_source("src/gameplay_transport/seam_rooms.rs");
    let mut foreign = Foreign::default();
    foreign.visit_file(&file);
    assert!(
        foreign.0.is_empty(),
        "seam_rooms.rs gained {:?}: its expansion could read a room outside the inventory",
        foreign.0
    );
    let mut accessors = BTreeMap::new();
    let mut structs = 0;
    for item in &file.items {
        match item {
            syn::Item::Use(_) => {}
            syn::Item::Struct(item) => {
                structs += 1;
                let syn::Fields::Named(fields) = &item.fields else {
                    panic!("`SeamRooms` has no named fields");
                };
                let fields: Vec<(String, bool)> = fields
                    .named
                    .iter()
                    .map(|field| {
                        let name = field.ident.as_ref().map(ToString::to_string);
                        let private = matches!(field.vis, syn::Visibility::Inherited);
                        (name.unwrap_or_default(), private)
                    })
                    .collect();
                assert!(
                    item.ident == "SeamRooms"
                        && fields == [("qualified".to_owned(), true), ("combat".to_owned(), true)],
                    "seam_rooms.rs no longer declares `SeamRooms` with exactly the two private \
                     fields `qualified` and `combat`: {fields:?}"
                );
            }
            syn::Item::Impl(block)
                if block.trait_.is_none()
                    && block.unsafety.is_none()
                    && matches!(&*block.self_ty, syn::Type::Path(ty)
                        if ty.qself.is_none() && path_name(&ty.path) == "SeamRooms") =>
            {
                for item in &block.items {
                    let syn::ImplItem::Fn(function) = item else {
                        panic!("the `SeamRooms` impl holds an item other than a function");
                    };
                    let name = function.sig.ident.to_string();
                    if name == "new" {
                        assert!(
                            sets_both_rooms(function),
                            "`SeamRooms::new` does more than `Self {{ qualified, combat }}`"
                        );
                        continue;
                    }
                    let arms = accessor_arms(function).unwrap_or_else(|| {
                        panic!(
                            "`SeamRooms::{name}` is neither one room read nor a `match key` of \
                             room reads: record its gate as one of those forms"
                        )
                    });
                    assert!(
                        accessors.insert(name.clone(), arms).is_none(),
                        "`SeamRooms::{name}` is declared twice"
                    );
                }
            }
            _ => panic!("seam_rooms.rs holds an item other than its uses, struct and impl"),
        }
    }
    assert_eq!(structs, 1, "seam_rooms.rs declares one struct");
    accessors
}

#[test]
fn every_room_accessor_is_inventoried() {
    let mut expected: BTreeMap<String, Vec<(String, Room)>> = BTreeMap::new();
    for entry in INVENTORY {
        let room = match entry.state {
            Gate::Open { .. } => Room::Combat,
            Gate::Closed { .. } | Gate::SourceWorldBranch => Room::Qualified,
        };
        expected
            .entry(entry.accessor.to_owned())
            .or_default()
            .push((entry.key.to_owned(), room));
    }
    assert_eq!(
        accessors(),
        expected,
        "a `SeamRooms` accessor changed: record its gate in INVENTORY (an opened gate names its \
         batch and merged PR, a closed gate its owning batch)"
    );
}

/// The number of `name` identifier tokens of a token stream, at any depth: a call, a path to the
/// accessor or any other mention.
fn mentions(mut cursor: syn::buffer::Cursor<'_>, name: &str) -> usize {
    let mut count = 0;
    while !cursor.eof() {
        if let Some((inner, _, _, next)) = cursor.any_group() {
            count += mentions(inner, name);
            cursor = next;
        } else if let Some((ident, next)) = cursor.ident() {
            count += usize::from(ident.unraw() == name);
            cursor = next;
        } else if let Some((_, next)) = cursor.token_tree() {
            cursor = next;
        } else {
            break;
        }
    }
    count
}

/// Every mention of each of `names` in the crate's sources, by name and file under `src/`,
/// leaving out `seam_rooms.rs` and this module's inventory.
#[allow(clippy::expect_used)]
fn mentions_of(names: &[&str]) -> BTreeMap<(String, String), usize> {
    let root = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../apps/game-server"
    ))
    .join("src");
    let mut calls = BTreeMap::new();
    let transport_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut sources = vec![root.clone(), transport_root.clone()];
    while let Some(path) = sources.pop() {
        if path.is_dir() {
            let entries = std::fs::read_dir(&path).expect("source directory");
            sources.extend(entries.map(|entry| entry.expect("source entry").path()));
            continue;
        }
        if path.extension().is_none_or(|extension| extension != "rs") {
            continue;
        }
        let relative = path
            .strip_prefix(&root)
            .or_else(|_| path.strip_prefix(&transport_root))
            .expect("source under src")
            .to_string_lossy()
            .replace('\\', "/");
        if matches!(
            relative.as_str(),
            "gameplay_transport/seam_rooms.rs" | "gameplay_transport/bundle_gates_tests.rs"
        ) {
            continue;
        }
        let source = std::fs::read_to_string(&path).expect("source text");
        let tokens = syn::buffer::TokenBuffer::new2(source.parse().expect("source lexes"));
        for name in names {
            let count = mentions(tokens.begin(), name);
            if count > 0 {
                calls.insert(((*name).to_owned(), relative.clone()), count);
            }
        }
    }
    calls
}

#[test]
fn every_gate_call_is_inventoried() {
    let expected: BTreeMap<(String, String), usize> = INVENTORY
        .iter()
        .flat_map(|entry| {
            entry
                .calls
                .iter()
                .map(|(file, count)| ((entry.accessor.to_owned(), (*file).to_owned()), *count))
        })
        .collect();
    let names: Vec<&str> = INVENTORY.iter().map(|entry| entry.accessor).collect();
    assert_eq!(
        mentions_of(&names),
        expected,
        "a call of a `SeamRooms` accessor was added, removed or moved: record it in INVENTORY"
    );
    for entry in INVENTORY.iter().filter(|entry| entry.key.is_empty()) {
        assert!(
            !entry.calls.is_empty(),
            "gate `{}` is never called",
            entry.gate
        );
    }
}

#[test]
fn every_gate_names_its_opener_or_owner() {
    for entry in INVENTORY {
        match entry.state {
            Gate::Open { batch, pr } => {
                assert!(!batch.is_empty() && pr > 0, "open gate `{}`", entry.gate);
            }
            Gate::Closed { owners } => {
                assert!(
                    !owners.is_empty(),
                    "closed gate `{}` has no owner",
                    entry.gate
                );
            }
            Gate::SourceWorldBranch => {}
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
    let clause = |entry: &Entry, names: &[String]| {
        clauses.iter().any(|clause| {
            clause.contains(entry.plan) && names.iter().all(|name| clause.contains(name.as_str()))
        })
    };
    for entry in INVENTORY {
        match entry.state {
            Gate::Open { batch, pr } => {
                assert!(
                    clause(entry, &[batch.to_owned(), format!("#{pr}")]),
                    "the plan row has no clause recording that {batch} #{pr} opened `{}` ({})",
                    entry.gate,
                    entry.plan
                );
            }
            Gate::Closed { owners } => {
                for owner in owners {
                    assert!(
                        plan.lines()
                            .any(|line| line.starts_with(&format!("| {owner} |"))),
                        "closed gate `{}` names `{owner}`, which is not a plan batch",
                        entry.gate
                    );
                }
                let names: Vec<String> = owners.iter().map(|owner| (*owner).to_owned()).collect();
                assert!(
                    clause(entry, &names),
                    "the plan row has no clause naming {owners:?} for `{}` ({})",
                    entry.gate,
                    entry.plan
                );
            }
            Gate::SourceWorldBranch => assert!(
                clause(
                    entry,
                    &["are not gates".to_owned(), "`bundle_map`".to_owned()]
                ),
                "the plan row has no clause recording that the source-World branch `{}` is not a \
                 gate because a bundle World is served by `bundle_map` ({})",
                entry.gate,
                entry.plan
            ),
        }
    }
}

/// `bundle.is_none().then_some(&qualified_room)`.
fn withholds_from_a_bundle(expr: &syn::Expr) -> bool {
    let is_name = |expr: &syn::Expr, name: &str| matches!(expr, syn::Expr::Path(path) if path.path.is_ident(name));
    let syn::Expr::MethodCall(then_some) = expr else {
        return false;
    };
    let syn::Expr::MethodCall(is_none) = &*then_some.receiver else {
        return false;
    };
    let reference = then_some.args.iter().collect::<Vec<_>>();
    then_some.method == "then_some"
        && is_none.method == "is_none"
        && is_none.args.is_empty()
        && is_name(&is_none.receiver, "bundle")
        && matches!(
            reference.as_slice(),
            [syn::Expr::Reference(reference)]
                if reference.mutability.is_none() && is_name(&reference.expr, "qualified_room")
        )
}

/// Every `SeamRooms::new(…)` call of a source file, at any depth.
#[derive(Default)]
struct Constructions<'ast>(Vec<&'ast syn::ExprCall>);

impl<'ast> Visit<'ast> for Constructions<'ast> {
    fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
        if let syn::Expr::Path(path) = &*call.func
            && path_name(&path.path).ends_with("SeamRooms::new")
        {
            self.0.push(call);
        }
        visit::visit_expr_call(self, call);
    }
}

/// Every mention of `SeamRooms` outside `seam_rooms.rs`: the seam's declarations of its two
/// owners in `mod.rs`, its one production construction in `serve.rs`, the shared test
/// constructor and explicit fixture room overrides.
const CONSTRUCTIONS: &[(&str, usize)] = &[
    ("gameplay_transport/composed_test_owner.rs", 2),
    ("gameplay_transport/item_use/item_use_postgres_tests.rs", 1),
    ("gameplay_transport/kill_reward_postgres_e2e_tests.rs", 1),
    ("gameplay_transport/mod.rs", 3),
    ("gameplay_transport/monk_save_postgres_tests.rs", 1),
    ("gameplay_transport/parameter_cast.rs", 3),
    ("gameplay_transport/postgres_tests/party_xp.rs", 1),
    ("gameplay_transport/rune_use_bridge_postgres_tests.rs", 2),
    ("node/serve.rs", 1),
    ("qualification.rs", 1),
];

#[test]
fn a_bundle_world_seam_has_no_qualified_room() {
    let expected: BTreeMap<(String, String), usize> = CONSTRUCTIONS
        .iter()
        .map(|(file, count)| (("SeamRooms".to_owned(), (*file).to_owned()), *count))
        .collect();
    assert_eq!(
        mentions_of(&["SeamRooms"]),
        expected,
        "`SeamRooms` is named in another place: a new construction could hand a bundle World its \
         qualified room"
    );
    let serve = parse_source("src/node/serve.rs");
    let mut constructions = Constructions::default();
    constructions.visit_file(&serve);
    assert!(
        matches!(constructions.0.as_slice(), [call]
            if call.args.first().is_some_and(withholds_from_a_bundle)),
        "serve.rs no longer builds `SeamRooms::new(bundle.is_none().then_some(&qualified_room), \
         …)`: every closed gate would open at once without its owner"
    );
}
