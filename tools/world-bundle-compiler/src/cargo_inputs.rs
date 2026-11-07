//! PIN-SCOPE-1: the root `Cargo.toml` and `Cargo.lock` as inputs of the `inputs_digest`, each
//! reduced to what can reach a build of the World crates, so a PR that adds an unrelated
//! workspace crate or bumps an unrelated dependency does not move the World pin.
//!
//! - `Cargo.lock`: one canonical line per package in the dependency closure of the World crates
//!   (name, version, source, checksum and resolved dependencies), walked over every
//!   `dependencies` edge of the lock. The lock graph holds normal, build, dev, optional and
//!   target-specific edges alike, so the closure is a superset of what a locked build of the
//!   compiler can compile. A path package in the closure must be a World crate, whose
//!   directory is a listed input; any other one is refused, never silently left out.
//! - root `Cargo.toml`: the whole file without comment and blank lines, without the
//!   `[workspace]` `members` and `exclude` lists (membership cannot change the closure without
//!   changing the lock), and without the `[workspace.dependencies]` entries of packages outside
//!   the closure (a World crate inherits only entries of packages in its closure). Profiles,
//!   patches, lints, `[workspace.package]` and every other table count as written.
//!
//! Anything the readers do not recognise is refused (`Cargo.lock`) or kept (`Cargo.toml`), so
//! a format change makes the digest fail or stricter, never weaker.

use std::collections::{BTreeMap, BTreeSet};

use oteryn_world_bundle_compiler::Error;

/// The workspace packages the World bundle is built from, with their directories. Each
/// directory is an `INPUT_PATHS` entry, so its sources are hashed blob by blob.
pub(super) const WORLD_PACKAGES: &[(&str, &str)] = &[
    (
        "oteryn-world-bundle-compiler",
        "tools/world-bundle-compiler/",
    ),
    ("oteryn-world-bundle", "crates/world-bundle/"),
];

fn lock_error(what: impl std::fmt::Display) -> Error {
    Error::Format(format!("Cargo.lock: {what}"))
}

fn manifest_error(what: impl std::fmt::Display) -> Error {
    Error::Format(format!("Cargo.toml: {what}"))
}

#[derive(Default)]
struct Package {
    name: Option<String>,
    version: Option<String>,
    source: Option<String>,
    checksum: Option<String>,
    dependencies: Option<Vec<String>>,
}

impl Package {
    fn name(&self) -> &str {
        self.name.as_deref().unwrap_or_default()
    }

    fn version(&self) -> &str {
        self.version.as_deref().unwrap_or_default()
    }

    /// `name version source`, with `path` for a package without a source.
    fn id(&self) -> String {
        format!(
            "{} {} {}",
            self.name(),
            self.version(),
            self.source.as_deref().unwrap_or("path")
        )
    }
}

/// A `"…"` value of the lock: no escapes, no inner quote.
fn quoted(value: &str) -> Result<String, Error> {
    value
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
        .filter(|inner| !inner.contains(['"', '\\']))
        .map(str::to_owned)
        .ok_or_else(|| lock_error(format!("`{value}` is not a plain string")))
}

/// The lock format version and its packages. Refuses any key, table or form it does not know.
fn parse_lock(text: &str) -> Result<(u32, Vec<Package>), Error> {
    enum At {
        Top,
        Package,
        UnusedPatch,
    }
    let mut at = At::Top;
    let mut version = None;
    let mut packages: Vec<Package> = Vec::new();
    let mut array: Option<Vec<String>> = None;
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(items) = array.as_mut() {
            if trimmed == "]" {
                if let Some(package) = packages.last_mut() {
                    package.dependencies = array.take();
                }
            } else {
                let item = trimmed
                    .strip_suffix(',')
                    .ok_or_else(|| lock_error(format!("`{trimmed}` in a dependency list")))?;
                items.push(quoted(item)?);
            }
            continue;
        }
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if trimmed.starts_with('[') {
            at = match trimmed {
                "[[package]]" => {
                    packages.push(Package::default());
                    At::Package
                }
                // An unused patch is applied to nothing; it cannot reach a build.
                "[[patch.unused]]" => At::UnusedPatch,
                other => return Err(lock_error(format!("unknown table `{other}`"))),
            };
            continue;
        }
        let (key, value) = trimmed
            .split_once(" = ")
            .ok_or_else(|| lock_error(format!("`{trimmed}` is not `key = value`")))?;
        match at {
            At::UnusedPatch => {}
            At::Top if key == "version" && version.is_none() => {
                version = Some(
                    value
                        .parse::<u32>()
                        .map_err(|_| lock_error(format!("version `{value}`")))?,
                );
            }
            At::Top => return Err(lock_error(format!("unknown top-level key `{key}`"))),
            At::Package => {
                let package = packages
                    .last_mut()
                    .ok_or_else(|| lock_error("a key outside a package"))?;
                let duplicate = || lock_error(format!("a package repeats `{key}`"));
                match key {
                    "name" | "version" | "source" | "checksum" => {
                        let slot = match key {
                            "name" => &mut package.name,
                            "version" => &mut package.version,
                            "source" => &mut package.source,
                            _ => &mut package.checksum,
                        };
                        if slot.replace(quoted(value)?).is_some() {
                            return Err(duplicate());
                        }
                    }
                    "dependencies" if package.dependencies.is_none() => match value {
                        "[" => array = Some(Vec::new()),
                        "[]" => package.dependencies = Some(Vec::new()),
                        other => {
                            return Err(lock_error(format!("dependencies `{other}`")));
                        }
                    },
                    "dependencies" => return Err(duplicate()),
                    other => return Err(lock_error(format!("unknown package key `{other}`"))),
                }
            }
        }
    }
    if array.is_some() {
        return Err(lock_error("an unterminated dependency list"));
    }
    let version = version.ok_or_else(|| lock_error("no format version"))?;
    // Format 3 and 4 hold each checksum on its package; earlier formats keep them elsewhere.
    if !(3..=4).contains(&version) {
        return Err(lock_error(format!(
            "format version {version} is not 3 or 4"
        )));
    }
    for package in &packages {
        if package.name.is_none() || package.version.is_none() {
            return Err(lock_error("a package without a name or version"));
        }
    }
    Ok((version, packages))
}

