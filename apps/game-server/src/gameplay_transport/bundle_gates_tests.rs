//! BUNDLE-GATES-1: the inventory of every `qualified_room` gate of the gameplay seam.
//!
//! `node/serve.rs` hands the seam `qualified_room: None` on a bundle World and `combat_room`, the
//! entry room's bundle view (BUNDLE-COMBAT-ENABLE-1, #2025). A seam read of `combat_room` is a
//! gate open on a bundle World; a read of `qualified_room` is a gate closed there. Every read in
//! this module's sources is listed below: an open gate names the merged PR that opened it, a closed
//! gate names the plan batch that opens it. The sources are parsed with `syn`, and every mention of
//! either field in production code is counted, whatever its receiver (`x.combat_room`,
//! `(*self).combat_room`) or form (a declaration, initializer or destructuring pattern), at its own
//! site: its syntax path (naming each enclosing call, method and macro) and its position in its
//! function. One that is added, removed, moved or
//! switched without an update of this table fails `every_seam_room_read_is_inventoried`.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use syn::Token;
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::visit::{self, Visit};

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
    /// The read's syntax path (`Site::site`).
    site: &'static str,
    /// The read's position among its function's seam room mentions (`Site::ordinal`).
    ordinal: usize,
    state: Gate,
}

#[allow(clippy::too_many_arguments)]
const fn read(
    gate: &'static str,
    plan: &'static str,
    file: &'static str,
    function: &'static str,
    field: &'static str,
    site: &'static str,
    ordinal: usize,
    state: Gate,
) -> Read {
    Read {
        gate,
        plan,
        file,
        function,
        field,
        site,
        ordinal,
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
        "<ComposedFreshAdmission as FreshAdmissionAuthority>::attack_target",
        "self.combat_room",
        "method_call:set_target/field/read",
        1,
        COMBAT_ENABLE,
    ),
    read(
        "auto-attack drain",
        "auto-attack drain",
        "attack.rs",
        "ComposedFreshAdmission::drain_auto_attacks",
        "self.combat_room",
        "let_else/field/read",
        1,
        COMBAT_ENABLE,
    ),
    read(
        "native combat cast",
        "native combat cast",
        "native_combat_cast.rs",
        "super::super::ComposedFreshAdmission::cast_native_combat_inner",
        "self.combat_room",
        "let_else/field/read",
        1,
        COMBAT_ENABLE,
    ),
    read(
        "native combat cast commit",
        "native combat cast",
        "native_combat_cast.rs",
        "super::super::ComposedFreshAdmission::cast_native_combat_inner",
        "owner.combat_room",
        "let/await/method_call:run_with_context/closure/call:pin/async/let/try/method_call:ok_or/field/read",
        2,
        COMBAT_ENABLE,
    ),
    read(
        "ordinary combat timer continuation",
        "timer continuation",
        "spell_timer_callbacks.rs",
        "ComposedFreshAdmission::drain_native_ai_timers",
        "owner.combat_room",
        "if(Some saved unresolved)/then/try/method_call:map_err/await/method_call:run_with_context/closure/block/call:pin/async/let/try/method_call:ok_or/field/read",
        1,
        COMBAT_ENABLE,
    ),
    read(
        "monster AI",
        "monster AI",
        "monster_ai_cycle.rs",
        "ComposedFreshAdmission::drain_monster_melee",
        "self.combat_room",
        "let_else/field/read",
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
        "ComposedFreshAdmission::tick_spell_periodic",
        "self.combat_room",
        "let/try/field/read",
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
        "super::super::ComposedFreshAdmission::cast_parameters_inner",
        "self.combat_room",
        "if(parameter_room key self qualified_room self combat_room is_none)/method_call:is_none/call:parameter_room/field/read",
        2,
        Gate::Open {
            batch: "MAGIC-CAST-1",
            pr: 2112,
        },
    ),
    read(
        "parameter cast commit: locate message",
        "`locate_message` parameter cast",
        "parameter_cast.rs",
        "super::super::ComposedFreshAdmission::cast_parameters_inner",
        "owner.combat_room",
        "let/await/method_call:run_with_context/closure/call:pin/async/let/try/method_call:ok_or/call:parameter_room/field/read",
        4,
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
        "super::super::ComposedFreshAdmission::cast_parameters_inner",
        "self.qualified_room",
        "if(parameter_room key self qualified_room self combat_room is_none)/method_call:is_none/call:parameter_room/field/read",
        1,
        Gate::Closed {
            owners: &["MAGIC-SUMMON-1"],
        },
    ),
    read(
        "parameter cast commit: house access, vertical move",
        "`house_access` and `vertical_move` parameter casts",
        "parameter_cast.rs",
        "super::super::ComposedFreshAdmission::cast_parameters_inner",
        "owner.qualified_room",
        "let/await/method_call:run_with_context/closure/call:pin/async/let/try/method_call:ok_or/call:parameter_room/field/read",
        3,
        Gate::Closed {
            owners: &["MAGIC-SUMMON-1"],
        },
    ),
    // `random_item_grant`: SPELL-RT-WORLD-1 (MAGIC-SUMMON-1 graph); `conjure`: MAGIC-ITEMS-1.
    read(
        "world item cast",
        "world item casts",
        "world_item_cast.rs",
        "super::super::ComposedFreshAdmission::cast_world_items_inner",
        "self.qualified_room",
        "if(self qualified_room is_none)/method_call:is_none/field/read",
        1,
        Gate::Closed {
            owners: &["MAGIC-SUMMON-1", "MAGIC-ITEMS-1"],
        },
    ),
    read(
        "world item cast commit",
        "world item casts",
        "world_item_cast.rs",
        "super::super::ComposedFreshAdmission::cast_world_items_inner",
        "owner.qualified_room",
        "let/await/method_call:run_with_context/closure/call:pin/async/let/try/method_call:ok_or/field/read",
        2,
        Gate::Closed {
            owners: &["MAGIC-SUMMON-1", "MAGIC-ITEMS-1"],
        },
    ),
    // Field ingress is the WORLDINT-ADMIT-1 child of LOOP-WORLD-1 (plan row MAGIC-SUMMON-1).
    read(
        "field step ingress",
        "field step ingress",
        "field_step_ingress.rs",
        "ComposedFreshAdmission::step_with_field_ingress",
        "self.qualified_room",
        "let_else/field/read",
        1,
        Gate::Closed {
            owners: &["LOOP-WORLD-1"],
        },
    ),
    read(
        "field step ingress commit",
        "field step ingress",
        "field_step_ingress.rs",
        "ComposedFreshAdmission::step_with_field_ingress",
        "owner.qualified_room",
        "let/await/method_call:run_with_context/closure/call:pin/async/let/try/method_call:ok_or/field/read",
        2,
        Gate::Closed {
            owners: &["LOOP-WORLD-1"],
        },
    ),
    // The durable custody of the map's Items; on a bundle World that is its Ground persistence.
    read(
        "source map initialization",
        "source map initialization",
        "source_item_cycle.rs",
        "ComposedFreshAdmission::ensure_source_map_initialized",
        "self.qualified_room",
        "let_else/method_call:filter/field/read",
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
        "<ComposedFreshAdmission as FreshAdmissionAuthority>::observe_world_object_overlay",
        "self.qualified_room",
        "if(self qualified_room is_some_and room room source_world is_some self bundle_map is_some)/binary/method_call:is_some_and/field/read",
        1,
        Gate::SourceWorldBranch,
    ),
    read(
        "source World step blocking",
        "door overlay, step blocking, USE",
        "mod.rs",
        "<ComposedFreshAdmission as FreshAdmissionAuthority>::paced_step",
        "self.qualified_room",
        "let/if(self qualified_room is_some_and room room source_world is_some)/method_call:is_some_and/field/read",
        1,
        Gate::SourceWorldBranch,
    ),
    read(
        "source World use",
        "door overlay, step blocking, USE",
        "mod.rs",
        "<ComposedFreshAdmission as FreshAdmissionAuthority>::use_object",
        "self.qualified_room",
        "if(self qualified_room is_some_and room room source_world is_some)/method_call:is_some_and/field/read",
        1,
        Gate::SourceWorldBranch,
    ),
    read(
        "seam owners",
        "",
        "mod.rs",
        "serve_gameplay",
        "owners.qualified_room",
        "let/struct/field_value/field/read",
        2,
        Gate::Plumbing,
    ),
    read(
        "seam owners",
        "",
        "mod.rs",
        "serve_gameplay",
        "owners.combat_room",
        "let/struct/field_value/field/read",
        4,
        Gate::Plumbing,
    ),
    // The two fields of `GameplaySeamOwners` and of the composed owner, and the owner initializer.
    read(
        "seam field declarations",
        "",
        "mod.rs",
        "<item>",
        "qualified_room",
        "struct/declaration",
        1,
        Gate::Declaration,
    ),
    read(
        "seam field declarations",
        "",
        "mod.rs",
        "<item>",
        "qualified_room",
        "struct/declaration",
        3,
        Gate::Declaration,
    ),
    read(
        "seam owner initializer",
        "",
        "mod.rs",
        "serve_gameplay",
        "qualified_room",
        "let/struct/initializer",
        1,
        Gate::Declaration,
    ),
    read(
        "seam field declarations",
        "",
        "mod.rs",
        "<item>",
        "combat_room",
        "struct/declaration",
        2,
        Gate::Declaration,
    ),
    read(
        "seam field declarations",
        "",
        "mod.rs",
        "<item>",
        "combat_room",
        "struct/declaration",
        4,
        Gate::Declaration,
    ),
    read(
        "seam owner initializer",
        "",
        "mod.rs",
        "serve_gameplay",
        "combat_room",
        "let/struct/initializer",
        3,
        Gate::Declaration,
    ),
];

