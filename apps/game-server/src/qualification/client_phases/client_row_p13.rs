//! P13 client replay capture: the real LOYALTY_SKILLS_QUERY snapshot, the empty rejection of a
//! malformed query and the unchanged snapshot after it. The consumer is `LoyaltySkillsPanel::finish`
//! and `panel_text` in `crates/client-ui/src/features/0274-loyalty-skills`.
use super::*;
use oteryn_protocol_oteryn::loyalty_skills::{
    CAPABILITY_LOYALTY_SKILLS_V1, COMMAND_TYPE_LOYALTY_SKILLS_QUERY, LoyaltySkills,
};

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    std::fs::write(Path::new(&root).join(name), bytes)?;

    Ok(())
}

async fn query(
    client: &mut oteryn_dev_client::DevClientSession,
    payload: &[u8],
) -> TestResult<(CommandStatus, Vec<u8>)> {
    let reply = client
        .session_mut()
        .gated_command(
            CAPABILITY_LOYALTY_SKILLS_V1,
            COMMAND_TYPE_LOYALTY_SKILLS_QUERY,
            payload,
        )
        .await?;
    Ok((reply.status, reply.payload))
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(120);
    let mut client = connect_world(clients, 0x32).await?;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
        if !client
            .selected_capabilities()
            .contains(&CAPABILITY_LOYALTY_SKILLS_V1)
        {
            return Err("admission omitted the loyalty-skills capability".into());
        }
        let (status, payload) = query(&mut client, &[]).await?;
        if status != CommandStatus::Accepted {
            return Err(format!("loyalty query status={status:?}").into());
        }
        capture("p13-query.bin", &payload)?;
        let snapshot = LoyaltySkills::decode(&payload).ok_or("undecodable loyalty snapshot")?;
        let (status, refused) = query(&mut client, &[0]).await?;
        if status != CommandStatus::Rejected || !refused.is_empty() {
            return Err(format!(
                "malformed query: status={status:?} payload={}",
                refused.len()
            )
            .into());
        }
        let (status, after) = query(&mut client, &[]).await?;
        if status != CommandStatus::Accepted || after != payload {
            return Err("malformed query changed the loyalty snapshot".into());
        }
        capture("p13-after.bin", &after)?;
        let skills: Vec<String> = snapshot
            .skills
            .iter()
            .map(|skill| format!("{}/{}", skill.base, skill.with_loyalty))
            .collect();
        let expected = format!(
            "points={}\nbonus_percent={}\ntitle={}\nskills={}\nrefused_status=Rejected\n",
            snapshot.points,
            snapshot.bonus_percent,
            snapshot.title.as_deref().unwrap_or(""),
            skills.join(",")
        );
        capture("p13-expected.txt", expected.as_bytes())?;
        Ok(expected.replace('\n', ";"))
    })
    .await
    .unwrap_or_else(|error| Err(error.into()));
    let logout_result = row_logout(&mut client).await;
    logout_result?;
    row_result
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match row_complete(journey(clients)).await {
        Ok(Ok(expected)) => evidence(&format!(
            "e2e_all scenario=P13.client status=CAPTURED evidence={}",
            expected.replace(' ', "_")
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=P13.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=P13.client status=FAIL evidence=timeout"),
    }
}
