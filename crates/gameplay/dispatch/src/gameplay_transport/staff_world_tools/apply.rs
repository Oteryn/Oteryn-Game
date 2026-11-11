use super::*;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) fn apply_staff_world(
        &self,
        runtime: &mut ChannelRuntimeV1,
        issuer: StaffActorProof,
        request: &rules::Request,
        target: Option<StaffActorProof>,
        destination: Option<StaffActorProof>,
    ) -> bool {
        if runtime.owner_fence().is_err() || !issuer.current(runtime) {
            return false;
        }
        let Some(target) = target else {
            return false;
        };
        if !target.current(runtime) {
            return false;
        }
        let (actor, session) = (target.actor, target.session);
        match &request.action {
            rules::Action::Broadcast { text, .. } => self
                .states
                .server_messages
                .staff_publish(&request.operation_id, None, text)
                .is_ok(),
            rules::Action::Notify { text, .. } => self
                .states
                .server_messages
                .staff_publish(&request.operation_id, Some(session), text)
                .is_ok(),
            rules::Action::Ghost { enabled, .. } => runtime
                .actor_staff_visibility(actor)
                .is_ok_and(|(_, invisible, _)| {
                    runtime
                        .set_staff_visibility(actor, session, *enabled, invisible)
                        .is_ok()
                }),
            rules::Action::Invisible { enabled, .. } => runtime
                .actor_staff_visibility(actor)
                .is_ok_and(|(ghost, _, _)| {
                    runtime
                        .set_staff_visibility(actor, session, ghost, *enabled)
                        .is_ok()
                }),
            rules::Action::Teleport { x, y, floor, .. } => self.staff_relocate(
                runtime,
                actor,
                session,
                crate::foundation::MovementLocalPosition {
                    x: *x,
                    y: *y,
                    floor: *floor,
                },
                false,
            ),
            rules::Action::TeleportToPlayer { .. } | rules::Action::Bring { .. } => {
                let Some(to) = destination else {
                    return false;
                };
                if !to.current(runtime) {
                    return false;
                }
                let Ok(at) = runtime.read_actor_position(to.actor) else {
                    return false;
                };
                self.staff_relocate(runtime, actor, session, at.position(), true)
            }
            _ => false,
        }
    }
    fn staff_relocate(
        &self,
        runtime: &mut ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        to: crate::foundation::MovementLocalPosition,
        nearby: bool,
    ) -> bool {
        let Some(map) = self.bundle_map() else {
            return false;
        };
        let closed = crate::map::floor::BundleDoors(&map.facts);
        let Ok(expected) = runtime.read_actor_position(actor) else {
            return false;
        };
        // Donor getClosestFreePosition: exact tile first, then bounded nearest neighbours.
        let offsets: &[(i32, i32)] = if nearby {
            &[
                (0, 0),
                (0, -1),
                (1, -1),
                (1, 0),
                (1, 1),
                (0, 1),
                (-1, 1),
                (-1, 0),
                (-1, -1),
            ]
        } else {
            &[(0, 0)]
        };
        for &(dx, dy) in offsets {
            let (Some(x), Some(y)) = (to.x.checked_add(dx), to.y.checked_add(dy)) else {
                continue;
            };
            let at = crate::foundation::MovementLocalPosition {
                x,
                y,
                floor: to.floor,
            };
            if let Ok(proof) = crate::map::floor::prepare_quest_callback_teleport(
                runtime,
                self.movement_cells(),
                actor,
                session,
                expected,
                at,
                false,
                &closed,
            ) {
                return runtime.commit_source_step(proof).is_ok();
            }
        }
        false
    }
}
