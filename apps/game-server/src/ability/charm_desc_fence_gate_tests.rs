//! D295 §3 item 3 hard wiring gate (CHARM-DESC-FENCE-1).
//!
//! No production code may reference the three Ability damage bridges until the A2 live attacker
//! fence merges. Only the A2 PR may remove or relax this test, together with the fence.

use std::path::Path;

const BRIDGES: [&str; 3] = [
    "commit_exact_owner_damage",
    "commit_exact_owner_primary_damage",
    "commit_exact_owner_charm_damage",
];

/// Blanks comments and string/char literal contents, keeping newlines so line numbers survive.
fn sanitize(src: &str) -> Vec<u8> {
    let b = src.as_bytes();
    let mut out = b.to_vec();
    let blank = |out: &mut Vec<u8>, from: usize, to: usize| {
        for c in &mut out[from..to] {
            if *c != b'\n' {
                *c = b' ';
            }
        }
    };
    let mut i = 0;
    while i < b.len() {
        let start = i;
        if b[i..].starts_with(b"//") {
            while i < b.len() && b[i] != b'\n' {
                i += 1;
            }
        } else if b[i..].starts_with(b"/*") {
            let mut depth = 0;
            while i < b.len() {
                if b[i..].starts_with(b"/*") {
                    depth += 1;
                    i += 2;
                } else if b[i..].starts_with(b"*/") {
                    depth -= 1;
                    i += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    i += 1;
                }
            }
        } else if b[i] == b'r' && matches!(b.get(i + 1), Some(b'"' | b'#')) {
            let hashes = b[i + 1..].iter().take_while(|c| **c == b'#').count();
            if b.get(i + 1 + hashes) != Some(&b'"') {
                i += 1;
                continue;
            }
            let close = [&b"\""[..], &vec![b'#'; hashes]].concat();
            i += 2 + hashes;
            while i < b.len() && !b[i..].starts_with(&close) {
                i += 1;
            }
            i = (i + close.len()).min(b.len());
        } else if b[i] == b'"' {
            i += 1;
            while i < b.len() && b[i] != b'"' {
                i += if b[i] == b'\\' { 2 } else { 1 };
            }
            i = (i + 1).min(b.len());
        } else if b[i] == b'\'' && (b.get(i + 1) == Some(&b'\\') || b.get(i + 2) == Some(&b'\'')) {
            i += 1;
            while i < b.len() && b[i] != b'\'' {
                i += if b[i] == b'\\' { 2 } else { 1 };
            }
            i = (i + 1).min(b.len());
        } else {
            i += 1;
            continue;
        }
        blank(&mut out, start, i.min(b.len()));
    }
    out
}

/// End of the item starting at `i`: its first top-level `;`, or its matching `}`.
fn item_end(code: &[u8], mut i: usize) -> usize {
    let mut depth = 0usize;
    let mut in_braces = false;
    while i < code.len() {
        match code[i] {
            b'(' | b'[' => depth += 1,
            b')' | b']' => depth = depth.saturating_sub(1),
            b'{' => {
                depth += 1;
                in_braces = true;
            }
            b'}' => {
                depth = depth.saturating_sub(1);
                if in_braces && depth == 0 {
                    return i + 1;
                }
            }
            b';' if depth == 0 => return i + 1,
            _ => {}
        }
        i += 1;
    }
    code.len()
}

/// End of the `[...]` group starting at `i`, or `i` when no group starts there.
fn bracket_end(code: &[u8], i: usize) -> usize {
    if code.get(i) != Some(&b'[') {
        return i;
    }
    let mut depth = 0usize;
    for (j, c) in code.iter().enumerate().skip(i) {
        match c {
            b'[' => depth += 1,
            b']' => {
                depth -= 1;
                if depth == 0 {
                    return j + 1;
                }
            }
            _ => {}
        }
    }
    code.len()
}

fn is_ident(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_'
}

/// Byte offsets of `word` in `code` as a whole identifier.
fn word_hits(code: &[u8], word: &str) -> Vec<usize> {
    let w = word.as_bytes();
    (0..code.len().saturating_sub(w.len() - 1))
        .filter(|&i| {
            code[i..].starts_with(w)
                && (i == 0 || !is_ident(code[i - 1]))
                && code.get(i + w.len()).is_none_or(|c| !is_ident(*c))
        })
        .collect()
}

