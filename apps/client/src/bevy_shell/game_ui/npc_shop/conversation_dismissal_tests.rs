//! Real pending commands may finish after Close without reopening the shared presentation.
use super::*;
fn panels(app: &mut App) -> usize {
    app.world_mut()
        .query_filtered::<Entity, With<Panel>>()
        .iter(app.world())
        .count()
}
fn close(app: &mut App) -> Result {
    let entity = app
        .world_mut()
        .query::<(Entity, &Action)>()
        .iter(app.world())
        .find(|(_, action)| matches!(action.kind, Kind::Close))
        .map(|(entity, _)| entity)
        .ok_or("close")?;
    app.world_mut().trigger(Activate { entity });
    app.update();
    Ok(())
}
#[test]
fn npc_dismissal_suppresses_late_conversation_and_refusal_until_explicit_new_talk() -> Result {
    for refusal in [false, true] {
        let (mut app, _, send, mut commands, outbox) = editor_app()?;
        app.world_mut().trigger(Activate { entity: send });
        app.update();
        assert!(matches!(commands.try_recv()?, PlayCommand::NpcTalk(_)));
        let before = app
            .world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .npc_dialogue_panel();
        assert!(before.pending);
        close(&mut app)?;
        assert_eq!(panels(&mut app), 0);
        assert!(
            app.world()
                .non_send::<HostOwner>()
                .play
                .as_ref()
                .ok_or("play")?
                .link
                .npc_dialogue_panel()
                .pending
        );
        if refusal {
            outbox.bevy_npc_dialogue_result(oteryn_session::NpcIntentDisposition::RateLimited);
        } else {
            reply(&outbox, &["Late reply: {trade}."]);
        }
        app.update();
        app.update();
        assert_eq!(
            panels(&mut app),
            0,
            "late NPC result must not reopen the dismissed shared window"
        );
        let link = &app
            .world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link;
        let retained = link.npc_dialogue_panel();
        assert!(!retained.pending);
        assert!(retained.visible(), "canonical result must still reduce");
        if refusal {
            assert_eq!(retained.lines, before.lines);
            assert!(retained.feedback.is_some());
        } else {
            assert_eq!(retained.lines, ["Late reply: {trade}."]);
        }
        assert!(
            !link.npc_dialogue_reply(npc(), &retained.lines, "trade"),
            "bound replies cannot undismiss"
        );
        assert!(commands.try_recv().is_err());
        link.talk_to_npc(npc(), "hi");
        let PlayCommand::NpcTalk(intent) = commands.try_recv()? else {
            return Err("new greeting".into());
        };
        assert_eq!(intent.text, "hi");
        assert_eq!(intent.npc_actor, npc());
        app.update();
        assert_eq!(
            panels(&mut app),
            1,
            "explicit successful user talk must reopen"
        );
        assert!(commands.try_recv().is_err());
    }
    Ok(())
}
#[test]
fn npc_dismissal_suppresses_late_shop_after_talk_or_purchase_and_resets_for_new_session() -> Result
{
    for purchase in [false, true] {
        let (mut app, _, _, mut commands, outbox) = editor_app()?;
        if purchase {
            publish(&outbox, 8, 12);
            app.update();
            let select = app
                .world_mut()
                .query::<(Entity, &Action)>()
                .iter(app.world())
                .find(|(_, action)| matches!(action.kind, Kind::Select(_)))
                .map(|(entity, _)| entity)
                .ok_or("offer")?;
            app.world_mut().trigger(Activate { entity: select });
            app.update();
            let buy = app
                .world_mut()
                .query::<(Entity, &Action)>()
                .iter(app.world())
                .find(|(_, action)| matches!(action.kind, Kind::Trade))
                .map(|(entity, _)| entity)
                .ok_or("buy")?;
            app.world_mut().trigger(Activate { entity: buy });
            assert!(matches!(commands.try_recv()?, PlayCommand::NpcTrade(_)));
        } else {
            let trade = app
                .world_mut()
                .query::<(Entity, &conversation::Control)>()
                .iter(app.world())
                .find(|(_, control)| matches!(control.kind, conversation::Kind::Trade))
                .map(|(entity, _)| entity)
                .ok_or("trade request")?;
            app.world_mut().trigger(Activate { entity: trade });
            let PlayCommand::NpcTalk(intent) = commands.try_recv()? else {
                return Err("trade talk".into());
            };
            assert_eq!(intent.text, "trade");
        }
        close(&mut app)?;
        assert_eq!(panels(&mut app), 0);
        let link = &app
            .world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link;
        assert!(if purchase {
            link.npc_shop_panel().live.pending
        } else {
            link.npc_dialogue_panel().pending
        });
        outbox.bevy_npc_dialogue_result(oteryn_session::NpcIntentDisposition::Ok);
        publish(&outbox, 9, 15);
        app.update();
        app.update();
        assert_eq!(
            panels(&mut app),
            0,
            "late shop must not bypass shared dismissal"
        );
        let link = &app
            .world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link;
        assert!(!link.npc_shop_is_open());
        assert!(!link.npc_dialogue_is_visible());
        let retained = link.npc_shop_panel();
        assert!(retained.open);
        assert!(!retained.live.pending);
        assert_eq!(retained.live.quotes()[0].price, 15);
        assert!(!link.npc_dialogue_panel().pending);
        assert!(commands.try_recv().is_err());
        let (fresh, _, fresh_outbox) = play_channel();
        fresh.use_shared_frontend();
        fresh_outbox.bevy_npc_dialogue_supported();
        reply(&fresh_outbox, &["New admitted session."]);
        assert!(
            fresh.npc_dialogue_is_visible(),
            "a new session mailbox must reset presentation dismissal"
        );
        assert!(!link.npc_dialogue_is_visible());
        assert!(link.npc_dialogue_submit(npc(), "trade"));
        let PlayCommand::NpcTalk(intent) = commands.try_recv()? else {
            return Err("explicit shop open".into());
        };
        assert_eq!(intent.text, "trade");
        app.update();
        assert_eq!(panels(&mut app), 1);
        assert!(commands.try_recv().is_err());
    }
    Ok(())
}
