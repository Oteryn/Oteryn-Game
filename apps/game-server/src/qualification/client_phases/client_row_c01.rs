//! C01 client replay capture: real domain-10 combat-state payloads after a fight/chase change.
use super::*;
use oteryn_dev_client::{DevClientSession, SessionEvent};
use oteryn_protocol_oteryn::attack::{
    self, AttackIntentDisposition, ChaseMode, FightMode, FightModes,
};

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    if std::env::var_os("E2E_ALL_CAPTURE_DIR").is_none() {
        return;
    }
    let result = row_complete(capture(clients)).await;
    match result {
        Ok(Ok(())) => evidence(
            "SEAM_EVIDENCE e2e_all scenario=C01.client status=PASS evidence=domain-10-snapshot-and-fight-mode-delta",
        ),
        Ok(Err(error)) => evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario=C01.client status=FAIL evidence={error}"
        )),
        Err(_) => {
            evidence("SEAM_EVIDENCE e2e_all scenario=C01.client status=FAIL evidence=timeout")
        }
    }
}

async fn capture(clients: &SeamClients<'_>) -> TestResult {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(20);
    let mut client = connect_world(clients, 0xc1).await?;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
        let snapshot = attack::encode_actor_combat_state(
            client
                .session_mut()
                .combat_state()
                .ok_or("domain-10 combat state absent")?,
        )
        .map_err(|error| format!("combat snapshot encode: {error:?}"))?;
        client.take_events();
        let modes = FightModes {
            fight_mode: FightMode::Offensive,
            chase: ChaseMode::Chase,
            secure: true,
        };
        let reply = client.session_mut().set_fight_modes(&modes).await?;
        if reply.disposition != AttackIntentDisposition::Ok {
            return Err(format!("fight/chase command refused: {reply:?}").into());
        }
        let state = wait_for_modes(&mut client, modes).await?;
        let delta_payload = attack::encode_actor_combat_state(&state)
            .map_err(|error| format!("combat state encode: {error:?}"))?;
        write_capture("c01-snapshot.bin", &snapshot)?;
        write_capture("c01-modes.bin", &delta_payload)?;
        write_capture(
            "c01-expected.txt",
            format!(
                "{} {} {} {}\n",
                state.modes.fight_mode as u8,
                state.modes.chase as u8,
                u8::from(state.modes.secure),
                u8::from(state.in_fight)
            )
            .as_bytes(),
        )?;
        Ok(())
    })
    .await
    .unwrap_or_else(|error| Err(error.into()));
    let logout_result = row_logout(&mut client).await;
    logout_result?;
    row_result
}

async fn wait_for_modes(
    client: &mut DevClientSession,
    modes: FightModes,
) -> TestResult<attack::ActorCombatState> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        client.service_liveness(Duration::from_millis(20)).await?;
        for event in client.take_events() {
            if let SessionEvent::ActorCombatState(delta) = event
                && delta.value.modes == modes
            {
                return Ok(delta.value);
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("fight/chase domain-10 delta absent".into());
        }
    }
}

fn write_capture(name: &str, bytes: &[u8]) -> TestResult {
    let Some(directory) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    std::fs::write(Path::new(&directory).join(name), bytes)?;

    Ok(())
}
