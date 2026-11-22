use super::build_mechanics::emit_cargo_directive;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

const SHARD_ENV: &str = "OTERYN_DISPATCH_PG_SHARD";

pub fn configure(root: &Path, out: &Path) -> Result<(), Box<dyn Error>> {
    emit_cargo_directive("rerun-if-env-changed", SHARD_ENV);
    emit_cargo_directive("rustc-check-cfg", "cfg(oteryn_pg_shard_primary)");
    emit_cargo_directive("rustc-check-cfg", "cfg(oteryn_pg_shard_secondary)");

    let (shard, count) = match std::env::var(SHARD_ENV) {
        Err(std::env::VarError::NotPresent) => {
            emit_cargo_directive("rustc-cfg", "oteryn_pg_shard_primary");
            return Ok(());
        }
        value => {
            let spec = value?;
            spec.split_once('/')
                .and_then(|(k, n)| Some((k.parse::<usize>().ok()?, n.parse::<usize>().ok()?)))
                .filter(|&(k, n)| (1..=n).contains(&k))
                .ok_or_else(|| format!("{SHARD_ENV}={spec:?} must be k/n with 1 <= k <= n"))?
        }
    };
    if shard == 1 {
        emit_cargo_directive("rustc-cfg", "oteryn_pg_shard_primary");
    } else {
        emit_cargo_directive("rustc-cfg", "oteryn_pg_shard_secondary");
    }

    let static_owner = root.join("src/gameplay_transport/monk_save_postgres_tests.rs");
    emit_cargo_directive("rerun-if-changed", static_owner.display());
    let mounts = out.join("postgres_tests_mounts.rs");
    let source = fs::read_to_string(&mounts)?;
    let mut groups = Vec::new();
    let mut reward_claim_group = Vec::new();
    let mut reward_claim_bytes = 0;
    let mut lines = source.lines();
    while let Some(path_line) = lines.next() {
        let module = lines.next().ok_or("expected a module after #[path]")?;
        let (path, _) = path_attribute(path_line).ok_or("expected a mount path")?;
        let bytes = source_bytes(Path::new(path))?;
        // The reward USE proof pins qready1_reward_claims.rs. Keep its callers
        // in the same shard so that the proof-bearing source stays intact.
        if matches!(
            module,
            "mod m35_claim_sets;"
                | "mod qa_global_reward_corrections;"
                | "mod qready1_reward_claims;"
        ) {
            reward_claim_group.push((path_line, module));
            reward_claim_bytes += bytes;
        } else {
            groups.push((bytes, vec![(path_line, module)]));
        }
    }
    if !reward_claim_group.is_empty() {
        groups.push((reward_claim_bytes, reward_claim_group));
    }
    groups.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1[0].1.cmp(b.1[0].1)));

    // Reserve room for the explicit test mounts in the primary shard. Common support is
    // the same size in every shard and does not affect assignment.
    let mut sizes = vec![0; count];
    sizes[0] = source_bytes(&static_owner)?;
    let mut kept = Vec::new();
    for (bytes, group) in groups {
        let lightest = (0..count)
            .min_by_key(|&i| (sizes[i], i))
            .ok_or("expected at least one shard")?;
        sizes[lightest] += bytes;
        if lightest + 1 == shard {
            kept.extend(group);
        }
    }
    kept.sort_unstable();
    let mounts_text: String = kept
        .into_iter()
        .map(|(path, module)| format!("{path}\n{module}\n"))
        .collect();
    fs::write(mounts, mounts_text)?;
    Ok(())
}

fn path_attribute(line: &str) -> Option<(&str, &str)> {
    let quoted = line
        .strip_prefix("#[path")?
        .trim_start()
        .strip_prefix('=')?
        .trim_start()
        .strip_prefix('"')?;
    let (path, rest) = quoted.split_once('"')?;
    Some((path, rest.trim_start().strip_prefix(']')?.trim_start()))
}

/// Count an entry and its out-of-line Rust modules, including named `#[path]` mounts
/// outside the module's own directory. Inline modules are already in the file size.
fn source_bytes(path: &Path) -> Result<u64, Box<dyn Error>> {
    let source = fs::read_to_string(path)?;
    let dir = path.parent().ok_or("test source must have a parent")?;
    let mut size = source.len() as u64;
    let mut override_path = None;
    for line in source.lines().map(str::trim) {
        let line = if let Some((child, rest)) = path_attribute(line) {
            override_path = Some(child);
            rest
        } else {
            line
        };
        if let Some(child) = line
            .strip_prefix("include!(\"")
            .and_then(|line| line.strip_suffix("\");"))
        {
            size += source_bytes(&dir.join(child))?;
        } else if let Some(name) = line
            .rsplit_once("mod ")
            .and_then(|(_, end)| end.strip_suffix(';'))
            .filter(|name| {
                !name.is_empty() && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
            })
        {
            let child = if let Some(child) = override_path.take() {
                Some(dir.join(child))
            } else {
                default_module_path(path, dir, name)
            };
            if let Some(child) = child {
                size += source_bytes(&child)?;
            }
        } else if !line.starts_with("#[") && !line.starts_with("//") && !line.is_empty() {
            override_path = None;
        }
    }
    Ok(size)
}

fn default_module_path(path: &Path, dir: &Path, name: &str) -> Option<PathBuf> {
    let stem = path.file_stem()?;
    [dir.join(stem), dir.to_owned()]
        .into_iter()
        .flat_map(|parent| {
            [
                parent.join(format!("{name}.rs")),
                parent.join(name).join("mod.rs"),
            ]
        })
        .find(|candidate| candidate.is_file())
}
