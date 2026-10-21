//! Gameplay layer and source ownership rules configured in workspace-boundaries.toml.

use super::{Policy, collect_rust_sources};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use syn::visit::{self, Visit};

#[derive(Debug, Default)]
pub(super) struct GameplayPolicy {
    layers: BTreeMap<String, u8>,
    pub(super) same_layer_edges: BTreeMap<String, BTreeSet<String>>,
    forbidden_normal_dependencies: Vec<String>,
    forbidden_name_fragments: Vec<String>,
    boss_identifier_paths: Vec<String>,
    boss_identifier_fragments: Vec<String>,
    /// Report only while the boss-code move is in progress. Switch the policy to `error`
    /// once the move lands so source ownership violations fail the workspace gate.
    pub(super) boss_identifier_mode: String,
}

impl GameplayPolicy {
    pub(super) fn add_layer(&mut self, key: &str, members: Vec<String>) -> Result<(), String> {
        let layer = key
            .strip_prefix('l')
            .and_then(|key| key.parse::<u8>().ok())
            .filter(|layer| *layer <= 5)
            .ok_or_else(|| format!("invalid gameplay layer {key}"))?;
        for member in members {
            if self.layers.insert(member.clone(), layer).is_some() {
                return Err(format!("duplicate gameplay layer assignment for {member}"));
            }
        }
        Ok(())
    }

    pub(super) fn set_rule(&mut self, key: &str, values: Vec<String>) -> Result<(), String> {
        match key {
            "forbidden_normal_dependencies" => self.forbidden_normal_dependencies = values,
            "forbidden_name_fragments" => self.forbidden_name_fragments = values,
            "boss_identifier_paths" => self.boss_identifier_paths = values,
            "boss_identifier_fragments" => self.boss_identifier_fragments = values,
            _ => return Err(format!("unknown gameplay rule {key}")),
        }
        Ok(())
    }
}

pub(super) fn validate(root: &Path, policy: &Policy, metadata: &Value) -> Result<(), String> {
    let gameplay = &policy.gameplay;
    validate_shape(policy)?;
    let mut violations = layer_violations(policy);
    violations.extend(purity_violations(gameplay, metadata)?);
    for (member, path) in &policy.member_paths {
        if path.starts_with("crates/gameplay/") && forbidden_name(gameplay, member) {
            violations.push(format!("forbidden gameplay package name: {member}"));
        }
    }
    collect_directory_violations(
        root,
        &root.join("crates/gameplay"),
        gameplay,
        &mut violations,
    )?;
    let mut files = Vec::new();
    for tree in &gameplay.boss_identifier_paths {
        collect_rust_sources(root, &root.join(tree), &mut files)?;
    }
    let findings = identifier_violations(gameplay, &files)?;
    match identifier_report(gameplay, &findings) {
        Ok(warnings) => {
            for warning in warnings {
                eprintln!("workspace-boundaries: WARN: {warning}");
            }
        }
        Err(error) => violations.push(error),
    }
    violations.sort();
    if violations.is_empty() {
        Ok(())
    } else {
        Err(format!("gameplay boundaries:\n{}", violations.join("\n")))
    }
}

fn validate_shape(policy: &Policy) -> Result<(), String> {
    let gameplay = &policy.gameplay;
    let expected = policy
        .member_paths
        .iter()
        .filter(|(_, path)| path.starts_with("crates/gameplay/"))
        .map(|(name, _)| name.clone())
        .collect::<BTreeSet<_>>();
    if gameplay.layers.keys().cloned().collect::<BTreeSet<_>>() != expected {
        return Err("gameplay layers must cover exactly the crates/gameplay members".to_owned());
    }
    for (member, targets) in &gameplay.same_layer_edges {
        let layer = gameplay.layers.get(member);
        for target in targets {
            if !matches!(layer, Some(2 | 3))
                || gameplay.layers.get(target) != layer
                || !policy
                    .edges
                    .get(member)
                    .is_some_and(|edges| edges.contains(target))
            {
                return Err(format!(
                    "invalid gameplay same-layer exception: {member} -> {target}"
                ));
            }
        }
    }
    if !matches!(gameplay.boss_identifier_mode.as_str(), "warn" | "error") {
        return Err("boss_identifier_mode must be warn or error".to_owned());
    }
    for tree in &gameplay.boss_identifier_paths {
        if !policy.member_paths.values().any(|path| path == tree)
            || !tree.starts_with("crates/gameplay/")
        {
            return Err(format!("unknown gameplay source ownership path: {tree}"));
        }
    }
    if gameplay.forbidden_name_fragments.is_empty()
        || gameplay.forbidden_normal_dependencies.is_empty()
        || gameplay.boss_identifier_paths.is_empty()
        || gameplay.boss_identifier_fragments.is_empty()
    {
        return Err(
            "gameplay rules must declare name, purity and source ownership guards".to_owned(),
        );
    }
    Ok(())
}

