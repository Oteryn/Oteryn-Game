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
    /// An explicitly allocated mechanic opens this gate in the candidate diff.
    AllocatedOpen { task: &'static str },
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
        gate: "spatial wire projection",
        plan: "spatial wire projection",
        accessor: "gate_wire_position",
        key: "",
        state: Gate::AllocatedOpen { task: "SRX-3" },
        calls: &[
            (
                "gameplay_transport/postgres_tests/srx3_primal_death_cutoff.rs",
                1,
            ),
            ("gameplay_transport/wire_position.rs", 1),
            // SRX-3 source/current-owner reads; gate semantics remain unchanged.
            ("gameplay_transport/native_cue_offer.rs", 2),
            ("gameplay_transport/native_activity.rs", 1),
            ("gameplay_transport/soulpit/scene.rs", 2),
            (
                "gameplay_transport/postgres_tests/boss_activity/primal.rs",
                1,
            ),
            ("gameplay_transport/postgres_tests/endgame_entry.rs", 1),
            ("gameplay_transport/postgres_tests/srx3_primal.rs", 1),
            (
                "gameplay_transport/rotten_blood_callbacks/advanced/footprints.rs",
                1,
            ),
        ],
    },
    Entry {
        gate: "attack target",
        plan: "player attack target",
        accessor: "gate_attack_target",
        key: "",
        state: COMBAT_ENABLE,
        calls: &[
            ("gameplay_transport/composed_admission/combat.rs", 1),
            ("gameplay_transport/player_trade/observation.rs", 1),
        ],
    },
    Entry {
        gate: "auto-attack drain",
        plan: "auto-attack drain",
        accessor: "gate_auto_attack",
        key: "",
        state: COMBAT_ENABLE,
        calls: &[
            ("gameplay_transport/attack/drain.rs", 1),
            ("gameplay_transport/attack/ranged.rs", 1),
            (
                "gameplay_transport/postgres_tests/daily_reward/effects.rs",
                1,
            ),
            ("gameplay_transport/postgres_tests/equipped_offence.rs", 2),
            ("gameplay_transport/postgres_tests/hunting_stamina.rs", 3),
            // SRX-2/P01: charm regressions select live targets through the existing attack gate.
            (
                "gameplay_transport/postgres_tests/srx2_charm_consumers.rs",
                2,
            ),
            // TASKBOARD-P04: real party kill/capture replay fixture uses the existing attack gate.
            (
                "gameplay_transport/postgres_tests/party_xp/bounty_talisman.rs",
                2,
            ),
            // The real equipment move must settle a prior death before changing slot 10.
            (
                "gameplay_transport/postgres_tests/party_xp/bounty_talisman/equipment.rs",
                1,
            ),
            // TASKBOARD-P04: chronological queued deaths use real target/clear attack calls.
            (
                "gameplay_transport/postgres_tests/party_xp/task_board_ordered.rs",
                2,
            ),
            // XMECH-E2E-1: the hunt, death and disconnect scenarios attack through the real gate.
            ("gameplay_transport/postgres_tests/scenarios/mod.rs", 3),
        ],
    },
    Entry {
        gate: "native combat cast and its commit",
        plan: "native combat cast",
        accessor: "gate_native_combat_cast",
        key: "",
        state: COMBAT_ENABLE,
        // Activity registration validates its combat room against the active content pin.
        calls: &[
            ("gameplay_transport/activity_runtime.rs", 1),
            ("gameplay_transport/native_combat_cast.rs", 2),
            ("gameplay_transport/rune_use_bridge/revision.rs", 1),
            // SRX-3: install the same source-qualified combat room and read Soulpit terrain.
            ("gameplay_transport/serving.rs", 1),
            // SRX-3 source/current-owner reads; gate semantics remain unchanged.
            ("gameplay_transport/soulpit/owner_runtime.rs", 1),
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
        calls: &[
            // Composed native callbacks retain the existing open monster gate.
            ("gameplay_transport/primal_callbacks/normal.rs", 1),
            ("gameplay_transport/kill_reward.rs", 1),
            ("gameplay_transport/raid_live_world/feroxa_map.rs", 1),
            ("gameplay_transport/primal_callbacks.rs", 3),
            ("gameplay_transport/soul_war_cloak_blood.rs", 1),
            ("gameplay_transport/monster_ai_cycle.rs", 5),
            // S10: source NPC wandering uses the same live-world movement gate.
            ("gameplay_transport/npc_actor/current_map.rs", 1),
            (
                "gameplay_transport/postgres_tests/m26_rotten_wave_production.rs",
                1,
            ),
            ("gameplay_transport/postgres_tests/m26_spite_chain.rs", 1),
            // Primal cadence uses the actual activated Source combat seam.
            (
                "gameplay_transport/postgres_tests/primal_menace_native.rs",
                1,
            ),
            // RAIDS-1 (pkg4): a raid's live world steps its spawned monsters through the same gate.
            ("gameplay_transport/raid_live_world.rs", 1),
            // W05d: the existing open monster gate resumes only source-reserved
            // canonical Cake actors, and its real machine-step PG fixture.
            ("gameplay_transport/raid_live_world/cake.rs", 1),
            ("gameplay_transport/postgres_tests/raid_live/cake.rs", 2),
            // W17: the physical Drome regression proves participant-only targeting.
            (
                "gameplay_transport/postgres_tests/reaudit_arena_lifecycle.rs",
                1,
            ),
            // SRX-3 source/current-owner reads; gate semantics remain unchanged.
            (
                "gameplay_transport/monster_profile_cycle/native_callbacks/death_items.rs",
                1,
            ),
            ("gameplay_transport/native_activity.rs", 1),
            (
                "gameplay_transport/postgres_tests/srx3_greed_retention.rs",
                1,
            ),
            (
                "gameplay_transport/postgres_tests/srx3_megalomania_step.rs",
                2,
            ),
            // The Outburst diagnostic requalifies its current source context before reading it.
            (
                "gameplay_transport/postgres_tests/srx3_native_source_cast.rs",
                1,
            ),
            ("gameplay_transport/postgres_tests/srx3_primal.rs", 1),
            ("gameplay_transport/postgres_tests/srx3_rotten_portal.rs", 1),
            (
                "gameplay_transport/postgres_tests/srx3_soul_war_attacks.rs",
                2,
            ),
            ("gameplay_transport/soul_war_callbacks/entry.rs", 1),
            (
                "gameplay_transport/soul_war_callbacks/megalomania/phases/death.rs",
                1,
            ),
            (
                "gameplay_transport/soul_war_callbacks/megalomania/phases/timer.rs",
                1,
            ),
            ("gameplay_transport/soul_war_callbacks/taint_five.rs", 1),
            ("gameplay_transport/soul_war_world.rs", 1),
            ("gameplay_transport/soul_war_world_raid.rs", 1),
            ("gameplay_transport/source_death.rs", 1),
            ("gameplay_transport/source_death/heart.rs", 1),
            ("gameplay_transport/spite_global.rs", 1),
        ],
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
        calls: &[
            ("gameplay_transport/player_field.rs", 1),
            ("gameplay_transport/spell_periodic.rs", 1),
        ],
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
        // The cast and its commit, and the three of `bundle_world_opens_locate_message_and_vertical_move`.
        calls: &[("gameplay_transport/parameter_cast.rs", 5)],
    },
    Entry {
        gate: "parameter cast and its commit: vertical move",
        plan: "`vertical_move` parameter cast",
        accessor: "gate_parameter_cast",
        key: "vertical_move",
        state: Gate::AllocatedOpen { task: "WAVE-A/M11" },
        calls: &[],
    },
    Entry {
        gate: "parameter cast and its commit: creature appearance",
        plan: "`creature_appearance` parameter cast",
        accessor: "gate_parameter_cast",
        key: "creature_appearance",
        state: Gate::AllocatedOpen {
            task: "WAVE-A2/C17c",
        },
        calls: &[],
    },
    // `house_access`: SPELL-RT-WORLD-1 of the MAGIC-SUMMON-1 graph.
    Entry {
        gate: "parameter cast and its commit: house access",
        plan: "`house_access` parameter cast",
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
        state: Gate::AllocatedOpen {
            task: "FIELD-RUNES-1",
        },
        calls: &[
            ("gameplay_transport/field_removal.rs", 1),
            ("gameplay_transport/field_step_ingress.rs", 2),
            ("gameplay_transport/postgres_tests/field_runes.rs", 2),
            // SRX-3: source-owned Primal Pod creation and expiry use the existing Ground gate.
            ("gameplay_transport/primal_callbacks.rs", 2),
            // SRX-3 source/current-owner reads; gate semantics remain unchanged.
            ("gameplay_transport/boss_activity/source_extra.rs", 1),
            ("gameplay_transport/outburst_entry.rs", 1),
            (
                "gameplay_transport/postgres_tests/srx3_megalomania_step.rs",
                2,
            ),
            (
                "gameplay_transport/postgres_tests/srx3_rotten_callbacks.rs",
                1,
            ),
            ("gameplay_transport/primal_entry.rs", 1),
            ("gameplay_transport/rotten_blood_entry.rs", 1),
            ("gameplay_transport/soul_war_entry.rs", 1),
            ("gameplay_transport/spite_step.rs", 3),
            ("gameplay_transport/spite_step/megalomania.rs", 1),
        ],
    },
    Entry {
        gate: "ground item actions",
        plan: "ground item actions",
        accessor: "gate_ground_item_action",
        key: "",
        state: Gate::AllocatedOpen { task: "TK-W19" },
        calls: &[
            ("gameplay_transport/cosmetic_item_use.rs", 1),
            // Carried-fluid dispensers retain the native ground owner proof.
            ("gameplay_transport/item_craft/dispenser.rs", 1),
            ("gameplay_transport/item_craft/planting.rs", 1),
            ("gameplay_transport/novelty_ground_use.rs", 1),
        ],
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
        calls: &[
            ("gameplay_transport/postgres_tests/m26_spite_chain.rs", 1),
            (
                "gameplay_transport/postgres_tests/m26_static_instance.rs",
                1,
            ),
            ("gameplay_transport/source_item_cycle.rs", 1),
        ],
    },
    // The door overlay, step blocking, `use_object` and the chase step's and a creature push's
    // door blocking serve a bundle World from `bundle_map`.
    Entry {
        gate: "source World door overlay, step blocking, USE, chase step and push blocking",
        plan: "door overlay, step blocking, USE",
        accessor: "gate_source_world",
        key: "",
        state: Gate::SourceWorldBranch,
        calls: &[
            // Activity construction inherits this gate; it cannot promote a bundle origin.
            ("gameplay_transport/activity_content.rs", 1),
            ("gameplay_transport/attack.rs", 1),
            // C10: bed sleep production no longer calls this gate; it checks the bundle map
            // digest or the source artifact digest itself and proves the bed head and House
            // from that same map (`bed_sleep_production.rs`).
            ("gameplay_transport/composed_admission/objects.rs", 2),
            ("gameplay_transport/creature_push.rs", 1),
            ("gameplay_transport/paced_movement.rs", 1),
            ("gameplay_transport/paced_movement/autowalk.rs", 1),
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

/// The exact borrowed view of `self.qualified` or `self.combat`.
fn field_room(expr: &syn::Expr) -> Option<Room> {
    let syn::Expr::MethodCall(view) = expr else {
        return None;
    };
    if view.method != "as_deref"
        || !view.args.is_empty()
        || !view.attrs.is_empty()
        || view.turbofish.is_some()
    {
        return None;
    }
    stored_field_room(&view.receiver)
}

fn stored_field_room(expr: &syn::Expr) -> Option<Room> {
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
    // syn 3 carries a match guard as `Pat::Guard`, so the literal and wildcard
    // pattern checks also reject guarded arms.
    for arm in keys {
        let syn::Pat::Lit(syn::ExprLit {
            lit: syn::Lit::Str(key),
            attrs,
        }) = &arm.pat
        else {
            return None;
        };
        if !attrs.is_empty() || !arm.attrs.is_empty() {
            return None;
        }
        arms.push((key.value(), field_room(&arm.body)?));
    }
    if !default.attrs.is_empty() {
        return None;
    }
    arms.push(("_".to_owned(), field_room(&default.body)?));
    Some(arms)
}

fn named_expr(expr: &syn::Expr, name: &str) -> bool {
    matches!(expr, syn::Expr::Path(path) if path.attrs.is_empty() && path.qself.is_none() && path.path.is_ident(name))
}

fn call_args<'a>(expr: &'a syn::Expr, name: &str) -> Option<Vec<&'a syn::Expr>> {
    let syn::Expr::Call(call) = expr else {
        return None;
    };
    if !call.attrs.is_empty()
        || !matches!(&*call.func, syn::Expr::Path(path) if path.qself.is_none() && path.attrs.is_empty() && path_name(&path.path) == name)
    {
        return None;
    }
    Some(call.args.iter().collect())
}

/// `Self { qualified: qualified.map(SharedOwner::from), combat: combat.map(SharedOwner::from) }`.
fn sets_both_rooms(function: &syn::ImplItemFn) -> bool {
    let [syn::Stmt::Expr(syn::Expr::Struct(init), None)] = function.block.stmts.as_slice() else {
        return false;
    };
    init.path.is_ident("Self") && init.attrs.is_empty() && init.rest.is_none()
        && init.fields.len() == 2 && init.fields.iter().zip(["qualified", "combat"]).all(|(field,name)| {
            let syn::Expr::MethodCall(map) = &field.expr else { return false; };
            field.attrs.is_empty() && field.colon_token.is_some()
                && matches!(&field.member,syn::Member::Named(member) if member == name)
                && map.attrs.is_empty() && map.method == "map" && map.turbofish.is_none()
                && named_expr(&map.receiver,name)
                && matches!(map.args.iter().collect::<Vec<_>>().as_slice(), [syn::Expr::Path(path)] if path_name(&path.path) == "SharedOwner::from" && path.attrs.is_empty() && path.qself.is_none())
        })
}

/// The owned constructor keeps combat present and qualification conditional on its input.
fn sets_owned_rooms(function: &syn::ImplItemFn) -> bool {
    let [syn::Stmt::Expr(syn::Expr::Struct(init), None)] = function.block.stmts.as_slice() else {
        return false;
    };
    let fields = init.fields.iter().collect::<Vec<_>>();
    let [qualified, combat] = fields.as_slice() else {
        return false;
    };
    if !init.path.is_ident("Self")
        || !init.attrs.is_empty()
        || init.rest.is_some()
        || !qualified.attrs.is_empty()
        || !combat.attrs.is_empty()
        || qualified.colon_token.is_none()
        || combat.colon_token.is_none()
        || !matches!(&qualified.member,syn::Member::Named(name) if name == "qualified")
        || !matches!(&combat.member,syn::Member::Named(name) if name == "combat")
    {
        return false;
    }
    let syn::Expr::MethodCall(then) = &qualified.expr else {
        return false;
    };
    let then_args = then.args.iter().collect::<Vec<_>>();
    let [syn::Expr::Closure(select)] = then_args.as_slice() else {
        return false;
    };
    if then.method != "then"
        || !then.attrs.is_empty()
        || then.turbofish.is_some()
        || !named_expr(&then.receiver, "qualified")
        || !select.inputs.is_empty()
        || !select.attrs.is_empty()
        || select.capture.is_some()
        || select.asyncness.is_some()
    {
        return false;
    }
    let Some(owned) = call_args(&select.body, "SharedOwner::Owned") else {
        return false;
    };
    let [cloned] = owned.as_slice() else {
        return false;
    };
    let Some(clone_args) = call_args(cloned, "Arc::clone") else {
        return false;
    };
    let [syn::Expr::Reference(reference)] = clone_args.as_slice() else {
        return false;
    };
    if reference.mutability.is_some()
        || !reference.attrs.is_empty()
        || !named_expr(&reference.expr, "room")
    {
        return false;
    }
    let Some(some) = call_args(&combat.expr, "Some") else {
        return false;
    };
    let [owned] = some.as_slice() else {
        return false;
    };
    call_args(owned, "SharedOwner::Owned")
        .is_some_and(|args| matches!(args.as_slice(),[room] if named_expr(room,"room")))
}

/// Every macro invocation and every attribute other than a doc comment or the struct's
/// `#[derive(Clone)]`, at any depth.
#[derive(Default)]
struct Foreign(Vec<String>);

impl<'ast> Visit<'ast> for Foreign {
    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        self.0.push(format!("macro `{}`", path_name(&mac.path)));
        visit::visit_macro(self, mac);
    }

    fn visit_attribute(&mut self, attr: &'ast syn::Attribute) {
        if attr.path().is_ident("doc") || derives_clone(attr) {
            return;
        }
        self.0
            .push(format!("attribute `{}`", path_name(attr.path())));
    }
}

