//! Runtime capability facts (CAPS-SPLIT-1). Contract facts remain in the REGISTRY-SPLIT-1 tree.

use serde_json::Value;
use std::collections::BTreeSet;

const OFFER_SETS: &[(&str, &str)] = &[
    ("production", "PRODUCTION_OFFERS_V1"),
    ("item_view", "ITEM_VIEW_OFFERS_V1"),
    ("bundle_world", "BUNDLE_WORLD_OFFERS_V1"),
];

fn number(value: &Value) -> Result<u32, String> {
    value
        .as_u64()
        .and_then(|id| u32::try_from(id).ok())
        .filter(|id| *id != 0)
        .ok_or_else(|| format!("expected nonzero u32, got {value}"))
}

fn ids(value: &Value) -> Result<Vec<u32>, String> {
    let values = value.as_array().ok_or("expected id array")?;
    let ids: Vec<_> = values.iter().map(number).collect::<Result<_, _>>()?;
    if !ids.windows(2).all(|pair| pair[0] < pair[1]) {
        return Err(format!("ids must be unique and ascending: {value}"));
    }
    Ok(ids)
}

fn flag(value: &Value) -> Result<bool, String> {
    value
        .as_bool()
        .ok_or_else(|| format!("expected boolean, got {value}"))
}

fn const_path(value: &Value) -> Result<&str, String> {
    let path = value
        .as_str()
        .ok_or("constant must be a module const path")?;
    if path.split("::").count() < 2
        || !path.split("::").all(|part| {
            !part.is_empty()
                && part.bytes().enumerate().all(|(index, byte)| {
                    byte == b'_'
                        || byte.is_ascii_alphabetic()
                        || (index > 0 && byte.is_ascii_digit())
                })
        })
    {
        return Err(format!("invalid module const path: {path}"));
    }
    Ok(path)
}

fn contract_ids(entry: &Value, field: &str) -> Result<Vec<u32>, String> {
    if entry[field].is_null() {
        Ok(Vec::new())
    } else {
        ids(&entry[field])
    }
}

/// Validate the complete set before emitting any Rust. Gaps are permitted: unallocated and
/// reserved ids remain unknown. Neither scanning nor sorting may fill them in.
fn validate(entries: &[Value], registry: &Value) -> Result<(), String> {
    let contracts = registry["capabilities"]
        .as_array()
        .ok_or("contract capabilities")?;
    let registered: BTreeSet<_> = contracts
        .iter()
        .map(|c| number(&c["id"]))
        .collect::<Result<_, _>>()?;
    let mut seen = BTreeSet::new();
    let mut names = BTreeSet::new();
    let mut commands = BTreeSet::new();
    let mut domains = BTreeSet::new();
    for entry in entries {
        let fields = [
            "id",
            "name",
            "constant",
            "requires",
            "acceptance_requires",
            "commands",
            "domains",
            "supersedes",
            "offers",
        ];
        let object = entry
            .as_object()
            .ok_or("runtime fragment must be an object")?;
        if object.len() != fields.len() || fields.iter().any(|field| !object.contains_key(*field)) {
            return Err("unknown or missing runtime fragment field".into());
        }
        let id = number(&entry["id"])?;
        if !seen.insert(id) {
            return Err(format!("duplicate capability {id}"));
        }
        let contract = contracts
            .iter()
            .find(|c| c["id"] == id)
            .ok_or_else(|| format!("unregistered capability {id}"))?;
        let name = entry["name"].as_str().ok_or("capability name")?;
        if name != contract["name"].as_str().ok_or("contract name")?
            || !names.insert(name.to_lowercase())
        {
            return Err(format!("invalid or duplicate capability name {name}"));
        }
        const_path(&entry["constant"])?;
        let requires = ids(&entry["requires"])?;
        if requires != contract_ids(contract, "requires")?
            || requires
                .iter()
                .any(|required| !registered.contains(required) || *required == id)
        {
            return Err(format!("invalid requirements for {id}"));
        }
        flag(&entry["acceptance_requires"])?;
        let owned = ids(&entry["commands"])?;
        if owned != contract_ids(contract, "command_types")? {
            return Err(format!("command ownership differs for {id}"));
        }
        for command in owned {
            if !commands.insert(command) {
                return Err(format!("command {command} is owned twice"));
            }
            let command_entry = registry["command_types"]
                .as_array()
                .ok_or("command types")?
                .iter()
                .find(|c| c["id"] == command)
                .ok_or_else(|| format!("unregistered command {command}"))?;
            if number(&command_entry["capability"])? != id {
                return Err(format!(
                    "command {command} has a different capability owner"
                ));
            }
        }
        // VIS-2 extends a core payload. Only its codec gates it; gating all of domain 1 would
        // prevent legacy clients from seeing the core spatial snapshot.
        if id == 6 {
            if !entry["domains"].is_null() || contract_ids(contract, "state_domains")? != [1] {
                return Err("capability 6 extends core domain 1".into());
            }
        } else {
            let owned = ids(&entry["domains"])?;
            if owned != contract_ids(contract, "state_domains")? {
                return Err(format!("domain ownership differs for {id}"));
            }
            for domain in owned {
                if !domains.insert(domain) {
                    return Err(format!("domain {domain} is owned twice"));
                }
                let domain_entry = registry["state_domains"]
                    .as_array()
                    .ok_or("state domains")?
                    .iter()
                    .find(|entry| entry["id"] == domain)
                    .ok_or_else(|| format!("unregistered domain {domain}"))?;
                if number(&domain_entry["capability"])? != id {
                    return Err(format!("domain {domain} has a different capability owner"));
                }
            }
        }
        let supersedes = ids(&entry["supersedes"])?;
        // SUPERSEDED remains the accepted MAP-WIRE-1 rule, rather than a new contract surface.
        if supersedes != if id == 18 { vec![2] } else { Vec::new() } {
            return Err(format!("superseded core domains differ for {id}"));
        }
        let offers = entry["offers"].as_object().ok_or("offer flags")?;
        if offers.len() != OFFER_SETS.len() {
            return Err(format!("unknown or missing offer set for {id}"));
        }
        for (set, _) in OFFER_SETS {
            flag(&entry["offers"][set])?;
        }
        if flag(&entry["offers"]["item_view"])? != flag(&contract["offered"])? {
            return Err(format!("Item view offer differs from registry for {id}"));
        }
        if flag(&entry["offers"]["production"])?
            != (flag(&contract["offered"])? && id != 4 && !requires.contains(&4))
        {
            return Err(format!("production fallback offer differs for {id}"));
        }
        if flag(&entry["offers"]["item_view"])? && !flag(&entry["offers"]["bundle_world"])? {
            return Err(format!("bundle offer omits Item view capability {id}"));
        }
    }
    if seen != registered {
        return Err("runtime capability fragments must cover the complete registry".into());
    }
    for command in registry["command_types"]
        .as_array()
        .ok_or("command types")?
    {
        if !command["capability"].is_null() && !commands.contains(&number(&command["id"])?) {
            return Err(format!(
                "gated command {} has no capability owner",
                command["id"]
            ));
        }
    }
    for domain in registry["state_domains"]
        .as_array()
        .ok_or("state domains")?
    {
        if !domain["capability"].is_null() && !domains.contains(&number(&domain["id"])?) {
            return Err(format!(
                "gated domain {} has no capability owner",
                domain["id"]
            ));
        }
    }
    for (set, _) in OFFER_SETS {
        for entry in entries.iter().filter(|entry| entry["offers"][set] == true) {
            for required in ids(&entry["requires"])? {
                if !entries
                    .iter()
                    .any(|other| other["id"] == required && other["offers"][set] == true)
                {
                    return Err(format!(
                        "{set} offer {} lacks requirement {required}",
                        entry["id"]
                    ));
                }
            }
        }
    }
    Ok(())
}

