//! Crystal 201189f871d23597b82caf88fd611454a2ddefb8:
//! src/game/game.cpp:6821 whisper (adjacent text, viewport pspsps),
//! :6841 yell (level >1, 30000ms exhaustion), :6861 offline PM,
//! :6957 say viewport and doubled yell viewport.
//! src/map/map_const.hpp:20-21: viewport 8x6, hence yell 18x14.
//! src/creatures/interactions/chat.cpp:50 invite, :430 private channel creation.
//! src/creatures/players/player.cpp:3483-3502: message buffer mute 5*n*n seconds;
//! :4529 offline VIP notification, :4535 online status.
//! config.lua.dist:548: maxMessageBuffer=4 (first mute 5 seconds).
use super::bots::{self, Bot};
use super::*;
use oteryn_protocol_oteryn::chat::private_channel::PrivateChannelIntent as Channel;
use oteryn_protocol_oteryn::chat::{
    ChatDisposition, ChatIntent, ChatLine, ChatRoom, ChatSpeechMode,
};

fn row(case: &str, result: TestResult) -> bool {
    match result {
        Ok(()) => {
            evidence(&format!("MP_CHECK MP07 {case} PASS observed_on_real_wire"));
            true
        }
        Err(error) => {
            evidence(&format!("MP_CHECK MP07 {case} FAIL {error}"));
            false
        }
    }
}
async fn send(bot: &mut Bot<'_, '_>, intent: ChatIntent, expected: ChatDisposition) -> TestResult {
    let reply = bot.client()?.chat(&intent).await?;
    if reply.disposition != expected
        || (expected == ChatDisposition::Ok && reply.status != CommandStatus::Accepted)
    {
        return Err(format!("intent={intent:?} reply={reply:?};source:crates/gameplay/dispatch/src/gameplay_transport/chat_intent.rs:716").into());
    }
    Ok(())
}
fn contains(bot: &Bot<'_, '_>, text: &str) -> bool {
    bot.session
        .as_ref()
        .and_then(|s| s.chat_log())
        .is_some_and(|log| {
            log.lines().any(|line| match line {
                ChatLine::Local { text: t, .. }
                | ChatLine::Private { text: t, .. }
                | ChatLine::Room { text: t, .. } => t == text,
                ChatLine::PrivateChannel(line) => line.text == text,
                _ => false,
            })
        })
}
async fn delivered(bots: &mut [Bot<'_, '_>], text: &str, recipients: &[usize]) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    bots::wait_observation(bots, deadline, text, |bots, _| {
        Ok(recipients.iter().all(|&i| contains(&bots[i], text)))
    })
    .await?;
    // Observe a bounded quiet window as well, servicing every real socket.
    let end = tokio::time::Instant::now() + Duration::from_millis(300);
    while tokio::time::Instant::now() < end {
        bots::pump(bots).await?;
    }
    for (i, bot) in bots.iter().enumerate() {
        if contains(bot, text) != recipients.contains(&i) {
            return Err(format!(
                "recipient={i} text={text:?} reply_chat_log={:?}",
                bot.session.as_ref().and_then(|s| s.chat_log())
            )
            .into());
        }
    }
    Ok(())
}

