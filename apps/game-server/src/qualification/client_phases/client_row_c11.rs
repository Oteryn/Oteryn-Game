//! Client replay capture for SCENARIO_MATRIX C11.

use super::*;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    if let Err(error) = capture(clients).await.inspect(|line| evidence(line)) {
        evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario=C11.client status=FAIL evidence={error}"
        ));
    }
}

async fn capture(clients: &SeamClients<'_>) -> TestResult<String> {
    use oteryn_protocol_oteryn::battle_instinct_status as wire;

    let mut client = connect_world(clients, 0xcd).await?;
    let row_result: TestResult<_> = async {
    let reply = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_BATTLE_INSTINCT_STATUS_V1,
            wire::COMMAND_TYPE_BATTLE_INSTINCT_STATUS_QUERY,
            &[],
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("Battle Instinct query returned {:?}", reply.status).into());
    }
    let status = wire::BattleInstinctStatus::decode(&reply.payload)
        .ok_or("Battle Instinct query returned a malformed payload")?;

    if let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let path = Path::new(&path);
        let expected = format!(
            "active={} melee={} shielding={}\n",
            u8::from(status.active),
            status.melee,
            status.shielding
        );
        for (_name, result) in [
            ("c11-battle-instinct.bin", std::fs::write(path.join("c11-battle-instinct.bin"), &reply.payload)),
            ("c11-expected.txt", std::fs::write(path.join("c11-expected.txt"), expected)),
        ] {
            result?;
        }
    }
    let verdict = format!(
        "SEAM_EVIDENCE e2e_all scenario=C11.client status=PASS evidence=active-{}-melee-{}-shielding-{}",
        status.active, status.melee, status.shielding
    );
    Ok(verdict)

    }.await;
    let logout_result = row_logout(&mut client).await;
    logout_result?;
    row_result
}
