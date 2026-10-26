//! CLIENT-STRUCTURE-TARGET-1 §4.1 and accepted §6: direct client edges and purity.

use super::{collect_rust_sources, validate_acyclic};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use syn::visit::{self, Visit};

fn allowed(name: &str) -> Option<&'static [&'static str]> {
    Some(match name {
        "foundation" | "input-actions" | "protocol-oteryn" => &[],
        "client-domain" => &["foundation", "input-actions"],
        "client-simulation" => &["client-domain"],
        "client-runtime" => &[
            "client-domain",
            "client-simulation",
            "session",
            "protocol-oteryn",
            "diagnostics",
            "foundation",
        ],
        "session" => &["protocol-oteryn"],
        "session-tcp" => &["session"],
        "platform-client" => &["diagnostics", "foundation", "platform-contracts"],
        "renderer" => &["client-domain", "foundation"],
        "input-platform" => &["input-actions"],
        "client-assets" => &[],
        "client-ui" => &[
            "client-domain",
            "client-runtime",
            "renderer",
            "input-actions",
            "client-assets",
        ],
        _ => return None,
    })
}

pub fn validate(root: &Path, metadata: &Value) -> Result<(), String> {
    let packages = metadata["packages"].as_array().ok_or("metadata packages")?;
    let members: BTreeSet<_> = metadata["workspace_members"]
        .as_array()
        .ok_or("workspace members")?
        .iter()
        .filter_map(Value::as_str)
        .collect();
    let internal: BTreeSet<_> = packages
        .iter()
        .filter(|p| p["id"].as_str().is_some_and(|id| members.contains(id)))
        .filter_map(|p| p["name"].as_str())
        .collect();
    let mut edges = BTreeMap::new();
    for package in packages
        .iter()
        .filter(|p| p["id"].as_str().is_some_and(|id| members.contains(id)))
    {
        let full = package["name"].as_str().ok_or("package name")?;
        let name = full.strip_prefix("oteryn-").unwrap_or(full);
        let deps = package["dependencies"].as_array().ok_or("dependencies")?;
        let mut targets = BTreeSet::new();
        for dep in deps {
            let target = dep["name"].as_str().ok_or("dependency name")?;
            if internal.contains(target) {
                if dep["kind"] != "dev" {
                    targets.insert(target.to_owned());
                }
                if let Some(allowed) = allowed(name) {
                    let short = target.strip_prefix("oteryn-").unwrap_or(target);
                    let test_utility = dep["kind"] == "dev"
                        && ((name == "client-simulation" && short == "foundation")
                            || (name == "protocol-oteryn" && short == "test-support"));
                    if !allowed.contains(&short) && !test_utility {
                        return Err(format!("client layering: {name} -> {short} is forbidden"));
                    }
                }
            }
            // Domain/simulation purity also forbids third-party IO/async dependencies.
            if matches!(
                name,
                "client-domain" | "client-simulation" | "input-actions" | "client-ui"
            ) && [
                "tokio", "reqwest", "wgpu", "winit", "windows", "rustix", "libc", "socket2",
                "ureq", "hyper",
            ]
            .iter()
            .any(|b| target == *b || target.starts_with(&format!("{b}-")))
            {
                return Err(format!("client purity: {name} -> {target}"));
            }
        }
        edges.insert(full.to_owned(), targets);
    }
    validate_acyclic(&edges)?;
    let baseline_path = root.join("tools/architecture-check/client-purity-baseline.json");
    let baseline = if baseline_path.exists() {
        super::client_fragments::read(&baseline_path)?
    } else {
        serde_json::json!({})
    };
    for name in [
        "foundation",
        "input-actions",
        "protocol-oteryn",
        "client-domain",
        "client-simulation",
        "client-ui",
    ] {
        let dir = root.join(format!("crates/{name}/src"));
        if !dir.is_dir() {
            continue;
        }
        let mut files = Vec::new();
        collect_rust_sources(root, &dir, &mut files)?;
        for (path, contents) in files {
            if path.contains("/tests/")
                || path.rsplit('/').next().is_some_and(|n| n.contains("tests"))
            {
                continue;
            }
            let syntax = syn::parse_file(&contents).map_err(|e| format!("{path}: {e}"))?;
            let mut scan = Purity {
                ui: name == "client-ui",
                bans: BTreeMap::new(),
            };
            scan.visit_file(&syntax);
            // Exact existing imports are ratcheted; a new API/path cannot borrow
            // an exception for an unrelated runtime or IO API.
            for (import, count) in &mut scan.bans {
                let ceiling = baseline[&path][import].as_u64().unwrap_or(0);
                *count = count.saturating_sub(usize::try_from(ceiling).map_err(|e| e.to_string())?);
            }
            if scan.bans.values().any(|count| *count > 0) {
                return Err(format!("client purity: {path}: {:?}", scan.bans));
            }
        }
    }
    Ok(())
}

