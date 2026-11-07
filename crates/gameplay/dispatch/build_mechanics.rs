// Directory-scanned mechanic registration, following REGISTRY-SPLIT-1. No parser dependency:
// headers are key=value lines followed by a [route] Rust expression, copied without rewriting.
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::Path;

#[derive(Debug)]
struct Mechanic {
    name: String,
    commands: Vec<u32>,
    transport: Option<String>,
    composed: Option<String>,
    authority: Option<String>,
    offer: Option<(u32, String, String)>,
    route: String,
}

fn identifier(value: &str) -> bool {
    value.starts_with(|c: char| c.is_ascii_lowercase())
        && value
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_')
}

fn parse(text: &str) -> Result<Mechanic, String> {
    let (header, route) = text
        .split_once("[route]\n")
        .ok_or("missing [route] expression")?;
    if route.trim().is_empty() {
        return Err("empty [route] expression".into());
    }
    let mut fields = BTreeMap::new();
    for line in header.lines().filter(|line| !line.is_empty()) {
        let (key, value) = line.split_once('=').ok_or("expected key=value")?;
        if !matches!(
            key,
            "name" | "commands" | "transport" | "composed" | "authority" | "offer"
        ) {
            return Err(format!("unknown field {key}"));
        }
        if fields.insert(key, value).is_some() {
            return Err(format!("duplicate field {key}"));
        }
    }
    let name = *fields.get("name").ok_or("missing name")?;
    if !identifier(name) {
        return Err(format!("invalid mechanic name {name}"));
    }
    let commands = fields
        .get("commands")
        .ok_or("missing commands")?
        .split(',')
        .map(|id| {
            id.parse::<u32>()
                .ok()
                .filter(|id| *id != 0)
                .ok_or_else(|| format!("invalid command id {id}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    for key in ["composed", "authority"] {
        if let Some(value) = fields.get(key)
            && !identifier(value)
        {
            return Err(format!("invalid {key} module {value}"));
        }
    }
    if let Some(value) = fields.get("transport")
        && !identifier(value.strip_prefix("pub ").unwrap_or(value))
    {
        return Err(format!("invalid transport module {value}"));
    }
    let offer = fields
        .get("offer")
        .map(|value| {
            let parts: Vec<_> = value.split(',').collect();
            let [order, path, available] = parts[..] else {
                return Err("offer expects order,path,availability_method".to_owned());
            };
            if !path.split("::").all(identifier) || !identifier(available) {
                return Err(format!("invalid offer {value}"));
            }
            Ok((
                order.parse::<u32>().map_err(|_| "invalid offer order")?,
                path.to_owned(),
                available.to_owned(),
            ))
        })
        .transpose()?;
    Ok(Mechanic {
        name: name.to_owned(),
        commands,
        transport: fields.get("transport").map(|s| (*s).to_owned()),
        composed: fields.get("composed").map(|s| (*s).to_owned()),
        authority: fields.get("authority").map(|s| (*s).to_owned()),
        offer,
        route: route.to_owned(),
    })
}

fn validate(mechanics: &[Mechanic]) -> Result<(), String> {
    let mut names = BTreeSet::new();
    let mut commands = BTreeSet::new();
    let mut transports = BTreeSet::new();
    let mut offers = BTreeSet::new();
    for mechanic in mechanics {
        if !names.insert(&mechanic.name) {
            return Err(format!("duplicate mechanic name {}", mechanic.name));
        }
        for command in &mechanic.commands {
            if !commands.insert(command) {
                return Err(format!("duplicate command id {command}"));
            }
        }
        if let Some(module) = &mechanic.transport
            && !transports.insert(module.strip_prefix("pub ").unwrap_or(module))
        {
            return Err(format!("duplicate transport module {module}"));
        }
        if let Some((order, _, _)) = &mechanic.offer
            && !offers.insert(order)
        {
            return Err(format!("duplicate offer order {order}"));
        }
    }
    Ok(())
}

pub(super) fn generate(root: &Path, out: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let directory = root.join("mechanics");
    println!("cargo:rerun-if-changed={}", directory.display());
    let mut paths = std::fs::read_dir(&directory)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?;
    paths.retain(|path| path.extension().is_some_and(|ext| ext == "mechanic"));
    paths.sort();
    let mut mechanics = Vec::new();
    for path in paths {
        println!("cargo:rerun-if-changed={}", path.display());
        let text = std::fs::read_to_string(&path)?.replace("\r\n", "\n");
        let mechanic = parse(&text).map_err(|error| format!("{}: {error}", path.display()))?;
        mechanics.push(mechanic);
    }
    if mechanics.is_empty() {
        return Err("no mechanic fragments".into());
    }
    validate(&mechanics)?;
    write_outputs(&mechanics, root, out)
}

fn write_outputs(
    mechanics: &[Mechanic],
    root: &Path,
    out: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut methods = String::new();
    for (name, pick) in [
        (
            "composed",
            (|m: &Mechanic| &m.composed) as fn(&Mechanic) -> &Option<String>,
        ),
        (
            "authority",
            (|m: &Mechanic| &m.authority) as fn(&Mechanic) -> &Option<String>,
        ),
    ] {
        writeln!(methods, "macro_rules! mechanic_{name}_methods {{ () => {{")?;
        for module in mechanics.iter().filter_map(|m| pick(m).as_ref()) {
            writeln!(
                methods,
                "$crate::gameplay_transport::{module}::{name}_methods!();"
            )?;
        }
        writeln!(methods, "}}; }}")?;
    }
    std::fs::write(out.join("mechanic_methods.rs"), methods)?;
    let mut modules = String::new();
    for module in mechanics.iter().filter_map(|m| m.transport.as_ref()) {
        let (visibility, name) = module
            .strip_prefix("pub ")
            .map_or(("", module.as_str()), |name| ("pub ", name));
        let path = root
            .join("src/gameplay_transport")
            .join(format!("{name}.rs"));
        writeln!(modules, "#[path = {path:?}] {visibility}mod {name};")?;
    }
    std::fs::write(out.join("mechanic_modules.rs"), modules)?;
    // Offer hooks retain the original order explicitly; dispatch order comes from file names.
    let mut hooks: Vec<_> = mechanics.iter().filter_map(|m| m.offer.as_ref()).collect();
    hooks.sort_by_key(|(order, _, _)| order);
    let mut offers = String::from(
        "fn mechanic_offers<A: FreshAdmissionAuthority>(authority: &A, offered: Vec<OfferedCapability>) -> Vec<OfferedCapability> {\n",
    );
    for (index, (_, path, available)) in hooks.iter().enumerate() {
        writeln!(
            offers,
            "{}crate::gameplay_transport::{path}(&offered, authority.{available}()){}",
            if index + 1 == hooks.len() {
                ""
            } else {
                "let offered = "
            },
            if index + 1 == hooks.len() { "" } else { ";" },
        )?;
    }
    if hooks.is_empty() {
        offers.push_str("offered\n");
    }
    offers.push_str("}\n");
    std::fs::write(out.join("mechanic_offers.rs"), offers)?;
    let mut routes = String::from(
        "pub(super) async fn execute<S, A>(stream: &mut S, authority: &A, session: &mut ServeState, command: oteryn_protocol_oteryn::ClientCommandView<'_>, outcome: &mut CommandOutcome) -> Result<Dispatch, ServeEnd>\nwhere S: AsyncRead + AsyncWrite + Unpin, A: FreshAdmissionAuthority {\nroute_registered(stream, authority, session, command, outcome).await\n}\n\nasync fn route_registered<S, A>(stream: &mut S, authority: &A, session: &mut ServeState, command: oteryn_protocol_oteryn::ClientCommandView<'_>, outcome: &mut CommandOutcome) -> Result<Dispatch, ServeEnd>\nwhere S: AsyncRead + AsyncWrite + Unpin, A: FreshAdmissionAuthority {\nOk(match command.command_type {\n",
    );
    let mut table = String::from("#[cfg(test)]\nconst REGISTERED_ROUTES: &[(u32, &str)] = &[\n");
    for mechanic in mechanics {
        let pattern = mechanic
            .commands
            .chunk_by(|left, right| left.checked_add(1) == Some(*right))
            .map(|ids| {
                if ids.len() >= 3 {
                    format!("{}..={}", ids[0], ids[ids.len() - 1])
                } else {
                    ids.iter()
                        .map(u32::to_string)
                        .collect::<Vec<_>>()
                        .join(" | ")
                }
            })
            .collect::<Vec<_>>()
            .join(" | ");
        writeln!(routes, "{pattern} => {{\n{}\n}},", mechanic.route)?;
        for command in &mechanic.commands {
            writeln!(table, "({command}, {:?}),", mechanic.name)?;
        }
    }
    routes.push_str("_ => Dispatch::Unregistered,\n})\n}\n");
    table.push_str("] ;\n");
    routes.push_str(&table);
    std::fs::write(out.join("mechanic_routes.rs"), routes)?;
    Ok(())
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn registry_rejects_duplicate_names_and_command_ids() {
        let first = "name=first\ncommands=7\n[route]\nDispatch::Unregistered\n";
        let second = "name=second\ncommands=8\n[route]\nDispatch::Unregistered\n";
        assert!(
            validate(&[
                parse(first).expect("valid registry fixture"),
                parse(second).expect("valid registry fixture")
            ])
            .is_ok()
        );
        let same_name = second.replace("second", "first");
        assert!(
            validate(&[
                parse(first).expect("valid registry fixture"),
                parse(&same_name).expect("valid registry fixture")
            ])
            .expect_err("invalid registry fixture")
            .contains("duplicate mechanic name")
        );
        let same_command = second.replace("commands=8", "commands=7");
        assert!(
            validate(&[
                parse(first).expect("valid registry fixture"),
                parse(&same_command).expect("valid registry fixture")
            ])
            .expect_err("invalid registry fixture")
            .contains("duplicate command id")
        );
        assert!(
            validate(&[parse(&first.replace("commands=7", "commands=7,7"))
                .expect("valid registry fixture")])
            .expect_err("invalid registry fixture")
            .contains("duplicate command id")
        );
    }

    #[test]
    fn registry_rejects_invalid_headers() {
        for text in [
            "name=first\ncommands=0\n[route]\nbody",
            "name=first\ncommands=7\nunknown=x\n[route]\nbody",
            "name=first\nname=first\ncommands=7\n[route]\nbody",
            "name=first\ncommands=7\n[route]\n",
            "name=first\ncommands=7\noffer=1,bad-path,available\n[route]\nbody",
        ] {
            assert!(parse(text).is_err(), "{text}");
        }
    }
}
