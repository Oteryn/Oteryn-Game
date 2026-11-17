use super::*;
use crate::spectate::spectate_test_support::facts;

#[test]
fn changed_epoch_cannot_deliver_content_before_durable_audit_ack() -> Result<(), Error> {
    let mut owner = CastOwner::new(100)?;
    let target = facts(1, 1, 1);
    let state = owner.start(target, Settings::default(), 0)?;
    let id = owner.join(&ticket(state, 1, 1), 0)?;
    owner.model_audit_commit();
    owner.update(target, state.revision, Settings::default(), 0)?;
    owner.publish(
        target,
        &[],
        Scene {
            tiles: vec![],
            actors: vec![],
        },
        0,
    )?;
    assert!(
        owner
            .drain(id, 0)?
            .iter()
            .all(|v| matches!(v, Content::Clear))
    );
    owner.model_audit_commit();
    assert!(
        owner
            .drain(id, 0)?
            .iter()
            .any(|v| matches!(v, Content::Scene(_)))
    );
    Ok(())
}

fn ticket(state: State, account: u8, nonce: u8) -> VerifiedAdmission {
    VerifiedAdmission {
        cast_id: state.cast_id,
        epoch: state.epoch,
        account: [account; 16],
        reservation: [nonce; 16],
        access_revision: 1,
        expires_ms: 30_000,
        chat: true,
    }
}

#[test]
fn privacy_default_slate_withdrawal_purges_delayed_scene_and_requires_fresh_baseline()
-> Result<(), Error> {
    let mut owner = CastOwner::new(100)?;
    let target = facts(1, 1, 1);
    let state = owner.start(
        target,
        Settings {
            delay_seconds: 1,
            ..Settings::default()
        },
        0,
    )?;
    let id = owner.join(&ticket(state, 1, 1), 0)?;
    owner.model_audit_commit();
    let mut bytes = *target.authority.commit().character_id().as_bytes();
    bytes[15] ^= 1;
    let player = oteryn_gameplay_domain::domain::CharacterId::from_bytes(bytes)
        .map_err(|_| Error::Invalid)?;
    let scene = Scene {
        tiles: vec![(1, 2, 3)],
        actors: vec![],
    };
    owner.publish(target, &[player], scene.clone(), 0)?;
    assert_eq!(
        owner.drain(id, 1000)?,
        vec![
            Content::Metadata {
                viewers: 1,
                delay_seconds: 1
            },
            Content::Slate
        ]
    );
    owner.consent(
        player,
        oteryn_gameplay_durability::durability::cast_consent::CastConsent {
            allowed: true,
            revision: 1,
        },
    )?;
    owner.publish(target, &[player], scene.clone(), 1000)?;
    owner.consent(
        player,
        oteryn_gameplay_durability::durability::cast_consent::CastConsent {
            allowed: false,
            revision: 2,
        },
    )?;
    assert_eq!(owner.drain(id, 1000)?, vec![Content::Clear]);
    assert!(
        owner
            .drain(id, 2000)?
            .iter()
            .all(|c| !matches!(c, Content::Scene(_)))
    );
    assert_eq!(
        owner.consent(
            player,
            oteryn_gameplay_durability::durability::cast_consent::CastConsent {
                allowed: true,
                revision: 1
            }
        ),
        Err(Error::Conflict)
    );
    // A hidden/off-screen character is absent from this committed interest set.
    owner.publish(target, &[], scene, 2000)?;
    assert!(
        owner
            .drain(id, 3000)?
            .iter()
            .any(|c| matches!(c, Content::Scene(_)))
    );
    Ok(())
}