struct Purity {
    ui: bool,
    bans: BTreeMap<String, usize>,
}

impl Purity {
    fn path(&mut self, parts: &[String]) {
        let path = parts.join("::");
        let banned: &[&str] = if self.ui {
            &[
                "std::net",
                "tokio::net",
                "reqwest",
                "oteryn_session",
                "oteryn_session_tcp",
                "oteryn_protocol_oteryn",
                "oteryn_platform_client",
            ]
        } else {
            &[
                "std::net",
                "std::fs",
                "std::os",
                "tokio",
                "reqwest",
                "wgpu",
                "winit",
                "windows",
                "libc",
                "rustix",
                "oteryn_protocol_oteryn",
            ]
        };
        for ban in banned {
            if path == *ban || path.starts_with(&format!("{ban}::")) {
                *self.bans.entry(path.clone()).or_default() += 1;
            }
        }
    }

    fn imports(&mut self, prefix: Vec<String>, tree: &syn::UseTree) {
        match tree {
            syn::UseTree::Path(p) => {
                let mut prefix = prefix;
                prefix.push(p.ident.to_string());
                self.imports(prefix, &p.tree);
            }
            syn::UseTree::Group(g) => {
                for t in &g.items {
                    self.imports(prefix.clone(), t);
                }
            }
            syn::UseTree::Name(n) => {
                let mut p = prefix;
                p.push(n.ident.to_string());
                self.path(&p);
            }
            syn::UseTree::Rename(n) => {
                let mut p = prefix;
                p.push(n.ident.to_string());
                self.path(&p);
            }
            syn::UseTree::Glob(_) => self.path(&prefix),
        }
    }
}

impl<'ast> Visit<'ast> for Purity {
    fn visit_item(&mut self, node: &'ast syn::Item) {
        let attrs = match node {
            syn::Item::Mod(m) => &m.attrs,
            syn::Item::Fn(f) => &f.attrs,
            _ => {
                visit::visit_item(self, node);
                return;
            }
        };
        // Test fixtures legitimately read files; production sources remain pure.
        if attrs.iter().any(|a| {
            a.path().is_ident("test")
                || (a.path().is_ident("cfg")
                    && a.parse_args::<syn::Ident>().is_ok_and(|i| i == "test"))
        }) {
            return;
        }
        visit::visit_item(self, node);
    }
    fn visit_item_extern_crate(&mut self, node: &'ast syn::ItemExternCrate) {
        self.path(&[node.ident.to_string()]);
    }
    fn visit_item_use(&mut self, node: &'ast syn::ItemUse) {
        self.imports(Vec::new(), &node.tree);
    }
    fn visit_path(&mut self, node: &'ast syn::Path) {
        self.path(
            &node
                .segments
                .iter()
                .map(|p| p.ident.to_string())
                .collect::<Vec<_>>(),
        );
        visit::visit_path(self, node);
    }
}
