//! Guard: the set of `impl <seal>::Sealed for T` sites (plus `impl PremiumGateOwner for T`)
//! across workspace `crates/` and `apps/` sources must equal the committed allowlist.
//!
//! Rust has no friend crates, so after the gameplay carve several sealed traits must
//! expose their `Sealed` supertrait modules as `pub`. Reachability is unchanged from the
//! single-crate layout; this test stops the set of implementers from growing silently.
//! A follow-up moves the implementations into their owning crates.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const ALLOWLIST: &str = include_str!("sealed_impl_allowlist.txt");
const SELF_PATH: &str = "apps/game-server/tests/sealed_impl_allowlist.rs";

fn is_ident(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn collect_sources(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if path.is_dir() {
            if !name.starts_with("target") && !name.starts_with('.') {
                collect_sources(&path, out);
            }
        } else if name.ends_with(".rs") {
            out.push(path);
        }
    }
}

fn impl_sites(rel: &str, src: &str, out: &mut Vec<String>) {
    let cleaned = src
        .lines()
        .map(|line| {
            if line.trim_start().starts_with("//") {
                ""
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    let bytes = cleaned.as_bytes();
    let mut from = 0;
    while let Some(offset) = cleaned[from..].find("impl") {
        let start = from + offset;
        from = start + 4;
        if (start > 0 && is_ident(bytes[start - 1]))
            || bytes.get(start + 4).is_some_and(|b| is_ident(*b))
        {
            continue;
        }
        let rest = &cleaned[start + 4..];
        let Some(end) = rest.find(['{', ';']) else {
            continue;
        };
        let mut header = rest[..end].split_whitespace().collect::<Vec<_>>().join(" ");
        if header.starts_with('<') {
            let mut depth = 0usize;
            let mut close = None;
            for (idx, ch) in header.char_indices() {
                match ch {
                    '<' => depth += 1,
                    '>' => {
                        depth -= 1;
                        if depth == 0 {
                            close = Some(idx);
                            break;
                        }
                    }
                    _ => {}
                }
            }
            if let Some(idx) = close {
                header = header[idx + 1..].trim().to_owned();
            }
        }
        let Some((trait_path, ty)) = header.split_once(" for ") else {
            continue;
        };
        let ty = ty.split(" where ").next().unwrap_or(ty).trim();
        let segments: Vec<&str> = trait_path.trim().split("::").collect();
        let trait_key = match segments.last().copied() {
            Some("Sealed") => segments[segments.len().saturating_sub(2)..].join("::"),
            Some("PremiumGateOwner") => "PremiumGateOwner".to_owned(),
            _ => continue,
        };
        out.push(format!("{rel} | {trait_key} | {ty}"));
    }
}

fn counts(lines: impl IntoIterator<Item = String>) -> BTreeMap<String, usize> {
    let mut map = BTreeMap::new();
    for line in lines {
        *map.entry(line).or_insert(0) += 1;
    }
    map
}

#[test]
fn cross_crate_sealed_impl_sites_match_allowlist() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut files = Vec::new();
    for top in ["crates", "apps"] {
        collect_sources(&root.join(top), &mut files);
    }
    assert!(!files.is_empty(), "no workspace sources found under {root:?}");
    let mut found = Vec::new();
    for file in files {
        let rel = file
            .strip_prefix(&root)
            .unwrap_or(&file)
            .to_string_lossy()
            .replace('\\', "/");
        let rel = rel.trim_start_matches("./").to_owned();
        if rel == SELF_PATH {
            continue;
        }
        let src = fs::read_to_string(&file).unwrap_or_else(|e| panic!("read {rel}: {e}"));
        impl_sites(&rel, &src, &mut found);
    }
    let found = counts(found);
    let allowed = counts(
        ALLOWLIST
            .lines()
            .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
            .map(str::to_owned),
    );
    let mut problems = Vec::new();
    for (site, n) in &found {
        if *n > allowed.get(site).copied().unwrap_or(0) {
            problems.push(format!(
                "new cross-crate Sealed impl {site}: seals must stay inside the owning crate; add to the allowlist only with CP review"
            ));
        }
    }
    for (site, n) in &allowed {
        if *n > found.get(site).copied().unwrap_or(0) {
            problems.push(format!(
                "stale allowlist entry {site}: remove it from sealed_impl_allowlist.txt"
            ));
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}
