//! Registry-driven wiring inventory (MECHWIRE-0): every command type and state domain in the
//! protocol registry must be wired on the server and on the client, or be listed below as a
//! known gap with the evidence for it.
//!
//! The three links, all derived from the registry (nothing here is a hand list of commands):
//!
//! 1. Client request builder. A command owned by capability K is built when the client has a
//!    typed adapter for K: a `features/NNNN-*` directory numbered K in `crates/client-runtime`,
//!    `crates/client-ui` or `apps/client`, the capability constant (the last path segment of the
//!    runtime capability fragment's `constant`), `feature_selected(K)`, or a `K =>` match arm in
//!    the session loop. A core command (no capability) is built when its
//!    `COMMAND_TYPE_<registry name>` constant is used by a client crate. The check is per
//!    capability, not per command: it proves the client has an adapter for the capability, not
//!    that each of its commands is sent.
//! 2. Server dispatch handler. The command id is in the `commands=` header of a
//!    `crates/gameplay/dispatch/mechanics/*.mechanic` file (the generated route table), or in
//!    `ROUTED_OUTSIDE_MECHANIC_FILES`, whose anchor text must still be present in its file.
//! 3. Client reducer. Every registry state domain, the server push message, has a
//!    `STATE_DOMAIN_*`/`DOMAIN_*` constant in `crates/protocol-oteryn` that a client crate uses.
//!    A command result is consumed by the builder's adapter in (1).
//!
//! Client sources are the non-test files of `client-runtime`, `session`, `client-ui`,
//! `client-ui-bevy` and `apps/client`. This is text evidence over real sources: a client that
//! reaches a command only through a glob import or a numeric literal outside the patterns above
//! shows up as a gap, which is why every gap entry carries its own citation.
//!
//! The test fails when a command or domain gains no link (a new gap), and when a listed gap is
//! wired but still listed. The three `*_GAPS` tables are the inventory output.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

type TestResult = Result<(), Box<dyn Error>>;

const COMMAND_REGISTRY: &str = "docs/contracts/protocol-oteryn/v1/registry/command_types";
const DOMAIN_REGISTRY: &str = "docs/contracts/protocol-oteryn/v1/registry/state_domains";
const CAPABILITY_FRAGMENTS: &str = "crates/protocol-oteryn/capabilities";
const MECHANIC_FILES: &str = "crates/gameplay/dispatch/mechanics";
const PROTOCOL_SOURCES: &str = "crates/protocol-oteryn/src";
const CLIENT_SOURCES: &[&str] = &[
    "crates/client-runtime/src",
    "crates/session/src",
    "crates/client-ui/src",
    "crates/client-ui-bevy/src",
    "apps/client/src",
];
const CLIENT_FEATURE_ROOTS: &[&str] = &[
    "crates/client-runtime/src/features",
    "crates/client-ui/src/features",
    "apps/client/src/features",
];
const SESSION_LOOP_DIRECTORY: &str = "apps/client/src/play";

/// Commands the connection routes without a `.mechanic` file: (command id, file, anchor text that
/// must be present in the file). The anchor is the routing comparison itself, so removing the
/// route fails the test.
const ROUTED_OUTSIDE_MECHANIC_FILES: &[(u32, &str, &str)] = &[
    // command.rs:272, the paced step branch of `execute`.
    (
        1,
        "crates/gameplay/dispatch/src/gameplay_transport/connection/serve/command.rs",
        "command.command_type == COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT",
    ),
    // house_transfer.rs:297, the serialised HOUSE_TRANSFER branch.
    (
        102,
        "crates/gameplay/dispatch/src/gameplay_transport/connection/serve/house_transfer.rs",
        "command.command_type == COMMAND_TYPE_HOUSE_TRANSFER",
    ),
    // house_hireling/mod.rs:58, the store command group routed to `routing::store`.
    (
        186,
        "crates/gameplay/dispatch/src/gameplay_transport/connection/serve/house_hireling/mod.rs",
        "store::COMMAND_TYPE_STORE_INBOX_CLAIM",
    ),
    // activity_transfer.rs:153, the activity scope acknowledgement read during a transfer.
    (
        710,
        "crates/gameplay/dispatch/src/gameplay_transport/connection/serve/activity_transfer.rs",
        "command.command_type == COMMAND_TYPE_ACTIVITY_SCOPE_ACK",
    ),
];

