use crate::play::feature_0199_cyclopedia_character::view::native as native_controls;
use oteryn_client_runtime::raw_feature::{CommandStatus, GatedCommandResult};
use oteryn_client_ui::cyclopedia_character::{
    CyclopediaCharacterPanel, Intent, apply_result, request,
};
#[test]
fn native_character_navigation_and_input_gates() {
    let mut p = CyclopediaCharacterPanel {
        selected: true,
        ..Default::default()
    };
    for tab in 1..=10 {
        p.pending = false;
        let c = native_controls::submit(&mut p, 100 + tab, ["258", "20", ""]);
        let expected = if tab == 4 {
            vec![128, 0]
        } else {
            vec![tab as u8, 2, 1, 20]
        };
        assert_eq!(c.map(|c| c.payload), Some(expected));
        assert!(native_controls::submit(&mut p, 100 + tab, ["258", "20", ""]).is_none());
    }
    p.pending = false;
    assert!(native_controls::submit(&mut p, 101, ["0", "0", ""]).is_none());
    assert!(!p.pending);
    assert!(p.feedback.is_some());
    assert_eq!(
        native_controls::submit(&mut p, 111, ["bad", "bad", "bad"]).map(|c| c.kind),
        Some(387)
    );
    p.result = Some("Badges: Hero".into());
    assert!(native_controls::display(&p).contains("Badges: Hero"));
}

#[test]
fn native_character_target_and_page_validation() {
    let mut p = CyclopediaCharacterPanel {
        selected: true,
        ..Default::default()
    };
    for values in [
        ["65536", "20", ""],
        ["0", "51", ""],
        ["0", "20", "invalid"],
        ["0", "20", "é"],
    ] {
        assert!(native_controls::submit(&mut p, 101, values).is_none());
        assert!(!p.pending);
    }
    let command = native_controls::submit(
        &mut p,
        107,
        ["0", "1", "00112233-4455-6677-8899-aabbccddeeff"],
    );
    assert_eq!(
        command.map(|c| c.payload),
        Some(vec![
            7, 0, 0, 1, 0, 17, 34, 51, 68, 85, 102, 119, 136, 153, 170, 187, 204, 221, 238, 255
        ])
    );
    p.pending = false;
    p.selected = false;
    assert!(native_controls::submit(&mut p, 111, ["0", "20", ""]).is_none());
}

#[test]
fn gap_character_tabs_render_and_reject_malformed_replies() {
    use oteryn_client_ui::cyclopedia_character::panel_text;
    let mut p = CyclopediaCharacterPanel {
        selected: true,
        ..Default::default()
    };
    let mut body = vec![0; 42];
    body[0] = 1;
    body[9..13].copy_from_slice(&42_u32.to_le_bytes());
    let mut reply = GatedCommandResult {
        command_id: 1,
        status: CommandStatus::DuplicateReplay,
        server_sequence: 1,
        payload: body,
    };
    assert!(request(&mut p, Intent::Tab).is_some());
    assert!(apply_result(&mut p, &reply));
    assert!(panel_text(&p).contains("Level: 42"));
    let confirmed = p.result.clone();
    for payload in [vec![1, 0], vec![1; 16385], vec![2, 0, 0]] {
        assert!(request(&mut p, Intent::Tab).is_some());
        reply.payload = payload;
        assert!(!apply_result(&mut p, &reply));
        assert_eq!(p.result, confirmed);
    }
}

