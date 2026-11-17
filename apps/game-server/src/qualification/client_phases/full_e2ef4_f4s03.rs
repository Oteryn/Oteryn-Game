//! S03 uses player commands and server observations, including a second live member.
use super::*;
use oteryn_dev_client::{
    ChatDisposition, ChatIntent, ChatLine, client_runtime::guild as runtime, client_ui::guild as ui,
};
use oteryn_protocol_oteryn::{guild as wire, guild_bank as bank, guild_war as war};

/// Poll the operation to completion without cancelling a partially read frame. An idle peer
/// must answer heartbeats during walking, reconnection and logout, just as in mp/bots.rs.
async fn with_liveness<T>(
    idle: &mut DevClientSession,
    operation: impl std::future::Future<Output = TestResult<T>>,
) -> TestResult<T> {
    let mut operation = std::pin::pin!(operation);
    let mut tick = tokio::time::interval(Duration::from_millis(250));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        tokio::select! {
            biased;
            result = &mut operation => return result,
            _ = tick.tick() => {
                progress();
                idle.service_liveness(Duration::from_millis(20)).await?;
                idle.take_events();
            }
        }
    }
}

/// Also feed progress during a single-session admission or a long hunt.
async fn progressing<T>(
    operation: impl std::future::Future<Output = TestResult<T>>,
) -> TestResult<T> {
    let mut operation = std::pin::pin!(operation);
    let mut tick = tokio::time::interval(Duration::from_secs(10));
    loop {
        tokio::select! {
            biased;
            result = &mut operation => return result,
            _ = tick.tick() => progress(),
        }
    }
}

async fn request(
    client: &mut DevClientSession,
    idle: &mut DevClientSession,
    panel: &mut ui::GuildPanel,
    intent: ui::Intent,
) -> TestResult {
    let command = ui::request(panel, intent).ok_or("S03 guild UI request unavailable")?;
    let reply = with_liveness(idle, async {
        Ok(runtime::send(client.session_mut(), &command).await?)
    })
    .await?;
    ui::apply_result(panel, &reply);
    let accepted = match &reply {
        runtime::GuildReply::Intent { status, result } => {
            *status == CommandStatus::Accepted && result.as_ref().is_some_and(|r| r.code == 1)
        }
        runtime::GuildReply::Query { status, result } => {
            *status == CommandStatus::Accepted && result.is_some()
        }
    };
    if !accepted || panel.pending {
        return Err(format!("S03 guild request refused: {reply:?}").into());
    }
    progress();
    Ok(())
}

fn summary(panel: &ui::GuildPanel) -> TestResult<&wire::GuildSummary> {
    panel
        .own
        .as_ref()
        .ok_or_else(|| "S03 own guild absent".into())
}

async fn bank_request(
    client: &mut DevClientSession,
    idle: &mut DevClientSession,
    guild: [u8; 16],
    action: Option<bank::GuildBankAction>,
) -> TestResult<bank::GuildBankReply> {
    let (kind, payload) = match action {
        Some(action) => (
            bank::COMMAND_TYPE_GUILD_BANK_INTENT,
            bank::encode_guild_bank_intent(&bank::GuildBankIntent { guild, action }),
        ),
        None => (
            bank::COMMAND_TYPE_GUILD_BANK_QUERY,
            bank::encode_guild_bank_query(&bank::GuildBankQuery {
                guild: Some(guild),
                before: None,
                after_claim: None,
            }),
        ),
    };
    let payload = payload.map_err(|e| format!("S03 guild bank input: {e:?}"))?;
    let reply = with_liveness(idle, async {
        Ok(client
            .session_mut()
            .gated_command(bank::CAPABILITY_GUILD_BANK_V1, kind, &payload)
            .await?)
    })
    .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("S03 guild bank refused: {reply:?}").into());
    }
    let result = bank::decode_guild_bank_reply(&reply.payload)
        .map_err(|e| format!("S03 guild bank reply: {e:?}"))?;
    if result.code != 1 {
        return Err(format!("S03 guild bank refused: {result:?}").into());
    }
    progress();
    Ok(result)
}

