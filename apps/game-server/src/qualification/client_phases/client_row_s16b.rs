//! S16b client replay capture: the changed-vocation statistics a fresh admission re-queries after
//! the server-side `full_row_s16b` conversion (Character zero is a level-8 knight by then). The
//! raw GeneralStats tab and SkillProgress replies are written byte for byte; the observed values
//! are written to `s16b-expected.txt`.
use super::*;
use oteryn_protocol_oteryn::{cyclopedia_character as general, skill_progress as progress};

fn capture(name: &str, bytes: &[u8]) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = std::fs::write(Path::new(&root).join(name), bytes) {
        evidence(&format!("S16B_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"));
    }
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let mut client = connect_character(clients, 0x47, 0).await?;
    let stats = client
        .session_mut()
        .gated_command(
            general::CAPABILITY_CYCLOPEDIA_CHARACTER_V1,
            general::COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB,
            &[1, 0, 0, 1],
        )
        .await?;
    let skills = client
        .session_mut()
        .gated_command(
            progress::CAPABILITY_SKILL_PROGRESS_V1,
            progress::COMMAND_TYPE_SKILL_PROGRESS_QUERY,
            &[],
        )
        .await?;
    let vitals = client.actor_vitals().copied();
    let _ = client.session_mut().logout().await;
    if stats.status != CommandStatus::Accepted || stats.payload.len() != 42 || stats.payload[0] != 1
    {
        return Err(format!("step=general-stats status={:?} len={}", stats.status, stats.payload.len()).into());
    }
    let snapshot = progress::SkillProgressSnapshot::decode(&skills.payload)
        .filter(|_| skills.status == CommandStatus::Accepted)
        .ok_or("step=skill-progress reply missing or malformed")?;
    let vitals = vitals.ok_or("step=vitals missing ActorVitals frame")?;
    let level = u32::from_le_bytes(stats.payload[9..13].try_into()?);
    if level != 8 || vitals.max_health != 185 || vitals.max_mana != 90 {
        return Err(format!("step=changed-stats level={level} vitals={vitals:?}; expected level-8 knight 185/90").into());
    }
    capture("s16b-stats.bin", &stats.payload);
    capture("s16b-skills.bin", &skills.payload);
    let mut expected = format!(
        "xp={}\nlevel={level}\nmagic={}\n",
        u64::from_le_bytes(stats.payload[1..9].try_into()?),
        u16::from_le_bytes(stats.payload[13..15].try_into()?),
    );
    for (name, index) in ["fist", "club", "sword", "axe", "distance", "shielding", "fishing"]
        .iter()
        .zip(0..)
    {
        expected.push_str(&format!(
            "{name}={}\n",
            u16::from_le_bytes([stats.payload[15 + index * 2], stats.payload[16 + index * 2]])
        ));
    }
    expected.push_str(&format!(
        "stamina={}\noffline={}\nknown={}\nhealth={}\nmana={}\n",
        u16::from_le_bytes([stats.payload[29], stats.payload[30]]),
        u16::from_le_bytes([stats.payload[31], stats.payload[32]]),
        stats.payload[33],
        u32::from_le_bytes(stats.payload[34..38].try_into()?),
        u32::from_le_bytes(stats.payload[38..42].try_into()?),
    ));
    for (index, skill) in snapshot.skills.iter().enumerate() {
        expected.push_str(&format!(
            "skill{index}={},{},{},{}\n",
            skill.level, skill.progress, skill.required, skill.percent
        ));
    }
    capture("s16b-expected.txt", expected.as_bytes());
    Ok(format!("level={level} vitals={}/{}", vitals.max_health, vitals.max_mana))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    if std::env::var_os("E2E_ALL_CAPTURE_DIR").is_none() {
        return;
    }
    match verdict::indexed_timeout(Duration::from_secs(90), verdict::observe(journey(clients))).await {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=S16b.client status=CAPTURED evidence={observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=S16b.client status=FAIL evidence={}",
            error.to_string().replace(' ', "_")
        )),
        Err(_) => evidence("e2e_all scenario=S16b.client status=FAIL evidence=timeout"),
    }
}
