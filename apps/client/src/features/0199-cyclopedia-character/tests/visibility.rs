use crate::{
    combat_input::PlayCommand,
    play::{mailbox::lock, play_channel},
};
use oteryn_client_runtime::{
    cyclopedia_character::{Badge, badge_visibility as wire},
    raw_feature::{CommandStatus, GatedCommandResult},
};
use oteryn_client_ui::cyclopedia_character::{BadgeFilter, Intent, badge_rows};
#[test]
fn public_badge_visibility_caller_waits_for_server_and_filters_confirmed_state()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, mut commands, outbox) = play_channel();
    outbox.cyclopedia_character_initialize(true);
    link.cyclopedia_character_request(Intent::Badges);
    let PlayCommand::Feature(query) = commands.try_recv()? else {
        return Err("missing query".into());
    };
    assert_eq!(
        wire::Request::decode(&query.payload).map_err(|_| "request")?,
        wire::Request::Query(None)
    );
    let reply = |hidden| -> Result<GatedCommandResult, Box<dyn std::error::Error>> {
        Ok(GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: CommandStatus::Accepted,
            payload: wire::Reply {
                earned: vec![Badge::FledgelingHero],
                hidden,
            }
            .encode()
            .map_err(|_| "reply")?,
        })
    };
    outbox.cyclopedia_character_result(&reply(vec![])?);
    assert!(link.cyclopedia_character_panel().badge_visibility_known);
    lock(&outbox.0).cyclopedia_character.hidden_badges.insert(1);
    link.cyclopedia_character_request(Intent::SetBadgeVisibility {
        badge: 1,
        public: false,
    });
    let PlayCommand::Feature(set) = commands.try_recv()? else {
        return Err("missing set".into());
    };
    assert_eq!(
        wire::Request::decode(&set.payload).map_err(|_| "set")?,
        wire::Request::Set {
            badge: Badge::FledgelingHero,
            public: false
        }
    );
    assert!(
        link.cyclopedia_character_panel().server_hidden_badges.is_empty(),
        "no optimistic visibility change"
    );
    outbox.cyclopedia_character_result(&reply(vec![Badge::FledgelingHero])?);
    {
        let mut mailbox = lock(&outbox.0);
        mailbox.cyclopedia_character.badge_filter = BadgeFilter::HiddenPublic;
        assert_eq!(badge_rows(&mailbox.cyclopedia_character).len(), 1);
        mailbox.cyclopedia_character.badge_filter = BadgeFilter::Public;
        assert!(badge_rows(&mailbox.cyclopedia_character).is_empty());
    }
    link.cyclopedia_character_request(Intent::SetBadgeVisibility {
        badge: 1,
        public: true,
    });
    let _ = commands.try_recv()?;
    outbox.cyclopedia_character_result(&GatedCommandResult {
        status: CommandStatus::Rejected,
        payload: vec![],
        command_id: 2,
        server_sequence: 2,
    });
    assert!(link.cyclopedia_character_panel().server_hidden_badges.contains(&1));
    assert!(link.cyclopedia_character_panel().hidden_badges.contains(&1), "server visibility updates preserve local hiding");
    lock(&outbox.0)
        .cyclopedia_character
        .change_target(Some([7; 16]));
    link.cyclopedia_character_request(Intent::SetBadgeVisibility {
        badge: 1,
        public: true,
    });
    assert!(
        commands.try_recv().is_err(),
        "another character cannot be edited"
    );
    Ok(())
}

#[test]
fn native_badge_controls_route_public_filters_and_confirmed_visibility_changes()
-> Result<(), &'static str> {
    use crate::play::feature_0199_cyclopedia_character::view::native;
    let mut panel = oteryn_client_ui::cyclopedia_character::CyclopediaCharacterPanel {
        selected: true,
        badges: Some(vec![1]),
        badge_visibility_known: true,
        ..Default::default()
    };
    assert!(native::submit(&mut panel, 152, ["", "", ""]).is_none());
    assert_eq!(panel.badge_filter, BadgeFilter::Public);
    let command = native::submit(&mut panel, 153, ["1", "", ""]).ok_or("native toggle")?;
    assert_eq!(
        wire::Request::decode(&command.payload).map_err(|_| "wire")?,
        wire::Request::Set {
            badge: Badge::FledgelingHero,
            public: false
        }
    );
    assert!(panel.server_hidden_badges.is_empty());
    panel.pending = false;
    panel.change_target(Some([7; 16]));
    assert!(native::submit(&mut panel, 153, ["1", "", ""]).is_none());
    Ok(())
}
