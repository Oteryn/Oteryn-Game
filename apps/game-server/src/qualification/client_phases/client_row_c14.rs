//! Drive C14 combat progression and capture skill-display and spell-catalog frames
//! from the same admitted client session after the kills.

use super::*;
use oteryn_protocol_oteryn::{actor_spell_catalog, skill_display};
use std::io::Write;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        evidence(&format!(
            "e2e_all scenario=C14.client status=FAIL seconds={:.3} evidence=capture-directory-missing",
            started.elapsed().as_secs_f64()
        ));
        return;
    };
    if let Err(error) = verdict::observe(capture(clients, Path::new(&path))).await {
        evidence(&format!(
            "C14_CLIENT_CAPTURE_ERROR {}",
            error.to_string().replace(['\n', '\r'], " ")
        ));
        evidence(&format!(
            "e2e_all scenario=C14.client status=FAIL seconds={:.3} evidence=capture-error",
            started.elapsed().as_secs_f64()
        ));
    }
}

async fn capture(clients: &SeamClients<'_>, path: &Path) -> TestResult {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?;
    let path = path.canonicalize()?;
    if !path.starts_with(root) {
        return Err("E2E_ALL_CAPTURE_DIR must be inside the Game worktree".into());
    }

    let mut client = connect_character(clients, 0xc1, 0).await?;
    let session = client.join_snapshot().game_session_id;
    let (expected_level, expected_xp) = full_e2ef18::train_for_capture(clients, &mut client).await?;
    let before_sequence = client.last_server_sequence();
    let reply = client
        .session_mut()
        .gated_command(
            skill_display::CAPABILITY_SKILL_DISPLAY_V1,
            skill_display::COMMAND_TYPE_SKILL_DISPLAY_QUERY,
            &[],
        )
        .await?;
    if reply.status != CommandStatus::Accepted || reply.server_sequence <= before_sequence {
        return Err(format!("C14 skill-display reply was {reply:?}").into());
    }
    let display = skill_display::SkillDisplay::decode(&reply.payload)
        .ok_or("C14 skill-display payload was malformed")?;
    if (display.level, display.experience) != (expected_level, expected_xp) {
        return Err(format!("C14 skill-display progression mismatch: got level {} XP {}, expected {expected_level}/{expected_xp}", display.level, display.experience).into());
    }
    let catalog = client
        .session_mut()
        .spell_catalog()?
        .ok_or("C14 spell catalog was absent")?;
    let catalog_snapshot = client
        .session_mut()
        .gated_snapshot(actor_spell_catalog::STATE_DOMAIN_ACTOR_SPELL_CATALOG)
        .ok_or("C14 raw spell-catalog snapshot was absent")?;

    let expected_display = expected(&display, &catalog.entries);
    let capture_session = uuid_text(session.as_bytes());
    let catalog_metadata = format!(
        "{capture_session} {} {} {} {}\n",
        reply.server_sequence,
        catalog_snapshot.domain_id,
        catalog_snapshot.snapshot_type,
        catalog_snapshot.revision
    );
    for (file, bytes) in [
        ("c14-skill-display.bin", reply.payload.as_slice()),
        ("c14-spell-catalog.bin", catalog_snapshot.payload.as_slice()),
        ("c14-spell-catalog-meta.txt", catalog_metadata.as_bytes()),
        ("c14-expected.txt", expected_display.as_bytes()),
    ] {
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path.join(file))?
            .write_all(bytes)?;
    }
    evidence(&format!(
        "C14_CLIENT_CAPTURE session={session:?} skill_sequence={} catalog_revision={} level={} experience={}",
        reply.server_sequence, catalog_snapshot.revision, display.level, display.experience,
    ));
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