const FIELDS: &[&str] = &["qualified_room", "combat_room"];

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// One seam room mention of production code (`Scan`).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Site {
    /// The enclosing function, a method named with its impl (`Type::f`, `<Type as Trait>::f`) or
    /// trait (`Trait::f`); `<item>` outside any function.
    function: String,
    /// `<receiver>.<field>` for a field access (`<expr>` when the receiver is not a plain name),
    /// the bare `<field>` for any other form.
    field: String,
    /// The syntax path from the function body (or file) to the mention: the kind of each
    /// statement, expression and item it sits in.
    site: String,
    /// The mention's position among the function's mentions, in source order, from 1.
    ordinal: usize,
}

/// Every seam room mention of the module tree's production code, keyed by file. The sources are
/// the files the compiler loads for `gameplay_transport` outside tests: each production `mod`
/// declaration is followed from `mod.rs`, with its `#[path]` when it has one, wherever that file
/// lies. Test code is left out: a `#[cfg(test)]` module is never followed, and every
/// `#[cfg(test)]` item, variant, statement, expression, field or arm is skipped at any depth.
#[allow(clippy::expect_used, clippy::panic)]
fn module_reads() -> BTreeSet<(String, Site)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/gameplay_transport");
    let root = dir.canonicalize().expect("gameplay_transport directory");
    let mut pending = vec![(dir.join("mod.rs"), true)];
    let mut seen = BTreeSet::new();
    let mut reads = BTreeSet::new();
    while let Some((path, mod_rs)) = pending.pop() {
        let path = path.canonicalize().expect("module source");
        if !seen.insert(path.clone()) {
            continue;
        }
        let source = std::fs::read_to_string(&path).expect("source text");
        let scan = scan(&source);
        let parent = path.parent().expect("source directory");
        // A `mod.rs` or `#[path]` file holds its children beside it; `a.rs` holds them in `a/`.
        let children = if mod_rs {
            parent.to_path_buf()
        } else {
            parent.join(path.file_stem().expect("source stem"))
        };
        for module in &scan.modules {
            let base = module
                .inline
                .iter()
                .fold(children.clone(), |base, name| base.join(name));
            let next = match &module.path {
                Some(file) if module.inline.is_empty() => (parent.join(file), true),
                Some(file) => (base.join(file), true),
                None => {
                    let file = base.join(format!("{}.rs", module.name));
                    if file.exists() {
                        (file, false)
                    } else {
                        (base.join(&module.name).join("mod.rs"), true)
                    }
                }
            };
            pending.push(next);
        }
        let name = match path.strip_prefix(&root) {
            Ok(name) => name.to_str().expect("source name").to_owned(),
            Err(_) => {
                let src = root.parent().expect("src directory");
                let name = path.strip_prefix(src).expect("source under src");
                format!("../{}", name.to_str().expect("source name"))
            }
        }
        .replace('\\', "/");
        if let Some(site) = scan.sites.iter().find(|site| in_macro_rules(site)) {
            panic!(
                "{name}: `{}` is named inside a `macro_rules!` definition, where one inventoried \
                 site would stand for every invocation; write the read at its call site",
                site.field
            );
        }
        reads.extend(scan.sites.into_iter().map(|site| (name.clone(), site)));
    }
    reads
}

/// Whether a mention lies inside a `macro_rules!` definition: its expansion sites cannot be told
/// apart, so each invocation would add or move a gate without changing the inventory.
fn in_macro_rules(site: &Site) -> bool {
    site.site.split('/').any(|segment| segment == "macro_rules")
}

/// The seam room mentions of one source file, parsed with `syn`, and the production modules it
/// declares out of line (`mod x;`), each with its `#[path]` when it has one.
#[allow(clippy::expect_used)]
fn scan(source: &str) -> Scan {
    let file = syn::parse_file(source).expect("gameplay_transport source parses");
    let mut scan = Scan::default();
    scan.visit_file(&file);
    scan
}

#[derive(Default)]
struct Scan {
    sites: Vec<Site>,
    modules: Vec<Module>,
    inline: Vec<String>,
    functions: Vec<String>,
    /// The type or trait of each enclosing `impl` or `trait`, innermost last.
    owners: Vec<String>,
    path: Vec<String>,
}

/// A production `mod x;` declaration.
#[derive(Debug, PartialEq, Eq)]
struct Module {
    /// The inline `mod m { ... }` blocks it sits in, outermost first.
    inline: Vec<String>,
    name: String,
    /// Its `#[path = "..."]`.
    path: Option<String>,
}

impl Scan {
    fn function(&self) -> String {
        self.functions
            .last()
            .cloned()
            .unwrap_or_else(|| "<item>".to_owned())
    }

    fn record(&mut self, field: String, last: &'static str) {
        let function = self.function();
        let ordinal = 1 + self
            .sites
            .iter()
            .filter(|site| site.function == function)
            .count();
        let mut site = self.path.clone();
        site.push(last.to_owned());
        self.sites.push(Site {
            function,
            field,
            site: site.join("/"),
            ordinal,
        });
    }

    fn seam(ident: &syn::Ident) -> Option<String> {
        FIELDS
            .iter()
            .find(|field| ident == *field)
            .map(|field| (*field).to_owned())
    }

    fn within(&mut self, kind: impl Into<String>, visit: impl FnOnce(&mut Self)) {
        self.path.push(kind.into());
        visit(self);
        self.path.pop();
    }