/// Commands with no server handler: not in a `.mechanic` file and not routed outside one. The
/// connection answers them `Dispatch::Unregistered` (a rejected result).
const SERVER_GAPS: &[(u32, &str)] = &[
    (
        4,
        "CHARM_UNLOCK_STAGE_INTENT: protocol-oteryn/src/charm.rs:37 defines it and \
         gameplay/dispatch/src/gameplay_transport/charm.rs:149 `dispatch_charm_command` and \
         charm_native.rs:336 `dispatch` decode it, but no mechanics/*.mechanic lists 4 and no \
         connection route calls either function",
    ),
    (
        5,
        "CHARM_ASSIGN_INTENT: protocol-oteryn/src/charm.rs:39; same unrouted dispatch as \
         command 4 (charm.rs:149, charm_native.rs:336)",
    ),
    (
        15,
        "PUSH_INTENT: protocol-oteryn/src/world_interaction.rs:48 defines and encodes it; no \
         mechanics/*.mechanic lists 15 and nothing under gameplay/dispatch/src routes it \
         (CREATURE_PUSH_INTENT 98 is the routed push)",
    ),
    (
        34,
        "OBSERVE_GRANT_SET: protocol-oteryn/src/spectate.rs:8; the observer path in \
         gameplay_transport/connection.rs:760 `refuse_observer_bootstrap` only refuses a \
         bootstrap, and no mechanics/*.mechanic lists 34",
    ),
    (
        35,
        "OBSERVE_GRANT_REVOKE: protocol-oteryn/src/spectate.rs:9; same as command 34 \
         (connection.rs:760)",
    ),
    (
        172,
        "RAID_SUBSCRIBE: protocol-oteryn/src/raid_schedule.rs:9; \
         dispatch/mechanics/0434-raid_schedule.mechanic:2 lists only `commands=171` and \
         gameplay_transport/raid_schedule.rs:12 decodes only the query payload",
    ),
    (
        173,
        "RAID_NOTICE_ACK: protocol-oteryn/src/raid_schedule.rs:10; same as command 172 \
         (0434-raid_schedule.mechanic:2)",
    ),
];

/// Commands whose capability (or, for a core command, constant) has no client adapter. Empty:
/// every registry command has one.
const CLIENT_BUILDER_GAPS: &[(u32, &str)] = &[];

/// State domains with no client reducer: no non-test client source uses a `DOMAIN` constant of
/// that id. The citation is the protocol constant the client does not use.
const CLIENT_REDUCER_GAPS: &[(u32, &str)] = &[
    (
        4,
        "CHARACTER_BESTIARY: protocol-oteryn/src/bestiary.rs:29 STATE_DOMAIN_CHARACTER_BESTIARY \
         is used by no client crate",
    ),
    (
        5,
        "CHARACTER_CHARMS: protocol-oteryn/src/charm.rs:41 STATE_DOMAIN_CHARACTER_CHARMS is \
         used by no client crate",
    ),
    (
        24,
        "DAILY_REWARD: protocol-oteryn/src/daily_reward.rs:11 STATE_DOMAIN_DAILY_REWARD is used \
         by no client crate",
    ),
    (
        25,
        "OBSERVE_TARGET: protocol-oteryn/src/spectate.rs:10 STATE_DOMAIN_OBSERVE_TARGET is used \
         by no client crate",
    ),
    (
        48,
        "CHANNEL_LIST: protocol-oteryn/src/channel_list.rs:6 STATE_DOMAIN_CHANNEL_LIST is used \
         by no client crate",
    ),
    (
        63,
        "QUICK_LOOT: registry state_domains/0063-quick-loot.json (capability 98) has no \
         `DOMAIN` constant in crates/protocol-oteryn/src, so no client crate can name it",
    ),
    (
        97,
        "INSPECTION: protocol-oteryn/src/inspection.rs:17 STATE_DOMAIN_INSPECTION is used by no \
         client crate",
    ),
    (
        102,
        "CAST_VIEW: protocol-oteryn/src/cast.rs:15 STATE_DOMAIN_CAST_VIEW is used by no client \
         crate",
    ),
];

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn sorted_children(directory: &Path) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut children = fs::read_dir(directory)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?;
    children.sort();
    Ok(children)
}