#[test]
fn moderation_and_live_caster_chat_do_not_bypass_viewer_delay() -> Result<(), Error> {
    let mut owner = CastOwner::new(100)?;
    let target = facts(1, 1, 1);
    let state = owner.start(
        target,
        Settings {
            delay_seconds: 300,
            ..Settings::default()
        },
        0,
    )?;
    let id = owner.join(&ticket(state, 1, 1), 0)?;
    owner.model_audit_commit();
    owner.chat(id, "cafe\u{301}", 0)?;
    assert_eq!(
        owner.take_caster_chat(target.authority.current_game_session_id())[0]
            .text
            .as_ref(),
        "café"
    );
    assert!(owner.drain(id, 0)?.is_empty());
    owner.mute(target, state.revision, id, true)?;
    assert_eq!(owner.chat(id, "blocked", 0), Err(Error::Muted));
    owner.mute(target, state.revision, id, false)?;
    owner.model_audit_commit();
    assert_eq!(owner.chat(id, "\u{202e}spoof", 0), Err(Error::Invalid));
    for now in (4000..300_000).step_by(4000) {
        let mut renewed = ticket(state, 1, 1);
        renewed.expires_ms = now + 5000;
        owner.renew(id, &renewed, now)?;
        owner.keepalive(id, now)?;
    }
    assert!(
        owner
            .drain(id, 300_000)?
            .iter()
            .any(|line| matches!(line, Content::Chat(_)))
    );
    owner.kick(target, state.revision, id)?;
    assert_eq!(
        owner.chat(id, "after kick", 300_000),
        Err(Error::Unavailable)
    );
    assert_eq!(owner.drain(id, 300_000), Err(Error::Unavailable));
    Ok(())
}

#[test]
fn admission_caps_accounts_without_characters_and_rejects_replay() -> Result<(), Error> {
    let mut owner = CastOwner::new(100)?;
    let target = facts(1, 1, 1);
    let state = owner.start(target, Settings::default(), 0)?;
    let first = owner.join(&ticket(state, 1, 1), 0)?;
    let second = owner.join(&ticket(state, 1, 2), 0)?;
    assert_eq!(owner.join(&ticket(state, 1, 3), 0), Err(Error::Full));
    owner.leave(first)?;
    assert_eq!(owner.join(&ticket(state, 1, 1), 0), Err(Error::Unavailable));
    owner.leave(second)?;
    for i in 1..=50 {
        owner.join(&ticket(state, i, i + 3), 0)?;
    }
    assert_eq!(owner.join(&ticket(state, 99, 99), 0), Err(Error::Full));
    let mut wrong = ticket(state, 99, 100);
    wrong.access_revision = 2;
    assert_eq!(owner.join(&wrong, 0), Err(Error::Unavailable));
    assert_eq!(
        owner
            .state(target.authority.current_game_session_id())
            .map(|s| s.viewers),
        Some(50)
    );
    owner.stop(target, state.revision)?;
    assert_eq!(owner.take_releases().len(), 52);
    Ok(())
}

#[test]
fn lifecycle_defaults_live_and_fences_scope_without_competitive_refusal() -> Result<(), Error> {
    let mut owner = CastOwner::new(100)?;
    let mut target = facts(1, 1, 1);
    target.observable = false; // Old competitive classification is not an S17 refusal.
    let first = owner.start(target, Settings::default(), 0)?;
    assert_eq!(first.delay_seconds, 0);
    assert_eq!(
        owner.start(target, Settings::default(), 0),
        Err(Error::Conflict)
    );
    let changed = owner.update(
        target,
        first.revision,
        Settings {
            delay_seconds: 300,
            ..Settings::default()
        },
        1,
    )?;
    assert!(changed.epoch > first.epoch);
    assert_eq!(
        owner.update(
            target,
            changed.revision,
            Settings {
                delay_seconds: 301,
                ..Settings::default()
            },
            2
        ),
        Err(Error::Invalid)
    );
    assert_eq!(owner.stop(target, first.revision), Err(Error::Conflict));
    owner.stop(target, changed.revision)?;
    assert!(
        owner
            .state(target.authority.current_game_session_id())
            .is_none()
    );
    let next = owner.start(target, Settings::default(), 3)?;
    assert!(next.revision > changed.revision);
    owner.refresh(facts(2, 1, 1));
    assert!(
        owner
            .state(target.authority.current_game_session_id())
            .is_none()
    );
    Ok(())
}

