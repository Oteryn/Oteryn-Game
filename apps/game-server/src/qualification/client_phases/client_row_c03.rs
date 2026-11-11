//! C03 client replay capture: request the real native Offence and Defence combat-stat pages and
//! preserve the command-result payloads consumed by the Cyclopedia character panel.

use super::*;
use oteryn_dev_client::client_runtime::raw_feature::GatedCommandResult;
use oteryn_dev_client::client_ui::cyclopedia_character::{
    CyclopediaCharacterPanel, Intent, apply_result, panel_text, request,
};
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
) -> TestResult<(Vec<u8>, wire::combat::CombatStats)> {
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
        payload: result.payload.clone(),
    };
    if !apply_result(&mut panel, &reply) {
        return Err(format!("C03 client rejected real {tab:?} combat page").into());
    }
    if !panel_text(&panel).contains("Character revision:") {
        return Err(format!("C03 client panel did not render the {tab:?} page").into());
    }
    Ok((result.payload, stats))
}

/// Server-observed numbers, one `Key=value` line per statistic, for the replay test to render.
fn expected_numbers(stats: &wire::combat::CombatStats) -> String {
    let mut text = format!(
        "tab={:?} observed={} character={} equipment={}\n",
        stats.tab(),
        stats.observed_at_unix_ms,
        stats.character_revision,
        stats.equipment_revision
    );
    for key in wire::combat::CombatStat::for_tab(stats.tab()) {
        match stats.get(*key) {
            Some(value) => text.push_str(&format!("{key:?}={value}\n")),
            None => text.push_str(&format!("{key:?}=unavailable\n")),
        }
    }
    text
}

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    let Some(directory) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    std::fs::write(std::path::Path::new(&directory).join(name), bytes)?;

    Ok(())
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let result: TestResult = async {
        let mut client = connect_world(clients, 0xc3).await?;
        let row_result: TestResult<_> = async {
            let (offence, offence_stats) =
                combat_page(&mut client, wire::CharacterTab::Offence).await?;
            let (defence, defence_stats) =
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
            capture("c03-offence.bin", &offence)?;
            capture("c03-defence.bin", &defence)?;
            capture(
                "c03-expected.txt",
                format!(
                    "{}{}",
                    expected_numbers(&offence_stats),
                    expected_numbers(&defence_stats)
                )
                .as_bytes(),
            )?;
            Ok(())
        }
        .await;
        row_logout(&mut client).await?;
        row_result
    }
    .await;
    if result.is_ok() {
        evidence("e2e_all scenario=C03.client status=PASS payload=cyclopedia-combat-pages");
    }
    if let Err(error) = result {
        evidence(&format!(
            "e2e_all scenario=C03.client status=FAIL evidence={}",
            evidence_token(&error.to_string())
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evidence_tokens_cover_empty_and_whitespace() {
        assert_eq!(evidence_token(""), "");
        assert!(!evidence_token("a b\tc\nd").chars().any(char::is_whitespace));
        assert_eq!(evidence_token("source:path.rs:12"), "source:path.rs:12");
    }
}