/// The one package a lock dependency entry names: `name`, `name version` or
/// `name version (source)`.
fn resolve<'a>(packages: &'a [Package], entry: &str) -> Result<&'a Package, Error> {
    let mut parts = entry.splitn(3, ' ');
    let name = parts.next().unwrap_or_default();
    let version = parts.next();
    let source = match parts.next() {
        Some(source) => Some(
            source
                .strip_prefix('(')
                .and_then(|rest| rest.strip_suffix(')'))
                .ok_or_else(|| lock_error(format!("dependency `{entry}`")))?,
        ),
        None => None,
    };
    let mut hits = packages.iter().filter(|package| {
        package.name() == name
            && version.is_none_or(|version| package.version() == version)
            && source.is_none_or(|source| package.source.as_deref() == Some(source))
    });
    match (hits.next(), hits.next()) {
        (Some(package), None) => Ok(package),
        (None, _) => Err(lock_error(format!("dependency `{entry}` names no package"))),
        (Some(_), Some(_)) => Err(lock_error(format!("dependency `{entry}` is ambiguous"))),
    }
}

/// The World crates' dependency closure in the lock: its canonical text and package names.
pub(super) fn lock_closure(text: &str) -> Result<(String, BTreeSet<String>), Error> {
    let (version, packages) = parse_lock(text)?;
    let mut queue = Vec::new();
    for (name, _) in WORLD_PACKAGES {
        let mut roots = packages
            .iter()
            .filter(|package| package.name() == *name && package.source.is_none());
        match (roots.next(), roots.next()) {
            (Some(root), None) => queue.push(root),
            _ => {
                return Err(lock_error(format!(
                    "the World crate `{name}` is not exactly one path package"
                )));
            }
        }
    }
    let mut closure: BTreeMap<String, String> = BTreeMap::new();
    let mut names = BTreeSet::new();
    while let Some(package) = queue.pop() {
        let id = package.id();
        if closure.contains_key(&id) {
            continue;
        }
        if package.source.is_none()
            && !WORLD_PACKAGES
                .iter()
                .any(|(name, _)| *name == package.name())
        {
            return Err(lock_error(format!(
                "the World crates reach the path package `{}`; add its directory to \
                 INPUT_PATHS (and WORLD_BUNDLE_INPUT_PATHS) and the package to WORLD_PACKAGES",
                package.name()
            )));
        }
        let mut edges = BTreeSet::new();
        for entry in package.dependencies.iter().flatten() {
            let dependency = resolve(&packages, entry)?;
            edges.insert(dependency.id());
            queue.push(dependency);
        }
        let edges: Vec<String> = edges.into_iter().collect();
        closure.insert(
            id.clone(),
            format!(
                "{id} {} [{}]\n",
                package.checksum.as_deref().unwrap_or("-"),
                edges.join(", ")
            ),
        );
        names.insert(package.name().to_owned());
    }
    let mut canonical = format!("lock format {version}\n");
    canonical.extend(closure.into_values());
    Ok((canonical, names))
}

/// Where a scan of TOML text stands.
#[derive(Clone, Copy, PartialEq)]
enum Lexical {
    Code,
    Comment,
    Basic,
    Literal,
    MultiBasic,
    MultiLiteral,
}

