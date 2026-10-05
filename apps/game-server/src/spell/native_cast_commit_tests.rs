#![allow(clippy::expect_used)]
use super::super::cast::{CastContext, cast, v1_spell_book};
use super::*;
use crate::ability::{AbilityOccurrence, RevisionSet};
use crate::gameplay_transport::actor_spell::tests::FACTS;
use oteryn_protocol_oteryn::actor_spell::{SpellCastIntent, SpellTarget};
use std::num::NonZeroU32;

fn paid_fixture() -> (
    PlayerSpellState,
    PlayerSpellState,
    SpellBook,
    SemanticTimeMicros,
) {
    let book = v1_spell_book().expect("admitted book");
    let mut before = PlayerSpellState::new(
        super::super::cast::CharacterCastFacts { level: 20, ..FACTS },
        0,
        0,
    )
    .expect("actual player state");
    before.cooldowns.spells.insert(
        "existing:long".into(),
        SemanticTimeMicros::from_micros(9100000),
    );
    before.cooldowns.spells.insert(
        "existing:short".into(),
        SemanticTimeMicros::from_micros(1100000),
    );
    let now = SemanticTimeMicros::from_micros(100000);
    let occurrence = AbilityOccurrence::new(
        "native-metadata:test",
        RevisionSet::new("rules:1", "content:1", "world:1", "formula:1", "sim:1")
            .expect("revisions"),
    )
    .expect("occurrence");
    let next = cast(
        &book,
        &before,
        &SpellCastIntent {
            spell: NonZeroU32::new(1).expect("spell"),
            target: SpellTarget::None,
            aim_at_target: false,
        },
        CastContext {
            caster: "actor:fixture",
            owner_scope: "owner",
            occurrence,
            now,
            draw: &mut |low, _| low,
        },
    )
    .expect("real common paid cast");
    (before, next, book, now)
}

#[test]
fn beam_reduces_prior_spell_deadlines_then_rearms_new_own_cd_without_extra_payment() {
    let (before, mut next, book, now) = paid_fixture();
    let spell = &book.spells[0];
    let common = next
        .payment_anchor_from(&before)
        .expect("actual common paid anchor");
    let changes = CastOwnerChanges {
        cooldown_ms: Some(4000),
        reduce_all_spell_cooldowns_ms: 3000,
        ..CastOwnerChanges::default()
    };
    let anchor = next
        .apply_native_owner_changes(&before, spell, &book, &changes, now)
        .expect("source order");
    assert_eq!(
        next.cooldowns
            .spell_ready_at("existing:long")
            .expect("prior")
            .get(),
        6100000
    );
    assert_eq!(
        next.cooldowns
            .spell_ready_at("existing:short")
            .expect("clamped")
            .get(),
        100000
    );
    assert_eq!(
        next.cooldowns
            .spell_ready_at(&spell.key)
            .expect("new postcast")
            .get(),
        4100000
    );
    assert_eq!(anchor.expected_revision, common.expected_revision);
    assert_eq!(anchor.next_revision, common.next_revision);
    assert_eq!(
        (anchor.paid_mana, anchor.paid_soul),
        (common.paid_mana, common.paid_soul)
    );
    assert_eq!(
        next.cooldowns.groups,
        before_groups_with_new_cast(&book, now)
    );
}
fn before_groups_with_new_cast(
    book: &SpellBook,
    now: SemanticTimeMicros,
) -> std::collections::BTreeMap<String, SemanticTimeMicros> {
    let mut cooldowns = super::super::Cooldowns::default();
    cooldowns
        .rearm(&book.spells[0], now)
        .expect("common groups");
    cooldowns.groups
}

#[test]
fn shared_peer_source_name_resolves_actual_book_identity_instead_of_slugging_text() {
    let (before, mut next, book, now) = paid_fixture();
    let spell = &book.spells[0];
    let peer = &book.spells[1];
    let changes = CastOwnerChanges {
        shared_cooldown: Some((peer.name.to_ascii_uppercase(), 6000)),
        ..CastOwnerChanges::default()
    };
    next.apply_native_owner_changes(&before, spell, &book, &changes, now)
        .expect("uniquely qualified peer");
    assert_eq!(
        next.cooldowns
            .spell_ready_at(&peer.key)
            .expect("exact canonical identity")
            .get(),
        6100000
    );
    assert!(
        next.cooldowns
            .spell_ready_at(&peer.name.to_ascii_uppercase())
            .is_none()
    );
}

#[test]
fn missing_peer_or_time_overflow_does_not_partially_change_paid_successor() {
    let (before, mut next, book, now) = paid_fixture();
    let original = next.clone();
    let spell = &book.spells[0];
    for changes in [
        CastOwnerChanges {
            shared_cooldown: Some(("unqualified peer".into(), 1)),
            ..CastOwnerChanges::default()
        },
        CastOwnerChanges {
            cooldown_ms: Some(u64::MAX),
            ..CastOwnerChanges::default()
        },
        CastOwnerChanges {
            reduce_all_spell_cooldowns_ms: u64::MAX,
            ..CastOwnerChanges::default()
        },
    ] {
        assert!(
            next.apply_native_owner_changes(&before, spell, &book, &changes, now)
                .is_err()
        );
        assert_eq!(next, original);
    }
}
