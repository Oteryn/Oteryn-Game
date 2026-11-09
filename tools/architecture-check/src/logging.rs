//! Event-registry coverage and the server console-output ratchet.

use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Emission {
    module: String,
    event: String,
    file: String,
    line: usize,
    structured: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct ConsoleCall {
    owner: String,
    site: String,
}

/// Validate event fragments and reject increases in direct console output.
pub fn validate(root: &Path, update_baseline: bool) -> Result<(), String> {
    let emissions = scan_emissions(root)?;
    let registry = read_registry(root)?;
    let mut errors = Vec::new();
    for emission in &emissions {
        if !registry
            .get(&emission.module)
            .is_some_and(|events| events.contains(&emission.event))
        {
            errors.push(format!(
                "{}:{}: unregistered logging event {}/{}",
                emission.file, emission.line, emission.module, emission.event
            ));
        }
    }
    let emitted = emissions
        .iter()
        .map(|emission| (emission.module.as_str(), emission.event.as_str()))
        .collect::<BTreeSet<_>>();
    for (module, events) in &registry {
        for event in events {
            if !emitted.contains(&(module, event)) {
                eprintln!("warning: registered logging event is not emitted: {module}/{event}");
            }
        }
    }
    validate_console_ratchet(root, update_baseline, &mut errors)?;
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("\n"))
    }
}

/// Return a deterministic, per-module logging coverage report.
pub fn coverage(root: &Path) -> Result<String, String> {
    let emissions = scan_emissions(root)?;
    let registry = read_registry(root)?;
    let calls = scan_console_calls(root)?;
    let mut output = String::new();
    for (module, events) in registry {
        let module_emissions = emissions.iter().filter(|item| item.module == module);
        let structured = module_emissions
            .clone()
            .filter(|item| item.structured)
            .count();
        let free_form = module_emissions.filter(|item| !item.structured).count();
        let console = calls
            .iter()
            .filter(|call| module_for_path(&call.site) == module)
            .count();
        output.push_str(&format!(
            "module={module} registered={} structured={structured} free_form={free_form} console={console}\n",
            events.len()
        ));
    }
    Ok(output)
}

fn read_registry(root: &Path) -> Result<BTreeMap<String, BTreeSet<String>>, String> {
    let directory = root.join("docs/contracts/logging/events");
    let mut paths = fs::read_dir(&directory)
        .map_err(|error| format!("cannot read {}: {error}", directory.display()))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("cannot list {}: {error}", directory.display()))?;
    paths.sort();
    let mut registry = BTreeMap::new();
    let levels = ["error", "warn", "info", "debug"];
    for path in paths
        .into_iter()
        .filter(|path| path.extension().is_some_and(|x| x == "json"))
    {
        let value: Value = serde_json::from_str(
            &fs::read_to_string(&path)
                .map_err(|error| format!("cannot read {}: {error}", path.display()))?,
        )
        .map_err(|error| format!("{}: invalid JSON: {error}", path.display()))?;
        let object = value
            .as_object()
            .ok_or_else(|| format!("{}: fragment must be an object", path.display()))?;
        if object.len() != 3
            || !object.contains_key("module")
            || !object.contains_key("owner")
            || !object.contains_key("events")
        {
            return Err(format!(
                "{}: fragment must contain only module, owner and events",
                path.display()
            ));
        }
        let module = token(object.get("module"), "module", &path)?;
        token(object.get("owner"), "owner", &path)?;
        if path.file_stem().and_then(|x| x.to_str()) != Some(module) {
            return Err(format!(
                "{}: module must match the filename",
                path.display()
            ));
        }
        let entries = object
            .get("events")
            .and_then(Value::as_array)
            .ok_or_else(|| format!("{}: events must be an array", path.display()))?;
        let mut events = BTreeSet::new();
        for entry in entries {
            let event = entry
                .as_object()
                .ok_or_else(|| format!("{}: event must be an object", path.display()))?;
            let expected = ["event", "level", "span", "fields", "codes", "note"]
                .into_iter()
                .collect::<BTreeSet<_>>();
            if event.keys().map(String::as_str).collect::<BTreeSet<_>>() != expected {
                return Err(format!("{}: event has an invalid schema", path.display()));
            }
            let name = token(event.get("event"), "event", &path)?.to_owned();
            let level = event
                .get("level")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let fields_valid = event
                .get("fields")
                .and_then(Value::as_array)
                .is_some_and(|items| items.iter().all(|item| token_value(item, false)));
            let codes_valid = event
                .get("codes")
                .and_then(Value::as_array)
                .is_some_and(|items| {
                    items.iter().all(|item| {
                        item.as_str().is_some_and(|code| {
                            code.len() == 5
                                && code.starts_with('E')
                                && code[1..].bytes().all(|byte| byte.is_ascii_digit())
                        })
                    })
                });
            if !levels.contains(&level)
                || !event.get("span").is_some_and(Value::is_boolean)
                || !fields_valid
                || !codes_valid
                || !event.get("note").is_some_and(Value::is_string)
            {
                return Err(format!(
                    "{}: event {name} has an invalid schema",
                    path.display()
                ));
            }
            if !events.insert(name.clone()) {
                return Err(format!("{}: duplicate event {name}", path.display()));
            }
        }
        if registry.insert(module.to_owned(), events).is_some() {
            return Err(format!("duplicate module fragment {module}"));
        }
    }
    Ok(registry)
}

