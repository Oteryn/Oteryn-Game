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
        "ComposedFreshAdmission::cast_native_combat_inner",
        "self.combat_room",
        "let_else/field/read",
        1,
        COMBAT_ENABLE,
    ),
    read(
        "native combat cast commit",
        "native combat cast",
        "native_combat_cast.rs",
        "ComposedFreshAdmission::cast_native_combat_inner",
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
        "if/try/method_call:map_err/await/method_call:run_with_context/closure/block/call:pin/async/let/try/method_call:ok_or/field/read",
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
        "ComposedFreshAdmission::cast_parameters_inner",
        "self.combat_room",
        "if/method_call:is_none/call:parameter_room/field/read",
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
        "ComposedFreshAdmission::cast_parameters_inner",
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
        "ComposedFreshAdmission::cast_parameters_inner",
        "self.qualified_room",
        "if/method_call:is_none/call:parameter_room/field/read",
        1,
        Gate::Closed {
            owners: &["MAGIC-SUMMON-1"],
        },
    ),
    read(
        "parameter cast commit: house access, vertical move",
        "`house_access` and `vertical_move` parameter casts",
        "parameter_cast.rs",
        "ComposedFreshAdmission::cast_parameters_inner",
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
        "ComposedFreshAdmission::cast_world_items_inner",
        "self.qualified_room",
        "if/method_call:is_none/field/read",
        1,
        Gate::Closed {
            owners: &["MAGIC-SUMMON-1", "MAGIC-ITEMS-1"],
        },
    ),
    read(
        "world item cast commit",
        "world item casts",
        "world_item_cast.rs",
        "ComposedFreshAdmission::cast_world_items_inner",
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
        "if/binary/method_call:is_some_and/field/read",
        1,
        Gate::SourceWorldBranch,
    ),
    read(
        "source World step blocking",
        "door overlay, step blocking, USE",
        "mod.rs",
        "<ComposedFreshAdmission as FreshAdmissionAuthority>::paced_step",
        "self.qualified_room",
        "let/if/method_call:is_some_and/field/read",
        1,
        Gate::SourceWorldBranch,
    ),
    read(
        "source World use",
        "door overlay, step blocking, USE",
        "mod.rs",
        "<ComposedFreshAdmission as FreshAdmissionAuthority>::use_object",
        "self.qualified_room",
        "if/method_call:is_some_and/field/read",
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
#[allow(clippy::expect_used)]
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
        reads.extend(scan.sites.into_iter().map(|site| (name.clone(), site)));
    }
    reads
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
        let name = match self.owners.last() {
            Some(owner) => format!("{owner}::{name}"),
            None => name.to_string(),
        };
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

/// An impl's self type by its last path segment, without generic arguments or references.
fn type_name(ty: &syn::Type) -> String {
    match ty {
        syn::Type::Path(path) => path_name(&path.path),
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

/// The `#[path = "..."]` of a module declaration.
fn module_path(attrs: &[syn::Attribute]) -> Option<String> {
    attrs.iter().find_map(|attr| match &attr.meta {
        syn::Meta::NameValue(value) if value.path.is_ident("path") => match &value.value {
            syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(path),
                ..
            }) => Some(path.value()),
            _ => None,
        },
        _ => None,
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
            Some((_, trait_path, _)) => format!("<{ty} as {}>", path_name(trait_path)),
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
            let kind = callee.map_or_else(|| kind.to_owned(), |callee| format!("{kind}:{callee}"));
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

    fn visit_arm(&mut self, arm: &'ast syn::Arm) {
        if !test_only(&arm.attrs) {
            self.within("arm", |scan| visit::visit_arm(scan, arm));
        }
    }

    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        // A macro body is parsed as the comma-separated expressions most macros take; a body that
        // is not one still counts every seam field name in its tokens.
        let parser = Punctuated::<syn::Expr, Token![,]>::parse_terminated;
        match mac.parse_body_with(parser) {
            Ok(exprs) => self.within("macro", |scan| {
                exprs.iter().for_each(|expr| scan.visit_expr(expr));
            }),
            Err(_) => {
                let tokens = mac.tokens.to_string();
                for field in FIELDS {
                    let words = tokens
                        .split(|c: char| !(c.is_alphanumeric() || c == '_'))
                        .filter(|word| word == field)
                        .count();
                    for _ in 0..words {
                        self.record((*field).to_owned(), "macro_tokens");
                    }
                }
            }
        }
    }
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