/// Splits TOML text into statements: one header line, or one `key = value` with its whole
/// (possibly multi-line) value. Comment and blank lines are dropped.
fn statements(text: &str) -> Result<Vec<&str>, Error> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let end = text[i..].find('\n').map_or(bytes.len(), |n| i + n + 1);
        let line = text[i..end].trim();
        if line.is_empty() || line.starts_with('#') {
            i = end;
            continue;
        }
        if line.starts_with('[') {
            out.push(&text[i..end]);
            i = end;
            continue;
        }
        let start = i;
        let (mut state, mut depth, mut seen_equals) = (Lexical::Code, 0usize, false);
        loop {
            let Some(&byte) = bytes.get(i) else {
                if !matches!(state, Lexical::Code | Lexical::Comment) || depth != 0 {
                    return Err(manifest_error("an unterminated value"));
                }
                break;
            };
            let rest = &bytes[i..];
            match state {
                // The newline that ends a comment is read again as code.
                Lexical::Comment if byte == b'\n' => {
                    state = Lexical::Code;
                    continue;
                }
                Lexical::Comment => {}
                Lexical::Basic if byte == b'\\' => i += 1,
                Lexical::Basic if byte == b'"' => state = Lexical::Code,
                Lexical::Basic | Lexical::Literal if byte == b'\n' => {
                    return Err(manifest_error("a string runs past its line"));
                }
                Lexical::Literal if byte == b'\'' => state = Lexical::Code,
                Lexical::MultiBasic if byte == b'\\' => i += 1,
                Lexical::MultiBasic if rest.starts_with(b"\"\"\"") => {
                    state = Lexical::Code;
                    i += 2;
                }
                Lexical::MultiLiteral if rest.starts_with(b"'''") => {
                    state = Lexical::Code;
                    i += 2;
                }
                Lexical::Basic | Lexical::Literal | Lexical::MultiBasic | Lexical::MultiLiteral => {
                }
                Lexical::Code => match byte {
                    b'"' if rest.starts_with(b"\"\"\"") => {
                        state = Lexical::MultiBasic;
                        i += 2;
                    }
                    b'\'' if rest.starts_with(b"'''") => {
                        state = Lexical::MultiLiteral;
                        i += 2;
                    }
                    b'"' => state = Lexical::Basic,
                    b'\'' => state = Lexical::Literal,
                    b'#' => state = Lexical::Comment,
                    b'=' => seen_equals = true,
                    b'[' | b'{' if seen_equals => depth += 1,
                    b']' | b'}' if seen_equals => {
                        depth = depth
                            .checked_sub(1)
                            .ok_or_else(|| manifest_error("an unbalanced value"))?;
                    }
                    b'\n' if depth == 0 => {
                        i += 1;
                        break;
                    }
                    _ => {}
                },
            }
            i += 1;
        }
        if !seen_equals {
            return Err(manifest_error(format!(
                "`{}` is not `key = value`",
                text[start..i].trim()
            )));
        }
        out.push(&text[start..i.min(bytes.len())]);
    }
    Ok(out)
}

/// The table a header line opens (`[a.b]` or `[[a.b]]`), as written between the brackets.
fn table_name(header: &str) -> &str {
    let header = header.trim();
    let inner = header
        .strip_prefix("[[")
        .and_then(|rest| rest.split_once("]]"))
        .or_else(|| {
            header
                .strip_prefix('[')
                .and_then(|rest| rest.split_once(']'))
        })
        .map_or(header, |(inner, _)| inner);
    inner.trim()
}

/// The first segment of a statement's bare key, or `None` for a quoted key.
fn first_key(statement: &str) -> Option<&str> {
    let key = statement.split('=').next()?.trim();
    if key.starts_with(['"', '\'']) {
        return None;
    }
    key.split('.').next().map(str::trim)
}

/// The root manifest reduced to what can reach a build of the World crates (module doc).
pub(super) fn workspace_manifest(text: &str, closure: &BTreeSet<String>) -> Result<String, Error> {
    let mut table = "";
    let mut canonical = String::new();
    for statement in statements(text)? {
        if statement.trim_start().starts_with('[') {
            table = table_name(statement);
        } else {
            let key = first_key(statement);
            let dropped = match table {
                "workspace" => matches!(key, Some("members" | "exclude")),
                // A renamed entry (`package = …`) is kept whatever its key.
                "workspace.dependencies" => {
                    key.is_some_and(|key| !closure.contains(key) && !statement.contains("package"))
                }
                _ => false,
            };
            if dropped {
                continue;
            }
        }
        canonical.push_str(statement.trim_end());
        canonical.push('\n');
    }
    Ok(canonical)
}

/// The `[package] name` of a crate manifest, read line by line.
pub(super) fn package_name(manifest: &str) -> Option<String> {
    let mut in_package = false;
    for line in manifest.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_package = table_name(line) == "package";
        } else if in_package
            && let Some(value) = line
                .strip_prefix("name")
                .and_then(|rest| rest.trim_start().strip_prefix('='))
        {
            return value
                .trim()
                .strip_prefix('"')
                .and_then(|rest| rest.split_once('"'))
                .map(|(name, _)| name.to_owned());
        }
    }
    None
}

#[cfg(test)]
#[path = "cargo_inputs_tests.rs"]
mod tests;