fn token<'a>(value: Option<&'a Value>, field: &str, path: &Path) -> Result<&'a str, String> {
    let text = value
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{}: {field} must be a string", path.display()))?;
    if text.is_empty()
        || !text
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'/'))
    {
        return Err(format!("{}: {field} must be a token", path.display()));
    }
    Ok(text)
}

fn token_value(value: &Value, allow_path: bool) -> bool {
    value.as_str().is_some_and(|text| {
        !text.is_empty()
            && text.bytes().all(|byte| {
                byte.is_ascii_alphanumeric()
                    || matches!(byte, b'_' | b'-')
                    || (allow_path && byte == b'/')
            })
    })
}

fn scan_emissions(root: &Path) -> Result<Vec<Emission>, String> {
    let mut result = Vec::new();
    for path in rust_sources(root)? {
        let relative = relative(root, &path);
        let source = fs::read_to_string(&path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        let source = source.split("#[cfg(test)]").next().unwrap_or(&source);
        let lines = source.lines().collect::<Vec<_>>();
        for (index, line) in lines.iter().enumerate() {
            if let Some(position) = line.find("event=") {
                let event = line[position + 6..]
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                    .collect::<String>();
                if !event.is_empty()
                    && !line[..position].ends_with("{MAGIC}\\nscope={}\\ngeneration={}\\n")
                {
                    let module = line.find("module=").map_or_else(
                        || module_for_path(&relative),
                        |at| {
                            line[at + 7..]
                                .chars()
                                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                                .collect()
                        },
                    );
                    result.push(Emission {
                        module,
                        event,
                        file: relative.clone(),
                        line: index + 1,
                        structured: false,
                    });
                }
            }
            if line.contains("Line::new(") {
                let window = lines[index..].join("\n");
                if let Some((module, event)) = literal_line_event(&window) {
                    result.push(Emission {
                        module,
                        event,
                        file: relative.clone(),
                        line: index + 1,
                        structured: true,
                    });
                }
            }
        }
    }
    result.sort();
    result.dedup();
    Ok(result)
}

fn literal_line_event(text: &str) -> Option<(String, String)> {
    let (position, _) = text.match_indices("Line::new(").next()?;
    let call = &text[position..];
    // Parse only through the constructor's closing parenthesis, never its fields.
    let args = call.match_indices(')').find_map(|(end, _)| {
        match syn::parse_str::<syn::Expr>(&call[..=end]).ok()? {
            syn::Expr::Call(expression) => Some(expression.args),
            _ => None,
        }
    })?;
    if args.len() != 3 {
        return None;
    }
    let literal = |argument: &syn::Expr| match argument {
        syn::Expr::Lit(expression) => match &expression.lit {
            syn::Lit::Str(value) => Some(value.value()),
            _ => None,
        },
        _ => None,
    };
    Some((literal(&args[1])?, literal(&args[2])?))
}

fn rust_sources(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut paths = Vec::new();
    for base in [root.join("crates"), root.join("apps")] {
        visit(&base, &mut paths)?;
    }
    paths.sort();
    Ok(paths)
}

fn visit(directory: &Path, paths: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(directory)
        .map_err(|error| format!("cannot read {}: {error}", directory.display()))?
    {
        let path = entry.map_err(|error| error.to_string())?.path();
        if path.is_dir() {
            let name = path
                .file_name()
                .and_then(|x| x.to_str())
                .unwrap_or_default();
            if !matches!(name, "tests" | "examples" | "benches" | "test_support")
                && !name.ends_with("_tests")
                && name != "lifted_tests"
                && name != "postgres_tests"
            {
                visit(&path, paths)?;
            }
        } else if path.extension().is_some_and(|x| x == "rs")
            && !path
                .file_name()
                .and_then(|x| x.to_str())
                .is_some_and(|x| x == "tests.rs" || x.ends_with("_tests.rs"))
        {
            paths.push(path);
        }
    }
    Ok(())
}

fn scan_console_calls(root: &Path) -> Result<Vec<ConsoleCall>, String> {
    let mut calls = Vec::new();
    for path in rust_sources(root)? {
        let file = relative(root, &path);
        if !ratchet_path(&file) {
            continue;
        }
        let owner = owner_for_path(&file);
        let source = fs::read_to_string(&path).map_err(|error| error.to_string())?;
        let source = source.split("#[cfg(test)]").next().unwrap_or(&source);
        for (line, text) in source.lines().enumerate() {
            if ["eprintln!(", "println!(", "eprint!(", "print!("]
                .iter()
                .any(|needle| text.contains(needle))
            {
                calls.push(ConsoleCall {
                    owner: owner.clone(),
                    site: format!("{file}:{}", line + 1),
                });
            }
        }
    }
    calls.sort();
    Ok(calls)
}

fn validate_console_ratchet(
    root: &Path,
    update: bool,
    errors: &mut Vec<String>,
) -> Result<(), String> {
    let path = root.join("tools/architecture-check/logging-eprintln-baseline.json");
    let calls = scan_console_calls(root)?;
    let mut actual: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for call in calls {
        actual.entry(call.owner).or_default().push(call.site);
    }
    let baseline: BTreeMap<String, Vec<String>> = serde_json::from_str(
        &fs::read_to_string(&path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?,
    )
    .map_err(|error| format!("invalid baseline: {error}"))?;
    for (owner, sites) in &actual {
        let old = baseline.get(owner).map(Vec::as_slice).unwrap_or_default();
        if sites.len() > old.len() {
            let additions = sites
                .iter()
                .filter(|site| !old.contains(site))
                .cloned()
                .collect::<Vec<_>>();
            errors.push(format!("direct console output rose for {owner}: {} > {}; additions: {}. Use Line/Span instead", sites.len(), old.len(), additions.join(", ")));
        } else if sites.len() < old.len() && !update {
            eprintln!(
                "warning: direct console output fell for {owner}: {} < {}; rerun logging-events with --update-baseline",
                sites.len(),
                old.len()
            );
        }
    }
    if update {
        fs::write(
            &path,
            format!(
                "{}\n",
                serde_json::to_string_pretty(&actual).map_err(|error| error.to_string())?
            ),
        )
        .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
    }
    Ok(())
}

fn ratchet_path(path: &str) -> bool {
    if path.starts_with("apps/game-server/src/") {
        return !path.contains("/qualification/");
    }
    path.starts_with("crates/")
        && ![
            "crates/client-",
            "crates/renderer",
            "crates/input-",
            "crates/synthetic-",
        ]
        .iter()
        .any(|prefix| path.starts_with(prefix))
}

fn owner_for_path(path: &str) -> String {
    let mut parts = path.split('/');
    match (parts.next(), parts.next(), parts.next()) {
        (Some("crates"), Some("gameplay"), Some(name)) => format!("crates/gameplay/{name}"),
        (Some("crates"), Some(name), _) => format!("crates/{name}"),
        (Some("apps"), Some(name), _) => format!("apps/{name}"),
        _ => path.to_owned(),
    }
}

fn module_for_path(path: &str) -> String {
    if path.starts_with("apps/game-server/src/node/") {
        return "node".to_owned();
    }
    if path.contains("/dispatch/") {
        return "gameplay_transport".to_owned();
    }
    if path.contains("native-admission-source") || path.contains("character-recovery-fence") {
        return "admission".to_owned();
    }
    for module in [
        "durability",
        "social",
        "spell",
        "combat",
        "items",
        "economy",
        "housing",
        "quests",
        "npc",
        "progression",
        "encounters",
        "monster-ai",
    ] {
        if path.contains(&format!("/{module}/")) {
            return module.replace('-', "_");
        }
    }
    owner_for_path(path)
        .rsplit('/')
        .next()
        .unwrap_or("runtime")
        .replace('-', "_")
}

fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    fn fixture(name: &str) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("oteryn-logging-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        for directory in [
            "crates/example/src",
            "apps",
            "docs/contracts/logging/events",
            "tools/architecture-check",
        ] {
            fs::create_dir_all(root.join(directory)).unwrap();
        }
        fs::write(
            root.join("crates/example/src/lib.rs"),
            "fn emit() { println!(\"event=ready\"); }\n",
        )
        .unwrap();
        fs::write(
            root.join("tools/architecture-check/logging-eprintln-baseline.json"),
            "{\"crates/example\": [\"crates/example/src/lib.rs:1\"]}\n",
        )
        .unwrap();
        root
    }

    fn fragment(root: &Path, events: &str) {
        fs::write(
            root.join("docs/contracts/logging/events/example.json"),
            format!(r#"{{"module":"example","owner":"crates/example","events":[{events}]}}"#),
        )
        .unwrap();
    }

    #[test]
    fn registered_event_and_unchanged_ratchet_pass() {
        let root = fixture("registered");
        fragment(
            &root,
            r#"{"event":"ready","level":"info","span":false,"fields":[],"codes":[],"note":"fixture"}"#,
        );
        assert_eq!(validate(&root, false), Ok(()));
    }

    #[test]
    fn unregistered_event_and_ratchet_increase_fail_with_site() {
        let root = fixture("unregistered");
        fragment(&root, "");
        fs::write(
            root.join("tools/architecture-check/logging-eprintln-baseline.json"),
            "{}\n",
        )
        .unwrap();
        let error = validate(&root, false).unwrap_err();
        assert!(error.contains("lib.rs:1: unregistered"));
        assert!(error.contains("lib.rs:1. Use Line/Span"));
    }

    #[test]
    fn structured_events_use_only_constructor_arguments() {
        let root = fixture("constructor-arguments");
        fs::write(
            root.join("crates/example/src/lib.rs"),
            r#"fn emit() {
    let dynamic = Line::new(level, self.module, &end_event)
        .field("outcome", FieldValue::Token(self.outcome))
        .field("ms", FieldValue::U64(milliseconds));
    let literal = Line::new(
        Level::Info,
        "example",
        "ready",
    ).field("other", "value");
    let second = Line::new(level_for(")"), "example", "second");
}"#,
        )
        .unwrap();
        let emissions = scan_emissions(&root).unwrap();
        assert_eq!(emissions.len(), 2);
        assert!(
            emissions
                .iter()
                .all(|item| item.structured && item.module == "example")
        );
        assert_eq!(emissions[0].event, "ready");
        assert_eq!(emissions[1].event, "second");
    }

    #[test]
    fn schema_and_duplicate_violations_fail() {
        let root = fixture("schema");
        fragment(
            &root,
            r#"{"event":"ready","level":"bad","span":false,"fields":[],"codes":[],"note":"fixture"}"#,
        );
        assert!(read_registry(&root).unwrap_err().contains("invalid schema"));
        fragment(
            &root,
            r#"{"event":"ready","level":"info","span":false,"fields":[],"codes":[],"note":"one"},{"event":"ready","level":"info","span":false,"fields":[],"codes":[],"note":"two"}"#,
        );
        assert!(
            read_registry(&root)
                .unwrap_err()
                .contains("duplicate event")
        );
    }

    #[test]
    fn lower_ratchet_can_be_accepted() {
        let root = fixture("lower");
        fragment(
            &root,
            r#"{"event":"ready","level":"info","span":false,"fields":[],"codes":[],"note":"fixture"}"#,
        );
        fs::write(
            root.join("tools/architecture-check/logging-eprintln-baseline.json"),
            "{\"crates/example\":[\"old:1\",\"old:2\"]}\n",
        )
        .unwrap();
        validate(&root, true).unwrap();
        let baseline = fs::read_to_string(
            root.join("tools/architecture-check/logging-eprintln-baseline.json"),
        )
        .unwrap();
        assert!(baseline.contains("lib.rs:1"));
        assert!(!baseline.contains("old:2"));
    }
}