fn rust_files(directory: &Path, files: &mut Vec<PathBuf>) -> TestResult {
    for path in sorted_children(directory)? {
        if path.is_dir() {
            rust_files(&path, files)?;
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
    Ok(())
}

/// Test sources and the capability-support lists do not count as client wiring.
fn is_wiring_source(relative_path: &str) -> bool {
    !(relative_path.contains("_tests")
        || relative_path.contains("/tests")
        || relative_path.contains("test_support")
        || relative_path.ends_with("capability_support.rs"))
}

fn wiring_text(root: &Path, directories: &[&str]) -> Result<String, Box<dyn Error>> {
    let mut files = Vec::new();
    for directory in directories {
        rust_files(&root.join(directory), &mut files)?;
    }
    let mut text = String::new();
    for file in files {
        if is_wiring_source(&relative(root, &file)) {
            text.push_str(&fs::read_to_string(&file)?);
            text.push('\n');
        }
    }
    Ok(text)
}

fn json_files(directory: &Path) -> Result<Vec<serde_json::Value>, Box<dyn Error>> {
    let mut values = Vec::new();
    for path in sorted_children(directory)? {
        if path
            .extension()
            .is_some_and(|extension| extension == "json")
        {
            values.push(serde_json::from_str(&fs::read_to_string(&path)?)?);
        }
    }
    Ok(values)
}

fn id_of(value: &serde_json::Value) -> Result<u32, Box<dyn Error>> {
    let id = value
        .get("id")
        .and_then(serde_json::Value::as_u64)
        .ok_or("registry entry without a numeric id")?;
    Ok(u32::try_from(id)?)
}

fn id_list(value: &serde_json::Value, field: &str) -> Result<Vec<u32>, Box<dyn Error>> {
    match value.get(field).and_then(serde_json::Value::as_array) {
        None => Ok(Vec::new()),
        Some(items) => items
            .iter()
            .map(|item| -> Result<u32, Box<dyn Error>> {
                let id = item.as_u64().ok_or("non-numeric registry id")?;
                Ok(u32::try_from(id)?)
            })
            .collect(),
    }
}

fn is_identifier_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '_'
}

fn contains_word(text: &str, word: &str) -> bool {
    text.match_indices(word).any(|(start, _)| {
        let before = text[..start].chars().next_back();
        let after = text[start + word.len()..].chars().next();
        !before.is_some_and(is_identifier_character) && !after.is_some_and(is_identifier_character)
    })
}

/// `number =>` or `number |` as a match-arm pattern, not part of a longer number or path.
fn contains_number_arm(text: &str, number: u32) -> bool {
    let digits = number.to_string();
    text.match_indices(digits.as_str()).any(|(start, _)| {
        let before = text[..start].chars().next_back();
        let rest = &text[start + digits.len()..];
        let ends_number = !rest.chars().next().is_some_and(is_identifier_character);
        let rest = rest.trim_start();
        let arm = rest.starts_with("=>") || (rest.starts_with('|') && !rest.starts_with("||"));
        !before.is_some_and(|character| is_identifier_character(character) || character == '.')
            && ends_number
            && arm
    })
}

/// `pub const NAME: u32 = 123;` on one line, as in every protocol constant.
fn parse_constant(line: &str) -> Option<(String, u32)> {
    let line = line.trim_start();
    let line = line
        .strip_prefix("pub const ")
        .or_else(|| line.strip_prefix("pub(crate) const "))?;
    let (name, rest) = line.split_once(':')?;
    if name.is_empty() || !name.chars().all(is_identifier_character) {
        return None;
    }
    let rest = rest.trim_start().strip_prefix("u32")?;
    let rest = rest.trim_start().strip_prefix('=')?.trim_start();
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() || !rest[digits.len()..].trim_start().starts_with(';') {
        return None;
    }
    Some((name.to_owned(), digits.parse().ok()?))
}

fn protocol_constants(root: &Path) -> Result<BTreeMap<u32, BTreeSet<String>>, Box<dyn Error>> {
    let mut files = Vec::new();
    rust_files(&root.join(PROTOCOL_SOURCES), &mut files)?;
    let mut constants: BTreeMap<u32, BTreeSet<String>> = BTreeMap::new();
    for file in files {
        if !is_wiring_source(&relative(root, &file)) {
            continue;
        }
        for line in fs::read_to_string(&file)?.lines() {
            if let Some((name, value)) = parse_constant(line) {
                constants.entry(value).or_default().insert(name);
            }
        }
    }
    Ok(constants)
}

fn feature_directory_capabilities(root: &Path) -> Result<BTreeSet<u32>, Box<dyn Error>> {
    let mut capabilities = BTreeSet::new();
    for feature_root in CLIENT_FEATURE_ROOTS {
        let directory = root.join(feature_root);
        if !directory.is_dir() {
            continue;
        }
        for path in sorted_children(&directory)? {
            let name = path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned());
            if let Some((prefix, _)) = name.as_deref().and_then(|name| name.split_once('-'))
                && prefix.len() == 4
                && prefix.chars().all(|character| character.is_ascii_digit())
            {
                capabilities.insert(prefix.parse()?);
            }
        }
    }
    Ok(capabilities)
}

