//! MP06: three independently admitted guild participants.
//! Reference: Crystal 201189f871d23597b82caf88fd611454a2ddefb8,
//! data/scripts/talkactions/player/war.lua:54-65 (leader), 127-154 (accept),
//! 178-193 (explicit 1000-frag cap and 24 hours), 3-39 (emblem updates).
//! src/creatures/players/player.cpp:6504-6505 excludes war kills from unjustified
//! kills; :7798-7811 selects MEMBER/OTHER or ALLY/ENEMY/NEUTRAL emblems.
//! data/scripts/creaturescripts/player/death.lua:142-156 records and counts war kills.
use super::*;
use bots::Bot;
use oteryn_protocol_oteryn::{CommandStatus, guild as g, guild_war as w};

// Alphabetic names obey the same native name policy as characters. UUID suffixes
// distinguish founders even when their UUIDv7 timestamp prefixes are identical.
fn guild_name(character: &[u8; 16]) -> String {
    let mut name = String::from("Guild");
    for byte in &character[4..] {
        name.push(char::from(b'a' + (byte >> 4)));
        name.push(char::from(b'a' + (byte & 15)));
    }
    name
}

fn check(case: &str, result: TestResult<String>, failures: &mut Vec<String>) {
    match result {
        Ok(detail) => evidence(&format!("MP_CHECK MP06 {case} PASS {detail}")),
        Err(error) => {
            verdict::record_assertion(case);
            evidence(&format!(
                "MP_CHECK MP06 {case} FAIL {error};source:{}:{}",
                file!(),
                line!()
            ));
            failures.push(format!("{case}: {error}"));
        }
    }
}

async fn intent(bot: &mut Bot<'_, '_>, value: g::GuildIntent) -> TestResult<g::GuildIntentResult> {
    let payload = g::encode_guild_intent(&value).map_err(|e| format!("{e:?}"))?;
    let reply = bot
        .client()?
        .session_mut()
        .gated_command(
            g::CAPABILITY_GUILD_V1,
            g::COMMAND_TYPE_GUILD_INTENT,
            &payload,
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("intent={value:?} reply={reply:?}").into());
    }
    let result = g::decode_guild_intent_result(&reply.payload)
        .map_err(|e| format!("reply={reply:?} decode={e:?}"))?;
    if result.code != 1 {
        return Err(format!("intent={value:?} reply={result:?}").into());
    }
    Ok(result)
}

async fn own(bot: &mut Bot<'_, '_>) -> TestResult<Option<g::GuildSummary>> {
    let payload = g::encode_guild_query(&g::GuildQuery::Own).map_err(|e| format!("{e:?}"))?;
    let reply = bot
        .client()?
        .session_mut()
        .gated_command(
            g::CAPABILITY_GUILD_V1,
            g::COMMAND_TYPE_GUILD_QUERY,
            &payload,
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("Own reply={reply:?}").into());
    }
    match g::decode_guild_query_result(&reply.payload)
        .map_err(|e| format!("reply={reply:?} decode={e:?}"))?
    {
        g::GuildQueryResult::Own(summary) => Ok(summary),
        other => Err(format!("Own reply={other:?}").into()),
    }
}

async fn war(bot: &mut Bot<'_, '_>, value: w::GuildWarIntent) -> TestResult<w::GuildWarResult> {
    let payload = w::encode_guild_war_intent(&value).map_err(|e| format!("{e:?}"))?;
    let reply = bot
        .client()?
        .session_mut()
        .gated_command(
            w::CAPABILITY_GUILD_WAR_V1,
            w::COMMAND_TYPE_GUILD_WAR_INTENT,
            &payload,
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("war={value:?} reply={reply:?};source:crates/gameplay/dispatch/src/gameplay_transport/guild_war.rs:16;terms:crates/gameplay/social-rules/src/guild.rs:7;Crystal:data/scripts/talkactions/player/war.lua:188-190").into());
    }
    let result = w::decode_guild_war_result(&reply.payload)
        .map_err(|e| format!("reply={reply:?} decode={e:?}"))?;
    if result.code != 1 {
        return Err(format!("war={value:?} reply={result:?}").into());
    }
    Ok(result)
}