    fn function_body(&mut self, name: &syn::Ident, visit: impl FnOnce(&mut Self)) {
        let path = std::mem::take(&mut self.path);
        // A method is named with its impl's type and trait (or its trait), so the same method name
        // in two impls of one file is two functions.
        // A function in an inline `mod m { ... }` is named with its module path, so the same name
        // in two inline modules of one file is two functions.
        let mut name = match self.owners.last() {
            Some(owner) => format!("{owner}::{name}"),
            None => name.to_string(),
        };
        if !self.inline.is_empty() {
            name = format!("{}::{name}", self.inline.join("::"));
        }
        self.functions.push(name);
        visit(self);
        self.functions.pop();
        self.path = path;
    }
}

/// Whether the attributes hold a `#[cfg(...)]` that is false whenever `test` is.
fn test_only(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        attr.path().is_ident("cfg")
            && attr
                .parse_args::<syn::Meta>()
                .is_ok_and(|predicate| needs_test(&predicate))
    })
}

/// A path's last segment name, without generic arguments: `B` for `a::B<'_>`.
fn path_name(path: &syn::Path) -> String {
    path.segments
        .last()
        .map_or_else(String::new, |segment| segment.ident.to_string())
}

/// A pattern's name: its path's last segment (`Some`, `Locate`), its binding, `_`, or the names of
/// an or-pattern's cases joined by `|`; `None` for a literal, range, tuple or slice.
fn pat_name(pat: &syn::Pat) -> Option<String> {
    match pat {
        syn::Pat::Ident(pat) => Some(pat.ident.to_string()),
        syn::Pat::Path(pat) => Some(path_name(&pat.path)),
        syn::Pat::TupleStruct(pat) => Some(path_name(&pat.path)),
        syn::Pat::Struct(pat) => Some(path_name(&pat.path)),
        syn::Pat::Wild(_) => Some("_".to_owned()),
        syn::Pat::Reference(pat) => pat_name(&pat.pat),
        syn::Pat::Paren(pat) => pat_name(&pat.pat),
        syn::Pat::Or(pat) => pat
            .cases
            .iter()
            .map(pat_name)
            .collect::<Option<Vec<_>>>()
            .map(|names| names.join("|")),
        _ => None,
    }
}

/// A path's segments as written, without generic arguments: `a::Owner` stays apart from `b::Owner`.
fn full_path(path: &syn::Path) -> String {
    let segments: Vec<String> = path
        .segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect();
    let leading = if path.leading_colon.is_some() {
        "::"
    } else {
        ""
    };
    format!("{leading}{}", segments.join("::"))
}

/// An impl's self type by its full path, without generic arguments or references.
fn type_name(ty: &syn::Type) -> String {
    match ty {
        syn::Type::Path(path) => full_path(&path.path),
        syn::Type::Reference(reference) => type_name(&reference.elem),
        syn::Type::Paren(paren) => type_name(&paren.elem),
        syn::Type::Group(group) => type_name(&group.elem),
        _ => "<type>".to_owned(),
    }
}

/// Whether a cfg predicate is false whenever `test` is: `test` itself, an `all(..)` with such a
/// term, or a non-empty `any(..)` of only such terms. `not(..)` and every other key may hold.
fn needs_test(predicate: &syn::Meta) -> bool {
    match predicate {
        syn::Meta::Path(path) => path.is_ident("test"),
        syn::Meta::List(list) => {
            let terms = list
                .parse_args_with(Punctuated::<syn::Meta, Token![,]>::parse_terminated)
                .unwrap_or_default();
            if list.path.is_ident("all") {
                terms.iter().any(needs_test)
            } else if list.path.is_ident("any") {
                !terms.is_empty() && terms.iter().all(needs_test)
            } else {
                false
            }
        }
        syn::Meta::NameValue(_) => false,
    }
}

/// Whether a cfg predicate holds in a production build, where `test` is false: `Some` when the
/// predicate depends on `test` alone, `None` when it names another key (a feature, a target).
fn holds_without_test(predicate: &syn::Meta) -> Option<bool> {
    match predicate {
        syn::Meta::Path(path) if path.is_ident("test") => Some(false),
        syn::Meta::List(list) => {
            let terms = list
                .parse_args_with(Punctuated::<syn::Meta, Token![,]>::parse_terminated)
                .ok()?;
            let terms = terms
                .iter()
                .map(holds_without_test)
                .collect::<Option<Vec<_>>>()?;
            if list.path.is_ident("not") {
                match terms.as_slice() {
                    [term] => Some(!term),
                    _ => None,
                }
            } else if list.path.is_ident("all") {
                Some(terms.iter().all(|term| *term))
            } else if list.path.is_ident("any") {
                Some(terms.iter().any(|term| *term))
            } else {
                None
            }
        }
        _ => None,
    }
}

/// The `path = "..."` value of an attribute's meta, if it is one.
fn path_value(meta: &syn::Meta) -> Option<String> {
    match meta {
        syn::Meta::NameValue(value) if value.path.is_ident("path") => match &value.value {
            syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(path),
                ..
            }) => Some(path.value()),
            _ => None,
        },
        _ => None,
    }
}

/// The path a production build loads a module declaration from: its `#[path = "..."]`, or the
/// `path` of a `#[cfg_attr(predicate, path = "...")]` whose predicate holds outside tests. A
/// `cfg_attr` path whose predicate names another key cannot be decided here and fails the guard.
#[allow(clippy::panic)]
fn module_path(attrs: &[syn::Attribute]) -> Option<String> {
    attrs.iter().find_map(|attr| {
        if !attr.path().is_ident("cfg_attr") {
            return path_value(&attr.meta);
        }
        let metas = attr
            .parse_args_with(Punctuated::<syn::Meta, Token![,]>::parse_terminated)
            .ok()?;
        let mut metas = metas.iter();
        let predicate = metas.next()?;
        let path = metas.find_map(path_value)?;
        match holds_without_test(predicate) {
            Some(true) => Some(path),
            Some(false) => None,
            None => panic!("cfg_attr path {path:?} depends on a key other than test"),
        }
    })
}

fn item_attrs(item: &syn::Item) -> &[syn::Attribute] {
    use syn::Item;
    match item {
        Item::Const(item) => &item.attrs,
        Item::Enum(item) => &item.attrs,
        Item::ExternCrate(item) => &item.attrs,
        Item::Fn(item) => &item.attrs,
        Item::ForeignMod(item) => &item.attrs,
        Item::Impl(item) => &item.attrs,
        Item::Macro(item) => &item.attrs,
        Item::Mod(item) => &item.attrs,
        Item::Static(item) => &item.attrs,
        Item::Struct(item) => &item.attrs,
        Item::Trait(item) => &item.attrs,
        Item::TraitAlias(item) => &item.attrs,
        Item::Type(item) => &item.attrs,
        Item::Union(item) => &item.attrs,
        Item::Use(item) => &item.attrs,
        _ => &[],
    }
}

fn impl_item_attrs(item: &syn::ImplItem) -> &[syn::Attribute] {
    match item {
        syn::ImplItem::Const(item) => &item.attrs,
        syn::ImplItem::Fn(item) => &item.attrs,
        syn::ImplItem::Type(item) => &item.attrs,
        syn::ImplItem::Macro(item) => &item.attrs,
        _ => &[],
    }
}

fn trait_item_attrs(item: &syn::TraitItem) -> &[syn::Attribute] {
    match item {
        syn::TraitItem::Const(item) => &item.attrs,
        syn::TraitItem::Fn(item) => &item.attrs,
        syn::TraitItem::Type(item) => &item.attrs,
        syn::TraitItem::Macro(item) => &item.attrs,
        _ => &[],
    }
}