fn session_loop_text(root: &Path) -> Result<String, Box<dyn Error>> {
    let mut text = String::new();
    for path in sorted_children(&root.join(SESSION_LOOP_DIRECTORY))? {
        let is_loop = path
            .file_name()
            .is_some_and(|name| name.to_string_lossy().starts_with("session_loop"));
        if is_loop && path.extension().is_some_and(|extension| extension == "rs") {
            text.push_str(&fs::read_to_string(&path)?);
            text.push('\n');
        }
    }
    Ok(text)
}

fn mechanic_command_ids(root: &Path) -> Result<BTreeSet<u32>, Box<dyn Error>> {
    let mut ids = BTreeSet::new();
    for path in sorted_children(&root.join(MECHANIC_FILES))? {
        if path
            .extension()
            .is_none_or(|extension| extension != "mechanic")
        {
            continue;
        }
        let text = fs::read_to_string(&path)?;
        let header = text
            .lines()
            .find_map(|line| line.strip_prefix("commands="))
            .ok_or_else(|| format!("{} has no commands= header", relative(root, &path)))?;
        for id in header.split(',') {
            ids.insert(id.trim().parse()?);
        }
    }
    Ok(ids)
}

fn compare(kind: &str, found: &BTreeSet<u32>, listed: &[(u32, &str)], problems: &mut Vec<String>) {
    let expected: BTreeSet<u32> = listed.iter().map(|(id, _)| *id).collect();
    if expected.len() != listed.len() {
        problems.push(format!("{kind}: a gap is listed twice"));
    }
    for id in found.difference(&expected) {
        problems.push(format!(
            "{kind}: new gap {id}; wire it or list it with its citation"
        ));
    }
    for id in expected.difference(found) {
        problems.push(format!(
            "{kind}: gap {id} is wired now; remove it from the list"
        ));
    }
}