#[test]
fn live_admission_response_is_purpose_node_epoch_and_expiry_bound() -> Result<(), Error> {
    let node = [1; 16];
    let body = br#"{"purpose":"oteryn-cast-observer-v1","node":"01010101-0101-0101-0101-010101010101","account":"02020202-0202-0202-0202-020202020202","cast":"03030303030303030303030303030303","epoch":1,"accessRevision":2,"reservation":"04040404-0404-0404-0404-040404040404","expiresAt":104,"chat":true}"#;
    let proof = access::decode_response(body, node, 100, 1000)?;
    assert_eq!(proof.epoch, 1);
    assert_eq!(proof.expires_ms, 4000);
    assert!(access::decode_response(body, [2; 16], 100, 1000).is_err());
    assert!(access::decode_response(body, node, 104, 1000).is_err());
    assert!(access::decode_response(body, node, 98, 1000).is_err());
    let wrong =
        String::from_utf8_lossy(body).replace("oteryn-cast-observer-v1", "oteryn-native-player-v1");
    assert!(access::decode_response(wrong.as_bytes(), node, 100, 1000).is_err());
    Ok(())
}

#[test]
fn viewer_keepalive_cannot_extend_the_global_reservation_lease() -> Result<(), Error> {
    let mut owner = CastOwner::new(100)?;
    let target = facts(1, 1, 1);
    let state = owner.start(target, Settings::default(), 0)?;
    let mut grant = ticket(state, 1, 1);
    grant.expires_ms = 5000;
    let id = owner.join(&grant, 0)?;
    owner.keepalive(id, 4000)?;
    owner.poll(5000)?;
    assert_eq!(
        owner
            .state(target.authority.current_game_session_id())
            .ok_or(Error::Unavailable)?
            .viewers,
        0
    );
    Ok(())
}

#[test]
fn off_tick_service_uses_the_same_lifecycle_and_never_promotes_observers_to_players()
-> Result<(), Error> {
    let mut owner = CastOwner::new(4)?;
    let target = facts(1, 1, 1);
    let service::Reply::State(state) = service::apply(
        &mut owner,
        service::Operation::Start(target, Settings::default(), [1; 16]),
        0,
        0,
    )?
    else {
        return Err(Error::Invalid);
    };
    let service::Reply::Observer(id) = service::apply(
        &mut owner,
        service::Operation::Join(ticket(state, 1, 1)),
        0,
        0,
    )?
    else {
        return Err(Error::Invalid);
    };
    owner.model_audit_commit();
    service::apply(
        &mut owner,
        service::Operation::Chat(id, "live room".into()),
        0,
        0,
    )?;
    owner.model_audit_commit();
    let service::Reply::Chat(lines) = service::apply(
        &mut owner,
        service::Operation::CasterChat(target.authority.current_game_session_id()),
        0,
        0,
    )?
    else {
        return Err(Error::Invalid);
    };
    assert_eq!(lines.len(), 1);
    service::apply(
        &mut owner,
        service::Operation::Gone(target.authority.current_game_session_id()),
        0,
        0,
    )?;
    assert!(owner.viewers.is_empty());
    Ok(())
}

#[test]
fn a_precommit_consent_barrier_rejects_an_older_opt_in_read() -> Result<(), Error> {
    let mut owner = CastOwner::new(4)?;
    let target = facts(1, 1, 1);
    let bytes = *target.authority.commit().character_id().as_bytes();
    let character = CharacterId::from_bytes(bytes).map_err(|_| Error::Invalid)?;
    owner.consent(
        character,
        CastConsent {
            allowed: true,
            revision: 1,
        },
    )?;
    owner.invalidate(character, 1)?;
    assert_eq!(
        owner.consent(
            character,
            CastConsent {
                allowed: true,
                revision: 1
            }
        ),
        Err(Error::Conflict)
    );
    owner.consent(
        character,
        CastConsent {
            allowed: false,
            revision: 2,
        },
    )?;
    Ok(())
}

