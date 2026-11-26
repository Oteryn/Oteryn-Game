//! W13 client replay capture: the real DAILY_BOOST_QUERY reply (the day's boosted creature, boss
//! and reset epoch) and a malformed query refusal. The consumer is `apply_result` in
//! `crates/client-ui/src/features/0151-daily-boost/protocol.rs`.
use super::*;
use oteryn_dev_client::client_ui::daily_boost::{self, DailyBoostPanel};
use oteryn_protocol_oteryn::daily_boost as wire;

fn capture(root: &Path, name: &str, bytes: &[u8]) -> TestResult {
    use std::io::Write;

    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(root.join(name))?;
    file.write_all(bytes)?;
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let mut client = connect_world(clients, 0xfa).await?;
    if !client
        .selected_capabilities()
        .contains(&wire::CAPABILITY_DAILY_BOOST_V1)
    {
        return Err("admission did not offer daily-boost capability 151".into());
    }
    let reply = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_DAILY_BOOST_V1,
            wire::COMMAND_TYPE_DAILY_BOOST_QUERY,
            &[],
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("DAILY_BOOST_QUERY status={:?}", reply.status).into());
    }
    let mut panel = DailyBoostPanel {
        selected: true,
        ..Default::default()
    };
    daily_boost::apply_result(&mut panel, &reply);
    let view = panel.boost.ok_or("accepted reply did not decode")?;
    if view.creature_key.is_empty() || view.boss_key.is_empty() {
        return Err(format!("boost view has an empty selection: {view:?}").into());
    }
    let refused = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_DAILY_BOOST_V1,
            wire::COMMAND_TYPE_DAILY_BOOST_QUERY,
            &[0],
        )
        .await?;
    if refused.status != CommandStatus::Rejected || !refused.payload.is_empty() {
        return Err(format!("malformed query was not refused: {refused:?}").into());
    }
    let root = std::env::var_os("E2E_ALL_CAPTURE_DIR").ok_or("capture directory missing")?;
    let root = Path::new(&root);
    capture(root, "w13-boost.bin", &reply.payload)?;
    capture(
        root,
        "w13-expected.txt",
        format!(
            "epoch={}\ncreature={}\nboss={}\n",
            view.reset_epoch, view.creature_key, view.boss_key
        )
        .as_bytes(),
    )?;
    Ok(format!(
        "epoch={};creature={};boss={}",
        view.reset_epoch, view.creature_key, view.boss_key
    ))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients)))
        .await
    {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_capture scenario=W13.client status=CAPTURED evidence={observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=W13.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=W13.client status=FAIL evidence=timeout"),
    }
}