fn layer_violations(policy: &Policy) -> Vec<String> {
    let gameplay = &policy.gameplay;
    let mut violations = Vec::new();
    for (member, layer) in &gameplay.layers {
        if let Some(targets) = policy.edges.get(member) {
            for target in targets {
                if let Some(target_layer) = gameplay.layers.get(target) {
                    let allowed_peer = target_layer == layer
                        && gameplay
                            .same_layer_edges
                            .get(member)
                            .is_some_and(|edges| edges.contains(target));
                    if target_layer >= layer && !allowed_peer {
                        violations.push(format!("gameplay layer order: {member} (L{layer}) -> {target} (L{target_layer})"));
                    }
                }
            }
        }
    }
    violations
}

fn purity_violations(gameplay: &GameplayPolicy, metadata: &Value) -> Result<Vec<String>, String> {
    let packages = metadata
        .get("packages")
        .and_then(Value::as_array)
        .ok_or_else(|| "metadata lacks packages".to_owned())?;
    let mut violations = Vec::new();
    for package in packages {
        let name = package
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| "package lacks name".to_owned())?;
        if gameplay.layers.get(name) != Some(&3) {
            continue;
        }
        let dependencies = package
            .get("dependencies")
            .and_then(Value::as_array)
            .ok_or_else(|| format!("package {name} lacks dependencies"))?;
        for dependency in dependencies {
            if dependency.get("kind").and_then(Value::as_str) == Some("dev") {
                continue;
            }
            // Cargo reports the original package name, including for renamed dependencies.
            let target = dependency
                .get("name")
                .and_then(Value::as_str)
                .ok_or_else(|| format!("dependency in {name} lacks name"))?;
            if gameplay
                .forbidden_normal_dependencies
                .iter()
                .any(|name| name == target)
                || gameplay.layers.get(target).is_some_and(|layer| *layer >= 4)
            {
                violations.push(format!("L3 purity: {name} -> {target}"));
            }
        }
    }
    Ok(violations)
}

fn forbidden_name(gameplay: &GameplayPolicy, name: &str) -> bool {
    gameplay
        .forbidden_name_fragments
        .iter()
        .any(|fragment| name.contains(fragment))
}

fn collect_directory_violations(
    root: &Path,
    dir: &Path,
    gameplay: &GameplayPolicy,
    violations: &mut Vec<String>,
) -> Result<(), String> {
    for entry in
        fs::read_dir(dir).map_err(|error| format!("cannot read {}: {error}", dir.display()))?
    {
        let entry = entry.map_err(|error| format!("cannot read {}: {error}", dir.display()))?;
        let path = entry.path();
        if path.is_dir() {
            if forbidden_name(gameplay, &entry.file_name().to_string_lossy()) {
                violations.push(format!(
                    "forbidden gameplay directory: {}",
                    path.strip_prefix(root)
                        .map_err(|error| error.to_string())?
                        .display()
                ));
            }
            collect_directory_violations(root, &path, gameplay, violations)?;
        }
    }
    Ok(())
}

// Compare both snake_case and CamelCase identifiers with the configured snake_case fragments.
fn boss_identifier(gameplay: &GameplayPolicy, identifier: &str) -> bool {
    let compact = |value: &str| value.replace('_', "").to_lowercase();
    let identifier = compact(identifier);
    gameplay
        .boss_identifier_fragments
        .iter()
        .any(|fragment| identifier.contains(&compact(fragment)))
}

struct IdentifierVisitor<'a> {
    gameplay: &'a GameplayPolicy,
    identifiers: BTreeSet<String>,
}

impl IdentifierVisitor<'_> {
    fn record(&mut self, ident: &syn::Ident) {
        let name = ident.to_string();
        if boss_identifier(self.gameplay, &name) {
            self.identifiers.insert(name);
        }
    }
}

impl<'ast> Visit<'ast> for IdentifierVisitor<'_> {
    fn visit_signature(&mut self, node: &'ast syn::Signature) {
        self.record(&node.ident);
        visit::visit_signature(self, node);
    }

    fn visit_item_struct(&mut self, node: &'ast syn::ItemStruct) {
        self.record(&node.ident);
        visit::visit_item_struct(self, node);
    }

    fn visit_item_enum(&mut self, node: &'ast syn::ItemEnum) {
        self.record(&node.ident);
        visit::visit_item_enum(self, node);
    }

    fn visit_item_mod(&mut self, node: &'ast syn::ItemMod) {
        self.record(&node.ident);
        visit::visit_item_mod(self, node);
    }
}