fn derives_clone(attr: &syn::Attribute) -> bool {
    let mut derived = Vec::new();
    attr.path().is_ident("derive")
        && attr
            .parse_nested_meta(|meta| {
                derived.push(path_name(&meta.path));
                Ok(())
            })
            .is_ok()
        && derived == ["Clone"]
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
                            "`SeamRooms::new` must only wrap each borrowed room in SharedOwner"
                        );
                        continue;
                    }
                    if name == "owned" {
                        assert!(
                            sets_owned_rooms(function),
                            "`SeamRooms::owned` must preserve its conditional qualification and shared combat room"
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
            Gate::Open { .. } | Gate::AllocatedOpen { .. } => Room::Combat,
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
fn every_gate_names_its_opener_or_owner() -> Result<(), Box<dyn std::error::Error>> {
    for entry in INVENTORY {
        match entry.state {
            Gate::Open { batch, pr } => {
                assert!(!batch.is_empty() && pr > 0, "open gate `{}`", entry.gate);
            }
            Gate::AllocatedOpen { task } => {
                if task == "WAVE-A/M11" {
                    assert_eq!(entry.accessor, "gate_parameter_cast");
                    assert_eq!(entry.key, "vertical_move");
                    continue;
                }
                if task == "WAVE-A2/C17c" {
                    assert_eq!(entry.accessor, "gate_parameter_cast");
                    assert_eq!(entry.key, "creature_appearance");
                    continue;
                }
                if task == "SRX-3" {
                    assert_eq!(entry.accessor, "gate_wire_position");
                    assert_eq!(entry.key, "");
                    continue;
                }
                if task == "TK-W19" {
                    assert_eq!(entry.accessor, "gate_ground_item_action");
                    assert_eq!(entry.key, "");
                    continue;
                }
                assert_eq!(task, "FIELD-RUNES-1");
                let contract: serde_json::Value = serde_json::from_str(include_str!(
                    "../../../../../docs/contracts/protocol-oteryn/v1/registry/capabilities/0091-field-runes-v1.json"
                ))?;
                assert!(
                    contract["owner_decision"]
                        .as_str()
                        .is_some_and(|decision| decision.starts_with(task))
                );
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
    Ok(())
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
            Gate::AllocatedOpen { task } => {
                assert!(
                    clause(entry, &[task.to_owned()]),
                    "allocated gate {} missing from plan",
                    entry.gate
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
            && (path_name(&path.path).ends_with("SeamRooms::new")
                || path_name(&path.path).ends_with("SeamRooms::owned"))
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
    ("gameplay_transport/postgres_tests/m25_ahau_exit.rs", 1),
    ("gameplay_transport/postgres_tests/engenc2_nomads_use.rs", 1),
    ("gameplay_transport/postgres_tests/engenc2_cults.rs", 1),
    // Qualified activity content inherits the origin gate, checked below.
    ("gameplay_transport/activity_content.rs", 3),
    // Scoped activity assets explicitly withhold qualification, checked below.
    ("gameplay_transport/activity_runtime.rs", 2),
    // Compatibility return type only; exact delegation is checked below.
    ("gameplay_transport/effective_rooms.rs", 1),
    ("gameplay_transport/composed_test_owner.rs", 2),
    ("gameplay_transport/item_use/item_use_postgres_tests.rs", 1),
    (
        "gameplay_transport/postgres_tests/world_reset_activation.rs",
        2,
    ),
    (
        "gameplay_transport/postgres_tests/planned_world_reset_source_map.rs",
        1,
    ),
    ("gameplay_transport/kill_reward_postgres_e2e_tests.rs", 1),
    ("gameplay_transport/mod.rs", 4),
    ("gameplay_transport/monk_save_postgres_tests.rs", 1),
    ("gameplay_transport/parameter_cast.rs", 3),
    ("gameplay_transport/postgres_tests/admit_cap_2.rs", 2),
    // Atelier resource fixtures bind a real Bundle with qualified room None.
    ("gameplay_transport/postgres_tests/atelier/wire_paid.rs", 1),
    // Boss PG fixtures explicitly bind their activated source and Instance rooms.
    ("gameplay_transport/postgres_tests/boss_activity.rs", 3),
    // Primal's test-only source scene binds its actual pinned qualified/combat room.
    (
        "gameplay_transport/postgres_tests/boss_activity/primal.rs",
        1,
    ),
    // Movement fixtures bind only their actual pinned Bundle combat view.
    ("gameplay_transport/postgres_tests/echo_warden.rs", 1),
    (
        "gameplay_transport/postgres_tests/browse_field/fixture.rs",
        2,
    ),
    ("gameplay_transport/postgres_tests/creature_push.rs", 1),
    ("gameplay_transport/postgres_tests/cyclopedia.rs", 3),
    ("gameplay_transport/postgres_tests/dawnport_trial.rs", 1),
    // Q6's condition-potion fixture supplies only combat; its qualified room stays None.
    ("gameplay_transport/postgres_tests/condition_potions.rs", 1),
    // W05d's native Cake tile fixture supplies combat only, never qualification.
    ("gameplay_transport/postgres_tests/cake_phases.rs", 1),
    ("gameplay_transport/postgres_tests/cyclopedia_tracker.rs", 1),
    // W19 fixtures supply only combat; bundle Worlds still have no qualified room.
    (
        "gameplay_transport/postgres_tests/event_containers/ground.rs",
        2,
    ),
    ("gameplay_transport/postgres_tests/field_runes.rs", 2),
    ("gameplay_transport/postgres_tests/item_cosmetics.rs", 1),
    ("gameplay_transport/postgres_tests/newhaven_corruptor.rs", 1),
    // The carried-fluid fixture supplies an action room and no qualified room.
    (
        "gameplay_transport/postgres_tests/item_craft_carried_fluid.rs",
        1,
    ),
    ("gameplay_transport/postgres_tests/item_seed_planting.rs", 1),
    ("gameplay_transport/postgres_tests/m26_spite_chain.rs", 2),
    (
        "gameplay_transport/postgres_tests/m26_static_instance.rs",
        2,
    ),
    ("gameplay_transport/postgres_tests/magic2_conjure.rs", 2),
    ("gameplay_transport/postgres_tests/newhaven_corruptor.rs", 1),
    (
        "gameplay_transport/postgres_tests/novelty_items/scenario.rs",
        1,
    ),
    ("gameplay_transport/postgres_tests/party_buffs.rs", 1),
    ("gameplay_transport/postgres_tests/party_xp.rs", 1),
    ("gameplay_transport/postgres_tests/player_trade.rs", 2),
    // PREY-DIFF (2026-11-01): the loot fixture supplies combat only;
    // its qualified room remains None.
    (
        "gameplay_transport/postgres_tests/prey_loot_measurement.rs",
        1,
    ),
    // Primal's regression binds only its real activated Source room, never a Bundle substitute.
    (
        "gameplay_transport/postgres_tests/primal_menace_native.rs",
        1,
    ),
    // The proficiency fixture binds its Bundle combat view and withholds qualification.
    (
        "gameplay_transport/postgres_tests/proficiency_modification.rs",
        1,
    ),
    ("gameplay_transport/postgres_tests/ranged_attack.rs", 1),
    // QUESTPC4 source callback and reset fixtures bind their explicit source room.
    (
        "gameplay_transport/postgres_tests/quest_world_callbacks/scenario.rs",
        2,
    ),
    // Rotten Blood progression supplies its source combat room; qualified room stays None.
    (
        "gameplay_transport/postgres_tests/rotten_blood_progression.rs",
        1,
    ),
    // S10 source NPC fixtures supply combat only; qualified room remains None.
    ("gameplay_transport/postgres_tests/s10_travel.rs", 1),
    ("gameplay_transport/postgres_tests/s10_wandering.rs", 1),
    // Soul War's Bundle fixture supplies combat only, never qualification.
    ("gameplay_transport/postgres_tests/soul_war.rs", 1),
    // I14 special-potion fixture supplies only combat; its qualified room stays None.
    ("gameplay_transport/postgres_tests/special_potions.rs", 1),
    // Soulpit's production owner and PG fixture both withhold qualification.
    ("gameplay_transport/soulpit/content.rs", 2),
    (
        "gameplay_transport/postgres_tests/soulpit_shared_entry/native.rs",
        2,
    ),
    ("gameplay_transport/postgres_tests/task_board.rs", 1),
    ("gameplay_transport/postgres_tests/vertical_move.rs", 2),
    ("gameplay_transport/postgres_tests/utility_runes.rs", 3),
    // Wheel socket/unsocket fixtures add two explicit empty-room overrides;
    // the production qualified-room construction remains unchanged.
    ("gameplay_transport/postgres_tests/wheel_allocation.rs", 5),
    ("gameplay_transport/rune_use_bridge_postgres_tests.rs", 2),
    ("node/serve.rs", 1),
    ("qualification.rs", 1),
    // SRX-3 fixture room bindings; production Bundle closure is asserted separately.
    ("gameplay_transport/postgres_tests/hireling_lifecycle.rs", 1),
    (
        "gameplay_transport/postgres_tests/srx3_activity_source_fixture.rs",
        2,
    ),
    (
        "gameplay_transport/postgres_tests/srx3_claustrophobic.rs",
        1,
    ),
    // The published BundleWorld corpse fixture supplies only its original combat room.
    (
        "gameplay_transport/postgres_tests/srx3_poor_soul_world_corpse.rs",
        1,
    ),
    (
        "gameplay_transport/postgres_tests/srx3_rotten_area_think.rs",
        1,
    ),
    (
        "gameplay_transport/postgres_tests/srx3_rotten_callbacks.rs",
        2,
    ),
    (
        "gameplay_transport/postgres_tests/srx3_soul_war_world_entrance.rs",
        1,
    ),
    (
        "gameplay_transport/postgres_tests/srx3_soulpit_native.rs",
        1,
    ),
    ("gameplay_transport/postgres_tests/srx3_spite_global.rs", 1),
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

/// Activity content may replace the room, but never widen the origin seam's qualification.
fn inherits_origin_qualification(expr: &syn::Expr) -> bool {
    let syn::Expr::MethodCall(selection) = expr else {
        return false;
    };
    if !selection.attrs.is_empty() || selection.turbofish.is_some() {
        return false;
    }
    let select_origin = if selection.method == "is_some" {
        selection.args.is_empty()
    } else if selection.method == "map" {
        matches!(selection.args.iter().collect::<Vec<_>>().as_slice(),
            [syn::Expr::Closure(select)] if select.inputs.len() == 1
                && matches!(select.inputs.first(), Some(syn::Pat::Wild(_)))
                && matches!(&*select.body, syn::Expr::Path(path) if path.path.is_ident("room")))
    } else {
        false
    };
    let syn::Expr::MethodCall(gate) = &*selection.receiver else {
        return false;
    };
    if !select_origin
        || gate.method != "gate_source_world"
        || !gate.args.is_empty()
        || !gate.attrs.is_empty()
        || gate.turbofish.is_some()
    {
        return false;
    }
    match &*gate.receiver {
        syn::Expr::Field(rooms) => {
            rooms.attrs.is_empty()
                && matches!(&rooms.member, syn::Member::Named(name) if name == "rooms")
                && matches!(&*rooms.base, syn::Expr::Path(path) if path.path.is_ident("self"))
        }
        syn::Expr::MethodCall(rooms) => {
            rooms.attrs.is_empty()
                && rooms.method == "rooms"
                && rooms.args.is_empty()
                && rooms.turbofish.is_none()
                && matches!(&*rooms.receiver, syn::Expr::Path(path) if path.path.is_ident("self"))
        }
        _ => false,
    }
}

#[test]
fn an_activity_instance_preserves_the_origin_room_gate() {
    let content = parse_source("src/gameplay_transport/activity_content.rs");
    let mut constructions = Constructions::default();
    constructions.visit_file(&content);
    assert!(
        matches!(constructions.0.as_slice(), [source, inherited]
            if matches!(&*source.func, syn::Expr::Path(path) if path_name(&path.path).ends_with("SeamRooms::owned"))
                && source.args.len() == 2
                && matches!(source.args.first(), Some(syn::Expr::Call(clone))
                    if matches!(&*clone.func, syn::Expr::Path(path)
                        if path_name(&path.path) == "std::sync::Arc::clone")
                        && clone.args.len() == 1
                        && matches!(clone.args.first(), Some(syn::Expr::Reference(reference))
                            if reference.mutability.is_none()
                                && named_expr(&reference.expr, "owned_room")))
                && matches!(source.args.last(), Some(syn::Expr::Lit(literal))
                    if matches!(&literal.lit, syn::Lit::Bool(value) if !value.value))
                && matches!(&*inherited.func, syn::Expr::Path(path) if path_name(&path.path).ends_with("SeamRooms::owned"))
                && inherited.args.len() == 2
                && inherited.args.first().is_some_and(|arg| named_expr(arg, "room"))
                && inherited.args.last().is_some_and(inherits_origin_qualification)),
        "activity content must inherit the origin room gate; qualifying a template cannot open unrelated bundle gates"
    );
}

#[test]
fn private_activity_room_constructions_withhold_qualification() {
    // SharedOwner already owns the selected private room. The two accessors
    // must borrow those exact fields, never construct an origin-room fallback.
    for file in [
        "src/gameplay_transport/mod.rs",
        "src/gameplay_transport/activity_runtime.rs",
    ] {
        let source = parse_source(file);
        let mut constructions = Constructions::default();
        constructions.visit_file(&source);
        assert!(
            constructions.0.is_empty(),
            "{file}: forwarding must not construct another seam"
        );
        for name in ["rooms", "movement_cells"] {
            let functions: Vec<_> = source
                .items
                .iter()
                .filter_map(|item| match item {
                    syn::Item::Impl(item) => Some(item),
                    _ => None,
                })
                .flat_map(|item| item.items.iter())
                .filter_map(|item| match item {
                    syn::ImplItem::Fn(function) if function.sig.ident == name => Some(function),
                    _ => None,
                })
                .collect();
            assert!(
                matches!(functions.as_slice(), [function] if matches!(function.block.stmts.as_slice(),
                [syn::Stmt::Expr(syn::Expr::Reference(reference), None)]
                    if reference.mutability.is_none() && reference.attrs.is_empty()
                        && matches!(&*reference.expr, syn::Expr::Field(field)
                            if field.attrs.is_empty()
                                && matches!(&field.member, syn::Member::Named(member) if member == name)
                                && matches!(&*field.base, syn::Expr::Path(path) if path.path.is_ident("self"))))),
                "{file}: {name} must forward only its actual owned field"
            );
        }
    }
    let source = parse_source("src/gameplay_transport/soulpit/content.rs");
    let mut constructions = Constructions::default();
    constructions.visit_file(&source);
    assert!(
        matches!(constructions.0.as_slice(), [call]
        if matches!(&*call.func,syn::Expr::Path(path) if path_name(&path.path).ends_with("SeamRooms::owned"))
            && call.args.len() == 2
            && call.args.first().is_some_and(|arg| named_expr(arg,"private_room"))
            && matches!(call.args.last(), Some(syn::Expr::Lit(value)) if matches!(&value.lit, syn::Lit::Bool(value) if !value.value))),
        "Soulpit's exact private room may supply combat geometry but never source-room qualification"
    );
}

#[test]
fn inherited_qualification_recognizer_rejects_widened_room_selection()
-> Result<(), Box<dyn std::error::Error>> {
    for expression in [
        "self.rooms.gate_source_world().is_some()",
        "self.rooms().gate_source_world().is_some()",
        "self.rooms.gate_source_world().map(|_| room)",
        "self.rooms().gate_source_world().map(|_| room)",
    ] {
        assert!(inherits_origin_qualification(&syn::parse_str(expression)?));
    }
    for expression in [
        "Some(room)",
        "other.rooms().gate_source_world().map(|_| room)",
        "self.rooms(other).gate_source_world().map(|_| room)",
        "self.rooms().gate_attack_target().map(|_| room)",
        "self.rooms().gate_source_world().map(|_| other)",
        "self.rooms().gate_source_world().or(Some(room))",
    ] {
        assert!(!inherits_origin_qualification(&syn::parse_str(expression)?));
    }
    Ok(())
}

#[test]
fn owned_room_parser_rejects_qualification_and_accessor_gate_bypasses()
-> Result<(), Box<dyn std::error::Error>> {
    let valid: syn::ImplItemFn = syn::parse_str(
        "pub fn owned(room: Arc<QualifiedNativeEntryRoom>, qualified: bool) -> Self { Self { qualified: qualified.then(|| SharedOwner::Owned(Arc::clone(&room))), combat: Some(SharedOwner::Owned(room)) } }",
    )?;
    assert!(sets_owned_rooms(&valid));
    for body in [
        "Self { qualified: Some(SharedOwner::Owned(Arc::clone(&room))), combat: Some(SharedOwner::Owned(room)) }",
        "Self { qualified: true.then(|| SharedOwner::Owned(Arc::clone(&room))), combat: Some(SharedOwner::Owned(room)) }",
        "Self { qualified: qualified.then(|| SharedOwner::Owned(Arc::clone(&room))), combat: None }",
    ] {
        let function = syn::parse_str(&format!(
            "pub fn owned(room: Arc<QualifiedNativeEntryRoom>, qualified: bool) -> Self {{ {body} }}"
        ))?;
        assert!(!sets_owned_rooms(&function));
    }
    assert!(inherits_origin_qualification(&syn::parse_str(
        "self.rooms.gate_source_world().is_some()"
    )?));
    for expr in [
        "true",
        "self.rooms.gate_native_combat_cast().is_some()",
        "self.rooms.gate_source_world().is_none()",
    ] {
        assert!(!inherits_origin_qualification(&syn::parse_str(expr)?));
    }
    assert_eq!(
        field_room(&syn::parse_str("self.qualified.as_deref()")?),
        Some(Room::Qualified)
    );
    assert_eq!(
        field_room(&syn::parse_str("self.combat.as_deref()")?),
        Some(Room::Combat)
    );
    assert_eq!(
        field_room(&syn::parse_str(
            "self.qualified.or(self.combat).as_deref()"
        )?),
        None
    );
    Ok(())
}

#[test]
fn cake_bundle_fixture_keeps_its_qualified_room_closed() {
    let fixture = parse_source("src/gameplay_transport/postgres_tests/cake_phases.rs");
    let mut constructions = Constructions::default();
    constructions.visit_file(&fixture);
    assert!(matches!(constructions.0.as_slice(), [call]
        if matches!(call.args.first(), Some(syn::Expr::Path(path)) if path.path.is_ident("None"))));
}

#[test]
fn poor_soul_bundle_fixture_keeps_only_its_original_combat_room() {
    let fixture =
        parse_source("src/gameplay_transport/postgres_tests/srx3_poor_soul_world_corpse.rs");
    let mut constructions = Constructions::default();
    constructions.visit_file(&fixture);
    assert!(matches!(constructions.0.as_slice(), [call]
        if matches!(&*call.func, syn::Expr::Path(path)
            if path_name(&path.path).ends_with("SeamRooms::new"))
        && call.args.len() == 2
        && matches!(call.args.first(), Some(syn::Expr::Path(path))
            if path.path.is_ident("None"))
        && matches!(call.args.iter().nth(1), Some(syn::Expr::Call(some))
            if matches!(&*some.func, syn::Expr::Path(path) if path.path.is_ident("Some"))
            && some.args.len() == 1
            && matches!(some.args.first(), Some(syn::Expr::Reference(reference))
                if reference.mutability.is_none()
                && matches!(&*reference.expr, syn::Expr::Field(field)
                    if matches!(&field.member, syn::Member::Named(name) if name == "room")
                    && matches!(&*field.base, syn::Expr::Path(path)
                        if path.path.is_ident("scene")))))));
}

#[test]
fn qualification_guard_rejects_unconditional_or_inverted_bundle_room()
-> Result<(), Box<dyn std::error::Error>> {
    let correct = syn::parse_str::<syn::Expr>("bundle.is_none().then_some(&qualified_room)")?;
    assert!(withholds_from_a_bundle(&correct));
    for wrong in [
        "Some(&qualified_room)",
        "bundle.is_some().then_some(&qualified_room)",
        "bundle.is_none().then_some(&combat_room)",
        "bundle.is_none().then_some(&mut qualified_room)",
    ] {
        let candidate = syn::parse_str::<syn::Expr>(wrong)?;
        assert!(!withholds_from_a_bundle(&candidate), "{wrong}");
    }
    Ok(())
}

/// An independently owned source Bundle has no qualified entry-world room.
/// Its shared map owner may supply combat, never generic casting or House gates.
#[test]
fn a_source_instance_effective_room_constructs_only_none_some_combat() {
    // Source runtimes now use the canonical shared Activity owner. Its owned
    // room closes qualification; the separate inherited-owner path must retain
    // exactly its origin's qualification rather than opening it unconditionally.
    let source = parse_source("src/gameplay_transport/activity_content.rs");
    let mut constructions = Constructions::default();
    constructions.visit_file(&source);
    assert_eq!(
        constructions.0.len(),
        2,
        "exact source and inherited owned rooms"
    );
    let source = constructions.0[0];
    assert!(matches!(&*source.func, syn::Expr::Path(path)
        if path_name(&path.path).ends_with("SeamRooms::owned")));
    assert_eq!(source.args.len(), 2);
    assert!(
        matches!(source.args.iter().nth(1), Some(syn::Expr::Lit(value))
        if matches!(value.lit, syn::Lit::Bool(ref flag) if !flag.value))
    );
    assert!(matches!(source.args.first(), Some(syn::Expr::Call(clone))
        if matches!(&*clone.func, syn::Expr::Path(path)
            if path_name(&path.path) == "std::sync::Arc::clone")
        && clone.args.len() == 1
        && matches!(clone.args.first(), Some(syn::Expr::Reference(reference))
            if reference.mutability.is_none()
            && matches!(&*reference.expr, syn::Expr::Path(path)
                if path.path.is_ident("owned_room")))));
    let inherited = constructions.0[1];
    assert!(matches!(&*inherited.func, syn::Expr::Path(path)
        if path_name(&path.path).ends_with("SeamRooms::owned")));
    assert_eq!(inherited.args.len(), 2);
    assert!(
        inherited
            .args
            .iter()
            .nth(1)
            .is_some_and(inherits_origin_qualification)
    );
}

/// Compatibility names must not introduce a second room or movement owner.
#[test]
fn source_room_compatibility_accessors_delegate_to_canonical_owner()
-> Result<(), Box<dyn std::error::Error>> {
    let source = parse_source("src/gameplay_transport/effective_rooms.rs");
    let [syn::Item::Impl(block)] = source.items.as_slice() else {
        return Err("source room aliases must contain only their owner impl".into());
    };
    assert!(block.trait_.is_none() && block.unsafety.is_none());
    assert!(matches!(&*block.self_ty, syn::Type::Path(path)
        if path_name(&path.path) == "super::ComposedFreshAdmission"));
    assert_eq!(block.items.len(), 2);
    for (item, (alias, canonical)) in block.items.iter().zip([
        ("effective_movement_cells", "movement_cells"),
        ("effective_rooms", "rooms"),
    ]) {
        let syn::ImplItem::Fn(function) = item else {
            return Err("source room aliases may only declare delegate functions".into());
        };
        assert_eq!(function.sig.ident, alias);
        let [syn::Stmt::Expr(syn::Expr::MethodCall(call), None)] = function.block.stmts.as_slice()
        else {
            return Err("source room aliases must directly return the canonical owner".into());
        };
        assert_eq!(call.method, canonical);
        assert!(call.args.is_empty() && call.attrs.is_empty() && call.turbofish.is_none());
        assert!(matches!(&*call.receiver, syn::Expr::Path(path)
            if path.path.is_ident("self")));
    }
    Ok(())
}