pub(super) async fn run(
    core: &SeamClients<'_>,
    packets: &SeamClients<'_>,
    _world: &BundleWorld,
    _towns: &oteryn_world_bundle::subarea::Table,
) -> TestResult {
    let mut failures = Vec::new();
    let (mut participants, setup) =
        bots::connect_bots(&[(core, 0, None), (packets, 0, None), (packets, 1, None)]).await;
    let setup_error = setup.as_ref().err().map(ToString::to_string);
    check(
        "admissions",
        setup.map(|()| "three distinct accounts admitted".into()),
        &mut failures,
    );
    let mut guilds = None;
    let result = async {
        if let Some(error) = &setup_error {
            return Err(format!("admission reply={error}").into());
        }
        let a = intent(
            &mut participants[0],
            g::GuildIntent::Found {
                name: guild_name(&core.characters[0]),
            },
        )
        .await?;
        let c = intent(
            &mut participants[2],
            g::GuildIntent::Found {
                name: guild_name(&packets.characters[1]),
            },
        )
        .await?;
        guilds = Some((a.guild, c.guild));
        let name = participants[1].name.clone();
        intent(
            &mut participants[0],
            g::GuildIntent::Invite {
                guild: a.guild,
                name,
            },
        )
        .await?;
        let joined = intent(
            &mut participants[1],
            g::GuildIntent::Join { guild: a.guild },
        )
        .await?;
        for (index, bot) in participants.iter_mut().enumerate() {
            let expected = if index == 2 { c.guild } else { a.guild };
            let reply = own(bot).await?;
            if !reply.as_ref().is_some_and(|s| s.guild == expected) {
                return Err(format!("membership reply={reply:?}").into());
            }
        }
        Ok(format!("found A={a:?} C={c:?} join B={joined:?}"))
    }
    .await;
    let dependency = result.as_ref().err().map(ToString::to_string);
    check("found-invite-join", result, &mut failures);
    let result = async {
        if let Some(error) = &dependency {
            return Err(format!("membership reply={error}").into());
        }
        use oteryn_dev_client::{ChatDisposition, ChatIntent, ChatLine};
        for sender in 0..2 {
            let text = format!("MP06 guild message from {}", participants[sender].name);
            let reply = participants[sender]
                .client()?
                .chat(&ChatIntent::Guild { text: text.clone() })
                .await?;
            if reply.disposition != ChatDisposition::Ok {
                // Members are bound at admission and checked per line (guild_chat/dequeue.rs:65);
                // ChatUnavailable is the unconfigured World relay (guild_chat/sender_gate.rs:74).
                return Err(format!("guild chat reply={reply:?};relay:crates/gameplay/dispatch/src/gameplay_transport/chat_relay.rs:124;depends on mp07 run.sh chat relay fix").into());
            }
            let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
            loop {
                bots::pump(&mut participants).await?;
                if participants[..2].iter_mut().all(|b| {
                    b.client()
                        .ok()
                        .and_then(|c| c.chat_log())
                        .is_some_and(|log| {
                            log.lines()
                                .any(|line| matches!(line, ChatLine::Guild(g) if g.text == text))
                        })
                }) {
                    break;
                }
                if tokio::time::Instant::now() >= deadline {
                    return Err(format!("guild delivery deadline; sender reply={reply:?}").into());
                }
            }
        }
        Ok("A and B received both guild messages".into())
    }
    .await;
    check("guild-channel", result, &mut failures);
    let result = async {
        if let Some(error) = &dependency {
            return Err(format!("membership reply={error}").into());
        }
        let mut before = Vec::new();
        for bot in &mut participants {
            before.push((
                own(bot).await?,
                bot.client()?.join_snapshot().game_session_id,
            ));
        }
        bots::relog_all(&mut participants).await?;
        let mut after = Vec::new();
        for (bot, (_, previous)) in participants.iter_mut().zip(&before) {
            let reply = own(bot).await?;
            let session = bot.client()?.join_snapshot().game_session_id;
            if session == *previous {
                return Err(
                    format!("relog reused session={session:?}; Own reply={reply:?}").into(),
                );
            }
            after.push(reply);
        }
        let before = before
            .into_iter()
            .map(|(summary, _)| summary)
            .collect::<Vec<_>>();
        if before != after {
            return Err(format!("before={before:?} after={after:?}").into());
        }
        Ok(format!("membership before={before:?} after={after:?}"))
    }
    .await;
    check("relog-membership", result, &mut failures);
    let result = async {
        let (a, c) = guilds.ok_or_else(|| format!("guild dependency reply={dependency:?}"))?;
        // Crystal war.lua:188-190 allows 24 hours; the server minimum is seven days
        // (social-rules/src/guild.rs:7, migration 0232 CHECK), so the war uses seven days.
        let declared = war(
            &mut participants[0],
            w::GuildWarIntent::Declare {
                opponent: c,
                frag_limit: 1000,
                duration_days: 7,
            },
        )
        .await?;
        let accepted = war(
            &mut participants[2],
            w::GuildWarIntent::Accept { war: declared.war },
        )
        .await?;
        if accepted.war != declared.war {
            return Err(format!("declared={declared:?} accepted={accepted:?}").into());
        }
        Ok(format!(
            "guild={a:?} declared={declared:?} accepted={accepted:?}; parity gap: 7-day server minimum vs Crystal 24 hours"
        ))
    }
    .await;
    let war_reply = format!("{result:?}");
    check("declare-accept", result, &mut failures);
    for case in ["war-kill-frags-skull", "relog-war-state"] {
        check(case, Err(format!("SERVER_GAP war reply={war_reply}; no player war-state/frag query on the wire:crates/protocol-oteryn/src/guild_war.rs:11-25; frags are durable only:crates/gameplay/durability/src/durability/guild_war.rs:123-127").into()), &mut failures);
    }
    check("visibility-emblems", Err(format!("SERVER_GAP war reply={war_reply}; missing guild/war emblem fields:crates/protocol-oteryn/src/world_spatial_entities.rs:63-68; Crystal war.lua:3-39 requires ally/enemy updates").into()), &mut failures);
    let result = async {
        let (guild, _) = guilds.ok_or_else(|| format!("guild dependency reply={dependency:?}"))?;
        let summary = own(&mut participants[0])
            .await?
            .ok_or("founder Own reply=None")?;
        let character = participants[1].character;
        let ranked = intent(
            &mut participants[0],
            g::GuildIntent::SetRank {
                guild,
                character,
                level: 2,
                expected_revision: summary.revision,
            },
        )
        .await?;
        let view = own(&mut participants[1]).await?;
        if !view.as_ref().is_some_and(|s| s.own_rank == 2) {
            return Err(format!("rank reply={ranked:?} B Own={view:?}").into());
        }
        let kicked = intent(
            &mut participants[0],
            g::GuildIntent::Kick { guild, character },
        )
        .await?;
        let view = own(&mut participants[1]).await?;
        if view.is_some() {
            return Err(format!("kick reply={kicked:?} B Own={view:?}").into());
        }
        Ok(format!("rank={ranked:?} kick={kicked:?} B Own={view:?}"))
    }
    .await;
    check("rank-kick", result, &mut failures);
    let result = async {
        let (guild, _) = guilds.ok_or_else(|| format!("guild dependency reply={dependency:?}"))?;
        let name = participants[1].name.clone();
        let invited = intent(&mut participants[0], g::GuildIntent::Invite { guild, name }).await?;
        let joined = intent(&mut participants[1], g::GuildIntent::Join { guild }).await?;
        let left = intent(&mut participants[1], g::GuildIntent::Leave { guild }).await?;
        let view = own(&mut participants[1]).await?;
        if view.is_some() {
            return Err(format!("Leave={left:?} Own={view:?}").into());
        }
        Ok(format!(
            "Invite={invited:?} Join={joined:?} Leave={left:?} Own={view:?}"
        ))
    }
    .await;
    check("leave", result, &mut failures);
    // Disband commits and pays out inline (guild_bank.rs:125-135); member and leader cleanup run only
    // in the guild World pass, one step per Guild every 60 s (node/guild_scheduler.rs:37,
    // social/src/guild/disband.rs:30-34), and its cursor can defer a Guild by one more pass
    // (social/src/guild.rs:27-43). Both founders disband first so one pass covers both Guilds.
    let mut disband_errors = Vec::new();
    let mut disbanding = Vec::new();
    for index in [0, 2] {
        let result: TestResult<String> = async {
            use oteryn_protocol_oteryn::guild_bank as bank;
            let summary = own(&mut participants[index]).await?.ok_or_else(|| format!("founder {index} Own=None; foundation reply={dependency:?}"))?;
            let payload = bank::encode_guild_bank_intent(&bank::GuildBankIntent {
                guild: summary.guild, action: bank::GuildBankAction::Disband(summary.name),
            }).map_err(|e| format!("{e:?}"))?;
            let reply = participants[index].client()?.session_mut().gated_command(bank::CAPABILITY_GUILD_BANK_V1, bank::COMMAND_TYPE_GUILD_BANK_INTENT, &payload).await?;
            if reply.status != CommandStatus::Accepted { return Err(format!("Disband reply={reply:?};source:crates/gameplay/dispatch/src/gameplay_transport/guild_bank.rs:86").into()); }
            let result = bank::decode_guild_bank_reply(&reply.payload).map_err(|e| format!("reply={reply:?} decode={e:?}"))?;
            if result.code != 1 { return Err(format!("Disband reply={result:?}").into()); }
            Ok(format!("founder {index} Disband={result:?}"))
        }.await;
        match result {
            Ok(reply) => disbanding.push((index, reply)),
            Err(error) => disband_errors.push(error.to_string()),
        }
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3 * 60 + 30);
    verdict::Harness::global().bounded_wait(deadline, "MP06-disband-cleanup");
    let mut disband_replies = Vec::new();
    for (index, disbanded) in disbanding {
        let result: TestResult<String> = async {
            let mut last = String::from("none");
            loop {
                let payload = g::encode_guild_query(&g::GuildQuery::Own).map_err(|e| format!("{e:?}"))?;
                let reply = participants[index].client()?.session_mut().gated_command(g::CAPABILITY_GUILD_V1, g::COMMAND_TYPE_GUILD_QUERY, &payload).await?;
                // An empty Rejected Own is the busy durable holder (guild/query.rs:22,
                // guild_intent.rs:234-238) while the World pass runs; poll again.
                if reply.status == CommandStatus::Accepted {
                    match g::decode_guild_query_result(&reply.payload).map_err(|e| format!("reply={reply:?} decode={e:?}"))? {
                        g::GuildQueryResult::Own(None) => break,
                        other => last = format!("{other:?}"),
                    }
                } else if reply.status == CommandStatus::Rejected && reply.payload.is_empty() {
                    last = format!("{reply:?}");
                } else {
                    return Err(format!("{disbanded}; Own reply={reply:?}").into());
                }
                if tokio::time::Instant::now() >= deadline {
                    return Err(format!("Disband cleanup deadline; {disbanded}; last Own={last};scheduler:apps/game-server/src/node/guild_scheduler.rs:37").into());
                }
                let next = tokio::time::Instant::now() + Duration::from_secs(1);
                while tokio::time::Instant::now() < next {
                    bots::pump(&mut participants).await?;
                }
            }
            Ok(format!("{disbanded}; Own=None"))
        }.await;
        match result {
            Ok(reply) => disband_replies.push(reply),
            Err(error) => disband_errors.push(error.to_string()),
        }
    }
    check(
        "disband",
        if disband_errors.is_empty() {
            Ok(disband_replies.join("; "))
        } else {
            Err(format!(
                "{}; {}",
                disband_replies.join("; "),
                disband_errors.join("; ")
            )
            .into())
        },
        &mut failures,
    );
    check(
        "release",
        bots::release_all(&mut participants, core, packets)
            .await
            .map(|()| "all characters released".into()),
        &mut failures,
    );
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures.join("; ").into())
    }
}