async fn war_request(
    client: &mut DevClientSession,
    idle: &mut DevClientSession,
    intent: war::GuildWarIntent,
) -> TestResult<war::GuildWarResult> {
    let payload =
        war::encode_guild_war_intent(&intent).map_err(|e| format!("S03 guild war input: {e:?}"))?;
    let reply = with_liveness(idle, async {
        Ok(client
            .session_mut()
            .gated_command(
                war::CAPABILITY_GUILD_WAR_V1,
                war::COMMAND_TYPE_GUILD_WAR_INTENT,
                &payload,
            )
            .await?)
    })
    .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("S03 guild war refused: {reply:?}").into());
    }
    let result = war::decode_guild_war_result(&reply.payload)
        .map_err(|e| format!("S03 guild war reply: {e:?}"))?;
    if result.code != 1 {
        return Err(format!("S03 guild war refused: {result:?}").into());
    }
    progress();
    Ok(result)
}

fn guild_name(client: &DevClientSession, prefix: &str) -> String {
    // Session entropy encoded as letters satisfies the native 2..=29 ASCII name grammar.
    let suffix: String = client.join_snapshot().game_session_id.as_bytes()[8..]
        .iter()
        .flat_map(|byte| {
            [
                char::from(b'a' + (byte >> 4)),
                char::from(b'a' + (byte & 15)),
            ]
        })
        .collect();
    format!("{prefix} {suffix}")
}

async fn logout_peer(peer: &mut DevClientSession) -> TestResult {
    let (result, detail, attempts, busy_timed_out) =
        super::super::logout_retrying_busy(peer).await?;
    if result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!(
            "S03 peer logout refused: {detail};attempts={attempts};busy_timed_out={busy_timed_out}"
        )
        .into());
    }
    Ok(())
}

pub(super) async fn run(
    row: &mut Row,
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
) -> TestResult {
    evidence(
        "E2EF4_MISSING S03 motd source:crates/protocol-oteryn/src/guild.rs:17 native-GuildIntent-has-no-MOTD-write",
    );
    progressing(async {
        // Always earn new coins, even if earlier rows left money in the backpack or bank.
        let before = full_gold::coin_value(client)?;
        let target = before.checked_add(4).ok_or("S03 gold target overflow")?;
        let bytes = std::fs::read(std::env::var("NODE_BOOT_WORLD_BUNDLE")?)?;
        let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
        full_gold::earn(client, world, &metadata.spawns, "S03", target).await?;
        let (banker, actor, balance) = find_banker(row, client, world, npcs).await?;
        let funds = full_gold::coin_value(client)?;
        let inventory = inventory_units(client)?;
        let refs = coin_refs()?;
        let noncoins = non_coin(&inventory, &refs);
        bank_exchange(
            client,
            actor,
            "deposit all",
            &format!("Do you want to deposit {funds} gold?"),
            &format!("You have deposited {funds} gold."),
        )
        .await?;
        settle(client, "S03-earned-deposit", |now| {
            wealth(now, &refs) == 0 && non_coin(now, &refs) == noncoins
        })
        .await?;
        let deposited = balance
            .checked_add(funds)
            .ok_or("S03 bank balance overflow")?;
        if bank_balance(client, actor).await? != deposited {
            return Err("S03 banker did not credit earned coins exactly".into());
        }
        // Both characters stay in the unchanged harness world; no island or Oracle flow.
        let mut peer = with_liveness(client, connect_character(clients, 0x89, 0)).await?;
        let journey = play(row, client, &mut peer, clients, deposited).await;
        let cleanup = with_liveness(client, logout_peer(&mut peer)).await;
        cleanup.map_err(|error| format!("S03 cleanup: {error};journey={journey:?}"))?;
        journey?;
        // Check the player bank through its NPC as well as through the guild bank view.
        let actor = approach_npc(client, world, banker).await?;
        speak(client, actor, "hi").await?;
        if bank_balance(client, actor).await? != deposited - 2 {
            return Err("S03 personal bank differs from guild ledger settlement".into());
        }
        row.missing_server = Some("source:crates/protocol-oteryn/src/guild.rs:17");
        Err("MOTD-write-unavailable;source:crates/protocol-oteryn/src/guild.rs:17".into())
    })
    .await
}