/// An expression's kind and attributes.
fn expr_kind(expr: &syn::Expr) -> (&'static str, &[syn::Attribute]) {
    use syn::Expr;
    match expr {
        Expr::Array(expr) => ("array", &expr.attrs),
        Expr::Assign(expr) => ("assign", &expr.attrs),
        Expr::Async(expr) => ("async", &expr.attrs),
        Expr::Await(expr) => ("await", &expr.attrs),
        Expr::Binary(expr) => ("binary", &expr.attrs),
        Expr::Block(expr) => ("block", &expr.attrs),
        Expr::Break(expr) => ("break", &expr.attrs),
        Expr::Call(expr) => ("call", &expr.attrs),
        Expr::Cast(expr) => ("cast", &expr.attrs),
        Expr::Closure(expr) => ("closure", &expr.attrs),
        Expr::Const(expr) => ("const", &expr.attrs),
        Expr::Continue(expr) => ("continue", &expr.attrs),
        Expr::Field(expr) => ("field", &expr.attrs),
        Expr::ForLoop(expr) => ("for", &expr.attrs),
        Expr::Group(expr) => ("group", &expr.attrs),
        Expr::If(expr) => ("if", &expr.attrs),
        Expr::Index(expr) => ("index", &expr.attrs),
        Expr::Infer(expr) => ("infer", &expr.attrs),
        Expr::Let(expr) => ("let", &expr.attrs),
        Expr::Lit(expr) => ("lit", &expr.attrs),
        Expr::Loop(expr) => ("loop", &expr.attrs),
        Expr::Macro(expr) => ("macro", &expr.attrs),
        Expr::Match(expr) => ("match", &expr.attrs),
        Expr::MethodCall(expr) => ("method_call", &expr.attrs),
        Expr::Paren(expr) => ("paren", &expr.attrs),
        Expr::Path(expr) => ("path", &expr.attrs),
        Expr::Range(expr) => ("range", &expr.attrs),
        Expr::Reference(expr) => ("reference", &expr.attrs),
        Expr::Repeat(expr) => ("repeat", &expr.attrs),
        Expr::Return(expr) => ("return", &expr.attrs),
        Expr::Struct(expr) => ("struct", &expr.attrs),
        Expr::Try(expr) => ("try", &expr.attrs),
        Expr::TryBlock(expr) => ("try_block", &expr.attrs),
        Expr::Tuple(expr) => ("tuple", &expr.attrs),
        Expr::Unary(expr) => ("unary", &expr.attrs),
        Expr::Unsafe(expr) => ("unsafe", &expr.attrs),
        Expr::While(expr) => ("while", &expr.attrs),
        Expr::Yield(expr) => ("yield", &expr.attrs),
        _ => ("expr", &[]),
    }
}

impl<'ast> Visit<'ast> for Scan {
    fn visit_item(&mut self, item: &'ast syn::Item) {
        let attrs = item_attrs(item);
        if test_only(attrs) {
            return;
        }
        if let syn::Item::Mod(module) = item {
            let name = module.ident.to_string();
            if module.content.is_none() {
                self.modules.push(Module {
                    inline: self.inline.clone(),
                    name,
                    path: module_path(attrs),
                });
                return;
            }
            self.inline.push(name);
            self.within("mod", |scan| visit::visit_item(scan, item));
            self.inline.pop();
            return;
        }
        let kind = match item {
            syn::Item::Struct(_) => "struct",
            syn::Item::Impl(_) => "impl",
            syn::Item::Const(_) => "const",
            syn::Item::Static(_) => "static",
            _ => "item",
        };
        self.within(kind, |scan| visit::visit_item(scan, item));
    }

    fn visit_item_impl(&mut self, item: &'ast syn::ItemImpl) {
        let ty = type_name(&item.self_ty);
        let owner = match &item.trait_ {
            Some((_, trait_path, _)) => format!("<{ty} as {}>", full_path(trait_path)),
            None => ty,
        };
        self.owners.push(owner);
        visit::visit_item_impl(self, item);
        self.owners.pop();
    }

    fn visit_item_trait(&mut self, item: &'ast syn::ItemTrait) {
        self.owners.push(item.ident.to_string());
        visit::visit_item_trait(self, item);
        self.owners.pop();
    }

    fn visit_impl_item(&mut self, item: &'ast syn::ImplItem) {
        if !test_only(impl_item_attrs(item)) {
            visit::visit_impl_item(self, item);
        }
    }

