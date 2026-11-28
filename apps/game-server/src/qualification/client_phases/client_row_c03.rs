//! C03 client replay capture: request the real native Offence and Defence combat-stat pages and
//! preserve the command-result payloads consumed by the Cyclopedia character panel.

use super::*;
use oteryn_dev_client::client_ui::cyclopedia_character::{
    CyclopediaCharacterPanel, Intent, apply_result, panel_text, request,
};
use oteryn_dev_client::client_runtime::raw_feature::GatedCommandResult;
use oteryn_protocol_oteryn::cyclopedia_character as wire;

fn evidence_token(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || "._:-".contains(character) {
                character
            } else {
                '-'
            }
        })
        .collect()
}

async fn combat_page(
    client: &mut oteryn_dev_client::DevClientSession,
    tab: wire::CharacterTab,
) -> TestResult<(CyclopediaCharacterPanel, GatedCommandResult, wire::combat::CombatStats)> {
    let mut panel = CyclopediaCharacterPanel {
        selected: true,
        tab,
        ..Default::default()
    };
    let command = request(&mut panel, Intent::Tab).ok_or("C03 client request was not built")?;
    let result = client
        .session_mut()
        .gated_command(command.capability, command.kind, &command.payload)
        .await?;
    if result.status != CommandStatus::Accepted {
        return Err(format!("C03 {tab:?} combat page refused: {:?}", result.status).into());
    }
    let stats = wire::combat::CombatStats::decode(&result.payload)
        .map_err(|error| format!("C03 server {tab:?} payload undecodable: {error:?}"))?;
    if stats.tab() != tab {
        return Err(format!("C03 server answered {:?} for {tab:?}", stats.tab()).into());
    }
    let reply = GatedCommandResult {
        command_id: result.command_id,
        status: result.status,
        server_sequence: result.server_sequence,
        payload: result.payload,
    };
    Ok((panel, reply, stats))
}

fn replay(
    directory: &std::path::Path,
    stats: &wire::combat::CombatStats,
    mut panel: CyclopediaCharacterPanel,
    mut reply: GatedCommandResult,
) -> TestResult {
    use std::io::Write as _;
    let tab = stats.tab();
    let path = directory.join(format!("{tab:?}-{}.bin", reply.server_sequence));
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)?
        .write_all(&reply.payload)?;
    let captured = std::fs::read(path)?;
    if captured != reply.payload {
        return Err(format!("C03 {tab:?} capture differs from the live reply").into());
    }
    reply.payload = captured;
    if !apply_result(&mut panel, &reply) {
        return Err(format!("C03 client rejected captured {tab:?} combat page").into());
    }
    let mut expected = format!(
        "Cyclopedia character — {tab:?}, offset 0, limit 20\r\n{tab:?}\nObserved at: {} ms\nCharacter revision: {}\nEquipment revision: {}",
        stats.observed_at_unix_ms, stats.character_revision, stats.equipment_revision
    );
    use wire::combat::{CombatStat, CombatStatUnit};
    for key in CombatStat::for_tab(tab) {
        use std::fmt::Write as _;
        let shown = match stats.get(*key) {
            None => "unavailable".to_owned(),
            Some(value) => match key.unit() {
                CombatStatUnit::PartsPerMillion => {
                    let magnitude = value.unsigned_abs();
                    let sign = if value < 0 { "-" } else { "" };
                    format!("{sign}{}.{:04}%", magnitude / 10_000, magnitude % 10_000)
                }
                CombatStatUnit::Seconds => format!("{value} seconds"),
                _ => value.to_string(),
            },
        };
        write!(expected, "\n{key:?}: {shown}")?;
    }
    if panel_text(&panel) != expected {
        return Err(format!("C03 client panel did not render the {tab:?} page").into());
    }
    Ok(())
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let result: TestResult = verdict::observe(async {
        let mut client = connect_world(clients, 0xc3).await?;
        let (offence_panel, offence, offence_stats) =
            combat_page(&mut client, wire::CharacterTab::Offence).await?;
        let (defence_panel, defence, defence_stats) =
            combat_page(&mut client, wire::CharacterTab::Defence).await?;
        // Server-owned anchors: Onslaught is always reported on Offence and the Defence skill id
        // is the fixed shielding id, so the numbers the client renders are not placeholders.
        if offence_stats
            .get(wire::combat::CombatStat::Onslaught)
            .is_none()
        {
            return Err("C03 server Offence page lacks the Onslaught total".into());
        }
        if defence_stats.get(wire::combat::CombatStat::DefenceSkillId) != Some(6) {
            return Err("C03 server Defence page lacks the shielding skill id".into());
        }
        let session = client.join_snapshot().game_session_id;
        let parent = std::env::var_os("E2E_ALL_CAPTURE_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        let directory = parent.join(format!("c03-{session:?}"));
        std::fs::create_dir(&directory)?;
        if defence.server_sequence <= offence.server_sequence {
            return Err("C03 Defence reply sequence did not follow Offence".into());
        }
        replay(&directory, &offence_stats, offence_panel, offence)?;
        replay(&directory, &defence_stats, defence_panel, defence)?;
        evidence("e2e_all scenario=C03.client status=PASS payload=cyclopedia-combat-pages");
        Ok(())
    })
    .await;
    if let Err(error) = result {
        evidence(&format!(
            "e2e_all scenario=C03.client status=FAIL evidence={}",
            evidence_token(&error.to_string())
        ));
    }
}
