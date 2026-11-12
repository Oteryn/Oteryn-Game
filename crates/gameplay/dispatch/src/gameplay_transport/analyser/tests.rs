#![allow(clippy::expect_used)]
use super::*;
use crate::gameplay_transport::actor_spell::tests::runtime_with_player;
use oteryn_protocol_oteryn::analyser::decode_analyser_facts;

#[test]
fn queues_are_selected_session_private_and_stale_connections_cannot_drain_or_discard() {
    let (_, actor, session) = runtime_with_player(31);
    let streams = AnalyserStreams::default();
    streams.impact(actor, session, 3, None);
    assert!(streams.next(actor, session, 1).is_none());
    assert_eq!(streams.snapshot(actor, session, 1, None), Some(false));
    streams.impact(actor, session, 3, None);
    assert_eq!(streams.snapshot(actor, session, 2, None), Some(true));
    streams.impact(actor, session, 7, None);
    assert_eq!(streams.snapshot(actor, session, 1, None), None);
    assert!(streams.next(actor, session, 1).is_none());
    assert_eq!(
        decode_analyser_facts(&streams.next(actor, session, 2).expect("current batch"))
            .expect("typed"),
        vec![AnalyserFact::Impact(AnalyserImpact::Healing(
            NonZeroU32::new(7).expect("positive")
        ))]
    );
    streams.forget(session);
    streams.impact(actor, session, 9, None);
    assert!(streams.next(actor, session, 2).is_none());
}

#[test]
fn analyser_reports_actual_incoming_damage_and_capped_self_healing() {
    use crate::gameplay_transport::actor_spell::{ChannelSpellStates, tests::FACTS};
    let (runtime, actor, session) = runtime_with_player(32);
    let mut spells = ChannelSpellStates::default();
    let now = oteryn_simulation_determinism::SemanticTimeMicros::from_micros(1_000_000);
    spells
        .initialize(&runtime, actor, session, FACTS, (0, 0), now)
        .expect("vitals");
    spells
        .analyser
        .snapshot(actor, session, 1, None)
        .expect("selected stream");
    let before = spells
        .get(&runtime, actor, session)
        .expect("current")
        .vitals()
        .health;
    let hit = spells
        .apply_creature_damage_kind(
            &runtime,
            actor,
            session,
            11,
            crate::player_lethal::PlayerHitKind::Fire,
            crate::foundation::owner_timer::SemanticTimeMicros::from_micros(now.get()),
            oteryn_protocol_oteryn::analyser::AnalyserDamageSource::Creature(None),
        )
        .expect("committed hit");
    let applied = before - hit.damage.health_after;
    assert!(applied > 0);
    let (healed, _) = spells
        .apply_health_gain(&runtime, actor, session, u64::MAX)
        .expect("self heal");
    assert_eq!(healed, applied);
    let facts = decode_analyser_facts(&spells.analyser.next(actor, session, 1).expect("facts"))
        .expect("decode");
    assert_eq!(
        facts,
        vec![
            AnalyserFact::DamageInput {
                value: NonZeroU32::new(applied).expect("positive"),
                element: DamageElement::Fire,
                source: oteryn_protocol_oteryn::analyser::AnalyserDamageSource::Creature(None),
            },
            AnalyserFact::Impact(AnalyserImpact::Healing(
                NonZeroU32::new(healed).expect("positive")
            ))
        ]
    );
    assert_eq!(
        spells
            .apply_health_gain(&runtime, actor, session, u64::MAX)
            .expect("full pool")
            .0,
        0
    );
    assert!(spells.analyser.next(actor, session, 1).is_none());
}