    fn visit_trait_item(&mut self, item: &'ast syn::TraitItem) {
        if !test_only(trait_item_attrs(item)) {
            visit::visit_trait_item(self, item);
        }
    }

    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        self.function_body(&item.sig.ident, |scan| visit::visit_item_fn(scan, item));
    }

    fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
        self.function_body(&item.sig.ident, |scan| {
            visit::visit_impl_item_fn(scan, item);
        });
    }

    fn visit_trait_item_fn(&mut self, item: &'ast syn::TraitItemFn) {
        self.function_body(&item.sig.ident, |scan| {
            visit::visit_trait_item_fn(scan, item);
        });
    }

    fn visit_local(&mut self, local: &'ast syn::Local) {
        if test_only(&local.attrs) {
            return;
        }
        let diverges = local
            .init
            .as_ref()
            .is_some_and(|init| init.diverge.is_some());
        let kind = if diverges { "let_else" } else { "let" };
        self.within(kind, |scan| visit::visit_local(scan, local));
    }

    fn visit_expr(&mut self, expr: &'ast syn::Expr) {
        let (kind, attrs) = expr_kind(expr);
        if !test_only(attrs) {
            // A call names its callee, so a read moved into another operation is another site.
            let callee = match expr {
                syn::Expr::Call(call) => match &*call.func {
                    syn::Expr::Path(path) => path.path.segments.last().map(|s| s.ident.to_string()),
                    _ => None,
                },
                syn::Expr::MethodCall(call) => Some(call.method.to_string()),
                syn::Expr::Macro(mac) => mac.mac.path.segments.last().map(|s| s.ident.to_string()),
                _ => None,
            };
            // A control-flow node names the condition (or iterated pattern and source) it tests, so
            // a read moved to a sibling `if`, `while` or `for` is another site.
            let condition = match expr {
                syn::Expr::If(expr) => Some(fingerprint(|idents| idents.visit_expr(&expr.cond))),
                syn::Expr::While(expr) => Some(fingerprint(|idents| idents.visit_expr(&expr.cond))),
                syn::Expr::ForLoop(expr) => Some(fingerprint(|idents| {
                    idents.visit_pat(&expr.pat);
                    idents.visit_expr(&expr.expr);
                })),
                _ => None,
            };
            let kind = match (callee, condition) {
                (Some(callee), _) => format!("{kind}:{callee}"),
                (None, Some(condition)) => format!("{kind}({condition})"),
                (None, None) => kind.to_owned(),
            };
            self.within(kind, |scan| visit::visit_expr(scan, expr));
        }
    }

    fn visit_stmt(&mut self, stmt: &'ast syn::Stmt) {
        // A macro statement names its macro, as a macro expression does.
        if let syn::Stmt::Macro(mac) = stmt {
            if !test_only(&mac.attrs) {
                let name = mac.mac.path.segments.last().map(|s| s.ident.to_string());
                let kind = name.map_or_else(|| "macro".to_owned(), |name| format!("macro:{name}"));
                self.within(kind, |scan| visit::visit_stmt_macro(scan, mac));
            }
            return;
        }
        visit::visit_stmt(self, stmt);
    }

    fn visit_expr_field(&mut self, expr: &'ast syn::ExprField) {
        if let syn::Member::Named(member) = &expr.member
            && let Some(field) = Self::seam(member)
        {
            let receiver = match &*expr.base {
                syn::Expr::Path(path) => path.path.get_ident().map(ToString::to_string),
                _ => None,
            };
            let receiver = receiver.unwrap_or_else(|| "<expr>".to_owned());
            self.record(format!("{receiver}.{field}"), "read");
        }
        visit::visit_expr_field(self, expr);
    }

    fn visit_expr_path(&mut self, expr: &'ast syn::ExprPath) {
        if let Some(field) = expr.path.get_ident().and_then(Self::seam) {
            self.record(field, "name");
        }
        visit::visit_expr_path(self, expr);
    }

    fn visit_pat_ident(&mut self, pat: &'ast syn::PatIdent) {
        if let Some(field) = Self::seam(&pat.ident) {
            self.record(field, "binding");
        }
        visit::visit_pat_ident(self, pat);
    }

    fn visit_variant(&mut self, variant: &'ast syn::Variant) {
        if !test_only(&variant.attrs) {
            visit::visit_variant(self, variant);
        }
    }

    fn visit_field(&mut self, field: &'ast syn::Field) {
        if test_only(&field.attrs) {
            return;
        }
        if let Some(name) = field.ident.as_ref().and_then(Self::seam) {
            self.record(name, "declaration");
        }
        visit::visit_field(self, field);
    }

    fn visit_field_value(&mut self, value: &'ast syn::FieldValue) {
        if test_only(&value.attrs) {
            return;
        }
        if let syn::Member::Named(member) = &value.member
            && let Some(field) = Self::seam(member)
        {
            self.record(field, "initializer");
            if value.colon_token.is_none() {
                // `S { field }`: the shorthand's value is the same mention.
                return;
            }
        }
        self.within("field_value", |scan| visit::visit_field_value(scan, value));
    }

    fn visit_field_pat(&mut self, pat: &'ast syn::FieldPat) {
        if test_only(&pat.attrs) {
            return;
        }
        if let syn::Member::Named(member) = &pat.member
            && let Some(field) = Self::seam(member)
        {
            self.record(field, "pattern");
            if pat.colon_token.is_none() {
                return;
            }
        }
        visit::visit_field_pat(self, pat);
    }

    fn visit_expr_if(&mut self, expr: &'ast syn::ExprIf) {
        // The condition sits in the `if` itself; its two branches are told apart.
        for attr in &expr.attrs {
            self.visit_attribute(attr);
        }
        self.visit_expr(&expr.cond);
        self.within("then", |scan| scan.visit_block(&expr.then_branch));
        if let Some((_, other)) = &expr.else_branch {
            self.within("else", |scan| scan.visit_expr(other));
        }
    }

    fn visit_expr_match(&mut self, expr: &'ast syn::ExprMatch) {
        // Each production arm is keyed by its position and its pattern's name, so a read moved to
        // another branch of the same match is another site.
        for attr in &expr.attrs {
            self.visit_attribute(attr);
        }
        self.visit_expr(&expr.expr);
        let arms = expr.arms.iter().filter(|arm| !test_only(&arm.attrs));
        for (index, arm) in arms.enumerate() {
            let kind = match pat_name(&arm.pat) {
                Some(name) => format!("arm#{}:{name}", index + 1),
                None => format!("arm#{}", index + 1),
            };
            self.within(kind, |scan| visit::visit_arm(scan, arm));
        }
    }

    fn visit_arm(&mut self, arm: &'ast syn::Arm) {
        if !test_only(&arm.attrs) {
            visit::visit_arm(self, arm);
        }
    }

    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        let buffer = syn::buffer::TokenBuffer::new2(mac.tokens.clone());
        if mac.path.is_ident("macro_rules") {
            // A definition's mentions are marked so that `module_reads` refuses them.
            self.within("macro_rules", |scan| scan.macro_tokens(buffer.begin()));
        } else {
            self.macro_tokens(buffer.begin());
        }
    }
}

impl Scan {
    /// A macro body, or a delimited group inside one, is parsed as the comma-separated expressions
    /// most macros take, else as statements, else as items, and visited with its test code left
    /// out. A body that is none of these (a `macro_rules!` matcher, a transcriber with `$x`) is
    /// walked token by token: each group is tried the same way, a `#[cfg(test)]` attribute drops
    /// the tokens it governs (up to its `;`, `,` or braced body), and every other seam field name counts.
    fn macro_tokens(&mut self, cursor: syn::buffer::Cursor<'_>) {
        let tokens = cursor.token_stream();
        let exprs = Punctuated::<syn::Expr, Token![,]>::parse_terminated;
        if let Ok(exprs) = exprs.parse2(tokens.clone()) {
            self.within("macro", |scan| {
                exprs.iter().for_each(|expr| scan.visit_expr(expr));
            });
        } else if let Ok(stmts) = syn::Block::parse_within.parse2(tokens.clone()) {
            self.within("macro", |scan| {
                stmts.iter().for_each(|stmt| scan.visit_stmt(stmt));
            });
        } else if let Ok(file) = syn::parse2::<syn::File>(tokens) {
            self.within("macro", |scan| scan.visit_file(&file));
        } else {
            let mut cursor = cursor;
            while !cursor.eof() {
                if let Some((punct, next)) = cursor.punct()
                    && punct.as_char() == '#'
                    && let Some((inner, _, _, after)) = next.any_group()
                    && syn::parse2::<syn::Meta>(inner.token_stream())
                        .is_ok_and(|meta| meta.path().is_ident("cfg") && cfg_needs_test(&meta))
                {
                    cursor = skip_governed(after);
                } else if let Some((ident, next)) = cursor.ident() {
                    if let Some(field) = Self::seam(&ident) {
                        self.record(field, "macro_tokens");
                    }
                    cursor = next;
                } else if let Some((inner, _, _, next)) = cursor.any_group() {
                    self.macro_tokens(inner);
                    cursor = next;
                } else if let Some((_, next)) = cursor.token_tree() {
                    cursor = next;
                } else {
                    break;
                }
            }
        }
    }
}

/// The identifiers of a syntax node, in source order, joined by spaces.
fn fingerprint(visit: impl FnOnce(&mut Idents)) -> String {
    let mut idents = Idents::default();
    visit(&mut idents);
    idents.0.join(" ")
}

#[derive(Default)]
struct Idents(Vec<String>);

impl<'ast> Visit<'ast> for Idents {
    fn visit_ident(&mut self, ident: &'ast syn::Ident) {
        self.0.push(ident.to_string());
    }
}

/// Whether a `cfg(...)` meta's predicate is test-only.
fn cfg_needs_test(meta: &syn::Meta) -> bool {
    match meta {
        syn::Meta::List(list) => list.parse_args::<syn::Meta>().is_ok_and(|p| needs_test(&p)),
        _ => false,
    }
}

