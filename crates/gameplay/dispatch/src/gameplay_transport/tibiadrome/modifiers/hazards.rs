//! Current bounded native tile projection; immutable command receipt persistence owns bytes.
use super::*;

pub(super) fn publish_hazard(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    runtime: &ChannelRuntimeV1,
    modifiers: &mut ModifierOwner,
    kind: DromeModifier,
    positions: &[MovementLocalPosition],
    deadline: u64,
) -> bool {
    let kind = match kind {
        DromeModifier::TheFloorIsLava => wire::DromeHazardKind::Lava,
        DromeModifier::BeamMeUp => wire::DromeHazardKind::Beam,
        DromeModifier::TankedUp => wire::DromeHazardKind::Tanked,
        DromeModifier::SownSorrow => wire::DromeHazardKind::Sorrow,
        DromeModifier::BadRoots => wire::DromeHazardKind::Roots,
        _ => return false,
    };
    let Some(positions) = positions
        .iter()
        .map(|at| {
            owner
                .wire_observation(runtime, *at)
                .map(|value| value.actor_position)
        })
        .collect::<Option<Vec<_>>>()
    else {
        return false;
    };
    modifiers.hazards.retain(|hazard| hazard.kind != kind);
    modifiers.hazards.push(wire::DromeHazard {
        kind,
        expires_at_micros: deadline,
        positions,
    });
    true
}
pub(in crate::gameplay_transport) fn snapshot(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    session: GameSessionId,
) -> Option<wire::DromeHazards> {
    let live = owner.states.tibiadrome.live.lock().ok()?;
    let current = live.get(&session).filter(|current| !current.ended)?;
    let battle = current.battle.lock().ok()?;
    if battle.ended {
        return None;
    }
    let modifiers = battle.modifiers.as_ref()?;
    let now = owner.owner_now().get();
    Some(wire::DromeHazards {
        selected: modifiers.selected().map(|kind| match kind {
            DromeModifier::Somersault => 1,
            DromeModifier::GoingDownWithMe => 2,
            DromeModifier::ExplodingCorpses => 3,
            DromeModifier::ThatEscalatedQuickly => 4,
            DromeModifier::TheFloorIsLava => 5,
            DromeModifier::BeamMeUp => 6,
            DromeModifier::TankedUp => 7,
            DromeModifier::SownSorrow => 8,
            DromeModifier::BadRoots => 9,
        }),
        observed_at_micros: now,
        entries: modifiers
            .hazards
            .iter()
            .filter(|hazard| !battle.preparing && hazard.expires_at_micros > now)
            .cloned()
            .collect(),
    })
}
