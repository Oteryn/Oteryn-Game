//! Machine validation for workspace membership, roles and internal edges.

use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path};
use std::process::Command;

#[derive(Debug)]
struct Policy {
    members: BTreeSet<String>,
    paths: BTreeSet<String>,
    member_paths: BTreeMap<String, String>,
    production: BTreeSet<String>,
    production_roots: BTreeSet<String>,
    synthetic: BTreeSet<String>,
    test: BTreeSet<String>,
    tool: BTreeSet<String>,
    forbidden_fragments: Vec<String>,
    edges: BTreeMap<String, BTreeSet<String>>,
}

struct MemberPathMapping {
    members: BTreeSet<String>,
    paths: BTreeSet<String>,
    mapping: BTreeMap<String, String>,
}

pub fn validate_workspace(root: &Path) -> Result<(), String> {
    let policy = parse_policy(&root.join("workspace-boundaries.toml"))?;
    validate_policy_shape(&policy)?;
    let metadata = cargo_metadata(root)?;
    let actual = validate_workspace_package_paths(&policy, &metadata)?;
    let actual_edges = internal_edges(&metadata, &actual)?;
    for member in &policy.members {
        let expected = policy
            .edges
            .get(member)
            .ok_or_else(|| format!("missing edge declaration for {member}"))?;
        let observed = actual_edges.get(member).cloned().unwrap_or_default();
        if &observed != expected {
            return Err(format!(
                "internal edges differ for {member}: expected {expected:?}, actual {observed:?}"
            ));
        }
    }
    validate_acyclic(&policy.edges)?;
    validate_production_closure(&policy)?;
    for member in &policy.members {
        if policy
            .forbidden_fragments
            .iter()
            .any(|fragment| member.contains(fragment))
        {
            return Err(format!(
                "forbidden package name entered workspace: {member}"
            ));
        }
    }
    Ok(())
}

fn validate_workspace_package_paths(
    policy: &Policy,
    metadata: &Value,
) -> Result<BTreeSet<String>, String> {
    let actual_paths = workspace_package_paths(metadata)?;
    let actual = actual_paths.keys().cloned().collect::<BTreeSet<_>>();
    if actual != policy.members {
        return Err(format!(
            "workspace members differ: expected {:?}, actual {:?}",
            policy.members, actual
        ));
    }
    if actual_paths != policy.member_paths {
        return Err(format!(
            "workspace package paths differ: expected {:?}, actual {:?}",
            policy.member_paths, actual_paths
        ));
    }
    Ok(actual)
}

fn parse_policy(path: &Path) -> Result<Policy, String> {
    let content = fs::read_to_string(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let mut arrays: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut forbidden_fragments = Vec::new();
    let mut edges = BTreeMap::new();
    let mut in_edges = false;
    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line == "[edges]" {
            in_edges = true;
            continue;
        }
        let Some((key, value)) = line.split_once(" = ") else {
            continue;
        };
        if key == "schema_version" {
            if value != "1" {
                return Err("unsupported workspace boundary schema".to_owned());
            }
            continue;
        }
        let parsed: Vec<String> = serde_json::from_str(value)
            .map_err(|error| format!("invalid array for {key}: {error}"))?;
        if in_edges {
            edges.insert(key.to_owned(), parsed.into_iter().collect());
        } else if key == "forbidden_package_fragments" {
            forbidden_fragments = parsed;
        } else {
            arrays.insert(key.to_owned(), parsed);
        }
    }
    let take = |key: &str| {
        arrays
            .get(key)
            .cloned()
            .ok_or_else(|| format!("missing policy array {key}"))
    };
    let members = take("members")?;
    let paths = take("paths")?;
    let member_paths = member_path_mapping(&members, &paths)?;
    let take_set = |key: &str| take(key).map(|values| values.into_iter().collect());
    Ok(Policy {
        members: member_paths.members,
        paths: member_paths.paths,
        member_paths: member_paths.mapping,
        production: take_set("production")?,
        production_roots: take_set("production_roots")?,
        synthetic: take_set("synthetic")?,
        test: take_set("test")?,
        tool: take_set("tool")?,
        forbidden_fragments,
        edges,
    })
}

