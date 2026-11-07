//! CLIENT-STRUCTURE-TARGET-1 §§4.2–4.5, 6: additive ownership and migration ratchets.
use super::{client_fragments, collect_rust_sources};
use std::fs;
use std::path::Path;
use syn::visit::{self, Visit};

const TREES: &[&str] = &[
    "apps/client",
    "crates/client-domain",
    "crates/client-simulation",
    "crates/client-runtime",
    "crates/client-ui",
    "crates/client-assets",
    "crates/renderer",
    "crates/input-actions",
    "crates/input-platform",
    "crates/session",
    "crates/session-tcp",
    "crates/platform-client",
];

pub fn validate(root: &Path) -> Result<(), String> {
    let baseline =
        client_fragments::read(&root.join("tools/architecture-check/client-size-baseline.json"))?;
    let mut largest = Vec::new();
    for (path, contents) in sources(root)? {
        let lines = contents.lines().count();
        let (warning, hard) = limits(&path);
        let prior = &baseline[&path];
        if !prior.is_null() {
            let max_lines = prior["lines"].as_u64().ok_or("ratchet lines")?;
            let max_bytes = prior["bytes"].as_u64().ok_or("ratchet bytes")?;
            if lines as u64 > max_lines || contents.len() as u64 > max_bytes {
                return Err(format!(
                    "client size ratchet grew: {path}: {lines} lines, {} bytes",
                    contents.len()
                ));
            }
        } else if lines > hard {
            return Err(format!("client file exceeds {hard} lines: {path}: {lines}"));
        }
        if lines > warning {
            eprintln!(
                "client structure: WARN: {path}: {lines} lines (warning {warning}, hard {hard})"
            );
        }
        if path.contains("/generated/")
            || path.ends_with("_generated.rs")
            || path.ends_with("/generated.rs")
            || contents.lines().take(5).any(|line| {
                line.contains("@generated") || line.contains("Generated; do not commit")
            })
        {
            return Err(format!("generated Rust must remain in OUT_DIR: {path}"));
        }
        if !path.contains("/features/")
            && !path.contains("/tests/")
            && !path.rsplit('/').next().is_some_and(|n| n.contains("tests"))
            && [
                "apps/client/",
                "crates/client-ui/",
                "crates/client-domain/",
                "crates/client-runtime/",
            ]
            .iter()
            .any(|prefix| path.starts_with(prefix))
        {
            let syntax = syn::parse_file(&contents).map_err(|e| format!("{path}: {e}"))?;
            let mut registry = CentralRegistry::default();
            registry.visit_file(&syntax);
            let allowed = prior["central_registrations"].as_u64().unwrap_or(0);
            if registry.count > allowed {
                return Err(format!(
                    "central client registration in {path}; use feature fragments"
                ));
            }
        }
        largest.push((lines, path));
    }
    for path in client_fragments::listing(&root.join("crates/client-ui/src"))? {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or("UI source name")?;
        if !["lib.rs", "features", "components"].contains(&name) {
            return Err(format!(
                "client-ui source belongs in features or components: {name}"
            ));
        }
    }
    let mechanics = client_fragments::mechanics(root)?;
    for tree in ["client-domain", "client-runtime", "client-ui"] {
        for feature in client_fragments::listing(&root.join(format!("crates/{tree}/src/features")))?
        {
            let name = feature
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or("feature name")?;
            if !feature.is_dir()
                || (!mechanics.contains_key(name) && !client_fragments::local_name(name))
            {
                return Err(format!("invalid feature directory: {}", feature.display()));
            }
            if tree != "client-ui" && feature.join("registrations").exists() {
                return Err("client registrations belong to client-ui feature ownership".into());
            }
        }
    }
    for tree in TREES {
        validate_fragment_layout(root, &root.join(tree))?;
    }
    let (entries, coverage) = client_fragments::scan(root)?;
    let (rust, _) = client_fragments::render(&entries, &coverage)?;
    syn::parse_file(&rust).map_err(|e| format!("invalid generated Rust: {e}"))?;
    eprintln!(
        "client generated Rust: {} lines, {} bytes (OUT_DIR only)",
        rust.lines().count(),
        rust.len()
    );
    println!(
        "client mechanic coverage:\n{}",
        serde_json::to_string_pretty(&coverage).map_err(|e| e.to_string())?
    );
    largest.sort_by(|a, b| b.cmp(a));
    eprintln!(
        "client largest files: {:?}",
        &largest[..largest.len().min(10)]
    );
    Ok(())
}

