#![allow(clippy::expect_used, clippy::unwrap_used)]
use super::*;
use crate::durability::spell_field_policy::FieldWorldType;
use crate::spell::owned_cast_facts::CastFactsBinding;

fn fixture() -> (ChannelRuntimeV1, CastFactsBinding, CommandRef) {
    let (mut runtime, actor, session) =
        crate::gameplay_transport::actor_spell::tests::runtime_with_player(0x71);
    runtime
        .initialize_movement_test_position(
            actor,
            MovementLocalPosition {
                x: 10,
                y: 10,
                floor: 7,
            },
        )
        .unwrap();
    let command = CommandRef::new(session, crate::foundation::CommandId::new(1).unwrap());
    let facts = CastFactsBinding {
        actor,
        session,
        character: *session.as_bytes(),
        character_revision: 1,
        lease_generation: 1,
        connection_generation: 1,
        player_revision: 1,
        content_digest: [0x12; 32],
        equipment_revision: 1,
    };
    (runtime, facts, command)
}

#[test]
fn qualified_safe_context_keeps_source_conversion_and_decay_ids() {
    for (source, selected) in [
        (2123, 21465),
        (2118, 21465),
        (2121, 2134),
        (105, 2134),
        (2126, 2135),
        (2122, 2135),
        (2124, 2119),
        (2125, 2120),
        (2119, 2119),
        (2120, 2120),
    ] {
        assert_eq!(
            select_safe_field(source, FieldWorldType::NoPvp, Some(false)).unwrap(),
            selected
        );
        assert_eq!(
            select_safe_field(source, FieldWorldType::Pvp, Some(true)).unwrap(),
            selected
        );
        assert_eq!(
            select_safe_field(source, FieldWorldType::PvpEnforced, Some(true)).unwrap(),
            selected
        );
    }
}

#[test]
fn losing_safe_tile_or_changing_world_context_cannot_admit_pvp_creation() {
    for source in [2123, 2121, 2126] {
        assert!(select_safe_field(source, FieldWorldType::Pvp, Some(true)).is_ok());
        assert!(select_safe_field(source, FieldWorldType::Pvp, Some(false)).is_err());
        assert!(select_safe_field(source, FieldWorldType::PvpEnforced, Some(false)).is_err());
        assert!(select_safe_field(source, FieldWorldType::NoPvp, Some(false)).is_ok());
    }
}

#[test]
fn unknown_tile_policy_is_unavailable_even_in_a_no_pvp_world() {
    for mode in [
        FieldWorldType::NoPvp,
        FieldWorldType::Pvp,
        FieldWorldType::PvpEnforced,
    ] {
        assert!(select_safe_field(2123, mode, None).is_err());
    }
}

#[test]
fn field_descriptors_cannot_cross_command_character_lease_connection_or_content() {
    // This tests description equality only. No detached fixture creates SQL authority.
    let (_, facts, command) = fixture();
    let check = |value: &CastFactsBinding, actual_command| {
        check_caster_binding(
            value,
            command,
            actual_command,
            facts.character,
            1,
            1,
            facts.content_digest,
        )
    };
    assert!(check(&facts, command).is_ok());
    let next = CommandRef::new(facts.session, crate::foundation::CommandId::new(2).unwrap());
    assert!(check(&facts, next).is_err());
    for field in 0..5 {
        let mut stale = facts.clone();
        match field {
            0 => stale.character[15] ^= 1,
            1 => stale.lease_generation = 2,
            2 => stale.connection_generation = 2,
            3 => stale.content_digest[0] ^= 1,
            _ => stale.session = fixture_other_session(),
        }
        assert!(check(&stale, command).is_err());
    }
    assert!(
        check_caster_binding(
            &facts,
            command,
            command,
            facts.character,
            0,
            1,
            facts.content_digest
        )
        .is_err()
    );
    assert!(
        check_caster_binding(
            &facts,
            command,
            command,
            facts.character,
            1,
            0,
            facts.content_digest
        )
        .is_err()
    );
}

fn fixture_other_session() -> GameSessionId {
    crate::gameplay_transport::actor_spell::tests::runtime_with_player(0x72).2
}

#[test]
fn real_current_caster_read_refuses_a_foreign_session() {
    let (runtime, facts, _) = fixture();
    assert!(check_present_caster(&runtime, facts.actor, facts.session).is_ok());
    assert!(check_present_caster(&runtime, facts.actor, fixture_other_session()).is_err());
    assert!(check_present_caster(&runtime, facts.actor, facts.session).is_ok());
}

#[test]
fn present_actor_without_actual_position_is_not_a_field_caster() {
    let (runtime, actor, session) =
        crate::gameplay_transport::actor_spell::tests::runtime_with_player(0x73);
    assert!(runtime.player_control_facts(actor, session).is_ok());
    assert!(matches!(
        check_present_caster(&runtime, actor, session),
        Err(SpellItemError::Rejected(
            "field caster position unavailable"
        ))
    ));
}

#[test]
fn ended_actor_and_reused_slot_do_not_revive_old_field_caster() {
    let (mut runtime, facts, _) = fixture();
    runtime
        .remove_terminal_session(facts.session, facts.actor)
        .unwrap();
    assert!(check_present_caster(&runtime, facts.actor, facts.session).is_err());
    let session = fixture_other_session();
    let reservation = runtime.reserve_fresh_session(session).unwrap();
    let successor = runtime.commit_fresh_session(reservation).unwrap();
    runtime
        .initialize_movement_test_position(
            successor,
            MovementLocalPosition {
                x: 10,
                y: 10,
                floor: 7,
            },
        )
        .unwrap();
    assert_ne!(successor, facts.actor);
    assert!(check_present_caster(&runtime, successor, session).is_ok());
    assert!(check_present_caster(&runtime, facts.actor, facts.session).is_err());
}

#[test]
fn control_loss_keeps_actor_present_but_rejects_new_field_creation() {
    let (mut runtime, facts, _) = fixture();
    let character = crate::foundation::CharacterId::decode(&facts.character).unwrap();
    runtime
        .bind_attacker_lease(
            facts.actor,
            facts.session,
            crate::foundation::CharacterLease::new(character, 1).unwrap(),
        )
        .unwrap();
    runtime
        .record_control_loss(
            facts.actor,
            facts.session,
            crate::foundation::ControlLossMark {
                epoch: 1,
                grace_deadline: 100,
            },
        )
        .unwrap();
    assert!(
        runtime
            .player_control_facts(facts.actor, facts.session)
            .is_ok()
    );
    assert!(check_present_caster(&runtime, facts.actor, facts.session).is_err());
}
