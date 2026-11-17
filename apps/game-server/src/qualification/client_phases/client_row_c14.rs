//! Capture the real C14 skill-display reply and spell-catalog snapshot after `full_e2ef18` has
//! driven combat experience, skill progression, a level-up, and automatic spell learning.

use super::*;
use oteryn_protocol_oteryn::{actor_spell_catalog, skill_display};

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::observe(capture(clients)).await {
        Ok(()) => evidence("e2e_all scenario=C14.client status=PASS evidence=skill-and-spell-catalog-snapshots"),
        Err(error) => evidence(&failure_evidence(&error.to_string())),
    }
}

fn failure_evidence(error: &str) -> String {
    format!("e2e_all scenario=C14.client status=FAIL evidence={}", error.replace(['\n', '\r'], " "))
}

async fn capture(clients: &SeamClients<'_>) -> TestResult {
    client_row_c20b::require_current_run(
        verdict::Harness::global().completed_prerequisite("C14.full"), "C14.full",
    )?;
    let mut client = connect_character(clients, 0xc1, 0).await?;
    let reply = client
        .session_mut()
        .gated_command(
            skill_display::CAPABILITY_SKILL_DISPLAY_V1,
            skill_display::COMMAND_TYPE_SKILL_DISPLAY_QUERY,
            &[],
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("C14 skill-display reply was {reply:?}").into());
    }
    let display = skill_display::SkillDisplay::decode(&reply.payload)
        .ok_or("C14 skill-display payload was malformed")?;
    let catalog = client
        .session_mut()
        .spell_catalog()?
        .ok_or("C14 spell catalog was absent")?;
    let catalog_payload = client
        .session_mut()
        .gated_snapshot(actor_spell_catalog::STATE_DOMAIN_ACTOR_SPELL_CATALOG)
        .ok_or("C14 raw spell-catalog snapshot was absent")?
        .payload
        .clone();

    if let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        write_capture(Path::new(&path), &reply.payload, &catalog_payload, &expected(&display, &catalog.entries))
            .map_err(|error| verdict::context(format!("C14 capture output: {error}"), error))?;
    }
    Ok(())
}

fn write_capture(path: &Path, skill: &[u8], catalog: &[u8], expected: &str) -> TestResult {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize()?;
    let path = path.canonicalize()?;
    if !path.starts_with(root) {
        return Err("E2E_ALL_CAPTURE_DIR must be inside the Game worktree".into());
    }
    std::fs::write(path.join("c14-skill-display.bin"), skill)?;
    std::fs::write(path.join("c14-spell-catalog.bin"), catalog)?;
    std::fs::write(path.join("c14-expected.txt"), expected)?;
    Ok(())
}

fn expected(
    display: &skill_display::SkillDisplay,
    entries: &[actor_spell_catalog::SpellCatalogEntry],
) -> String {
    let skills = display
        .trained
        .skills
        .iter()
        .map(|skill| {
            format!(
                "{}:{}/{}:{}",
                skill.level, skill.progress, skill.required, skill.percent
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let learned = entries
        .iter()
        .filter(|entry| entry.learned)
        .map(|entry| entry.cast_index.to_string())
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "level={} experience={} level_percent={} skills={} learned={}\n",
        display.level, display.experience, display.level_percent, skills, learned
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skill_query_failure_does_not_claim_an_artifact_write() {
        let text = failure_evidence("C14 skill-display reply was rejected");
        assert_eq!(text, "e2e_all scenario=C14.client status=FAIL evidence=C14 skill-display reply was rejected");
    }
}