fn sources(root: &Path) -> Result<Vec<(String, String)>, String> {
    let mut files = Vec::new();
    for tree in TREES.iter().copied().chain([
        "tools/architecture-check/src/client_fragments",
        "tools/architecture-check/src/client_structure",
        "tools/architecture-check/tests/support",
    ]) {
        let dir = root.join(tree);
        if dir.is_dir() {
            collect_rust_sources(root, &dir, &mut files)?;
        }
    }
    for file in [
        "tools/architecture-check/src/client_fragments.rs",
        "tools/architecture-check/src/client_layering.rs",
        "tools/architecture-check/src/client_structure.rs",
        "tools/architecture-check/tests/client_layering.rs",
        "tools/architecture-check/tests/client_structure.rs",
    ] {
        let path = root.join(file);
        if path.exists() {
            files.push((
                file.to_owned(),
                fs::read_to_string(path).map_err(|e| e.to_string())?,
            ));
        }
    }
    Ok(files)
}

fn validate_fragment_layout(root: &Path, dir: &Path) -> Result<(), String> {
    for path in client_fragments::listing(dir)? {
        if path.is_dir() {
            if path.file_name().is_some_and(|name| name == "registrations")
                && path.parent().and_then(Path::parent)
                    != Some(root.join("crates/client-ui/src/features").as_path())
            {
                return Err(format!(
                    "misplaced client registrations: {}",
                    path.display()
                ));
            }
            validate_fragment_layout(root, &path)?;
        } else if path.extension().is_some_and(|e| e == "json")
            && path.parent().is_some_and(|p| p.ends_with("registrations"))
        {
            let lines = fs::read_to_string(&path)
                .map_err(|e| e.to_string())?
                .lines()
                .count();
            if lines > 200 {
                return Err(format!(
                    "registration exceeds 200 lines: {}",
                    path.display()
                ));
            }
            if lines > 100 {
                eprintln!(
                    "client structure: WARN: registration exceeds 100 lines: {}",
                    path.display()
                );
            }
        }
    }
    Ok(())
}

fn limits(path: &str) -> (usize, usize) {
    let name = path.rsplit('/').next().unwrap_or(path);
    if name == "mod.rs"
        || (path.starts_with("apps/client/src/")
            && [
                "lib.rs",
                "main.rs",
                "composition.rs",
                "window.rs",
                "shutdown.rs",
            ]
            .contains(&name))
    {
        (200, 300)
    } else if path.contains("/tests/") || name.contains("tests") || name.starts_with("test_") {
        (700, 1000)
    } else {
        (400, 600)
    }
}

#[derive(Default)]
struct CentralRegistry {
    count: u64,
}

impl<'ast> Visit<'ast> for CentralRegistry {
    fn visit_item_const(&mut self, item: &'ast syn::ItemConst) {
        if central_name(&item.ident.to_string()) {
            self.count += 1;
        }
        visit::visit_item_const(self, item);
    }
    fn visit_item_static(&mut self, item: &'ast syn::ItemStatic) {
        if central_name(&item.ident.to_string()) {
            self.count += 1;
        }
        visit::visit_item_static(self, item);
    }
    fn visit_expr_match(&mut self, node: &'ast syn::ExprMatch) {
        struct Patterns {
            count: u64,
        }
        impl<'a> Visit<'a> for Patterns {
            fn visit_path(&mut self, path: &'a syn::Path) {
                if path.segments.iter().any(|s| {
                    [
                        "ServerEvent",
                        "SessionEvent",
                        "PanelId",
                        "ClientAction",
                        "SettingsSection",
                    ]
                    .contains(&s.ident.to_string().as_str())
                }) {
                    self.count += 1;
                }
                visit::visit_path(self, path);
            }
        }
        let mut patterns = Patterns { count: 0 };
        for arm in &node.arms {
            patterns.visit_pat(&arm.pat);
        }
        self.count += patterns.count;
        visit::visit_expr_match(self, node);
    }
}

fn central_name(name: &str) -> bool {
    [
        "PANELS",
        "HANDLERS",
        "SETTINGS_SECTIONS",
        "SETTINGS_CATALOG",
        "DEFAULT_BINDINGS",
        "DEFAULT_KEYBINDINGS",
        "CYCLOPEDIA_PAGES",
    ]
    .contains(&name)
}

// The build script is the production consumer. The architecture tests exercise
// generation from a synthetic mechanic through this same implementation.
#[cfg(test)]
mod tests;
