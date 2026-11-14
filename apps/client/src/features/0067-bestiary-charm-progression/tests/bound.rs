use super::{BestiaryBinding, BestiaryControl as C};
use crate::{
    combat_input::PlayCommand,
    play::{mailbox::lock, play_channel},
};
use oteryn_client_ui::charm_progression::Intent;
type Result = std::result::Result<(), Box<dyn std::error::Error>>;
fn setup() -> (
    crate::play::PlayLink,
    tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    crate::play::Outbox,
) {
    let (link, rx, out) = play_channel();
    let mut m = lock(&link.mailbox);
    m.properties.charm_progression.selected = true;
    m.properties.charm_progression.open = true;
    drop(m);
    (link, rx, out)
}
#[test]
fn charm_actions_are_bound_and_confirm_paid() -> Result {
    let (link, mut rx, out) = setup();
    let binding: BestiaryBinding = link.bestiary_binding();
    assert!(link.bestiary_dispatch_bound(&binding, C::Request(Intent::Query), false));
    let PlayCommand::Feature(query) = rx.try_recv()? else {
        return Err("query".into());
    };
    assert!(!link.bestiary_dispatch_bound(&binding, C::Request(Intent::Query), false));
    let mut reply = fixture::reply()?;
    let answer = |command: &oteryn_client_runtime::raw_feature::FeatureCommand,
                  reply: &oteryn_client_runtime::charm_progression::CharmProgressionReply|
     -> Result {
        out.bevy_bestiary_reply(
            command,
            oteryn_client_runtime::charm_progression::encode_charm_progression_reply(reply)
                .map_err(|e| format!("{e:?}"))?,
        );
        Ok(())
    };
    answer(&query, &reply)?;
    assert!(link.bestiary_dispatch_bound(&link.bestiary_binding(), C::SelectCharm(1), false));
    for intent in [Intent::Unlock, Intent::Assign, Intent::Remove] {
        if intent == Intent::Assign {
            assert!(link.bestiary_dispatch_bound(
                &link.bestiary_binding(),
                C::SelectRace(7),
                false
            ));
        }
        let binding = link.bestiary_binding();
        let before = binding.panel.clone();
        if intent != Intent::Assign {
            assert!(!link.bestiary_dispatch_bound(&binding, C::Request(intent), false));
            assert_eq!(link.charm_progression_panel(), before);
            assert!(rx.try_recv().is_err());
        }
        assert!(link.bestiary_dispatch_bound(&binding, C::Request(intent), true));
        assert_eq!(link.charm_progression_panel().reply, before.reply);
        assert!(!link.bestiary_dispatch_bound(&binding, C::Request(intent), true));
        let PlayCommand::Feature(command) = rx.try_recv()? else {
            return Err("command".into());
        };
        use oteryn_client_runtime::charm_progression::{self as w, CharmProgressionAction as A};
        let action =
            w::decode_charm_progression_request(&command.payload).map_err(|e| format!("{e:?}"))?;
        assert!(matches!(
            (intent, action),
            (Intent::Unlock, A::Unlock { .. })
                | (Intent::Assign, A::Assign { .. })
                | (Intent::Remove, A::Remove { .. })
        ));
        // Only the reply advances confirmed progress, balances and assignment.
        reply.outcome = match intent {
            Intent::Unlock => {
                w::CharmProgressionOutcome::Unlock(w::charm::CharmUnlockDisposition::Unlocked)
            }
            Intent::Assign => {
                w::CharmProgressionOutcome::Assign(w::charm::CharmAssignDisposition::Assigned)
            }
            _ => w::CharmProgressionOutcome::Remove(w::CharmRemoveDisposition::Removed),
        };
        reply.fee_gold = if intent == Intent::Remove { 1000 } else { 0 };
        reply.charms.charms[0].unlocked_stage = 1;
        reply.charms.charm_points_available = 700;
        reply.charms.charms[0].assigned_race = if intent == Intent::Assign {
            Some(reply.bestiary[0].race)
        } else {
            None
        };
        answer(&command, &reply)?;
        assert_eq!(link.charm_progression_panel().reply, Some(reply.clone()));
    }
    Ok(())
}
#[test]
fn charm_actions_rejected_when_session_replaced() -> Result {
    let (old, mut old_rx, _) = setup();
    let binding = old.bestiary_binding();
    let (new, mut new_rx, _) = setup();
    for c in controls() {
        let before = new.charm_progression_panel();
        assert!(!new.bestiary_dispatch_bound(&binding, c, true));
        assert_eq!(new.charm_progression_panel(), before);
    }
    let current = new.bestiary_binding();
    assert!(new.bestiary_dispatch_bound(&current, C::Close, false));
    let closed = new.charm_progression_panel();
    for c in controls() {
        assert!(!new.bestiary_dispatch_bound(&current, c, true));
        assert!(!new.bestiary_dispatch_bound(&new.bestiary_binding(), c, true));
        assert_eq!(new.charm_progression_panel(), closed);
    }
    let (ended, ended_rx, out) = setup();
    let binding = ended.bestiary_binding();
    drop(ended_rx);
    for c in controls() {
        assert!(!ended.bestiary_dispatch_bound(&binding, c, true));
        assert_eq!(ended.charm_progression_panel(), binding.panel);
    }
    out.bevy_bestiary_end();
    let before = ended.charm_progression_panel();
    for c in controls() {
        assert!(!ended.bestiary_dispatch_bound(&ended.bestiary_binding(), c, true));
        assert_eq!(ended.charm_progression_panel(), before);
    }
    assert!(old_rx.try_recv().is_err());
    assert!(new_rx.try_recv().is_err());
    Ok(())
}

#[path = "bound_fixture.rs"]
mod fixture;

fn controls() -> Vec<C> {
    let mut controls = vec![
        C::Request(Intent::Query),
        C::Request(Intent::Unlock),
        C::Request(Intent::Assign),
        C::Request(Intent::Remove),
        C::Request(Intent::ResetAll),
        C::CharmNames,
        C::RaceNames,
        C::SelectCharm(1),
        C::SelectRace(7),
        C::Close,
    ];
    controls.extend((0..=5).map(C::Catalogue));
    controls
}
