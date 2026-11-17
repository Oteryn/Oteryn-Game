//! Explicit mechanics-only fixture; never a production native-pool grant.
use super::*;
pub(super) fn context(
    _owner: &ComposedFreshAdmission<'_, '_, '_>,
    _runtime: &ChannelRuntimeV1,
    _run: [u8; 16],
    _players: usize,
    _center: MovementLocalPosition,
) -> Option<ModifierOwner> {
    if _owner
        .states
        .tibiadrome
        .mechanics_test_context
        .load(std::sync::atomic::Ordering::Acquire)
    {
        // Explicit mechanics-only test grant; this proves no donor room/pool qualification.
        let room = (-12..=12)
            .flat_map(|dy| {
                (-12..=12).map(move |dx| MovementLocalPosition {
                    x: _center.x + dx,
                    y: _center.y + dy,
                    floor: _center.floor,
                })
            })
            .filter(|at| walkable(_owner, _runtime, *at))
            .collect();
        let selected = if _owner
            .states
            .tibiadrome
            .mechanics_test_hazards
            .load(std::sync::atomic::Ordering::Acquire)
        {
            [DromeModifier::TheFloorIsLava, DromeModifier::SownSorrow]
        } else {
            [
                DromeModifier::Somersault,
                DromeModifier::ThatEscalatedQuickly,
            ]
        };
        return DromeModifierState::new(selected, _run, _players, room, _owner.owner_now().get())
            .ok()
            .map(|state| ModifierOwner {
                state,
                pending: Vec::new(),
                ordinal: 0,
                melee_ordinal: 0,
                admission_size: None,
                hazards: Vec::new(),
            });
    }
    None
}

pub(in crate::gameplay_transport) fn select(
    drome: &Drome,
    runtime: &mut ChannelRuntimeV1,
    session: GameSessionId,
    modifier: DromeModifier,
) -> Result<(), &'static str> {
    let live = drome.live.lock().map_err(|_| "Drome registry")?;
    let current = live
        .get(&session)
        .filter(|current| !current.ended)
        .ok_or("current Drome participant")?;
    let mut battle = current.battle.lock().map_err(|_| "Drome battle")?;
    if battle.ended
        || battle
            .opponents
            .iter()
            .any(|actor| !runtime.contains_live_creature(*actor))
    {
        return Err("select fixture modifiers before native combat");
    }
    let owner = battle
        .modifiers
        .as_mut()
        .ok_or("qualified Drome modifier owner")?;
    let other = if modifier == DromeModifier::ThatEscalatedQuickly {
        DromeModifier::Somersault
    } else {
        DromeModifier::ThatEscalatedQuickly
    };
    owner
        .state
        .select_test_modifiers([modifier, other])
        .map_err(|_| "distinct qualified Drome modifiers")?;
    owner.pending.clear();
    owner.hazards.clear();
    owner.ordinal = 0;
    owner.melee_ordinal = 0;
    // Keep the exact existing battle's actor policies coherent with this fixture selection.
    // The ordinary HP consumer still requires its selected modifier on the target slot.
    for (session, participant) in live.iter().filter(|(_, participant)| {
        !participant.ended
            && participant.run == current.run
            && Arc::ptr_eq(&participant.battle, &current.battle)
    }) {
        runtime
            .player_control_facts(participant.actor, *session)
            .map_err(|_| "current Drome participant session")?;
        runtime
            .bind_tibiadrome_modifiers(participant.actor, current.run, [modifier, other])
            .map_err(|_| "current Drome participant modifiers")?;
    }
    for actor in &battle.opponents {
        runtime
            .bind_tibiadrome_modifiers(*actor, current.run, [modifier, other])
            .map_err(|_| "current Drome opponent modifiers")?;
    }
    Ok(())
}