#[test]
fn a_full_three_hundred_second_delay_survives_continuous_committed_scenes() -> Result<(), Error> {
    let mut owner = CastOwner::new(4)?;
    let target = facts(1, 1, 1);
    let state = owner.start(
        target,
        Settings {
            delay_seconds: 300,
            ..Settings::default()
        },
        0,
    )?;
    let id = owner.join(&ticket(state, 1, 1), 0)?;
    owner.model_audit_commit();
    for second in 0..=300 {
        let now = second * 1000;
        let mut proof = ticket(state, 1, 1);
        proof.expires_ms = now + 5000;
        owner.renew(id, &proof, now)?;
        owner.publish(
            target,
            &[],
            Scene {
                tiles: vec![(1, 2, 3)],
                actors: vec![],
            },
            now,
        )?;
        let content = owner.drain(id, now)?;
        if second < 300 {
            assert!(content.iter().all(|v| !matches!(v, Content::Scene(_))));
        } else {
            assert!(content.iter().any(|v| matches!(v, Content::Scene(_))));
        }
    }
    Ok(())
}

#[test]
fn viewer_chat_requires_the_capability_bound_to_the_account_ticket() -> Result<(), Error> {
    let mut owner = CastOwner::new(4)?;
    let target = facts(1, 1, 1);
    let state = owner.start(target, Settings::default(), 0)?;
    let mut proof = ticket(state, 1, 1);
    proof.chat = false;
    let id = owner.join(&proof, 0)?;
    owner.model_audit_commit();
    assert_eq!(
        owner.chat(id, "unselected chat", 0),
        Err(Error::Unavailable)
    );
    assert!(
        owner
            .take_caster_chat(target.authority.current_game_session_id())
            .is_empty()
    );
    Ok(())
}

#[test]
fn a_capture_started_before_a_global_revocation_cannot_restore_unknown_old_consent()
-> Result<(), Error> {
    let mut owner = CastOwner::new(8)?;
    let target = facts(1, 1, 1);
    let state = owner.start(target, Settings::default(), 0)?;
    let id = owner.join(&ticket(state, 1, 1), 0)?;
    owner.model_audit_commit();
    let service::Reply::Capture(permit) =
        service::apply(&mut owner, service::Operation::Capture(target), 0, 0)?
    else {
        return Err(Error::Invalid);
    };
    let mut bytes = *target.authority.commit().character_id().as_bytes();
    bytes[15] ^= 1;
    let bystander = CharacterId::from_bytes(bytes).map_err(|_| Error::Invalid)?;
    let old = CastConsent {
        allowed: true,
        revision: 1,
    };
    assert!(matches!(
        service::apply(
            &mut owner,
            service::Operation::Publish(
                permit,
                target,
                vec![bystander],
                Scene {
                    tiles: vec![],
                    actors: vec![]
                },
                vec![(bystander, old)]
            ),
            0,
            1
        ),
        Err(Error::Unavailable)
    ));
    assert!(!owner.consents.contains_key(&bystander));
    assert!(
        owner
            .drain(id, 0)?
            .iter()
            .all(|v| !matches!(v, Content::Scene(_)))
    );
    Ok(())
}

