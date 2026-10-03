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

/// End of the `(..)`, `[..]` or `{..}` group starting at `i`, or `i` when no group starts there.
fn group_end(code: &[u8], i: usize) -> usize {
    let (open, close) = match code.get(i) {
        Some(b'(') => (b'(', b')'),
        Some(b'[') => (b'[', b']'),
        Some(b'{') => (b'{', b'}'),
        _ => return i,
    };
    let mut depth = 0usize;
    for (j, c) in code.iter().enumerate().skip(i) {
        if *c == open {
            depth += 1;
        } else if *c == close {
            depth -= 1;
            if depth == 0 {
                return j + 1;
            }
        }
    }
    code.len()
}

/// End of the `[...]` group starting at `i`, or `i` when no group starts there.
fn bracket_end(code: &[u8], i: usize) -> usize {
    if code.get(i) == Some(&b'[') {
        group_end(code, i)
    } else {
        i
    }
}

/// Identifier byte; any non-ASCII byte counts, as Rust identifiers may be Unicode.
fn is_ident(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_' || !c.is_ascii()
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

/// Byte ranges of macro token trees (`name!(..)`, `name![..]`, `name!{..}`, `macro_rules! m {..}`).
/// Attributes inside them are only tokens, so they never exempt anything.
fn macro_ranges(code: &[u8]) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    for (bang, _) in code.iter().enumerate().filter(|(_, c)| **c == b'!') {
        // Any name, including a Unicode one: only an inner attribute's `#!` is not a macro. Unary
        // `!(..)` or `if !x {..}` also match, which only withholds exemptions (fails closed).
        if code[..bang].trim_ascii_end().ends_with(b"#") {
            continue;
        }
        let mut open = bang + 1;
        while code.get(open).is_some_and(u8::is_ascii_whitespace) {
            open += 1;
        }
        // `macro_rules! name { .. }`
        while code.get(open).copied().is_some_and(is_ident) {
            open += 1;
        }
        while code.get(open).is_some_and(u8::is_ascii_whitespace) {
            open += 1;
        }
        let close = match code.get(open) {
            Some(b'(') => b')',
            Some(b'[') => b']',
            Some(b'{') => b'}',
            _ => continue,
        };
        let mut depth = 0usize;
        for (j, c) in code.iter().enumerate().skip(open) {
            if *c == code[open] {
                depth += 1;
            } else if *c == close {
                depth -= 1;
                if depth == 0 {
                    ranges.push((open, j + 1));
                    break;
                }
            }
        }
    }
    ranges
}

/// Whether the node after the attribute at `i` is an item or a `let` statement, whose extent
/// [`item_end`] bounds exactly.
fn attributes_item(code: &[u8], mut i: usize) -> bool {
    const KEYWORDS: [&[u8]; 16] = [
        b"mod",
        b"fn",
        b"use",
        b"impl",
        b"struct",
        b"enum",
        b"union",
        b"trait",
        b"type",
        b"const",
        b"static",
        b"unsafe",
        b"async",
        b"extern",
        b"macro_rules",
        b"let",
    ];
    let word = |i: &mut usize| {
        while code.get(*i).is_some_and(u8::is_ascii_whitespace) {
            *i += 1;
        }
        let start = *i;
        while code.get(*i).copied().is_some_and(is_ident) {
            *i += 1;
        }
        &code[start..*i]
    };
    loop {
        while code.get(i).is_some_and(u8::is_ascii_whitespace) {
            i += 1;
        }
        if code.get(i) != Some(&b'#') {
            break;
        }
        let end = bracket_end(code, i + 1);
        if end == i + 1 {
            return false;
        }
        i = end;
    }
    let mut next = word(&mut i);
    if next == b"pub" {
        while code.get(i).is_some_and(u8::is_ascii_whitespace) {
            i += 1;
        }
        if code.get(i) == Some(&b'(') {
            let mut depth = 0usize;
            for (j, c) in code.iter().enumerate().skip(i) {
                match c {
                    b'(' => depth += 1,
                    b')' => {
                        depth -= 1;
                        if depth == 0 {
                            i = j + 1;
                            break;
                        }
                    }
                    _ => {}
                }
            }
        }
        next = word(&mut i);
    }
    KEYWORDS.contains(&next)
}