fn member_path_mapping(
    members: &[String],
    paths: &[String],
) -> Result<MemberPathMapping, String> {
    if members.len() != paths.len() {
        return Err(format!(
            "workspace policy member/path cardinality differs: {} members, {} paths",
            members.len(),
            paths.len()
        ));
    }

    let member_set = members.iter().cloned().collect::<BTreeSet<_>>();
    if member_set.len() != members.len() {
        return Err("workspace policy contains a duplicate package name".to_owned());
    }
    let path_set = paths.iter().cloned().collect::<BTreeSet<_>>();
    if path_set.len() != paths.len() {
        return Err("workspace policy contains a duplicate package path".to_owned());
    }

    let mapping = members
        .iter()
        .cloned()
        .zip(paths.iter().cloned())
        .collect();
    Ok(MemberPathMapping {
        members: member_set,
        paths: path_set,
        mapping,
    })
}

fn validate_policy_shape(policy: &Policy) -> Result<(), String> {
    if policy.members.is_empty() {
        return Err("workspace policy must contain at least one member".to_owned());
    }
    if policy.paths.len() != policy.members.len() {
        return Err(format!(
            "workspace policy member/path cardinality differs: {} members, {} paths",
            policy.members.len(),
            policy.paths.len()
        ));
    }
    if policy.member_paths.keys().cloned().collect::<BTreeSet<_>>() != policy.members
        || policy.member_paths.values().cloned().collect::<BTreeSet<_>>() != policy.paths
    {
        return Err("workspace policy package/path mapping is ambiguous".to_owned());
    }
    if policy.production_roots.is_empty() {
        return Err("workspace policy must declare at least one production root".to_owned());
    }
    if !policy.production_roots.is_subset(&policy.production) {
        return Err("every production root must have the production release role".to_owned());
    }

    let role_sets = [
        &policy.production,
        &policy.synthetic,
        &policy.test,
        &policy.tool,
    ];
    let mut union = BTreeSet::new();
    for role in role_sets {
        for member in role {
            if !union.insert(member.clone()) {
                return Err(format!(
                    "package appears in multiple release roles: {member}"
                ));
            }
        }
    }
    if union != policy.members {
        return Err("release roles do not partition the workspace members".to_owned());
    }
    if policy.edges.keys().cloned().collect::<BTreeSet<_>>() != policy.members {
        return Err("edge declarations do not cover every workspace member".to_owned());
    }
    Ok(())
}

fn cargo_metadata(root: &Path) -> Result<Value, String> {
    let output = Command::new("cargo")
        .current_dir(root)
        .args([
            "metadata",
            "--locked",
            "--format-version",
            "1",
            "--manifest-path",
            "Cargo.toml",
        ])
        .output()
        .map_err(|error| format!("cannot execute cargo metadata: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "cargo metadata failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("cargo metadata JSON is invalid: {error}"))
}

fn workspace_package_paths(metadata: &Value) -> Result<BTreeMap<String, String>, String> {
    let member_ids = metadata
        .get("workspace_members")
        .and_then(Value::as_array)
        .ok_or_else(|| "metadata lacks workspace_members".to_owned())?
        .iter()
        .filter_map(Value::as_str)
        .collect::<BTreeSet<_>>();
    let packages = metadata
        .get("packages")
        .and_then(Value::as_array)
        .ok_or_else(|| "metadata lacks packages".to_owned())?;
    let workspace_root = metadata
        .get("workspace_root")
        .and_then(Value::as_str)
        .ok_or_else(|| "metadata lacks workspace_root".to_owned())?;
    let workspace_root = Path::new(workspace_root);
    let mut paths = BTreeMap::new();
    let mut observed_ids = BTreeSet::new();
    for package in packages {
        let id = package
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| "package lacks id".to_owned())?;
        if member_ids.contains(id) {
            observed_ids.insert(id);
            let name = package
                .get("name")
                .and_then(Value::as_str)
                .ok_or_else(|| "package lacks name".to_owned())?;
            let manifest_path = package
                .get("manifest_path")
                .and_then(Value::as_str)
                .ok_or_else(|| format!("workspace package {name} lacks manifest_path"))?;
            let manifest_directory = Path::new(manifest_path)
                .parent()
                .ok_or_else(|| format!("workspace package {name} has invalid manifest_path"))?
                .strip_prefix(workspace_root)
                .map_err(|_| {
                    format!(
                        "workspace package {name} manifest is outside workspace root: {manifest_path}"
                    )
                })?;
            let manifest_directory = portable_relative_path(manifest_directory)?;
            if paths
                .insert(name.to_owned(), manifest_directory)
                .is_some()
            {
                return Err(format!(
                    "workspace metadata contains duplicate package name {name}"
                ));
            }
        }
    }
    if observed_ids.len() != member_ids.len() {
        return Err("metadata does not describe every workspace member".to_owned());
    }
    Ok(paths)
}