fn identifier_violations(
    gameplay: &GameplayPolicy,
    files: &[(String, String)],
) -> Result<Vec<String>, String> {
    let mut violations = Vec::new();
    for (path, contents) in files {
        if Path::new(path)
            .file_stem()
            .and_then(|name| name.to_str())
            .is_some_and(|name| boss_identifier(gameplay, name))
        {
            violations.push(format!("{path}: boss-specific Rust file name"));
        }
        let syntax =
            syn::parse_file(contents).map_err(|error| format!("cannot parse {path}: {error}"))?;
        let mut visitor = IdentifierVisitor {
            gameplay,
            identifiers: BTreeSet::new(),
        };
        visitor.visit_file(&syntax);
        violations.extend(
            visitor
                .identifiers
                .into_iter()
                .map(|name| format!("{path}: boss-specific identifier {name}")),
        );
    }
    violations.sort();
    Ok(violations)
}

fn identifier_report(
    gameplay: &GameplayPolicy,
    findings: &[String],
) -> Result<Vec<String>, String> {
    match gameplay.boss_identifier_mode.as_str() {
        "warn" => Ok(findings.to_vec()),
        "error" if findings.is_empty() => Ok(Vec::new()),
        "error" => Err(findings.join("\n")),
        _ => Err("boss_identifier_mode must be warn or error".to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> Result<Policy, String> {
        super::super::parse_policy(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../workspace-boundaries.toml"),
        )
    }

    #[test]
    fn gameplay_layer_order_rejects_every_upward_or_unlisted_peer_edge() -> Result<(), String> {
        let mut policy = policy()?;
        policy.gameplay.layers.clear();
        policy.gameplay.same_layer_edges.clear();
        for source in 0..=5 {
            for target in 0..=5 {
                policy.gameplay.layers =
                    BTreeMap::from([("source".to_owned(), source), ("target".to_owned(), target)]);
                policy.edges =
                    BTreeMap::from([("source".to_owned(), BTreeSet::from(["target".to_owned()]))]);
                assert_eq!(
                    layer_violations(&policy).is_empty(),
                    target < source,
                    "L{source} -> L{target}"
                );
            }
        }
        Ok(())
    }

    #[test]
    fn gameplay_listed_l3_and_actor_carrier_edges_pass() -> Result<(), String> {
        let mut policy = policy()?;
        let edges = policy.gameplay.same_layer_edges.clone();
        policy.edges = edges;
        assert!(layer_violations(&policy).is_empty());
        Ok(())
    }

    #[test]
    fn gameplay_dev_edges_do_not_enter_layer_walk() -> Result<(), String> {
        let mut policy = policy()?;
        policy.edges.clear();
        policy.dev_edges.insert(
            "oteryn-gameplay-characters".to_owned(),
            BTreeSet::from(["oteryn-gameplay-dispatch".to_owned()]),
        );
        assert!(layer_violations(&policy).is_empty());
        Ok(())
    }

    #[test]
    fn gameplay_layers_cover_members_and_exceptions_are_bounded() -> Result<(), String> {
        let original = policy()?;
        validate_shape(&original)?;
        let mut missing = policy()?;
        missing.gameplay.layers.remove("oteryn-gameplay-domain");
        assert!(validate_shape(&missing).is_err());
        let mut unknown = policy()?;
        unknown
            .gameplay
            .layers
            .insert("oteryn-gameplay-invented".to_owned(), 0);
        assert!(validate_shape(&unknown).is_err());
        for (member, target) in [
            ("oteryn-gameplay-world", "oteryn-gameplay-durability"),
            ("oteryn-gameplay-characters", "oteryn-gameplay-quests"),
        ] {
            let mut invalid = policy()?;
            invalid
                .gameplay
                .same_layer_edges
                .insert(member.to_owned(), BTreeSet::from([target.to_owned()]));
            assert!(validate_shape(&invalid).is_err());
        }
        Ok(())
    }

    #[test]
    fn gameplay_layer_parser_rejects_duplicate_assignments_and_invalid_layers() {
        let mut gameplay = GameplayPolicy::default();
        assert!(gameplay.add_layer("l0", vec!["domain".to_owned()]).is_ok());
        assert!(gameplay.add_layer("l1", vec!["domain".to_owned()]).is_err());
        assert!(gameplay.add_layer("l6", Vec::new()).is_err());
        assert!(gameplay.add_layer("not-a-layer", Vec::new()).is_err());
    }

    #[test]
    fn gameplay_purity_checks_normal_build_renamed_and_target_specific_dependencies()
    -> Result<(), String> {
        let policy = policy()?;
        for target in [
            "sqlx",
            "tokio",
            "oteryn-gameplay-entities",
            "oteryn-gameplay-durability",
            "oteryn-gameplay-dispatch",
        ] {
            for kind in [
                Value::Null,
                Value::String("build".to_owned()),
                Value::String("dev".to_owned()),
            ] {
                let metadata = serde_json::json!({"packages": [{"name": "oteryn-gameplay-characters", "dependencies": [{"name": target, "rename": "alias", "kind": kind, "target": "cfg(windows)"}]}]});
                assert_eq!(
                    purity_violations(&policy.gameplay, &metadata)?.is_empty(),
                    kind == "dev",
                    "{target} {kind}"
                );
            }
        }
        let metadata = serde_json::json!({"packages": [{"name": "oteryn-gameplay-characters", "dependencies": [{"name": "oteryn-gameplay-domain", "kind": null}]}]});
        assert!(purity_violations(&policy.gameplay, &metadata)?.is_empty());
        Ok(())
    }

    #[test]
    fn gameplay_persistence_names_are_forbidden() -> Result<(), String> {
        let policy = policy()?;
        for name in [
            "persistence",
            "oteryn-gameplay-persistence",
            "old-persistence-adapter",
        ] {
            assert!(forbidden_name(&policy.gameplay, name));
        }
        assert!(!forbidden_name(&policy.gameplay, "durability"));
        // Walk real directories, including a nested directory that is not a Cargo member.
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(format!(".gameplay-directory-test-{}", std::process::id()));
        fs::create_dir_all(root.join("crates/gameplay/durability/src/old-persistence"))
            .map_err(|error| error.to_string())?;
        let mut violations = Vec::new();
        let result = collect_directory_violations(
            &root,
            &root.join("crates/gameplay"),
            &policy.gameplay,
            &mut violations,
        );
        fs::remove_dir_all(&root).map_err(|error| error.to_string())?;
        result?;
        assert_eq!(violations.len(), 1);
        assert!(violations[0].contains("old-persistence"));
        Ok(())
    }

    #[test]
    fn gameplay_boss_guard_checks_files_and_all_declaration_kinds() -> Result<(), String> {
        let policy = policy()?;
        let files = vec![(
            "src/smelly_cheese.rs".to_owned(),
            r#"
            // fn welter_in_a_comment() {}
            const EXAMPLE: &str = "struct BoneOverlord;";
            struct BoneOverlord;
            enum CrystalTentacle {}
            mod boreth { fn rum_barrel() {} }
            fn haragoth() { fn icicle() {} }
            trait Boss { fn welter(&self); }
            impl Boss for BoneOverlord { fn welter(&self) {} }
        "#
            .to_owned(),
        )];
        let violations = identifier_violations(&policy.gameplay, &files)?;
        assert_eq!(violations.len(), 8, "{violations:?}");
        for identifier in [
            "BoneOverlord",
            "CrystalTentacle",
            "boreth",
            "rum_barrel",
            "haragoth",
            "icicle",
            "welter",
        ] {
            assert!(
                violations
                    .iter()
                    .any(|finding| finding.ends_with(identifier))
            );
        }
        let clean = vec![(
            "src/actor.rs".to_owned(),
            "// mod welter;\nconst TEXT: &str = \"fn welter() {}\";".to_owned(),
        )];
        assert!(identifier_violations(&policy.gameplay, &clean)?.is_empty());
        Ok(())
    }

    #[test]
    fn gameplay_boss_guard_warns_or_errors_as_configured() -> Result<(), String> {
        let mut policy = policy()?;
        let findings = vec!["src/actor.rs: boss-specific identifier welter".to_owned()];
        assert_eq!(identifier_report(&policy.gameplay, &findings)?, findings);
        policy.gameplay.boss_identifier_mode = "error".to_owned();
        assert!(identifier_report(&policy.gameplay, &findings).is_err());
        assert!(identifier_report(&policy.gameplay, &[])?.is_empty());
        policy.gameplay.boss_identifier_mode = "typo".to_owned();
        assert!(identifier_report(&policy.gameplay, &[]).is_err());
        Ok(())
    }
}
