//! D295 §3 item 3 hard wiring gate (CHARM-DESC-FENCE-1), relaxed by A2 (#1635).
//!
//! A2 binds the attacker's lease to its runtime player slot and fences that slot for every
//! transition that ends the session's hold. The three Ability damage bridges now read attacker
//! authority from the bound slot, so production may call them. What stays forbidden is any
//! production damage write that does not go through the bound slot: the raw entry that takes a
//! caller-supplied attacker is test-only, and no production code may reference it.

use std::path::Path;

/// The unbound attributed write: a caller-supplied `AttackerCommand`, not the bound slot.
const BRIDGES: [&str; 1] = ["commit_damage_for_attacker"];

/// The sole home of the canonical unbound entry's definition.
const BRIDGE_MODULE: &str = "src/foundation/runtime_actor_carrier.rs";

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

/// 1-based lines of non-test production references to the unbound entry in one source file.
/// `bridge_module` is true only for [`BRIDGE_MODULE`], the sole home of its canonical definition;
/// a same-named function anywhere else is a reference, not an exempt definition. A production
/// caller inside that module is a reference too.
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

/// How an explicit load (`path` attribute or `include!`) names its file.
#[derive(Debug, PartialEq, Eq)]
enum Explicit {
    /// Not explicit: `mod name;` resolves inside the declaring file's own module directory.
    No,
    /// A literal path, resolved against the declaring file's directory; a `path` attribute inside
    /// inline `mod` blocks also descends through their names (outermost first).
    Literal(String, Vec<String>),
    /// A path no lexical reading can resolve.
    Unresolvable,
}

/// One construct that can load another file into the crate.
#[derive(Debug)]
struct Declaration {
    loads: Loads,
    /// `#[cfg(test)]` (or the file's inner `#![cfg(test)]`) keeps it out of production.
    gated: bool,
    explicit: Explicit,
}