fn compact(bytes: &[u8]) -> Vec<u8> {
    bytes
        .iter()
        .copied()
        .filter(|c| !c.is_ascii_whitespace())
        .collect()
}

/// Whether the file's leading inner attributes include `#![cfg(test)]`.
fn inner_cfg_test(code: &[u8]) -> bool {
    let mut i = 0;
    loop {
        while code.get(i).is_some_and(u8::is_ascii_whitespace) {
            i += 1;
        }
        if !code[i..].starts_with(b"#") {
            return false;
        }
        let mut open = i + 1;
        while code.get(open).is_some_and(u8::is_ascii_whitespace) {
            open += 1;
        }
        if code.get(open) != Some(&b'!') {
            return false;
        }
        open += 1;
        while code.get(open).is_some_and(u8::is_ascii_whitespace) {
            open += 1;
        }
        let end = bracket_end(code, open);
        if end == open {
            return false;
        }
        if compact(&code[i..end]) == b"#![cfg(test)]" {
            return true;
        }
        i = end;
    }
}

fn skip_ws(code: &[u8], mut i: usize) -> usize {
    while code.get(i).is_some_and(u8::is_ascii_whitespace) {
        i += 1;
    }
    i
}

/// Ranges that `#[cfg(test)]` removes from production: attributed items and `let` statements
/// outside macro token trees. On any other node (array or tuple element, field, variant, match
/// arm, expression) it exempts nothing.
fn cfg_test_ranges(code: &[u8], macros: &[(usize, usize)]) -> Vec<(usize, usize)> {
    let in_macro = |at: usize| macros.iter().any(|(from, to)| (*from..*to).contains(&at));
    let mut ranges = Vec::new();
    let mut i = 0;
    while let Some(off) = code[i..].iter().position(|c| *c == b'#') {
        let at = i + off;
        let attr_end = bracket_end(code, at + 1);
        let item_start = code[..at]
            .trim_ascii_end()
            .last()
            .is_none_or(|c| matches!(c, b';' | b'{' | b'}' | b']'));
        if compact(&code[at..attr_end]) == b"#[cfg(test)]"
            && item_start
            && !in_macro(at)
            && attributes_item(code, attr_end)
        {
            ranges.push((at, item_end(code, attr_end)));
        }
        i = at + 1;
    }
    ranges
}