async fn play(
    row: &mut Row,
    client: &mut DevClientSession,
    peer: &mut DevClientSession,
    clients: &SeamClients<'_>,
    personal_balance: u64,
) -> TestResult {
    let mut leader = ui::GuildPanel {
        selected: true,
        ..Default::default()
    };
    let mut member = leader.clone();
    let name = guild_name(client, "Sage");
    request(client, peer, &mut leader, ui::Intent::RefreshOwn).await?;
    request(peer, client, &mut member, ui::Intent::RefreshOwn).await?;
    if leader.own.is_some() || member.own.is_some() {
        return Err("S03 founding requires both real players to have no guild".into());
    }
    request(client, peer, &mut leader, ui::Intent::Found(name.clone())).await?;
    request(client, peer, &mut leader, ui::Intent::RefreshOwn).await?;
    let guild = summary(&leader)?.guild;
    if summary(&leader)?.name != name || summary(&leader)?.own_rank != 1 {
        return Err("S03 founder query has wrong guild name or leader rank".into());
    }
    let leader_name = own_name(client)?;
    let peer_name = own_name(peer)?;
    request(
        client,
        peer,
        &mut leader,
        ui::Intent::Invite(peer_name.clone()),
    )
    .await?;
    request(peer, client, &mut member, ui::Intent::RefreshInvitations).await?;
    let invitation = member
        .invitations
        .iter()
        .find(|i| i.guild == guild && i.name == name)
        .ok_or("S03 invited player cannot see named guild invitation")?
        .guild;
    request(peer, client, &mut member, ui::Intent::Join(invitation)).await?;
    request(peer, client, &mut member, ui::Intent::RefreshOwn).await?;
    request(client, peer, &mut leader, ui::Intent::RefreshRoster).await?;
    let joined = leader
        .roster
        .iter()
        .find(|m| m.name == peer_name)
        .ok_or("S03 joined member missing from founder roster")?;
    let character = joined.character;
    if summary(&member)?.guild != guild
        || summary(&member)?.name != name
        || character != clients.characters[0]
        || joined.level != summary(&member)?.own_rank
        || leader.roster.len() != 2
        || leader.next.is_some()
        || !leader
            .roster
            .iter()
            .any(|m| m.name == leader_name && m.level == 1)
    {
        return Err("S03 both players did not observe exact two-member guild".into());
    }
    row.pass("guild", "founder-roster-and-invited-player-own-query-agree");

    // Promote to vice and demote to member, checking both players' views.
    for level in [2, 3] {
        request(client, peer, &mut leader, ui::Intent::RefreshOwn).await?;
        request(
            client,
            peer,
            &mut leader,
            ui::Intent::SetRank { character, level },
        )
        .await?;
        request(client, peer, &mut leader, ui::Intent::RefreshRoster).await?;
        request(peer, client, &mut member, ui::Intent::RefreshOwn).await?;
        if summary(&member)?.own_rank != level
            || !leader
                .roster
                .iter()
                .any(|m| m.character == character && m.level == level)
        {
            return Err(format!("S03 rank {level} missing from member or roster view").into());
        }
    }
    row.pass("ranks", "two-rank-changes-observed-by-both-members");

    let text = format!("S03 greeting from {name}");
    let sent = with_liveness(peer, async {
        Ok(client
            .chat(&ChatIntent::Guild { text: text.clone() })
            .await?)
    })
    .await?;
    if sent.status != CommandStatus::Accepted || sent.disposition != ChatDisposition::Ok {
        return Err(format!("S03 guild chat refused: {sent:?}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    loop {
        if peer.chat_log().is_some_and(|log| {
            log.lines().any(|line| {
                matches!(line, ChatLine::Guild(line) if line.guild_id == guild
                && line.speaker_name == leader_name && line.rank == 1 && line.text == text)
            })
        }) {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("S03 other member did not receive guild chat".into());
        }
        progress();
        client.service_liveness(Duration::from_millis(20)).await?;
        peer.service_liveness(Duration::from_millis(20)).await?;
        client.take_events();
        peer.take_events();
    }
    row.pass(
        "chat",
        "other-member-received-exact-guild-speaker-rank-and-text",
    );

    let initial = bank_request(client, peer, guild, None).await?;
    if initial.bank_balance != personal_balance
        || initial.guild_balance != 0
        || !initial.entries.is_empty()
    {
        return Err(format!("S03 new guild bank unexpected: {initial:?}").into());
    }
    let deposit =
        bank_request(client, peer, guild, Some(bank::GuildBankAction::Deposit(4))).await?;
    if deposit.bank_balance != personal_balance - 4 || deposit.guild_balance != 4 {
        return Err(format!("S03 guild deposit balances: {deposit:?}").into());
    }
    let withdraw = bank_request(
        client,
        peer,
        guild,
        Some(bank::GuildBankAction::Withdraw(2)),
    )
    .await?;
    if withdraw.bank_balance != personal_balance - 2 || withdraw.guild_balance != 2 {
        return Err(format!("S03 guild withdrawal balances: {withdraw:?}").into());
    }
    let saved = bank_request(client, peer, guild, None).await?;
    let actor = clients.characters[1];
    if saved.bank_balance != personal_balance - 2
        || saved.guild_balance != 2
        || saved.entries.len() != 2
        || saved.next.is_some()
        || !saved.entries.iter().any(|e| {
            e.kind == 1 && e.amount == 4 && e.before == 0 && e.after == 4 && e.actor == actor
        })
        || !saved.entries.iter().any(|e| {
            e.kind == 2 && e.amount == 2 && e.before == 4 && e.after == 2 && e.actor == actor
        })
    {
        return Err(format!("S03 guild bank ledger not exact: {saved:?}").into());
    }
    let seen = bank_request(peer, client, guild, None).await?;
    if seen.guild_balance != saved.guild_balance || seen.entries != saved.entries {
        return Err("S03 other member guild bank/ledger differs".into());
    }
    row.pass(
        "bank",
        "earned-gold-deposit-withdraw-exact-balances-and-shared-ledger",
    );

    with_liveness(peer, relog(client, clients)).await?;
    let old_peer_session = peer.join_snapshot().game_session_id;
    with_liveness(client, async {
        logout_peer(peer).await?;
        *peer = connect_character(clients, 0x8e, 0).await?;
        Ok(())
    })
    .await?;
    if peer.join_snapshot().game_session_id == old_peer_session {
        return Err("S03 peer relog reused the previous session".into());
    }
    // Refresh brand-new panels after both sessions have been replaced.
    leader = ui::GuildPanel {
        selected: true,
        ..Default::default()
    };
    member = leader.clone();
    request(client, peer, &mut leader, ui::Intent::RefreshOwn).await?;
    request(peer, client, &mut member, ui::Intent::RefreshOwn).await?;
    request(client, peer, &mut leader, ui::Intent::RefreshRoster).await?;
    let remembered = bank_request(client, peer, guild, None).await?;
    let peer_remembered = bank_request(peer, client, guild, None).await?;
    if summary(&leader)?.guild != guild
        || summary(&leader)?.name != name
        || summary(&leader)?.own_rank != 1
        || summary(&member)?.guild != guild
        || summary(&member)?.own_rank != 3
        || leader.roster.len() != 2
        || !leader
            .roster
            .iter()
            .any(|m| m.character == character && m.name == peer_name && m.level == 3)
        || remembered != saved
        || peer_remembered != seen
    {
        return Err("S03 fresh admissions changed guild/ranks/bank/ledger".into());
    }
    row.pass(
        "relog-guild-bank",
        "both-fresh-admissions-retain-membership-ranks-balances-ledger",
    );

    request(peer, client, &mut member, ui::Intent::Leave).await?;
    request(peer, client, &mut member, ui::Intent::RefreshOwn).await?;
    if member.own.is_some() {
        return Err("S03 war opponent retained old membership".into());
    }
    let opponent_name = guild_name(peer, "Rival");
    request(
        peer,
        client,
        &mut member,
        ui::Intent::Found(opponent_name.clone()),
    )
    .await?;
    request(peer, client, &mut member, ui::Intent::RefreshOwn).await?;
    let opponent = summary(&member)?.guild;
    if opponent == guild
        || summary(&member)?.own_rank != 1
        || summary(&member)?.name != opponent_name
    {
        return Err("S03 opponent founding did not produce a distinct leader guild".into());
    }
    let declared = war_request(
        client,
        peer,
        war::GuildWarIntent::Declare {
            opponent,
            frag_limit: 10,
            duration_days: 7,
        },
    )
    .await?;
    let accepted = war_request(
        peer,
        client,
        war::GuildWarIntent::Accept { war: declared.war },
    )
    .await?;
    if accepted.war != declared.war {
        return Err("S03 opponent accepted a different war".into());
    }
    row.pass(
        "wars",
        "leader-declared-valid-terms-opposing-leader-accepted-same-war",
    );
    Ok(())
}