/// The cursor past the tokens a leading attribute governs: through the first `;` or `,` or
/// braced group at this level, or to the end. A `,` bounds one entry of a comma-delimited macro;
/// stopping early there only keeps test tokens in the count, never drops production ones. Commas
/// inside generic arguments (`call::<A, B>`, `<T as Trait<A, B>>::f`) do not bound the entry; if
/// those angle brackets never balance, the plain first `,` bounds it instead.
fn skip_governed(cursor: syn::buffer::Cursor<'_>) -> syn::buffer::Cursor<'_> {
    skip_entry(cursor, true).unwrap_or_else(|| skip_entry(cursor, false).unwrap_or(cursor))
}

/// [`skip_governed`]'s walk, optionally keeping commas inside generic arguments. A `<` opens
/// generic arguments inside others, at the start, or after a punctuation other than `<` or a
/// closing `>`, where it cannot be a comparison or shift; `>` closes one unless it ends `->`.
/// `None` when an opened `<` is still open at the end.
fn skip_entry(
    mut cursor: syn::buffer::Cursor<'_>,
    angles: bool,
) -> Option<syn::buffer::Cursor<'_>> {
    let mut depth = 0_usize;
    // The previous token when it was punctuation: its character and whether it closed a `<`.
    let mut previous: Option<(char, bool)> = None;
    let mut started = false;
    while !cursor.eof() {
        if let Some((punct, next)) = cursor.punct() {
            cursor = next;
            let ch = punct.as_char();
            let mut closed = false;
            if angles && ch == '<' {
                let opens =
                    depth > 0 || !started || matches!(previous, Some((p, false)) if p != '<');
                if opens {
                    depth += 1;
                }
            } else if angles && ch == '>' && depth > 0 && !matches!(previous, Some(('-', _))) {
                depth -= 1;
                closed = true;
            } else if depth == 0 && matches!(ch, ';' | ',') {
                return Some(cursor);
            }
            previous = Some((ch, closed));
        } else if let Some((_, delimiter, _, next)) = cursor.any_group() {
            cursor = next;
            previous = None;
            if depth == 0 && format!("{delimiter:?}") == "Brace" {
                return Some(cursor);
            }
        } else if let Some((_, next)) = cursor.token_tree() {
            cursor = next;
            previous = None;
        } else {
            break;
        }
        started = true;
    }
    (depth == 0).then_some(cursor)
}

#[allow(clippy::expect_used)]
fn plan() -> String {
    std::fs::read_to_string(repository_root().join(PLAN)).expect("completion plan")
}

#[test]
fn every_seam_room_read_is_inventoried() {
    let expected: BTreeSet<(String, Site)> = INVENTORY
        .iter()
        .map(|read| {
            let site = Site {
                function: read.function.to_owned(),
                field: read.field.to_owned(),
                site: read.site.to_owned(),
                ordinal: read.ordinal,
            };
            (read.file.to_owned(), site)
        })
        .collect();
    assert_eq!(
        module_reads(),
        expected,
        "a `qualified_room` or `combat_room` read of the seam changed: record it in INVENTORY \
         at its site (an opened gate names its batch and merged PR, a closed gate its owning batch)"
    );
}

#[test]
fn each_gate_reads_the_room_its_state_says() {
    for read in INVENTORY {
        // The field read, in any form: `x.combat_room`, a bare `combat_room` pattern or binding.
        let combat = read.field.rsplit('.').next() == Some("combat_room");
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
            // A declaration or initializer only: a destructuring pattern or binding is a read.
            Gate::Declaration => assert!(
                !read.field.contains('.')
                    && (read.site.ends_with("/declaration") || read.site.ends_with("/initializer")),
                "`{}`",
                read.gate
            ),
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
                clause(
                    read,
                    &["are not gates".to_owned(), "`bundle_map`".to_owned()]
                ),
                "the plan row has no clause recording that the source-World branch `{}` is not a \
                 gate because a bundle World is served by `bundle_map` ({})",
                read.gate,
                read.plan
            ),
            Gate::Plumbing | Gate::Declaration => {}
        }
    }
}

/// The production code of one source file: its struct literals and free functions, with every
/// `#[cfg(test)]` item, statement, expression, field and arm left out.
#[derive(Default)]
struct Production<'ast> {
    structs: Vec<&'ast syn::ExprStruct>,
    functions: Vec<&'ast syn::ItemFn>,
}

#[allow(clippy::expect_used)]
fn parse_source(relative: &str) -> syn::File {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(relative);
    let source = std::fs::read_to_string(&path).expect("production source");
    syn::parse_file(&source).expect("production source parses")
}

fn production(file: &syn::File) -> Production<'_> {
    let mut production = Production::default();
    production.visit_file(file);
    production
}

impl<'ast> Visit<'ast> for Production<'ast> {
    fn visit_item(&mut self, item: &'ast syn::Item) {
        if !test_only(item_attrs(item)) {
            visit::visit_item(self, item);
        }
    }

    fn visit_impl_item(&mut self, item: &'ast syn::ImplItem) {
        if !test_only(impl_item_attrs(item)) {
            visit::visit_impl_item(self, item);
        }
    }

    fn visit_trait_item(&mut self, item: &'ast syn::TraitItem) {
        if !test_only(trait_item_attrs(item)) {
            visit::visit_trait_item(self, item);
        }
    }

    fn visit_local(&mut self, local: &'ast syn::Local) {
        if !test_only(&local.attrs) {
            visit::visit_local(self, local);
        }
    }

    fn visit_stmt_macro(&mut self, mac: &'ast syn::StmtMacro) {
        if !test_only(&mac.attrs) {
            visit::visit_stmt_macro(self, mac);
        }
    }

    fn visit_expr(&mut self, expr: &'ast syn::Expr) {
        if !test_only(expr_kind(expr).1) {
            visit::visit_expr(self, expr);
        }
    }

    fn visit_field_value(&mut self, value: &'ast syn::FieldValue) {
        if !test_only(&value.attrs) {
            visit::visit_field_value(self, value);
        }
    }

    fn visit_arm(&mut self, arm: &'ast syn::Arm) {
        if !test_only(&arm.attrs) {
            visit::visit_arm(self, arm);
        }
    }

    fn visit_variant(&mut self, variant: &'ast syn::Variant) {
        if !test_only(&variant.attrs) {
            visit::visit_variant(self, variant);
        }
    }

    fn visit_expr_struct(&mut self, expr: &'ast syn::ExprStruct) {
        self.structs.push(expr);
        visit::visit_expr_struct(self, expr);
    }

    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        self.functions.push(item);
        visit::visit_item_fn(self, item);
    }
}

fn is_name(expr: &syn::Expr, name: &str) -> bool {
    matches!(expr, syn::Expr::Path(path) if path.path.is_ident(name))
}

