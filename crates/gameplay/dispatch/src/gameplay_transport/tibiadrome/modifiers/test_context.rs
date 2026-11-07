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