#[test]
fn s17_media_obeys_scene_consent_delay_and_revocation() -> Result<(), Error> {
    let mut owner = CastOwner::new(100)?;
    let f = facts(1, 1, 1);
    let state = owner.start(
        f,
        Settings {
            delay_seconds: 1,
            ..Settings::default()
        },
        0,
    )?;
    let id = owner.join(&ticket(state, 1, 1), 0)?;
    owner.model_audit_commit();
    let media = oteryn_protocol_oteryn::cast::PublicMedia::Map {
        reset: true,
        last: true,
        tiles: vec![],
    };
    owner.publish_media(f, &[], vec![media.clone()], 0)?;
    assert!(
        !owner
            .drain(id, 0)?
            .iter()
            .any(|c| matches!(c, Content::Media(_)))
    );
    assert!(
        owner
            .drain(id, 1000)?
            .iter()
            .any(|c| matches!(c, Content::Media(_)))
    );
    let mut bytes = *f.authority.commit().character_id().as_bytes();
    bytes[15] ^= 1;
    let other = CharacterId::from_bytes(bytes).map_err(|_| Error::Invalid)?;
    owner.publish_media(f, &[other], vec![media], 1000)?;
    assert!(
        !owner
            .drain(id, 2000)?
            .iter()
            .any(|c| matches!(c, Content::Media(_)))
    );
    Ok(())
}

#[test]
fn s17_caster_reply_is_live_and_viewer_copy_delayed() -> Result<(), Error> {
    let mut owner = CastOwner::new(100)?;
    let f = facts(1, 1, 1);
    let state = owner.start(
        f,
        Settings {
            delay_seconds: 1,
            ..Settings::default()
        },
        0,
    )?;
    let id = owner.join(&ticket(state, 1, 1), 0)?;
    owner.model_audit_commit();
    owner.reply(f, "Welcome", 0)?;
    assert_eq!(
        owner
            .take_caster_chat(f.authority.current_game_session_id())
            .len(),
        1
    );
    assert!(
        !owner
            .drain(id, 0)?
            .iter()
            .any(|c| matches!(c, Content::Chat(_)))
    );
    assert!(
        owner
            .drain(id, 1000)?
            .iter()
            .any(|c| matches!(c, Content::Chat(_)))
    );
    Ok(())
}

#[test]
fn s17_viewer_names_and_account_rate_limits_survive_session_changes() -> Result<(), Error> {
    let mut owner = CastOwner::new(50)?;
    let f = facts(1, 1, 1);
    let state = owner.start(f, Settings::default(), 0)?;
    let one = owner.join(&ticket(state, 1, 1), 0)?;
    let two = owner.join(&ticket(state, 1, 2), 0)?;
    owner.model_audit_commit();
    for id in [one, two, one] {
        owner.chat(id, "hi", 0)?;
    }
    assert_eq!(owner.chat(two, "fourth", 0), Err(Error::Full));
    let lines = owner.take_caster_chat(f.authority.current_game_session_id());
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0].name, lines[2].name);
    assert_ne!(lines[0].name, lines[1].name);
    owner.mute(f, state.revision, one, true)?;
    owner.model_audit_commit();
    assert_eq!(owner.chat(two, "muted account", 3000), Err(Error::Muted));
    owner.mute(f, state.revision, two, false)?;
    owner.model_audit_commit();
    owner.chat(two, "unmuted", 3000)?;
    Ok(())
}

#[test]
fn s17_quiet_viewer_is_named_to_caster_before_speaking() -> Result<(), Error> {
    let mut owner = CastOwner::new(50)?;
    let f = facts(1, 1, 1);
    let state = owner.start(f, Settings::default(), 0)?;
    let id = owner.join(&ticket(state, 1, 1), 0)?;
    assert!(
        owner
            .take_caster_presence(f.authority.current_game_session_id())
            .is_empty()
    );
    owner.model_audit_commit();
    let names = owner.take_caster_presence(f.authority.current_game_session_id());
    assert_eq!(names.len(), 1);
    assert_eq!(names[0].handle, id.0);
    assert!(names[0].name.starts_with("Viewer-"));
    owner.kick(f, state.revision, id)?;
    owner.model_audit_commit();
    assert_eq!(
        owner
            .take_caster_presence(f.authority.current_game_session_id())
            .len(),
        1
    );
    Ok(())
}