/// `bundle.is_none().then_some(&qualified_room)`.
fn withholds_from_a_bundle(expr: &syn::Expr) -> bool {
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

#[test]
fn a_bundle_world_seam_has_no_qualified_room() {
    let serve = parse_source("src/node/serve.rs");
    let owners: Vec<&syn::ExprStruct> = production(&serve)
        .structs
        .into_iter()
        .filter(|init| path_name(&init.path) == "GameplaySeamOwners")
        .collect();
    assert!(
        !owners.is_empty(),
        "serve.rs no longer builds `GameplaySeamOwners` in production code"
    );
    for init in owners {
        let room = init.fields.iter().find(
            |field| matches!(&field.member, syn::Member::Named(name) if name == "qualified_room"),
        );
        assert!(
            room.is_some_and(|field| withholds_from_a_bundle(&field.expr)),
            "a `GameplaySeamOwners` initializer in serve.rs no longer sets `qualified_room: \
             bundle.is_none().then_some(&qualified_room)`: every closed gate would open at once \
             without its owner"
        );
    }
}

/// `match key { "locate_message" => combat, _ => qualified }`, and nothing else.
fn opens_only_the_locate_message(function: &syn::ItemFn) -> bool {
    let [syn::Stmt::Expr(syn::Expr::Match(choice), None)] = function.block.stmts.as_slice() else {
        return false;
    };
    let [locate, other] = choice.arms.as_slice() else {
        return false;
    };
    let locate_key = matches!(
        &locate.pat,
        syn::Pat::Lit(syn::ExprLit { lit: syn::Lit::Str(key), .. }) if key.value() == "locate_message"
    );
    is_name(&choice.expr, "key")
        && locate_key
        && locate.guard.is_none()
        && is_name(&locate.body, "combat")
        && matches!(other.pat, syn::Pat::Wild(_))
        && other.guard.is_none()
        && is_name(&other.body, "qualified")
}

#[test]
fn parameter_room_opens_only_the_locate_message() {
    let source = parse_source("src/gameplay_transport/parameter_cast.rs");
    let rooms: Vec<&syn::ItemFn> = production(&source)
        .functions
        .into_iter()
        .filter(|function| function.sig.ident == "parameter_room")
        .collect();
    assert!(
        matches!(rooms.as_slice(), [room] if opens_only_the_locate_message(room)),
        "`parameter_room` changed which parameter keys read the combat room: record the opened \
         keys in INVENTORY"
    );
}

fn site(function: &str, field: &str, site: &str, ordinal: usize) -> Site {
    Site {
        function: function.to_owned(),
        field: field.to_owned(),
        site: site.to_owned(),
        ordinal,
    }
}

#[test]
fn every_test_only_item_is_left_out_at_any_depth() {
    let scan = scan(
        r#"
        struct Owner<'a> {
            combat_room: Option<&'a Room>,
            #[cfg(test)]
            qualified_room: Option<&'a Room>,
        }
        #[cfg(test)]
        struct Fixture { qualified_room: u8 }
        enum Probe {
            Live,
            #[cfg(test)]
            Fixture { qualified_room: u8 },
        }
        #[cfg(test)]
        const ROOM: Owner<'static> = Owner { combat_room: None };
        #[cfg(test)]
        static PROBE: fn(&Owner) -> bool = |owner| owner.combat_room.is_some();
        #[cfg(test)]
        impl Owner<'_> {
            fn test_read(&self) { let _ = self.combat_room; }
        }
        #[cfg(test)]
        mod inline { fn read(owner: &super::Owner) { owner.qualified_room; } }
        #[cfg(test)]
        mod out_of_line;
        #[cfg(test)]
        #[path = "elsewhere_checks.rs"]
        mod elsewhere;
        #[path = "../shared/live.rs"]
        mod live;
        mod nested {
            impl super::Owner<'_> {
                #[cfg(test)]
                fn nested_test(&self) { self.combat_room; }
                #[cfg(test)]
                const NESTED: u8 = 0;
            }
        }
        impl Owner<'_> {
            fn production(&self) {
                #[cfg(test)]
                let _probe = self.qualified_room;
                #[cfg(test)]
                probe(self.qualified_room);
                match 0 {
                    #[cfg(test)]
                    1 => drop(self.qualified_room),
                    _ => {}
                }
                let room = self.combat_room;
            }
        }
        "#,
    );
    assert_eq!(
        scan.sites,
        [
            site("<item>", "combat_room", "struct/declaration", 1),
            site("Owner::production", "self.combat_room", "let/field/read", 1),
        ]
    );
    // Only the production module is followed, wherever its `#[path]` points.
    assert_eq!(
        scan.modules,
        [Module {
            inline: Vec::new(),
            name: "live".to_owned(),
            path: Some("../shared/live.rs".to_owned()),
        }]
    );
}

#[test]
fn each_read_in_a_function_is_its_own_site() {
    let gate = scan(
        "impl Owner { fn cast(&self) { \
             let Some(_room) = self.combat_room else { return }; \
             commit(self.combat_room); \
         } }",
    );
    assert_eq!(
        gate.sites,
        [
            site("Owner::cast", "self.combat_room", "let_else/field/read", 1),
            site(
                "Owner::cast",
                "self.combat_room",
                "call:commit/field/read",
                2
            ),
        ]
    );
    // The preflight gate removed and another read of the same form added later in the function:
    // the same function, form and count, but not the same sites.
    let moved = scan(
        "impl Owner { fn cast(&self) { \
             commit(self.combat_room); \
             let _later = self.combat_room; \
         } }",
    );
    assert_eq!(moved.sites.len(), gate.sites.len());
    assert_ne!(moved.sites, gate.sites);
    // Two reads spelled alike in one function are told apart by their position.
    let twice = scan(
        "fn drain(owner: &Owner) { use_room(owner.combat_room); use_room(owner.combat_room); }",
    );
    assert_eq!(
        twice.sites,
        [
            site("drain", "owner.combat_room", "call:use_room/field/read", 1),
            site("drain", "owner.combat_room", "call:use_room/field/read", 2),
        ]
    );
    // The same read moved into another operation is another site.
    let validated = scan("fn attack(&self) { validate_attack(self.combat_room); }");
    let traded = scan("fn attack(&self) { commit_trade(self.combat_room); }");
    assert_ne!(validated.sites, traded.sites);
    let method = scan("fn attack(&self) { self.combat_room.ok_or(Missing)?; }");
    assert_eq!(
        method.sites,
        [site(
            "attack",
            "self.combat_room",
            "try/method_call:ok_or/field/read",
            1
        )]
    );
}

#[test]
fn every_production_module_is_followed_through_its_path() {
    let sources: BTreeSet<String> = module_reads().into_iter().map(|(file, _)| file).collect();
    // `item_view.rs` loads `interaction/corpse_open.rs` by `#[path]`; it is in the scan whether or
    // not it reads a room, so check the module list rather than the reads.
    let item_view = scan(
        &std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("src/gameplay_transport/item_view.rs"),
        )
        .unwrap_or_default(),
    );
    assert!(
        item_view
            .modules
            .iter()
            .any(|module| module.name == "corpse_open"
                && module.path.as_deref() == Some("../interaction/corpse_open.rs"))
    );
    // A test module declared by `#[path]` is never followed.
    assert!(
        !item_view
            .modules
            .iter()
            .any(|module| module.name == "tests")
    );
    assert!(sources.iter().all(|file| !file.ends_with("_tests.rs")));
}

#[test]
fn a_method_is_named_with_its_impl() {
    let first = scan(
        "impl Attack { fn drain(&self) { let Some(_r) = self.combat_room else { return }; } } \
         impl Trade { fn drain(&self) {} }",
    );
    let moved = scan(
        "impl Attack { fn drain(&self) {} } \
         impl Trade { fn drain(&self) { let Some(_r) = self.combat_room else { return }; } }",
    );
    assert_eq!(first.sites[0].function, "Attack::drain");
    assert_eq!(moved.sites[0].function, "Trade::drain");
    let traited = scan("impl<'a> Seam for Owner<'a> { fn room(&self) { self.combat_room; } }");
    assert_eq!(traited.sites[0].function, "<Owner as Seam>::room");
    let pathed = scan(
        "impl a::Owner { fn room(&self) { self.combat_room; } } \
         impl b::Owner { fn room(&self) {} } \
         impl x::Seam for b::Owner { fn hook(&self) { self.combat_room; } }",
    );
    let functions: Vec<&str> = pathed
        .sites
        .iter()
        .map(|site| site.function.as_str())
        .collect();
    assert_eq!(functions, ["a::Owner::room", "<b::Owner as x::Seam>::hook"]);
}

#[test]
fn every_test_only_cfg_predicate_is_left_out() {
    let scan = scan(
        "#[cfg(all(test))] struct A { combat_room: u8 } \
         #[cfg(any(test, test))] struct B { combat_room: u8 } \
         #[cfg(all(test, feature = \"x\"))] struct C { combat_room: u8 } \
         #[cfg(not(test))] struct D { combat_room: u8 } \
         #[cfg(any(test, feature = \"x\"))] struct E { combat_room: u8 }",
    );
    // `not(test)` and `any(test, ..)` with a non-test term may hold outside tests.
    assert_eq!(
        scan.sites,
        [
            site("<item>", "combat_room", "struct/declaration", 1),
            site("<item>", "combat_room", "struct/declaration", 2),
        ]
    );
}