fn portable_relative_path(path: &Path) -> Result<String, String> {
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => parts.push(
                part.to_str()
                    .ok_or_else(|| "workspace manifest path is not UTF-8".to_owned())?,
            ),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err("workspace manifest path is not a relative directory".to_owned());
            }
        }
    }
    if parts.is_empty() {
        return Err("workspace package manifest cannot be at workspace root".to_owned());
    }
    Ok(parts.join("/"))
}

fn internal_edges(
    metadata: &Value,
    members: &BTreeSet<String>,
) -> Result<BTreeMap<String, BTreeSet<String>>, String> {
    let packages = metadata
        .get("packages")
        .and_then(Value::as_array)
        .ok_or_else(|| "metadata lacks packages".to_owned())?;
    let mut edges = BTreeMap::new();
    for package in packages {
        let name = package
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| "package lacks name".to_owned())?;
        if !members.contains(name) {
            continue;
        }
        let mut dependencies = BTreeSet::new();
        for dependency in package
            .get("dependencies")
            .and_then(Value::as_array)
            .ok_or_else(|| format!("package {name} lacks dependencies"))?
        {
            if dependency.get("path").is_some_and(|value| !value.is_null()) {
                let dependency_name = dependency
                    .get("name")
                    .and_then(Value::as_str)
                    .ok_or_else(|| format!("dependency in {name} lacks name"))?;
                if members.contains(dependency_name) {
                    dependencies.insert(dependency_name.to_owned());
                }
            }
        }
        edges.insert(name.to_owned(), dependencies);
    }
    Ok(edges)
}

fn validate_acyclic(edges: &BTreeMap<String, BTreeSet<String>>) -> Result<(), String> {
    fn visit(
        node: &str,
        edges: &BTreeMap<String, BTreeSet<String>>,
        active: &mut BTreeSet<String>,
        complete: &mut BTreeSet<String>,
    ) -> Result<(), String> {
        if complete.contains(node) {
            return Ok(());
        }
        if !active.insert(node.to_owned()) {
            return Err(format!("internal dependency cycle reaches {node}"));
        }
        if let Some(children) = edges.get(node) {
            for child in children {
                visit(child, edges, active, complete)?;
            }
        }
        active.remove(node);
        complete.insert(node.to_owned());
        Ok(())
    }

    let mut active = BTreeSet::new();
    let mut complete = BTreeSet::new();
    for node in edges.keys() {
        visit(node, edges, &mut active, &mut complete)?;
    }
    Ok(())
}

