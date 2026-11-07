//! Literal Outburst source recipe through the shared native handoff owner.
use super::{ComposedFreshAdmission, connection::AdmittedSession};
use crate::content::bosses::activity_template::{
    PinnedActivitySourceRoom as Room, QualifiedActivityRoomTemplate,
};
use crate::durability::activity_handoff::{
    ActivityHandoffOwner, ActivityRefusal, boss_room::BossRoomDefinition,
};
use crate::foundation::MovementLocalPosition;

pub(super) fn is_outburst(key: &str) -> bool {
    matches!(key, "oteryn:boss.outburst" | "oteryn:creature.outburst")
}
fn source(key: &str) -> Option<Room> {
    is_outburst(key).then_some(Room::CallbackOutburst)
}
impl ComposedFreshAdmission<'_, '_, '_> {
    fn outburst_template(
        &self,
        key: &str,
    ) -> Result<QualifiedActivityRoomTemplate, ActivityRefusal> {
        let kind = source(key).ok_or(ActivityRefusal::ContentUnavailable)?;
        let active = self
            .active_generation
            .ok_or(ActivityRefusal::ContentUnavailable)?;
        let room = self
            .activity_runtimes
            .source_room(kind)
            .ok_or(ActivityRefusal::ContentUnavailable)?;
        let revision = active
            .native_gameplay()
            .and_then(|n| n.training_formula())
            .map(|f| f.content_revision())
            .ok_or(ActivityRefusal::ContentUnavailable)?;
        active
            .qualify_outburst_activity_template(room, revision)
            .map_err(|error| {
                let _ = &error;
                #[cfg(any(test, feature = "test-support"))]
                eprintln!("SRX3_OUTBURST_SOURCE_TEMPLATE_REFUSED {error:?}");
                ActivityRefusal::ContentUnavailable
            })
    }

    pub(super) async fn prepare_outburst_catalogue(
        &self,
        admitted: AdmittedSession,
        key: &str,
    ) -> Result<(), ActivityRefusal> {
        let template = self.outburst_template(key)?;
        let active = self
            .active_generation
            .ok_or(ActivityRefusal::ContentUnavailable)?;
        let origin = self
            .effective_rooms()
            .gate_field_step_ingress()
            .ok_or(ActivityRefusal::ContentUnavailable)?;
        // Both accepted paths authenticate the real current map, never raw XYZ.
        let runtime = self.runtime.lock().await;
        let qualified = origin.pinned_activity_source_room() == source(key)
            || active
                .qualify_published_world_context(origin, runtime.content_pin())
                .is_some();
        drop(runtime);
        if !qualified {
            return Err(ActivityRefusal::ContentUnavailable);
        }
        let fence = admitted
            .item_fence
            .ok_or(ActivityRefusal::AuthorityRejected)?;
        if !ActivityHandoffOwner::new(self.root.clone())
            .activate_outburst_boss_catalogue(self.character, self.holder, fence, active, &template)
            .await
            .map_err(|_| ActivityRefusal::AuthorityRejected)?
        {
            return Err(ActivityRefusal::ContentChanged);
        }
        Ok(())
    }
    pub(super) async fn ensure_outburst_definition_runtime(
        &self,
        definition: &BossRoomDefinition,
    ) -> Result<(), ActivityRefusal> {
        let template = self.outburst_template(&definition.boss_key)?;
        let cell = |p: crate::content::LogicalCell| {
            crate::durability::character_last_position::LastPositionCell {
                x: p.x,
                y: p.y,
                floor: p.z as i16,
            }
        };
        if definition.boss_key != template.boss_key()
            || definition.boss_creature_key != template.creature_key()
            || definition.content_revision != template.content_revision()
            || definition.room_template_key != template.template_key()
            || definition.template_revision != template.template_revision()
            || definition.policy.minimum_players != 1
            || definition.policy.maximum_players != 5
            || definition.policy.minimum_level != 1
            || definition.policy.cooldown_ms != u64::from(template.cooldown_seconds()) * 1000
            || definition.policy.time_limit_ms != 1_200_000
            || definition.after_kill_ms != 60_000
            || definition.lever_positions
                != template
                    .lever_positions()
                    .iter()
                    .copied()
                    .map(cell)
                    .collect::<Vec<_>>()
            || definition.entry_positions
                != template
                    .entry_positions()
                    .iter()
                    .copied()
                    .map(cell)
                    .collect::<Vec<_>>()
            || definition.boss_position != cell(template.boss_position())
            || definition.exit_position != cell(template.exit_position())
        {
            return Err(ActivityRefusal::ContentChanged);
        }
        let boss = definition.boss_position;
        self.ensure_pinned_source_runtime(
            source(&definition.boss_key).ok_or(ActivityRefusal::ContentUnavailable)?,
            Some((
                template.creature_key(),
                MovementLocalPosition {
                    x: boss.x,
                    y: boss.y,
                    floor: boss.floor,
                },
            )),
            1024,
        )
        .await?;
        Ok(())
    }
}
