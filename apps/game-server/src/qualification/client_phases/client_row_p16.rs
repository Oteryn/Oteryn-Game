//! P16 client replay capture: the real Eremo blessing dialogue (greeting, the quote for The
//! Wisdom of Solitude and the answer to the confirmation). The consumer is
//! `DialoguePanel::request`/`apply` in `crates/client-ui/src/features/0003-npc-service`.
use super::client_row_p20::{approach_npc, capture, hex, talk};
use super::*;

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(120);
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut client = connect_character(clients, 0x51, 0).await?;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
        let npc = approach_npc(&mut client, world, &metadata.npcs, "oteryn:npc.eremo").await?;
        let (greeting_bytes, greeting) = talk(&mut client, npc, "hi").await?;
        if greeting.is_empty() {
            return Err("hi produced an empty conversation".into());
        }
        capture("P16", "p16-greeting.bin", &greeting_bytes)?;
        let (quote_bytes, quote) = talk(&mut client, npc, "The Wisdom of Solitude").await?;
        if quote
            != ["Do you want to buy The Wisdom of Solitude at the price for your current level?"]
        {
            return Err(format!("blessing quote observed={quote:?}").into());
        }
        capture("P16", "p16-quote.bin", &quote_bytes)?;
        let (answer_bytes, answer) = talk(&mut client, npc, "no").await?;
        capture("P16", "p16-answer.bin", &answer_bytes)?;
        let expected = format!(
            "npc_identity={}\nnpc_generation={}\ngreeting={}\nquote={}\nanswer={}\n",
            hex(&npc.identity),
            npc.generation,
            greeting.join("|"),
            quote.join("|"),
            answer.join("|")
        );
        capture("P16", "p16-expected.txt", expected.as_bytes())?;

        Ok(expected.replace('\n', ";").replace(' ', "_"))
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
            "e2e_all scenario=P16.client status=CAPTURED evidence={expected}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=P16.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=P16.client status=FAIL evidence=timeout"),
    }
}