/// Exercise the production command loop, result routing and mailbox-backed panel reader.
#[test]
fn gap_cl1_session_loop_delivers_combat_modifiers_and_stamina()
-> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use crate::combat_input::{CombatCommand, CombatReport};
    use crate::play::{Stepper, play_channel, run_session};
    use oteryn_client_runtime::cyclopedia_character::{
        CharacterTab, GeneralStats, TabView,
        combat::{CombatStat, CombatStats},
    };
    use oteryn_client_runtime::raw_feature::FeatureCommand;
    use oteryn_session::{LogoutResult, SessionError, SessionEvent, StepDirection, StepOutcome};
    use std::time::Duration;

    struct Peer;
    impl Stepper for Peer {
        fn feature_selected(&self, capability: u32) -> bool {
            capability == 199
        }
        async fn feature(
            &mut self,
            command: &FeatureCommand,
        ) -> Result<GatedCommandResult, SessionError> {
            assert_eq!((command.capability, command.kind), (199, 388));
            let tab =
                oteryn_client_runtime::cyclopedia_character::decode_tab_request(&command.payload)
                    .map_err(|_| SessionError::SessionUnusable)?
                    .tab;
            let view = if tab == CharacterTab::GeneralStats {
                TabView::GeneralStats(GeneralStats {
                    stamina_minutes: 2520,
                    ..Default::default()
                })
            } else {
                let mut stats =
                    CombatStats::new(tab, 123, 1, 1).map_err(|_| SessionError::SessionUnusable)?;
                let key = *CombatStat::for_tab(tab)
                    .first()
                    .ok_or(SessionError::SessionUnusable)?;
                stats
                    .set(key, 250_000)
                    .map_err(|_| SessionError::SessionUnusable)?;
                TabView::Combat(stats)
            };
            Ok(GatedCommandResult {
                command_id: 1,
                status: CommandStatus::Accepted,
                server_sequence: 1,
                payload: view.encode().map_err(|_| SessionError::SessionUnusable)?,
            })
        }
        async fn step(&mut self, _: StepDirection) -> Result<StepOutcome, SessionError> {
            Err(SessionError::SessionUnusable)
        }
        async fn serve(&mut self, duration: Duration) -> Result<(), SessionError> {
            tokio::time::sleep(duration).await;
            Ok(())
        }
        fn drain(&mut self) -> Vec<SessionEvent> {
            vec![]
        }
        async fn combat(&mut self, _: CombatCommand) -> Result<CombatReport, SessionError> {
            Err(SessionError::SessionUnusable)
        }
        async fn logout(&mut self) -> Result<Option<LogoutResult>, SessionError> {
            Ok(None)
        }
    }
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;
            runtime.block_on(async {
                let (link, commands, outbox) = play_channel();
                let task = tokio::spawn(run_session(Peer, commands, outbox));
                tokio::time::timeout(Duration::from_secs(2), async {
                    while !link.cyclopedia_character_panel().selected {
                        tokio::task::yield_now().await;
                    }
                })
                .await?;
                for (tab, expected) in [
                    (CharacterTab::Offence, "CriticalChance: 25.0000%"),
                    (CharacterTab::Defence, "Dodge: 25.0000%"),
                    (CharacterTab::Misc, "Momentum: 25.0000%"),
                    (CharacterTab::GeneralStats, "Stamina: 42:00"),
                ] {
                    crate::play::mailbox::lock(&link.mailbox)
                        .cyclopedia_character
                        .tab = tab;
                    link.cyclopedia_character_request(Intent::Tab);
                    tokio::time::timeout(Duration::from_secs(2), async {
                        loop {
                            let panel = link.cyclopedia_character_panel();
                            if !panel.pending
                                && oteryn_client_ui::cyclopedia_character::panel_text(&panel)
                                    .contains(expected)
                            {
                                break;
                            }
                            tokio::task::yield_now().await;
                        }
                    })
                    .await?;
                }
                drop(link);
                task.await?;
                Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
            })
        })?
        .join()
        .map_err(|_| "session loop thread panicked")?
}

#[test]
fn gap_cl6_native_character_paging_preserves_cl5_controls_and_pending_fence() {
    use oteryn_client_runtime::cyclopedia_character::CharacterTab;
    let mut p = CyclopediaCharacterPanel {
        selected: true,
        tab: CharacterTab::RecentDeaths,
        ..Default::default()
    };
    let next = native_controls::submit(&mut p, 113, ["0", "20", ""]);
    assert_eq!(next.map(|c| c.payload), Some(vec![2, 20, 0, 20]));
    assert!(native_controls::submit(&mut p, 112, ["20", "20", ""]).is_none());
    assert_eq!(p.offset, 20);
    p.pending = false;
    let previous = native_controls::submit(&mut p, 112, ["20", "20", ""]);
    assert_eq!(previous.map(|c| c.payload), Some(vec![2, 0, 0, 20]));
    p.pending = false;
    let bounded = native_controls::submit(&mut p, 113, ["65530", "20", ""]);
    assert_eq!(bounded.map(|c| c.payload), Some(vec![2, 255, 255, 20]));
}

#[test]
fn native_pvp_filters_keep_the_confirmed_history_and_do_not_send_game_commands()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_client_runtime::cyclopedia_character::{CharacterTab, KillStatus, TabView};
    use oteryn_client_ui::cyclopedia_character::{
        CyclopediaCharacterPanel, apply_result, history_rows,
    };
    let mut p = CyclopediaCharacterPanel {
        selected: true,
        ..Default::default()
    };
    assert!(native_controls::submit(&mut p, 103, ["0", "20", ""]).is_some());
    assert_eq!(p.tab, CharacterTab::RecentPvpKills);
    let payload = TabView::RecentPvpKills(vec![
        (1, KillStatus::GuildWar, "A".into()),
        (2, KillStatus::Unjustified, "B".into()),
    ])
    .encode()
    .map_err(|_| "encode")?;
    assert!(apply_result(
        &mut p,
        &oteryn_client_runtime::raw_feature::GatedCommandResult {
            command_id: 1,
            status: oteryn_client_runtime::CommandStatus::Accepted,
            server_sequence: 1,
            payload
        }
    ));
    assert!(native_controls::submit(&mut p, 116, ["", "", ""]).is_none());
    assert_eq!(history_rows(&p)[0].description, "A");
    assert_eq!(history_rows(&p).len(), 1);
    assert!(native_controls::submit(&mut p, 115, ["", "", ""]).is_none());
    assert_eq!(history_rows(&p)[0].description, "B");
    assert!(native_controls::submit(&mut p, 114, ["", "", ""]).is_none());
    assert_eq!(history_rows(&p).len(), 2);
    assert!(!p.pending);
    Ok(())
}