fn slice(value: &Value) -> String {
    format!("&{value}")
}

pub(super) fn render(entries: &[Value], registry: &Value) -> Result<(String, String), String> {
    validate(entries, registry)?;
    let mut entries: Vec<_> = entries.iter().collect();
    entries.sort_by_key(|entry| entry["id"].as_u64());
    let mut registered =
        String::from("pub(super) const REGISTERED_CAPABILITY_IDS_V1: &[u32] = &[\n");
    for entry in &entries {
        registered.push_str(&format!("    {},\n", entry["id"]));
    }
    registered.push_str("];\n");
    registered.push_str("pub(super) const CAPABILITY_REQUIREMENTS_V1: &[(u32, &[u32])] = &[\n");
    for entry in entries
        .iter()
        .filter(|entry| entry["acceptance_requires"] == true)
    {
        registered.push_str(&format!(
            "    ({}, {}),\n",
            entry["id"],
            slice(&entry["requires"])
        ));
    }
    registered.push_str("];\n");
    let mut tables = String::from("pub const GATED: &[(u32, &[u32], &[u32])] = &[\n");
    for entry in entries.iter().filter(|entry| !entry["domains"].is_null()) {
        tables.push_str(&format!(
            "    ({}, {}, {}),\n",
            entry["id"],
            slice(&entry["commands"]),
            slice(&entry["domains"])
        ));
    }
    tables.push_str("];\n");
    for (set, name) in OFFER_SETS {
        tables.push_str(&format!("pub const {name}: &[(u32, &[u32])] = &[\n"));
        for entry in entries.iter().filter(|entry| entry["offers"][set] == true) {
            tables.push_str(&format!(
                "    ({}, {}),\n",
                entry["id"],
                slice(&entry["requires"])
            ));
        }
        tables.push_str("];\n");
    }
    for entry in &entries {
        registered.push_str(&format!(
            "const _: () = assert!(crate::{} == {});\n",
            const_path(&entry["constant"])?,
            entry["id"]
        ));
    }
    Ok((registered, tables))
}

pub(super) fn parse(name: &str, text: &str) -> Result<Value, String> {
    let entry: Value = serde_json::from_str(text).map_err(|error| format!("{name}: {error}"))?;
    let id = number(&entry["id"])?;
    let slug = entry["name"].as_str().ok_or("capability name")?;
    if name != format!("{id:04}-{}.json", slug.to_lowercase().replace('_', "-")) {
        return Err(format!("invalid runtime capability filename {name}"));
    }
    Ok(entry)
}