pub(super) async fn run(
    core: &SeamClients<'_>,
    packets: &SeamClients<'_>,
    world: &BundleWorld,
    _towns: &oteryn_world_bundle::subarea::Table,
) -> TestResult {
    let (mut bots, setup) =
        bots::connect_bots(&[(core, 0, None), (packets, 0, None), (packets, 1, None)]).await;
    let ready = row("admission", setup);
    let mut passed = ready;
    if ready {
        passed &= row(
            "range_setup",
            bots::approach(&mut bots, 1, 0, world, false).await,
        );
        passed &= row(
            "distant_witness",
            async {
                let origin = tile(bots[0].client()?.world_spatial().actor_position)?;
                let from = tile(bots[2].client()?.world_spatial().actor_position)?;
                let path = route(world, from, |at| {
                    at.floor == origin.floor
                        && at.x.abs_diff(origin.x) >= 10
                        && at.x.abs_diff(origin.x) <= 18
                        && at.y.abs_diff(origin.y) <= 6
                })
                .ok_or("no reachable distant yell witness tile")?;
                bots::walk_bot(&mut bots, 2, world, &path).await
            }
            .await,
        );
        for (case, mode) in [
            ("say", ChatSpeechMode::Say),
            ("whisper", ChatSpeechMode::Whisper),
            ("yell", ChatSpeechMode::Yell),
        ] {
            let result = async {
                let text = format!("MP07 {case} {}", uuid_text(&bots[0].character));
                let positions = bots
                    .iter_mut()
                    .map(|b| tile(b.client()?.world_spatial().actor_position))
                    .collect::<TestResult<Vec<_>>>()?;
                let mut recipients = Vec::new();
                for (i, p) in positions.iter().enumerate() {
                    let dx = p.x.abs_diff(positions[0].x);
                    let dy = p.y.abs_diff(positions[0].y);
                    let in_range = match mode {
                        ChatSpeechMode::Whisper => {
                            p.floor == positions[0].floor && dx <= 1 && dy <= 1
                        }
                        ChatSpeechMode::Yell => {
                            p.floor == positions[0].floor && dx <= 18 && dy <= 14
                        }
                        _ => p.floor == positions[0].floor && dx <= 8 && dy <= 6,
                    };
                    if in_range {
                        recipients.push(i);
                    }
                }
                send(
                    &mut bots[0],
                    ChatIntent::Say {
                        mode,
                        text: text.clone(),
                    },
                    ChatDisposition::Ok,
                )
                .await?;
                let text = if mode == ChatSpeechMode::Yell {
                    text.to_uppercase()
                } else {
                    text
                };
                delivered(&mut bots, &text, &recipients).await
            }
            .await;
            passed &= row(case, result);
        }
        let b = bots[1].name.clone();
        passed &= row(
            "private",
            async {
                let text = format!("MP07 private {}", uuid_text(&bots[0].character));
                send(
                    &mut bots[0],
                    ChatIntent::Private {
                        recipient_name: b.clone(),
                        text: text.clone(),
                    },
                    ChatDisposition::Ok,
                )
                .await?;
                // The sender may receive the negotiated private echo.
                bots::wait_observation(
                    &mut bots,
                    tokio::time::Instant::now() + Duration::from_secs(3),
                    "private B",
                    |bs, _| Ok(contains(&bs[1], &text)),
                )
                .await?;
                if contains(&bots[2], &text) {
                    return Err(format!(
                        "private leaked to C reply_chat_log={:?}",
                        bots[2].client()?.chat_log()
                    )
                    .into());
                }
                Ok(())
            }
            .await,
        );
        passed &= row("offline_private", async {
            // Use the real fourth Character, whose session is not admitted in MP07.
            let mut db = sqlx::PgConnection::connect(core.url).await?;
            let name: String = sqlx::query_scalar("SELECT name FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid").bind(core.characters[1].as_slice()).fetch_one(&mut db).await?;
            db.close().await?;
            send(&mut bots[0], ChatIntent::Private { recipient_name: name, text: "MP07 offline".into() }, ChatDisposition::NotOnline).await
        }.await);
        passed &= row(
            "private_channel",
            async {
                send(
                    &mut bots[0],
                    ChatIntent::PrivateChannel(Channel::Create),
                    ChatDisposition::Ok,
                )
                .await?;
                send(
                    &mut bots[0],
                    ChatIntent::PrivateChannel(Channel::Invite { name: b }),
                    ChatDisposition::Ok,
                )
                .await?;
                bots::wait_observation(
                    &mut bots,
                    tokio::time::Instant::now() + Duration::from_secs(3),
                    "channel list",
                    |bs, _| {
                        Ok(bs[0]
                            .session
                            .as_ref()
                            .and_then(|s| s.chat_log())
                            .and_then(|l| l.private_channels())
                            .and_then(|c| c.own_channel)
                            .is_some())
                    },
                )
                .await?;
                let id = bots[0]
                    .client()?
                    .chat_log()
                    .and_then(|l| l.private_channels())
                    .and_then(|c| c.own_channel)
                    .ok_or("own channel absent")?;
                send(
                    &mut bots[1],
                    ChatIntent::PrivateChannel(Channel::Join { channel_id: id }),
                    ChatDisposition::Ok,
                )
                .await?;
                send(
                    &mut bots[1],
                    ChatIntent::PrivateChannel(Channel::Say {
                        channel_id: id,
                        text: "MP07 channel".into(),
                    }),
                    ChatDisposition::Ok,
                )
                .await?;
                delivered(&mut bots, "MP07 channel", &[0, 1]).await
            }
            .await,
        );
        passed &= row(
            "default_channel",
            async {
                for bot in &mut bots {
                    send(
                        bot,
                        ChatIntent::OpenRoom(ChatRoom::Help),
                        ChatDisposition::Ok,
                    )
                    .await?;
                }
                send(
                    &mut bots[0],
                    ChatIntent::Room {
                        room: ChatRoom::Help,
                        text: "MP07 help".into(),
                    },
                    ChatDisposition::Ok,
                )
                .await?;
                delivered(&mut bots, "MP07 help", &[0, 1, 2]).await
            }
            .await,
        );
        passed &= row(
            "yell_exhaustion",
            send(
                &mut bots[0],
                ChatIntent::Say {
                    mode: ChatSpeechMode::Yell,
                    text: "MP07 exhaustion".into(),
                },
                ChatDisposition::Exhausted,
            )
            .await,
        );
        passed &= row(
            "spam_mute",
            async {
                let mut replies = Vec::new();
                // C has sent no speech in this scenario. Crystal's fifth line creates
                // the first five-second mute; the sixth is refused (game.cpp:6734,6748).
                for index in 0..6 {
                    let reply = bots[2].client()?.chat(&ChatIntent::Say { mode: ChatSpeechMode::Say, text: "MP07 spam".into() }).await?;
                    let correct = if index < 5 { reply.disposition == ChatDisposition::Ok } else { reply.disposition == ChatDisposition::Muted && reply.wait_seconds == 5 };
                    replies.push(reply);
                    if !correct { return Err(format!("Crystal buffer=4, fifth creates mute, sixth refused;reply={replies:?};source:crates/gameplay/social/src/chat/spam.rs:40").into()); }
                }
                bots::pump(&mut bots).await?;
                Ok(())
            }
            .await,
        );
        passed &= row("vip_relog", async {
            let name = bots[1].name.clone();
            vip_command(&mut bots[0], oteryn_protocol_oteryn::vip::Operation::Add, &name).await?;
            let before = vip_command(&mut bots[0], oteryn_protocol_oteryn::vip::Operation::Query, "").await?;
            bots::relog_all(&mut bots).await?;
            let after = vip_command(&mut bots[0], oteryn_protocol_oteryn::vip::Operation::Query, "").await?;
            for reply in [&before, &after] {
                let view: serde_json::Value = serde_json::from_str(&reply.result.view)?;
                if !view["entries"].as_array().is_some_and(|entries| entries.iter().any(|e| e["name"].as_str() == Some(&name))) {
                    return Err(format!("VIP name absent reply={reply:?};source:crates/gameplay/dispatch/src/gameplay_transport/vip.rs:111").into());
                }
            }
            presence(&mut bots, &name, oteryn_protocol_oteryn::vip_presence::Status::Online).await?;
            let (result, reply, _, _) = logout_retrying_busy(bots[1].client()?).await?;
            if result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) { return Err(format!("VIP logout reply={reply}").into()); }
            bots[1].session = None;
            let end = tokio::time::Instant::now() + Duration::from_secs(10);
            loop {
                bots[0].client()?.service_liveness(Duration::from_millis(20)).await?;
                let batch = bots[0].client()?.session_mut().vip_presence();
                if batch.as_ref().is_some_and(|b| b.entries.iter().any(|e| e.name == name && e.status == oteryn_protocol_oteryn::vip_presence::Status::Offline)) { break; }
                if tokio::time::Instant::now() >= end { return Err(format!("VIP offline deadline reply={batch:?};source:crates/gameplay/dispatch/src/gameplay_transport/vip.rs:10").into()); }
            }
            bots[1].session = Some(connect_character(packets, 0xb7, 0).await?);
            presence(&mut bots, &name, oteryn_protocol_oteryn::vip_presence::Status::Online).await?;
            Ok(())
        }.await);
        passed &= row("ignore_private", async {
            let recipient_name = bots[1].name.clone();
            let reply = bots[0].client()?.chat(&ChatIntent::Private { recipient_name, text: "MP07 ignore probe".into() }).await?;
            Err(format!("ignore list cannot be set through CHAT_V1;PM reply={reply:?};source:crates/gameplay/dispatch/src/gameplay_transport/chat_intent.rs:1176;wire:crates/protocol-oteryn/src/chat.rs:132;client-filter:crates/client-ui/src/features/0007-chat/state.rs:323").into())
        }.await);
    } else {
        for case in [
            "range_setup",
            "distant_witness",
            "say",
            "whisper",
            "yell",
            "private",
            "offline_private",
            "private_channel",
            "default_channel",
            "yell_exhaustion",
            "spam_mute",
            "vip_relog",
            "ignore_private",
        ] {
            row(
                case,
                Err("dependency admission failed;reply=see admission row".into()),
            );
        }
    }
    passed &= row("cleanup", bots::release_all(&mut bots, core, packets).await);
    if passed {
        Ok(())
    } else {
        Err("MP07 named checks failed;see MP_CHECK replies".into())
    }
}

