//! Capture the real C14 skill-display reply and spell-catalog snapshot after `full_e2ef18` has
//! driven combat experience, skill progression, a level-up, and automatic spell learning.

use super::*;
use super::{
    row_connect_character as connect_character,
    row_finish as row_logout,
};
use oteryn_protocol_oteryn::{actor_spell_catalog, skill_display};

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = capture(clients, Path::new(&path)).await {
        evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario=C14.client status=FAIL evidence={}",
            error.to_string().replace(['\n', '\r'], " ")
        ));
    }
}

async fn capture(clients: &SeamClients<'_>, path: &Path) -> TestResult {
    row_complete(async {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?;
    let path = path.canonicalize()?;
    if !path.starts_with(root) {
        return Err("E2E_ALL_CAPTURE_DIR must be inside the Game worktree".into());
    }

    let mut client = connect_character(clients, 0xc1, 0).await?;
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(300);
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
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

        std::fs::write(path.join("c14-skill-display.bin"), &reply.payload)?;
        std::fs::write(path.join("c14-spell-catalog.bin"), catalog_payload)?;
        std::fs::write(
            path.join("c14-expected.txt"),
            expected(&display, &catalog.entries),
        )?;
        Ok(())
    })
    .await
    .unwrap_or_else(|error| Err(error.into()));
    tokio::time::timeout(Duration::from_secs(10), row_logout(&mut client)).await??;
    row_result

    }).await?
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
    fn skill_evidence_keeps_full_level_and_experience() {
        let display = skill_display::SkillDisplay {
            level: u32::MAX,
            experience: u64::MAX,
            ..Default::default()
        };
        let text = expected(&display, &[]);
        assert!(text.starts_with("level=4294967295 experience=18446744073709551615 "));
        assert!(text.ends_with("learned=\n"));
    }
}
