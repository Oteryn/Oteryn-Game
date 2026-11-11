//! S04b client replay capture: a real private channel (create, invite, join, one line) and the
//! excluded-poster refusal. The session retains typed chat deltas only, so the captured channel
//! list and line are the server-delivered values encoded with the production chat encoders (the
//! same convention as C09). The consumer is `Session::apply_state_delta` and
//! `Session::private_channel_list` in `crates/session/src/private_channels.rs`.
use super::*;
use oteryn_dev_client::{ChatDisposition, ChatIntent, ChatLine, DevClientSession, SessionEvent};
use oteryn_protocol_oteryn::chat::{self, private_channel as wire};
use wire::{ChatChannelList, PrivateChannelIntent};

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    std::fs::write(Path::new(&root).join(name), bytes)?;

    Ok(())
}

fn own_name(client: &DevClientSession) -> TestResult<String> {
    own_name_from(client.world_entities())
}

fn own_name_from(input: Option<&oteryn_dev_client::WorldEntities>) -> TestResult<String> {
    let entities = input.ok_or("world entities missing")?;
    entities
        .iter()
        .find(|entity| entity.entity.identity == *entities.own_identity())
        .and_then(|entity| entity.display_name.as_ref())
        .ok_or("own display name missing")?
        .as_str()
        .map(str::to_owned)
        .map_err(|e| format!("invalid display name: {e:?}").into())
}

async fn chat(client: &mut DevClientSession, intent: ChatIntent) -> TestResult<ChatDisposition> {
    for attempt in 0..=2 {
        let reply = client.chat(&intent).await?;
        if !matches!(
            reply.disposition,
            ChatDisposition::Muted | ChatDisposition::Exhausted
        ) || attempt == 2
        {
            return Ok(reply.disposition);
        }
        client
            .service_liveness(Duration::from_secs(u64::from(reply.wait_seconds.max(1))))
            .await?;
    }
    Err("chat retry bound exhausted".into())
}

async fn accepted(client: &mut DevClientSession, intent: PrivateChannelIntent) -> TestResult {
    let disposition = chat(client, ChatIntent::PrivateChannel(intent)).await?;
    if disposition != ChatDisposition::Ok {
        return Err(format!("private channel command refused: {disposition:?}").into());
    }
    Ok(())
}

async fn channel_list(client: &mut DevClientSession) -> TestResult<ChatChannelList> {
    accepted(client, PrivateChannelIntent::List).await?;
    client.service_liveness(Duration::from_millis(250)).await?;
    client
        .take_events()
        .into_iter()
        .filter_map(|event| match event {
            SessionEvent::ChatChannelList(delta) => Some(delta.value),
            _ => None,
        })
        .next_back()
        .ok_or_else(|| "channel list delta missing".into())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(120);
    let mut a = connect_character(clients, 0x4d, 0).await?;
    let mut row_needs_logout_0 = true;
    let row_result: TestResult<_> = async {
        let mut b = connect_character(clients, 0x4e, 1).await?;
        let mut row_needs_logout_1 = true;
        let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
    let a_name = own_name(&a)?;
    let b_name = own_name(&b)?;
    if a_name == b_name {
        return Err("both admissions have the same character name".into());
    }
    let selected: Vec<String> = b.selected_capabilities().iter().map(u32::to_string).collect();

    accepted(&mut a, PrivateChannelIntent::Create).await?;
    let channel_id = channel_list(&mut a)
        .await?
        .own_channel
        .ok_or("own channel absent after Create")?;
    accepted(&mut a, PrivateChannelIntent::Invite { name: b_name.clone() }).await?;
    let invited = channel_list(&mut b).await?;
    if !invited
        .invited
        .iter()
        .any(|invite| invite.channel_id == channel_id && invite.owner_name == a_name)
    {
        return Err(format!("peer invitation missing: {invited:?}").into());
    }
    capture(
        "s04b-invited-list.bin",
        &wire::encode_channel_list(&invited).map_err(|e| format!("encode list: {e:?}"))?,
    )?;
    accepted(&mut b, PrivateChannelIntent::Join { channel_id }).await?;

    let text = "s04b-channel-line";
    accepted(
        &mut a,
        PrivateChannelIntent::Say {
            channel_id,
            text: text.into(),
        },
    )
    .await?;
    b.service_liveness(Duration::from_millis(250)).await?;
    let line = b
        .take_events()
        .into_iter()
        .find_map(|event| match event {
            SessionEvent::ChatLine(delta) => match &delta.value {
                ChatLine::PrivateChannel(line)
                    if line.channel_id == channel_id && line.text == text =>
                {
                    Some(delta.value)
                }
                _ => None,
            },
            _ => None,
        })
        .ok_or("channel line not delivered to the peer")?;
    capture(
        "s04b-line.bin",
        &chat::encode_chat_line(&line).map_err(|e| format!("encode line: {e:?}"))?,
    )?;

    accepted(&mut a, PrivateChannelIntent::Exclude { name: b_name.clone() }).await?;
    let refusal = chat(
        &mut b,
        ChatIntent::PrivateChannel(PrivateChannelIntent::Say {
            channel_id,
            text: "must refuse".into(),
        }),
    )
    .await?;
    if refusal != ChatDisposition::NotInvited {
        return Err(format!("excluded post disposition {refusal:?}").into());
    }
    let expected = format!(
        "owner={a_name}\nchannel={channel_id}\ntext={text}\nselected={}\nrefusal=NotInvited\n",
        selected.join(",")
    );
    capture("s04b-expected.txt", expected.as_bytes())?;
    accepted(&mut a, PrivateChannelIntent::Leave { channel_id }).await?;
    row_logout(&mut a).await?; row_needs_logout_0 = false;
    row_logout(&mut b).await?; row_needs_logout_1 = false;
    Ok(expected.replace('\n', ";"))

    }).await.unwrap_or_else(|error| Err(error.into()));
        if row_needs_logout_1 {
            row_logout(&mut b).await?;
        }
        row_result
    }
    .await;
    if row_needs_logout_0 {
        row_logout(&mut a).await?;
    }
    row_result
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match row_complete(journey(clients)).await {
        Ok(Ok(expected)) => evidence(&format!(
            "e2e_all scenario=S04b.client status=CAPTURED evidence={expected}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=S04b.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=S04b.client status=FAIL evidence=timeout"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn absent_projection_cannot_supply_evidence() {
        assert!(own_name_from(None).is_err());
    }
}