/// 1-based lines of non-test production references to the bridges in one source file.
/// `bridge_module` is true only for `src/ability/commit.rs`, the sole home of the canonical bridge
/// definitions; a same-named function anywhere else is a reference, not an exempt definition.
fn production_references(src: &str, bridge_module: bool) -> Vec<usize> {
    let mut code = sanitize(src);
    if inner_cfg_test(&code) {
        return Vec::new();
    }
    let macros = macro_ranges(&code);
    let in_macro = |at: usize| macros.iter().any(|(from, to)| (*from..*to).contains(&at));
    let mut exempt = cfg_test_ranges(&code, &macros);
    // The canonical bridges' own definitions and bodies: exactly one each, outside macros.
    if bridge_module {
        for name in BRIDGES {
            let defs: Vec<usize> = word_hits(&code, name)
                .into_iter()
                .filter(|&at| {
                    let head = code[..at].trim_ascii_end();
                    head.ends_with(b"fn")
                        && (head.len() == 2 || !is_ident(head[head.len() - 3]))
                        && !in_macro(at)
                })
                .collect();
            if let [at] = defs[..] {
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

/// What a declaration may load into the crate: one file by basename, or any file when the
/// declaration cannot be resolved lexically (macro-built names, non-literal or `cfg_attr` paths).
#[derive(Debug, PartialEq, Eq)]
enum Loads {
    File(String),
    Any,
}

/// Every construct in one source file that can load another file (`mod name;`, a `path`
/// attribute, `include!`), each with whether `#[cfg(test)]` keeps it out of production and whether
/// it names its file explicitly (a `path` or `include!`, which may reach outside `src/`).
/// Declarations are found by name anywhere, including inside inline `mod` blocks and macros, so
/// directory resolution never hides one; a macro-carried `#[cfg(test)]` token gates nothing.
fn file_declarations(src: &str) -> Vec<(Loads, bool, bool)> {
    let code = sanitize(src);
    let test_only = inner_cfg_test(&code);
    let macros = macro_ranges(&code);
    let ranges = cfg_test_ranges(&code, &macros);
    let gated = |at: usize| test_only || ranges.iter().any(|(from, to)| (*from..*to).contains(&at));
    // `sanitize` blanks literals in place, so a group that compacts to its delimiters holds at most
    // one string literal, read back from the same offsets in `src`.
    let literal = |from: usize, to: usize| {
        let raw = &src.as_bytes()[from..to];
        let open = raw.iter().position(|c| *c == b'"')?;
        let len = raw[open + 1..].iter().position(|c| *c == b'"')?;
        let value = std::str::from_utf8(&raw[open + 1..open + 1 + len]).ok()?;
        // An escape could spell any name, so only a plain literal names a file.
        (!value.contains('\\')).then(|| value.rsplit('/').next().unwrap_or(value).to_owned())
    };
    let mut decls = Vec::new();
    for at in word_hits(&code, "mod") {
        let mut i = skip_ws(&code, at + 3);
        if code[i..].starts_with(b"r#") {
            i += 2;
        }
        if code.get(i) == Some(&b'$') {
            decls.push((Loads::Any, gated(at), false));
            continue;
        }
        let start = i;
        while code.get(i).copied().is_some_and(is_ident) {
            i += 1;
        }
        if start < i && code.get(skip_ws(&code, i)) == Some(&b';') {
            decls.push((
                Loads::File(format!("{}.rs", &src[start..i])),
                gated(at),
                false,
            ));
        }
    }
    let mut i = 0;
    while let Some(off) = code[i..].iter().position(|c| *c == b'#') {
        let at = i + off;
        let end = bracket_end(&code, skip_ws(&code, at + 1));
        let attr = compact(&code[at..end]);
        if attr.windows(5).any(|w| w == b"path=") {
            let loads = match literal(at, end) {
                Some(name) if attr == b"#[path=]" => Loads::File(name),
                _ => Loads::Any,
            };
            decls.push((loads, gated(at), true));
        }
        i = at + 1;
    }
    for at in word_hits(&code, "include") {
        let bang = skip_ws(&code, at + 7);
        if code.get(bang) != Some(&b'!') {
            continue;
        }
        let open = skip_ws(&code, bang + 1);
        let end = group_end(&code, open);
        let loads = match literal(open, end) {
            Some(name) if end > open && compact(&code[open + 1..end - 1]).is_empty() => {
                Loads::File(name)
            }
            _ => Loads::Any,
        };
        decls.push((loads, gated(at), true));
    }
    decls
}

fn test_named(rel: &Path) -> bool {
    let name = rel.file_name().and_then(|n| n.to_str()).unwrap_or("");
    name.ends_with("_tests.rs") || name == "tests.rs"
}

/// `path:line` of every production reference in `files` (crate-relative path, source). A
/// test-named file is skipped only when every declaration that may load it is gated by
/// `#[cfg(test)]` or sits in a file already skipped; one declared nowhere is not in the crate.
/// Any other file, test-named or not, is scanned (an inner `#![cfg(test)]` still exempts it), and a
/// production `path` or `include!` to a file outside `src/` is itself a finding.
fn gate_findings(files: &[(std::path::PathBuf, String)]) -> Vec<String> {
    let decls: Vec<(usize, Loads, bool, bool)> = files
        .iter()
        .enumerate()
        .flat_map(|(from, (_, src))| {
            file_declarations(src)
                .into_iter()
                .map(move |(loads, gated, explicit)| (from, loads, gated, explicit))
        })
        .collect();
    let mut skipped = vec![false; files.len()];
    loop {
        let mut changed = false;
        for (n, (rel, _)) in files.iter().enumerate() {
            if skipped[n] || !test_named(rel) {
                continue;
            }
            let name = rel.file_name().and_then(|n| n.to_str()).unwrap_or("");
            let test_only = decls
                .iter()
                .filter(|(_, loads, _, _)| match loads {
                    Loads::File(file) => file == name,
                    Loads::Any => true,
                })
                .all(|(from, _, gated, _)| *gated || (*from != n && skipped[*from]));
            if test_only {
                skipped[n] = true;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    // A production `path` or `include!` naming no file under `src/` loads code this gate cannot see.
    let unseen = decls
        .iter()
        .filter_map(|(from, loads, gated, explicit)| match loads {
            Loads::File(name)
                if *explicit
                    && !*gated
                    && !skipped[*from]
                    && !files
                        .iter()
                        .any(|(rel, _)| rel.file_name().is_some_and(|f| f == name.as_str())) =>
            {
                Some(format!(
                    "{}: loads unscanned {name}",
                    files[*from].0.display()
                ))
            }
            _ => None,
        });
    let unseen: Vec<String> = unseen.collect();
    files
        .iter()
        .zip(skipped.iter().copied())
        .filter(|(_, skipped)| !skipped)
        .flat_map(|((rel, src), _)| {
            let bridge_module = rel == Path::new("src/ability/commit.rs");
            production_references(src, bridge_module)
                .into_iter()
                .map(move |line| format!("{}:{line}", rel.display()))
        })
        .chain(unseen)
        .collect()
}

fn collect(
    dir: &Path,
    root: &Path,
    files: &mut Vec<(std::path::PathBuf, String)>,
) -> std::io::Result<()> {
    let mut entries = std::fs::read_dir(dir)?
        .map(|e| e.map(|e| e.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    entries.sort();
    for path in entries {
        if path.is_dir() {
            collect(&path, root, files)?;
        } else if path.extension().is_some_and(|e| e == "rs") {
            let rel = path.strip_prefix(root).unwrap_or(&path).to_path_buf();
            files.push((rel, std::fs::read_to_string(&path)?));
        }
    }
    Ok(())
}

#[test]
fn no_production_caller_of_ability_damage_bridges_before_a2() -> std::io::Result<()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    collect(&root.join("src"), root, &mut files)?;
    let found = gate_findings(&files);
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
    assert_eq!(production_references(src, false), vec![2, 4]);
    assert_eq!(
        production_references("fn f() { g(commit_exact_owner_primary_damage) }", false),
        vec![1]
    );
    // `#[cfg(test)]` as macro tokens does not gate the expansion.
    let in_macro = "fn f() {\n    m!(#[cfg(test)] crate::x::commit_exact_owner_damage);\n}\n\
                    n! { #[cfg(test)] commit_exact_owner_charm_damage() }\n";
    assert_eq!(production_references(in_macro, false), vec![2, 4]);
    let unicode = "μ! {\n    #[cfg(test)]\n    fn live() { commit_exact_owner_damage() }\n}\n";
    assert_eq!(production_references(unicode, false), vec![3]);
    // A Unicode-prefixed identifier is a different name, not a bridge reference.
    assert_eq!(
        production_references("fn f() { μcommit_exact_owner_damage(); }", false),
        Vec::<usize>::new()
    );
    // `#[cfg(test)]` on an element, field or arm exempts nothing beyond it.
    let nodes = "fn f() {\n    let _ = [1, #[cfg(test)] 0, commit_exact_owner_damage()];\n\
                 let _ = S { #[cfg(test)] a: 0, b: commit_exact_owner_charm_damage() };\n\
                 match x { #[cfg(test)] A => 0, _ => commit_exact_owner_primary_damage() }\n\
                 #[cfg(test)] g(); commit_exact_owner_damage();\n}\n";
    assert_eq!(production_references(nodes, false), vec![2, 3, 4, 5]);
    // A same-named wrapper outside the bridge module is no exemption, nor is a duplicate inside it.
    let wrapper = "fn commit_exact_owner_damage() {\n    commit_exact_owner_charm_damage();\n}\n";
    assert_eq!(production_references(wrapper, false), vec![1, 2]);
    let twice = "fn commit_exact_owner_damage() {}\nmod m { fn commit_exact_owner_damage() {} }\n";
    assert_eq!(production_references(twice, true), vec![1, 2]);
    // A non-test cfg and a trailing caller after an exempt body are still references.
    let mixed = "#[cfg(not(test))]\nfn a() { commit_exact_owner_damage(); }\n\
                 fn commit_exact_owner_damage() {}\nfn b() { commit_exact_owner_damage(); }\n";
    assert_eq!(production_references(mixed, true), vec![2, 4]);
    // An inner cfg(test) after the leading attributes is no longer an inner attribute.
    let late =
        "#![allow(dead_code)]\nfn a() {}\n#![cfg(test)]\nfn b() { commit_exact_owner_damage(); }";
    assert_eq!(production_references(late, false), vec![4]);
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
#[cfg(test)]
#[allow(dead_code)]
pub(crate) fn helper() { commit_exact_owner_damage(1); }
fn body() {
    #[cfg(test)]
    let _probe = commit_exact_owner_charm_damage();
}
"##;
    assert_eq!(production_references(src, true), Vec::<usize>::new());
    for test_file in [
        "#![cfg(test)]\nfn t() { commit_exact_owner_damage(); }",
        "//! docs\n#![allow(dead_code)]\n# ! [ cfg ( test ) ]\nfn t() { commit_exact_owner_damage(); }",
    ] {
        assert_eq!(production_references(test_file, false), Vec::<usize>::new());
    }
}

#[test]
fn test_named_files_are_skipped_only_when_test_only() {
    let call = "fn live() { commit_exact_owner_damage(); }\n";
    let run = |decl: &str, extra: &[(&str, &str)]| {
        let mut files = vec![
            ("src/lib.rs".into(), decl.to_owned()),
            ("src/live_tests.rs".into(), call.to_owned()),
        ];
        files.extend(extra.iter().map(|(p, s)| ((*p).into(), (*s).to_owned())));
        gate_findings(&files)
    };
    let flagged = vec!["src/live_tests.rs:1".to_owned()];
    // Unconditional declarations put the file in production.
    for decl in [
        "mod live_tests;",
        "pub(crate) mod r#live_tests ;",
        "mod outer { mod live_tests; }",
        "#[path = \"live_tests.rs\"]\nmod live;",
        "#[cfg_attr(not(test), path = \"other.rs\")]\nmod other;",
        "include!(\"live_tests.rs\");",
        "include!(concat!(\"live\", \"_tests.rs\"));",
        "#[path = \"live\\x5ftests.rs\"]\nmod live;",
        "macro_rules! m { ($n:ident) => { mod $n; }; }\nm!(live_tests);",
        "m! { #[cfg(test)] mod live_tests; }",
        "#[cfg(not(test))]\nmod live_tests;",
        // `#[cfg(test)]` on a macro invocation is not trusted to gate what it expands to.
        "#[cfg(test)]\ninclude!(\"live_tests.rs\");",
    ] {
        assert_eq!(run(decl, &[]), flagged, "{decl}");
    }
    // Test-only declarations, transitively, or none at all keep it out of production.
    for (decl, extra) in [
        ("#[cfg(test)]\nmod live_tests;", &[][..]),
        ("#[cfg(test)]\nmod outer {\n    mod live_tests;\n}", &[]),
        (
            "#[cfg(test)]\n#[path = \"../src/live_tests.rs\"]\nmod live;",
            &[],
        ),
        ("", &[]),
        (
            "#[cfg(test)]\nmod a_tests;",
            &[("src/a_tests.rs", "mod live_tests;")],
        ),
        (
            "mod a;",
            &[(
                "src/a.rs",
                "#![allow(dead_code)]\n#![cfg(test)]\nmod live_tests;",
            )],
        ),
    ] {
        assert_eq!(run(decl, extra), Vec::<String>::new(), "{decl}");
    }
    // A production file declaring it unconditionally overrides a gated sibling declaration.
    assert_eq!(
        run(
            "#[cfg(test)]\nmod live_tests;",
            &[("src/b.rs", "mod live_tests;")]
        ),
        flagged
    );
    // A production load of a file outside `src/` is reported; a test-only one is not.
    assert_eq!(
        run("#[path = \"../../elsewhere/live.rs\"]\nmod live;", &[]),
        vec!["src/lib.rs: loads unscanned live.rs".to_owned()]
    );
    assert_eq!(
        run(
            "#[cfg(test)]\n#[path = \"../tests/support.rs\"]\nmod support;",
            &[]
        ),
        Vec::<String>::new()
    );
    // An inner `#![cfg(test)]` exempts the file's own body wherever it is declared.
    let inner = vec![
        ("src/lib.rs".into(), "mod live_tests;".to_owned()),
        ("src/live_tests.rs".into(), format!("#![cfg(test)]\n{call}")),
    ];
    assert_eq!(gate_findings(&inner), Vec::<String>::new());
}