async fn vip_command(
    bot: &mut Bot<'_, '_>,
    operation: oteryn_protocol_oteryn::vip::Operation,
    text: &str,
) -> TestResult<oteryn_dev_client::client_runtime::vip::VipResponse> {
    use oteryn_dev_client::client_runtime::vip;
    let command = vip::VipCommand::new(&oteryn_protocol_oteryn::vip::Request {
        operation,
        number: 0,
        text: text.into(),
        icon: 0,
        notify: true,
        groups: Vec::new(),
    })
    .ok_or("VIP encode failed")?;
    let reply = vip::send(bot.client()?.session_mut(), &command).await?;
    if reply.result.code != 1 {
        return Err(format!(
            "VIP reply={reply:?};source:crates/gameplay/dispatch/src/gameplay_transport/vip.rs:111"
        )
        .into());
    }
    Ok(reply)
}

async fn presence(
    bots: &mut [Bot<'_, '_>],
    name: &str,
    status: oteryn_protocol_oteryn::vip_presence::Status,
) -> TestResult {
    let end = tokio::time::Instant::now() + Duration::from_secs(10);
    loop {
        bots::pump(bots).await?;
        let batch = bots[0].client()?.session_mut().vip_presence();
        if batch.as_ref().is_some_and(|b| {
            b.entries
                .iter()
                .any(|e| e.name == name && e.status == status)
        }) {
            return Ok(());
        }
        if tokio::time::Instant::now() >= end {
            return Err(format!("VIP presence expected={status:?} reply={batch:?};source:crates/gameplay/dispatch/src/gameplay_transport/vip.rs:10").into());
        }
    }
}