fn validate_production_closure(policy: &Policy) -> Result<(), String> {
    let mut pending = policy.production_roots.iter().cloned().collect::<Vec<_>>();
    let mut visited = BTreeSet::new();
    while let Some(package) = pending.pop() {
        if !visited.insert(package.clone()) {
            continue;
        }
        if !policy.production.contains(&package) {
            return Err(format!(
                "production closure reaches non-production package {package}"
            ));
        }
        if let Some(dependencies) = policy.edges.get(&package) {
            pending.extend(dependencies.iter().cloned());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn structural_policy() -> Policy {
        Policy {
            members: BTreeSet::from(["app".to_owned(), "foundation".to_owned()]),
            paths: BTreeSet::from(["apps/app".to_owned(), "crates/foundation".to_owned()]),
            member_paths: BTreeMap::from([
                ("app".to_owned(), "apps/app".to_owned()),
                ("foundation".to_owned(), "crates/foundation".to_owned()),
            ]),
            production: BTreeSet::from(["app".to_owned(), "foundation".to_owned()]),
            production_roots: BTreeSet::from(["app".to_owned()]),
            synthetic: BTreeSet::new(),
            test: BTreeSet::new(),
            tool: BTreeSet::new(),
            forbidden_fragments: vec!["canary".to_owned()],
            edges: BTreeMap::from([
                ("app".to_owned(), BTreeSet::from(["foundation".to_owned()])),
                ("foundation".to_owned(), BTreeSet::new()),
            ]),
        }
    }

    fn metadata_with_paths(app_path: &str, foundation_path: &str) -> Value {
        serde_json::json!({
            "workspace_root": "/workspace",
            "workspace_members": ["app-id", "foundation-id"],
            "packages": [
                {
                    "id": "app-id",
                    "name": "app",
                    "manifest_path": format!("/workspace/{app_path}/Cargo.toml")
                },
                {
                    "id": "foundation-id",
                    "name": "foundation",
                    "manifest_path": format!("/workspace/{foundation_path}/Cargo.toml")
                }
            ]
        })
    }

    #[test]
    fn checked_in_policy_is_structurally_valid() -> Result<(), String> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .ok_or_else(|| "cannot resolve workspace root".to_owned())?;
        let policy = parse_policy(&root.join("workspace-boundaries.toml"))?;
        validate_policy_shape(&policy)
    }

    #[test]
    fn checked_in_workspace_metadata_matches_declared_paths() -> Result<(), String> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .ok_or_else(|| "cannot resolve workspace root".to_owned())?;
        let policy = parse_policy(&root.join("workspace-boundaries.toml"))?;
        let metadata = cargo_metadata(root)?;
        validate_workspace_package_paths(&policy, &metadata)?;
        Ok(())
    }

    #[test]
    fn same_cardinality_wrong_manifest_path_fails() {
        let metadata = metadata_with_paths("apps/wrong", "crates/foundation");
        assert!(validate_workspace_package_paths(&structural_policy(), &metadata).is_err());
    }

    #[test]
    fn same_path_set_paired_to_wrong_packages_fails() {
        let metadata = metadata_with_paths("crates/foundation", "apps/app");
        assert!(validate_workspace_package_paths(&structural_policy(), &metadata).is_err());
    }

    #[test]
    fn duplicate_policy_package_or_path_fails_closed() {
        let members = vec!["app".to_owned(), "app".to_owned()];
        let paths = vec!["apps/app".to_owned(), "apps/other".to_owned()];
        assert!(member_path_mapping(&members, &paths).is_err());

        let members = vec!["app".to_owned(), "foundation".to_owned()];
        let paths = vec!["apps/app".to_owned(), "apps/app".to_owned()];
        assert!(member_path_mapping(&members, &paths).is_err());
    }

    #[test]
    fn duplicate_metadata_package_name_fails_closed() {
        let mut metadata = metadata_with_paths("apps/app", "crates/foundation");
        metadata["packages"][1]["name"] = Value::String("app".to_owned());
        assert!(workspace_package_paths(&metadata).is_err());
    }

    #[test]
    fn structural_policy_does_not_freeze_member_count() -> Result<(), String> {
        validate_policy_shape(&structural_policy())
    }

    #[test]
    fn production_root_cannot_reach_synthetic_member() {
        let mut policy = structural_policy();
        policy.members.insert("fixture".to_owned());
        policy.paths.insert("crates/fixture".to_owned());
        policy
            .member_paths
            .insert("fixture".to_owned(), "crates/fixture".to_owned());
        policy.synthetic.insert("fixture".to_owned());
        policy.edges.insert("fixture".to_owned(), BTreeSet::new());
        if let Some(dependencies) = policy.edges.get_mut("app") {
            dependencies.insert("fixture".to_owned());
        }

        assert!(validate_policy_shape(&policy).is_ok());
        assert!(validate_production_closure(&policy).is_err());
    }

    #[test]
    fn production_root_must_have_production_role() {
        let mut policy = structural_policy();
        policy.production_roots = BTreeSet::from(["fixture".to_owned()]);
        assert!(validate_policy_shape(&policy).is_err());
    }
}