#[test]
fn every_registry_command_and_domain_is_wired_or_a_listed_gap() -> TestResult {
    let root = repository_root();

    let commands: Vec<(u32, String)> = json_files(&root.join(COMMAND_REGISTRY))?
        .iter()
        .map(|entry| -> Result<(u32, String), Box<dyn Error>> {
            let name = entry
                .get("name")
                .and_then(serde_json::Value::as_str)
                .ok_or("command without a name")?;
            Ok((id_of(entry)?, name.to_owned()))
        })
        .collect::<Result<_, _>>()?;
    let domains: Vec<u32> = json_files(&root.join(DOMAIN_REGISTRY))?
        .iter()
        .map(id_of)
        .collect::<Result<_, _>>()?;

    // Capability fragments carry command ownership and the capability constant path.
    let mut owner: BTreeMap<u32, u32> = BTreeMap::new();
    let mut capability_needle: BTreeMap<u32, String> = BTreeMap::new();
    for fragment in json_files(&root.join(CAPABILITY_FRAGMENTS))? {
        let capability = id_of(&fragment)?;
        for command in id_list(&fragment, "commands")? {
            owner.insert(command, capability);
        }
        let constant = fragment
            .get("constant")
            .and_then(serde_json::Value::as_str)
            .ok_or("capability fragment without a constant")?;
        let segments: Vec<&str> = constant.split("::").collect();
        let needle = match segments.as_slice() {
            [.., module, "CAPABILITY"] => format!("{module}::CAPABILITY"),
            [.., last] => (*last).to_owned(),
            [] => return Err("empty capability constant".into()),
        };
        capability_needle.insert(capability, needle);
    }

    let mut problems = Vec::new();

    // The compiled registry (the generated GATED table) and the fragments must agree on ownership.
    for (capability, gated_commands, _) in oteryn_protocol_oteryn::capability_tables::GATED
        .iter()
        .copied()
    {
        for command in gated_commands {
            if owner.get(command) != Some(&capability) {
                problems.push(format!(
                    "command {command} is gated by capability {capability} in capability_tables::GATED but not in its fragment"
                ));
            }
        }
    }
    for (command, _) in &commands {
        if let Some(capability) = owner.get(command)
            && !oteryn_protocol_oteryn::capability_tables::GATED
                .iter()
                .any(|(id, owned, _)| id == capability && owned.contains(command))
        {
            problems.push(format!(
                "command {command} of capability {capability} is missing from capability_tables::GATED"
            ));
        }
    }

    // Guard against a scanner that silently finds nothing.
    let mechanic_ids = mechanic_command_ids(&root)?;
    let client = wiring_text(&root, CLIENT_SOURCES)?;
    let session_loop = session_loop_text(&root)?;
    let feature_capabilities = feature_directory_capabilities(&root)?;
    let constants = protocol_constants(&root)?;
    if commands.len() < 100 || domains.len() < 20 || mechanic_ids.len() < 100 {
        problems.push("registry or mechanic scan found too few entries".to_owned());
    }
    if client.len() < 100_000 || session_loop.is_empty() || feature_capabilities.len() < 20 {
        problems.push("client source scan found too little".to_owned());
    }

    // Link 2: server dispatch handlers.
    let mut routed_outside: BTreeSet<u32> = BTreeSet::new();
    for (command, file, anchor) in ROUTED_OUTSIDE_MECHANIC_FILES {
        routed_outside.insert(*command);
        if mechanic_ids.contains(command) {
            problems.push(format!(
                "command {command} is in a .mechanic file now; remove it from ROUTED_OUTSIDE_MECHANIC_FILES"
            ));
        }
        if !fs::read_to_string(root.join(file))?.contains(*anchor) {
            problems.push(format!(
                "command {command}: routing anchor `{anchor}` is gone from {file}"
            ));
        }
    }
    let mut server_gaps = BTreeSet::new();
    let mut client_builder_gaps = BTreeSet::new();
    for (command, name) in &commands {
        if !mechanic_ids.contains(command) && !routed_outside.contains(command) {
            server_gaps.insert(*command);
        }
        // Link 1: client request builder.
        let built = match owner.get(command) {
            Some(capability) => {
                feature_capabilities.contains(capability)
                    || capability_needle
                        .get(capability)
                        .is_some_and(|needle| contains_word(&client, needle))
                    || client.contains(&format!("feature_selected({capability})"))
                    || contains_number_arm(&session_loop, *capability)
            }
            None => contains_word(&client, &format!("COMMAND_TYPE_{name}")),
        };
        if !built {
            client_builder_gaps.insert(*command);
        }
    }

    // Link 3: client reducers for every server push domain.
    let mut client_reducer_gaps = BTreeSet::new();
    for domain in &domains {
        let reduced = constants.get(domain).is_some_and(|names| {
            names
                .iter()
                .filter(|name| name.contains("DOMAIN"))
                .any(|name| contains_word(&client, name))
        });
        if !reduced {
            client_reducer_gaps.insert(*domain);
        }
    }

    compare("server dispatch", &server_gaps, SERVER_GAPS, &mut problems);
    compare(
        "client builder",
        &client_builder_gaps,
        CLIENT_BUILDER_GAPS,
        &mut problems,
    );
    compare(
        "client reducer",
        &client_reducer_gaps,
        CLIENT_REDUCER_GAPS,
        &mut problems,
    );

    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.join("\n").into())
    }
}