#[test]
fn a_destructured_combat_room_is_a_read_of_an_open_gate() {
    let scan = scan("fn drain(owner: &Owner) { let Owner { combat_room, .. } = owner; }");
    let read = &scan.sites[0];
    assert_eq!(read.field, "combat_room");
    assert_eq!(read.site, "let/pattern");
    // `each_gate_reads_the_room_its_state_says` takes it as a combat_room read, and not as a
    // declaration.
    assert_eq!(read.field.rsplit('.').next(), Some("combat_room"));
    assert!(!(read.site.ends_with("/declaration") || read.site.ends_with("/initializer")));
}

#[test]
fn a_read_moved_to_another_arm_is_another_site() {
    let attack = scan(
        "fn route(&self, m: Msg) { match m { Msg::Attack => use_room(self.combat_room), \
         Msg::Trade => {} } }",
    );
    let trade = scan(
        "fn route(&self, m: Msg) { match m { Msg::Attack => {} \
         Msg::Trade => use_room(self.combat_room) } }",
    );
    assert_eq!(
        attack.sites,
        [site(
            "route",
            "self.combat_room",
            "match/arm#1:Attack/call:use_room/field/read",
            1
        )]
    );
    assert_eq!(
        trade.sites,
        [site(
            "route",
            "self.combat_room",
            "match/arm#2:Trade/call:use_room/field/read",
            1
        )]
    );
}

#[test]
fn test_code_inside_a_macro_body_is_left_out() {
    let rules = scan(
        "macro_rules! gate { ($x:expr) => { \
         #[cfg(test)] fn probe(o: &O) { o.combat_room; } \
         $x.qualified_room } }",
    );
    assert_eq!(
        rules.sites,
        [site(
            "<item>",
            "qualified_room",
            "item/macro_rules/macro_tokens",
            1
        )]
    );
    // A field named in a definition is refused, as an invocation never names it.
    assert!(rules.sites.iter().all(in_macro_rules));
    let hidden = scan(
        "macro_rules! room { ($o:expr) => { $o.combat_room } } \
         fn drain(&self) { room!(self); }",
    );
    assert!(hidden.sites.iter().any(in_macro_rules));
    let invoked = scan("fn drain(&self) { gate!(self.combat_room); }");
    assert!(!invoked.sites.iter().any(in_macro_rules));
    let items = scan(
        "items! { #[cfg(test)] fn probe(o: &O) { o.combat_room; } \
         fn production(o: &O) { o.qualified_room; } }",
    );
    assert_eq!(
        items.sites,
        [site("production", "o.qualified_room", "field/read", 1)]
    );
}

#[test]
fn a_cfg_attr_path_is_the_one_production_loads() {
    let scan = scan(
        "#[cfg_attr(not(test), path = \"prod.rs\")] mod room; \
         #[cfg_attr(test, path = \"probe.rs\")] mod other;",
    );
    assert_eq!(
        scan.modules,
        [
            Module {
                inline: vec![],
                name: "room".to_owned(),
                path: Some("prod.rs".to_owned()),
            },
            Module {
                inline: vec![],
                name: "other".to_owned(),
                path: None,
            },
        ]
    );
}

#[test]
fn a_function_is_named_with_its_inline_module() {
    let scan = scan(
        "mod a { fn drain(o: &O) { o.combat_room; } } \
         mod b { fn drain(o: &O) { o.qualified_room; } }",
    );
    assert_eq!(
        scan.sites,
        [
            site("a::drain", "o.combat_room", "field/read", 1),
            site("b::drain", "o.qualified_room", "field/read", 1),
        ]
    );
}

#[test]
#[allow(clippy::expect_used)]
fn only_the_production_seam_owners_initializer_counts() {
    let file = syn::parse_file(
        "fn serve() { let owners = GameplaySeamOwners { qualified_room: Some(&qualified_room) }; } \
         fn helper() { let _ = GameplaySeamOwners { \
             qualified_room: bundle.is_none().then_some(&qualified_room) }; } \
         #[cfg(test)] fn probe() { let _ = GameplaySeamOwners { qualified_room: None }; } \
         enum Probe { #[cfg(test)] Exposed = { let _ = GameplaySeamOwners { qualified_room: None }; 0 } }",
    )
    .expect("source parses");
    let verdicts: Vec<bool> = production(&file)
        .structs
        .into_iter()
        .filter_map(|init| init.fields.first())
        .map(|field| withholds_from_a_bundle(&field.expr))
        .collect();
    // The exposing initializer fails even beside a withholding helper; the test-only one is not seen.
    assert_eq!(verdicts, [false, true]);
}

#[test]
#[allow(clippy::expect_used)]
fn parameter_room_is_judged_by_its_own_match() {
    let judge = |source: &str| {
        let file = syn::parse_file(source).expect("source parses");
        production(&file)
            .functions
            .first()
            .is_some_and(|function| opens_only_the_locate_message(function))
    };
    assert!(judge(
        "fn parameter_room() { match key { \"locate_message\" => combat, _ => qualified, } }"
    ));
    assert!(!judge(
        "fn parameter_room() { match key { \"locate_message\" | \"house_access\" => combat, \
         _ => qualified, } }"
    ));
    assert!(!judge(
        "fn parameter_room() { let _ = match key { \"locate_message\" => combat, \
         _ => qualified, }; combat }"
    ));
}

#[test]
fn a_read_moved_to_a_sibling_branch_is_another_site() {
    let first =
        scan("fn tick(&self) { if armed { consume(self.combat_room); } if idle { stop(); } }");
    let second =
        scan("fn tick(&self) { if armed { stop(); } if idle { consume(self.combat_room); } }");
    let otherwise =
        scan("fn tick(&self) { if armed { stop(); } else { consume(self.combat_room); } }");
    let looped = scan("fn tick(&self) { while armed { consume(self.combat_room); } }");
    let sites = |scan: &Scan| {
        scan.sites
            .iter()
            .map(|site| site.site.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(sites(&first), ["if(armed)/then/call:consume/field/read"]);
    assert_eq!(sites(&second), ["if(idle)/then/call:consume/field/read"]);
    assert_eq!(
        sites(&otherwise),
        ["if(armed)/else/block/call:consume/field/read"]
    );
    assert_eq!(sites(&looped), ["while(armed)/call:consume/field/read"]);
}

#[test]
fn a_test_only_macro_entry_ends_at_its_comma() {
    let scan = scan(
        "fn route(o: &O) { routes! { #[cfg(test)] probe => o.combat_room, live => o.qualified_room, } }",
    );
    let fields: Vec<&str> = scan.sites.iter().map(|site| site.field.as_str()).collect();
    assert_eq!(fields, ["qualified_room"]);
}

#[test]
fn a_test_only_macro_entry_keeps_its_generic_arguments() {
    let generic = scan(
        "fn route(o: &O) { routes! { \
             #[cfg(test)] probe => call::<A, B>(o.combat_room), \
             #[cfg(test)] cast => <T as Tr<A, B>>::f(o.combat_room), \
             #[cfg(test)] ret => run::<fn(A) -> B, C>(o.combat_room), \
             live => o.qualified_room, } }",
    );
    let fields: Vec<&str> = generic
        .sites
        .iter()
        .map(|site| site.field.as_str())
        .collect();
    assert_eq!(fields, ["qualified_room"]);
    let compared = scan(
        "fn route(o: &O) { routes! { #[cfg(test)] probe => a < b, live => o.qualified_room, } }",
    );
    assert_eq!(compared.sites.len(), 1);
}