/// 1-based lines of non-test production references to the bridges in one source file.
fn production_references(src: &str) -> Vec<usize> {
    let mut code = sanitize(src);
    let compact: Vec<u8> = code
        .iter()
        .copied()
        .filter(|c| !c.is_ascii_whitespace())
        .collect();
    if compact.starts_with(b"#![cfg(test)]") {
        return Vec::new();
    }
    let mut exempt = Vec::new();
    // `#[cfg(test)]` items, including any further attributes on them.
    let mut i = 0;
    while let Some(off) = code[i..].iter().position(|c| *c == b'#') {
        let at = i + off;
        let attr_end = bracket_end(&code, at + 1);
        let attr: Vec<u8> = code[at..attr_end]
            .iter()
            .copied()
            .filter(|c| !c.is_ascii_whitespace())
            .collect();
        if attr == b"#[cfg(test)]" {
            exempt.push((at, item_end(&code, attr_end)));
        }
        i = at + 1;
    }
    // The bridges' own definitions and bodies.
    for name in BRIDGES {
        for at in word_hits(&code, name) {
            let head = code[..at].trim_ascii_end();
            if head.ends_with(b"fn") && (head.len() == 2 || !is_ident(head[head.len() - 3])) {
                exempt.push((at, item_end(&code, at)));
            }
        }
    }
    for (from, to) in exempt {
        for c in &mut code[from..to] {
            if *c != b'\n' {
                *c = b' ';
            }
        }
    }
    let mut lines: Vec<usize> = BRIDGES
        .iter()
        .flat_map(|name| word_hits(&code, name))
        .map(|at| code[..at].iter().filter(|c| **c == b'\n').count() + 1)
        .collect();
    lines.sort_unstable();
    lines.dedup();
    lines
}

fn scan(dir: &Path, root: &Path, found: &mut Vec<String>) -> std::io::Result<()> {
    let mut entries = std::fs::read_dir(dir)?
        .map(|e| e.map(|e| e.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    entries.sort();
    for path in entries {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if path.is_dir() {
            scan(&path, root, found)?;
        } else if name.ends_with(".rs") && !name.ends_with("_tests.rs") && name != "tests.rs" {
            let src = std::fs::read_to_string(&path)?;
            for line in production_references(&src) {
                let rel = path.strip_prefix(root).unwrap_or(&path);
                found.push(format!("{}:{line}", rel.display()));
            }
        }
    }
    Ok(())
}

#[test]
fn no_production_caller_of_ability_damage_bridges_before_a2() -> std::io::Result<()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut found = Vec::new();
    scan(&root.join("src"), root, &mut found)?;
    assert!(
        found.is_empty(),
        "D295 §3 item 3 hard wiring gate: production references to the Ability damage bridges \
         {BRIDGES:?} before the A2 live attacker fence: {found:?}. Only the A2 PR, with the fence, \
         may relax this gate."
    );
    Ok(())
}

#[test]
fn scan_flags_a_production_reference() {
    let src = "fn live() {\n    let _ = commit_exact_owner_charm_damage(a, b, c, d);\n}\n\
               use crate::ability::commit::commit_exact_owner_damage;\n";
    assert_eq!(production_references(src), vec![2, 4]);
    assert_eq!(
        production_references("fn f() { g(commit_exact_owner_primary_damage) }"),
        vec![1]
    );
}

#[test]
fn scan_ignores_test_comment_string_and_bridge_bodies() {
    let src = r##"
// commit_exact_owner_damage in a line comment
/* commit_exact_owner_charm_damage /* nested */ still a comment */
const S: &str = "commit_exact_owner_damage";
const R: &str = r#"commit_exact_owner_damage"#;
const C: char = '{';
#[allow(dead_code)]
pub(crate) fn commit_exact_owner_damage(x: u8) -> u8 { x }
pub(crate) fn commit_exact_owner_primary_damage(x: u8) -> u8 {
    commit_exact_owner_damage(x)
}
#[cfg(test)]
mod tests {
    fn t() { commit_exact_owner_charm_damage(); }
}
#[cfg(test)]
use super::commit_exact_owner_damage;
"##;
    assert_eq!(production_references(src), Vec::<usize>::new());
    assert_eq!(
        production_references("#![cfg(test)]\nfn t() { commit_exact_owner_damage(); }"),
        Vec::<usize>::new()
    );
    // A non-test cfg and a trailing caller after an exempt body are still references.
    let mixed = "#[cfg(not(test))]\nfn a() { commit_exact_owner_damage(); }\n\
                 fn commit_exact_owner_damage() {}\nfn b() { commit_exact_owner_damage(); }\n";
    assert_eq!(production_references(mixed), vec![2, 4]);
}
