//! G09: pocket watch USE shows the world time (server package SRVPKG1, `novelty_item_use.rs`,
//! Crystal `watch.lua`).
//! 1. Admit the dedicated character and assert exactly one carried watch (item 2906).
//! 2. USE it by handle: expect an accepted `Committed` reply and a private "Game" chat line
//!    `The time is H:MM.` published by the server with an in-range Tibian minute.
//! 3. The watch is not consumed: the carried count is unchanged.
//! 4. Log out, freshly admit and use again: the line is published again (not a one-shot).
//!
//! The fixture supplies one authored pocket watch. Both clock reads use live wire traffic.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::{
    chat::ChatLine,
    world_object::{self, UseDisposition, UseIntent, UseTarget},
};
const WATCH: u32 = 2906;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(90), verdict::observe(journey(clients))).await;
    let (status, detail) = match result {
        Ok(Ok(())) => ("PASS", "watch-time-line-and-not-consumed".to_owned()),
        Ok(Err(error)) => ("FAIL", token(&error.to_string())),
        Err(_) => ("FAIL", "step=journey;observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=G09.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn token(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join("-")
}

/// Minute of day from a `The time is H:MM.` line, or `None` for any other text.
fn time_line_minute(text: &str) -> Option<u32> {
    let clock = text.strip_prefix("The time is ")?.strip_suffix('.')?;
    let (hours, minutes) = clock.split_once(':')?;
    let (hours, minutes): (u32, u32) = (hours.parse().ok()?, minutes.parse().ok()?);
    (minutes.to_string().len() <= 2 && hours < 24 && minutes < 60 && clock.len() >= 4)
        .then_some(hours * 60 + minutes)
}

fn game_time_lines(client: &DevClientSession) -> usize {
    client.chat_log().map_or(0, |log| {
        log.lines()
            .filter(|line| {
                matches!(line, ChatLine::Private { speaker_name, text }
                    if speaker_name == "Game" && time_line_minute(text).is_some())
            })
            .count()
    })
}

fn carried(
    client: &DevClientSession,
    definition: std::num::NonZeroU32,
) -> Vec<(oteryn_dev_client::ItemHandle, u32)> {
    client.inventory().map_or_else(Vec::new, |inventory| {
        inventory
            .main_backpack
            .iter()
            .chain(&inventory.entries)
            .filter(|entry| entry.item_definition_ref == definition)
            .map(|entry| (entry.handle, entry.count.get()))
            .collect()
    })
}

async fn use_target(
    client: &mut DevClientSession,
    target: UseTarget,
) -> TestResult<(CommandStatus, UseDisposition)> {
    let payload = world_object::encode_item_use_intent(&UseIntent {
        target,
        use_with: None,
        use_with_map_item: None,
        use_with_position: None,
    })
    .map_err(|error| format!("step=encode observed={error:?}"))?;
    let reply = client.session_mut().world_context_use(&payload).await?;
    let disposition = world_object::decode_use_result(&reply.payload)
        .map_err(|error| format!("step=use observed=reply-decode:{error:?}"))?;
    Ok((reply.status, disposition))
}

async fn read_time(
    client: &mut DevClientSession,
    handle: oteryn_dev_client::ItemHandle,
) -> TestResult {
    let before = game_time_lines(client);
    let (status, disposition) = use_target(client, UseTarget::Item(handle)).await?;
    if status != CommandStatus::Accepted || disposition != UseDisposition::Committed {
        return Err(
            format!("step=use observed=status:{status:?},disposition:{disposition:?}").into(),
        );
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while game_time_lines(client) <= before {
        if tokio::time::Instant::now() >= deadline {
            return Err("step=chat observed=no-server-time-line;needed=The-time-is-H:MM".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    }
    Ok(())
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=logout observed=refused".into());
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    character_prep::declare_seeded("G09", "one pocket watch 2906", "world-clock item use");
    let identity = character_prep::dedicated_character(clients, "NODE_BOOT_G09_ACCOUNT").await?;
    let clients = &identity.clients(clients);
    let definition = provisioning::item_reference(&provisioning::item_index()?, WATCH)?;
    let mut client = connect_character(clients, 0xcd, 0).await?;
    if !carried(&client, definition).is_empty() {
        return Err("step=fixture observed=watch-already-present".into());
    }
    character_prep::grant_item(&client, WATCH, 1).await?;
    logout(&mut client).await?;
    drop(client);

    let mut client = connect_character(clients, 0xce, 0).await?;
    let inventory = carried(&client, definition);
    let [(handle, 1)] = inventory.as_slice() else {
        return Err("step=fixture observed=expected-exactly-one-served-watch".into());
    };
    let handle = *handle;
    read_time(&mut client, handle).await?;
    if carried(&client, definition) != inventory {
        return Err("step=consume observed=watch-consumed;needed=clock-read-keeps-item".into());
    }
    logout(&mut client).await?;
    drop(client);
    let mut fresh = connect_character(clients, 0xcf, 0).await?;
    let durable_inventory = carried(&fresh, definition);
    let [(again, 1)] = durable_inventory.as_slice() else {
        return Err("step=fresh observed=expected-exactly-one-durable-watch".into());
    };
    let result = verdict::observe(read_time(&mut fresh, *again)).await;
    let after = carried(&fresh, definition);
    logout(&mut fresh).await?;
    result?;
    if after != durable_inventory {
        return Err("step=fresh observed=watch-consumed-on-second-read".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_line_is_parsed_only_for_the_server_format() {
        assert_eq!(time_line_minute("The time is 7:05."), Some(425));
        assert_eq!(time_line_minute("The time is 23:59."), Some(1439));
        assert_eq!(time_line_minute("The time is 24:00."), None);
        assert_eq!(time_line_minute("It is noon."), None);
        assert_eq!(token("a  b"), "a-b");
    }
}