/// Every construct in one source file that can load another file (`mod name;`, a `path`
/// attribute, `include!`). Declarations are found by name anywhere, including inside inline `mod`
/// blocks and macros, so directory resolution never hides one; a macro-carried `#[cfg(test)]`
/// token gates nothing.
fn file_declarations(src: &str) -> Vec<Declaration> {
    let code = sanitize(src);
    let test_only = inner_cfg_test(&code);
    let macros = macro_ranges(&code);
    let ranges = cfg_test_ranges(&code, &macros);
    let gated = |at: usize| test_only || ranges.iter().any(|(from, to)| (*from..*to).contains(&at));
    // An attribute is also gated by a `#[cfg(test)]` later in the same item's attribute sequence.
    let gated_sequence = |at: usize, end: usize| {
        let mut i = skip_ws(&code, end);
        while !gated(at) && code.get(i) == Some(&b'#') {
            if ranges.iter().any(|(from, _)| *from == i) {
                return true;
            }
            i = skip_ws(&code, bracket_end(&code, skip_ws(&code, i + 1)));
        }
        gated(at)
    };
    // A macro may emit its tokens anywhere, so a load inside a macro token tree has no lexical base.
    let in_macro = |at: usize| macros.iter().any(|(from, to)| (*from..*to).contains(&at));
    // `sanitize` blanks literals in place, so a group that compacts to its delimiters holds at most
    // one string literal, read back from the same offsets in `src`. An escape could spell any
    // path, so only a plain literal names a file.
    let literal = |from: usize, to: usize| {
        let raw = &src.as_bytes()[from..to];
        let open = raw.iter().position(|c| *c == b'"')?;
        let len = raw[open + 1..].iter().position(|c| *c == b'"')?;
        let value = std::str::from_utf8(&raw[open + 1..open + 1 + len]).ok()?;
        (!value.contains('\\')).then(|| value.to_owned())
    };
    let basename = |path: &str| path.rsplit('/').next().unwrap_or(path).to_owned();
    let mut decls = Vec::new();
    // Inline `mod name { .. }` blocks, for `path` attributes nested in them.
    let mut inline = Vec::new();
    for at in word_hits(&code, "mod") {
        let mut i = skip_ws(&code, at + 3);
        if code[i..].starts_with(b"r#") {
            i += 2;
        }
        if code.get(i) == Some(&b'$') {
            decls.push(Declaration {
                loads: Loads::Any,
                gated: gated(at),
                explicit: Explicit::No,
            });
            continue;
        }
        let start = i;
        while code.get(i).copied().is_some_and(is_ident) {
            i += 1;
        }
        let next = skip_ws(&code, i);
        if start < i && code.get(next) == Some(&b';') {
            decls.push(Declaration {
                loads: Loads::File(format!("{}.rs", &src[start..i])),
                gated: gated(at),
                explicit: Explicit::No,
            });
        } else if start < i && code.get(next) == Some(&b'{') {
            inline.push((next, group_end(&code, next), src[start..i].to_owned()));
        }
    }
    let mut i = 0;
    while let Some(off) = code[i..].iter().position(|c| *c == b'#') {
        let at = i + off;
        let end = bracket_end(&code, skip_ws(&code, at + 1));
        let attr = compact(&code[at..end]);
        if attr.windows(5).any(|w| w == b"path=") {
            let decl = match literal(at, end) {
                Some(path) if attr == b"#[path=]" && !in_macro(at) => {
                    let mut blocks: Vec<&(usize, usize, String)> = inline
                        .iter()
                        .filter(|(from, to, _)| (*from..*to).contains(&at))
                        .collect();
                    blocks.sort_by_key(|(from, _, _)| *from);
                    let blocks = blocks
                        .into_iter()
                        .map(|(_, _, name)| name.clone())
                        .collect();
                    (
                        Loads::File(basename(&path)),
                        Explicit::Literal(path, blocks),
                    )
                }
                _ => (Loads::Any, Explicit::Unresolvable),
            };
            decls.push(Declaration {
                loads: decl.0,
                gated: gated_sequence(at, end),
                explicit: decl.1,
            });
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
        let decl = match literal(open, end) {
            Some(path)
                if end > open && compact(&code[open + 1..end - 1]).is_empty() && !in_macro(at) =>
            {
                (
                    Loads::File(basename(&path)),
                    Explicit::Literal(path, Vec::new()),
                )
            }
            _ => (Loads::Any, Explicit::Unresolvable),
        };
        decls.push(Declaration {
            loads: decl.0,
            gated: gated(at),
            explicit: decl.1,
        });
    }
    decls
}

/// A crate root Cargo builds without any declaration: the library, the main binary and every
/// auto-discovered binary under `src/bin/`.
fn crate_root(rel: &Path) -> bool {
    let bin = Path::new("src/bin");
    rel == Path::new("src/lib.rs")
        || rel == Path::new("src/main.rs")
        || (rel.parent() == Some(bin) && rel.extension().is_some_and(|e| e == "rs"))
        || (rel.file_name().is_some_and(|f| f == "main.rs")
            && rel.parent().and_then(Path::parent) == Some(bin))
}

fn test_named(rel: &Path) -> bool {
    let name = rel.file_name().and_then(|n| n.to_str()).unwrap_or("");
    name.ends_with("_tests.rs") || name == "tests.rs"
}

/// `base` joined with `path`, `.` and `..` resolved lexically; `None` for an absolute path or one
/// that climbs above the crate root.
fn resolve(base: &Path, path: &str) -> Option<std::path::PathBuf> {
    use std::path::Component;
    if Path::new(path).is_absolute() {
        return None;
    }
    let mut out = std::path::PathBuf::new();
    for part in base.join(path).components() {
        match part {
            Component::Normal(part) => out.push(part),
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() {
                    return None;
                }
            }
            Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    Some(out)
}

/// Every file an explicit load in the file at `rel` may resolve to (Rust Reference, "The `path`
/// attribute"). A `path` attribute inside inline `mod` blocks descends through their names, and in
/// a non-`mod.rs` file also through the file's stem; because a file loaded through a `path`
/// attribute elsewhere resolves like a `mod.rs` file, both readings are returned for those files.
fn explicit_targets(rel: &Path, path: &str, blocks: &[String]) -> Vec<Option<std::path::PathBuf>> {
    let dir = rel.parent().unwrap_or(Path::new(""));
    if blocks.is_empty() {
        return vec![resolve(dir, path)];
    }
    let name = rel.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let mod_rs = matches!(name, "mod.rs" | "lib.rs" | "main.rs") || dir == Path::new("src/bin");
    let nested = |base: std::path::PathBuf| {
        let base = blocks.iter().fold(base, |base, block| base.join(block));
        resolve(&base, path)
    };
    let mut targets = vec![nested(dir.to_path_buf())];
    if !mod_rs {
        let stem = rel.file_stem().unwrap_or_default();
        targets.push(nested(dir.join(stem)));
    }
    targets
}

/// `path:line` of every production reference in `files` (crate-relative path, source). A
/// test-named file is skipped only when every declaration that may load it is gated by
/// `#[cfg(test)]` or sits in a file already skipped; one declared nowhere is not in the crate,
/// unless it is a crate root Cargo builds on its own.
/// Any other file, test-named or not, is scanned (an inner `#![cfg(test)]` still exempts it). A
/// production `path` or `include!` is itself a finding unless every file it may resolve to, by
/// exact path, is one of the scanned `files`.
fn gate_findings(files: &[(std::path::PathBuf, String)]) -> Vec<String> {
    let decls: Vec<(usize, Declaration)> = files
        .iter()
        .enumerate()
        .flat_map(|(from, (_, src))| file_declarations(src).into_iter().map(move |d| (from, d)))
        .collect();
    let mut skipped = vec![false; files.len()];
    loop {
        let mut changed = false;
        for (n, (rel, _)) in files.iter().enumerate() {
            if skipped[n] || !test_named(rel) || crate_root(rel) {
                continue;
            }
            let name = rel.file_name().and_then(|n| n.to_str()).unwrap_or("");
            let test_only = decls
                .iter()
                .filter(|(_, d)| match &d.loads {
                    Loads::File(file) => file == name,
                    Loads::Any => true,
                })
                .all(|(from, d)| d.gated || (*from != n && skipped[*from]));
            if test_only {
                skipped[n] = true;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    let scanned = |target: &Option<std::path::PathBuf>| {
        target
            .as_ref()
            .is_some_and(|target| files.iter().any(|(rel, _)| rel == target))
    };
    let unseen: Vec<String> = decls
        .iter()
        .filter(|(from, d)| !d.gated && !skipped[*from])
        .filter_map(|(from, d)| {
            let rel = &files[*from].0;
            match &d.explicit {
                Explicit::No => None,
                Explicit::Unresolvable => {
                    Some(format!("{}: loads an unresolvable file", rel.display()))
                }
                Explicit::Literal(path, blocks) => {
                    (!explicit_targets(rel, path, blocks).iter().all(scanned))
                        .then(|| format!("{}: loads unscanned {path}", rel.display()))
                }
            }
        })
        .collect();
    files
        .iter()
        .zip(skipped.iter().copied())
        .filter(|(_, skipped)| !skipped)
        .flat_map(|((rel, src), _)| {
            let bridge_module = rel == Path::new(BRIDGE_MODULE);
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
fn no_production_damage_write_bypasses_the_bound_attacker_slot() -> std::io::Result<()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    collect(&root.join("src"), root, &mut files)?;
    let found = gate_findings(&files);
    assert!(
        found.is_empty(),
        "D295 §3 item 3 gate (A2): production references to the unbound attacker write \
         {BRIDGES:?}: {found:?}. Production damage writes go through the bound slot \
         (`commit_damage_for_bound_attacker` and the Ability bridges)."
    );
    Ok(())
}

#[test]
fn gate_fails_a_synthetic_unbound_caller() {
    let carrier = (
        BRIDGE_MODULE,
        "#[cfg(test)]\npub(crate) fn commit_damage_for_attacker() {}\n\
         pub(crate) fn commit_damage_for_bound_attacker() {}\n",
    );
    let run = |extra: &[(&str, &str)]| {
        let files: Vec<(std::path::PathBuf, String)> = [carrier]
            .iter()
            .chain(extra)
            .map(|(p, s)| ((*p).into(), (*s).to_owned()))
            .collect();
        gate_findings(&files)
    };
    // The canonical test-only entry and the bound path are fine.
    assert_eq!(run(&[]), Vec::<String>::new());
    let bound = "fn cast(owner: &mut O) {\n    \
                 let _ = commit_exact_owner_damage(owner, resolved, plan, actor, command);\n    \
                 owner.commit_damage_for_bound_attacker(t, actor, command, 0, d);\n}\n";
    assert_eq!(run(&[("src/spell/bound.rs", bound)]), Vec::<String>::new());
    // A production caller with a caller-supplied attacker is flagged wherever it is.
    let unbound = "fn cast(owner: &mut O) {\n    \
                   owner.commit_damage_for_attacker(t, AttackerCommand::new(c, 1, cmd, 0), d);\n}\n";
    assert_eq!(
        run(&[("src/spell/cast.rs", unbound)]),
        vec!["src/spell/cast.rs:2".to_owned()]
    );
    let inside = format!(
        "{}fn live() {{ commit_damage_for_attacker(); }}\n",
        carrier.1
    );
    assert_eq!(
        gate_findings(&[(BRIDGE_MODULE.into(), inside)]),
        vec![format!("{BRIDGE_MODULE}:4")]
    );
    // Removing `#[cfg(test)]` from the canonical entry keeps it exempt only as a definition.
    let ungated = "pub(crate) fn commit_damage_for_attacker() {}\n";
    assert_eq!(
        gate_findings(&[("src/combat.rs".into(), ungated.to_owned())]),
        vec!["src/combat.rs:1".to_owned()]
    );
}

#[test]
fn scan_flags_a_production_reference() {
    let src = "fn live() {\n    let _ = commit_damage_for_attacker(a, b, c, d);\n}\n\
               use crate::ability::commit::commit_damage_for_attacker;\n";
    assert_eq!(production_references(src, false), vec![2, 4]);
    assert_eq!(
        production_references("fn f() { g(commit_damage_for_attacker) }", false),
        vec![1]
    );
    // `#[cfg(test)]` as macro tokens does not gate the expansion.
    let in_macro = "fn f() {\n    m!(#[cfg(test)] crate::x::commit_damage_for_attacker);\n}\n\
                    n! { #[cfg(test)] commit_damage_for_attacker() }\n";
    assert_eq!(production_references(in_macro, false), vec![2, 4]);
    let unicode = "μ! {\n    #[cfg(test)]\n    fn live() { commit_damage_for_attacker() }\n}\n";
    assert_eq!(production_references(unicode, false), vec![3]);
    // A Unicode-prefixed identifier is a different name, not a bridge reference.
    assert_eq!(
        production_references("fn f() { μcommit_damage_for_attacker(); }", false),
        Vec::<usize>::new()
    );
    // `#[cfg(test)]` on an element, field or arm exempts nothing beyond it.
    let nodes = "fn f() {\n    let _ = [1, #[cfg(test)] 0, commit_damage_for_attacker()];\n\
                 let _ = S { #[cfg(test)] a: 0, b: commit_damage_for_attacker() };\n\
                 match x { #[cfg(test)] A => 0, _ => commit_damage_for_attacker() }\n\
                 #[cfg(test)] g(); commit_damage_for_attacker();\n}\n";
    assert_eq!(production_references(nodes, false), vec![2, 3, 4, 5]);
    // A same-named wrapper outside the bridge module is no exemption, nor is a duplicate inside it.
    let wrapper = "fn commit_damage_for_attacker() {\n    commit_damage_for_attacker();\n}\n";
    assert_eq!(production_references(wrapper, false), vec![1, 2]);
    let twice =
        "fn commit_damage_for_attacker() {}\nmod m { fn commit_damage_for_attacker() {} }\n";
    assert_eq!(production_references(twice, true), vec![1, 2]);
    // A non-test cfg and a trailing caller after an exempt body are still references.
    let mixed = "#[cfg(not(test))]\nfn a() { commit_damage_for_attacker(); }\n\
                 fn commit_damage_for_attacker() {}\nfn b() { commit_damage_for_attacker(); }\n";
    assert_eq!(production_references(mixed, true), vec![2, 4]);
    // An inner cfg(test) after the leading attributes is no longer an inner attribute.
    let late =
        "#![allow(dead_code)]\nfn a() {}\n#![cfg(test)]\nfn b() { commit_damage_for_attacker(); }";
    assert_eq!(production_references(late, false), vec![4]);
}

#[test]
fn scan_ignores_test_comment_string_and_bridge_bodies() {
    let src = r##"
// commit_damage_for_attacker in a line comment
/* commit_damage_for_attacker /* nested */ still a comment */
const S: &str = "commit_damage_for_attacker";
const R: &str = r#"commit_damage_for_attacker"#;
const C: char = '{';
#[allow(dead_code)]
pub(crate) fn commit_damage_for_attacker(x: u8) -> u8 { x }
pub(crate) fn commit_damage_for_bound_attacker(x: u8) -> u8 {
    x
}
#[cfg(test)]
mod tests {
    fn t() { commit_damage_for_attacker(); }
}
#[cfg(test)]
use super::commit_damage_for_attacker;
#[cfg(test)]
#[allow(dead_code)]
pub(crate) fn helper() { commit_damage_for_attacker(1); }
fn body() {
    #[cfg(test)]
    let _probe = commit_damage_for_attacker();
}
"##;
    assert_eq!(production_references(src, true), Vec::<usize>::new());
    for test_file in [
        "#![cfg(test)]\nfn t() { commit_damage_for_attacker(); }",
        "//! docs\n#![allow(dead_code)]\n# ! [ cfg ( test ) ]\nfn t() { commit_damage_for_attacker(); }",
    ] {
        assert_eq!(production_references(test_file, false), Vec::<usize>::new());
    }
}

#[test]
fn test_named_files_are_skipped_only_when_test_only() {
    let call = "fn live() { commit_damage_for_attacker(); }\n";
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
        assert!(run(decl, &[]).contains(&flagged[0]), "{decl}");
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
        vec!["src/lib.rs: loads unscanned ../../elsewhere/live.rs".to_owned()]
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

#[test]
fn explicit_loads_resolve_by_exact_path() {
    let commit = (
        "src/ability/commit.rs",
        "pub(crate) fn commit_exact_owner_damage() {}",
    );
    let run = |decl_file: &str, decl: &str, extra: &[(&str, &str)]| {
        let mut files: Vec<(std::path::PathBuf, String)> = vec![
            (decl_file.into(), decl.to_owned()),
            (commit.0.into(), commit.1.to_owned()),
        ];
        files.extend(extra.iter().map(|(p, s)| ((*p).into(), (*s).to_owned())));
        gate_findings(&files)
    };
    let none = Vec::<String>::new();
    // A same-named file outside `src/` does not collide with the scanned `src/ability/commit.rs`.
    for (decl, path) in [
        (
            "#[path = \"../generated/commit.rs\"]\nmod generated;",
            "../generated/commit.rs",
        ),
        (
            "include!(\"../generated/commit.rs\");",
            "../generated/commit.rs",
        ),
        (
            "#[path = \"ability/../../src/x/commit.rs\"]\nmod x;",
            "ability/../../src/x/commit.rs",
        ),
        ("#[path = \"/abs/commit.rs\"]\nmod abs;", "/abs/commit.rs"),
        (
            "#[path = \"../../../../commit.rs\"]\nmod up;",
            "../../../../commit.rs",
        ),
    ] {
        assert_eq!(
            run("src/lib.rs", decl, &[]),
            vec![format!("src/lib.rs: loads unscanned {path}")],
            "{decl}"
        );
    }
    // The exact scanned file, through any spelling, is fine; so is a test-only load.
    for decl in [
        "#[path = \"ability/commit.rs\"]\nmod commit;",
        "#[path = \"./ability/../ability/commit.rs\"]\nmod commit;",
        "include!(\"ability/commit.rs\");",
        "#[cfg(test)]\n#[path = \"../generated/commit.rs\"]\nmod generated;",
    ] {
        assert_eq!(run("src/lib.rs", decl, &[]), none, "{decl}");
    }
    // Relative to the declaring file's directory, not the crate root.
    assert_eq!(
        run("src/ability/mod.rs", "#[path = \"commit.rs\"]\nmod c;", &[]),
        none
    );
    assert_eq!(
        run("src/combat.rs", "#[path = \"commit.rs\"]\nmod c;", &[]),
        vec!["src/combat.rs: loads unscanned commit.rs".to_owned()]
    );
    // Inside inline blocks: a `mod.rs`-like file descends through block names only; any other
    // file may also resolve through its stem, and both targets must be scanned.
    let nested = "mod inner {\n    #[path = \"x.rs\"]\n    mod x;\n}";
    assert_eq!(run("src/lib.rs", nested, &[("src/inner/x.rs", "")]), none);
    assert_eq!(
        run("src/a.rs", nested, &[("src/a/inner/x.rs", "")]),
        vec!["src/a.rs: loads unscanned x.rs".to_owned()]
    );
    assert_eq!(
        run(
            "src/a.rs",
            nested,
            &[("src/a/inner/x.rs", ""), ("src/inner/x.rs", "")]
        ),
        none
    );
    // A path no lexical reading can resolve is reported even for a non-test file.
    for decl in [
        "#[cfg_attr(not(test), path = \"../generated/commit.rs\")]\nmod generated;",
        "include!(concat!(env!(\"OUT_DIR\"), \"/commit.rs\"));",
        "#[path = \"ability\\x2fcommit.rs\"]\nmod commit;",
    ] {
        assert_eq!(
            run("src/lib.rs", decl, &[]),
            vec!["src/lib.rs: loads an unresolvable file".to_owned()],
            "{decl}"
        );
    }
}

#[test]
fn macro_loads_crate_roots_and_attribute_order() {
    let call = "fn live() { commit_damage_for_attacker(); }\n";
    let run = |files: &[(&str, &str)]| {
        let files: Vec<(std::path::PathBuf, String)> = files
            .iter()
            .map(|(p, s)| ((*p).into(), (*s).to_owned()))
            .chain([("src/ability/commit.rs".into(), String::new())])
            .collect();
        gate_findings(&files)
    };
    let unresolvable = vec!["src/lib.rs: loads an unresolvable file".to_owned()];
    // A load inside any macro token tree has no lexical base, whatever blocks surround it.
    for decl in [
        "m! { mod fake { #[path = \"../ability/commit.rs\"] mod external; } }",
        "macro_rules! m { () => { #[path = \"ability/commit.rs\"] mod c; }; }",
        "macro_rules! m { () => { include!(\"ability/commit.rs\"); }; }",
    ] {
        assert_eq!(run(&[("src/lib.rs", decl)]), unresolvable, "{decl}");
    }
    // Auto-discovered binaries are crate roots and never skipped, test-named or not.
    for bin in [
        "src/bin/tool_tests.rs",
        "src/bin/tests.rs",
        "src/bin/tool/main.rs",
    ] {
        assert_eq!(run(&[(bin, call)]), vec![format!("{bin}:1")], "{bin}");
    }
    // An undeclared test-named file elsewhere is still not in the crate.
    assert_eq!(run(&[("src/a/tool_tests.rs", call)]), Vec::<String>::new());
    // `#[cfg(test)]` gates the whole attribute sequence, before or after `#[path]`.
    for decl in [
        "#[path = \"../tests/support.rs\"]\n#[cfg(test)]\nmod support;",
        "#[path = \"../tests/support.rs\"]\n#[allow(dead_code)]\n#[cfg(test)]\nmod support;",
    ] {
        assert_eq!(run(&[("src/lib.rs", decl)]), Vec::<String>::new(), "{decl}");
    }
    assert_eq!(
        run(&[(
            "src/lib.rs",
            "#[path = \"../tests/support.rs\"]\n#[cfg(not(test))]\nmod support;"
        )]),
        vec!["src/lib.rs: loads unscanned ../tests/support.rs".to_owned()]
    );
}
