//! Source-qualified Primal lever admission through the shared W02 handoff owner.
use super::{ComposedFreshAdmission, connection::AdmittedSession};
use crate::content::bosses::activity_template::{
    PinnedActivitySourceRoom, QualifiedActivityRoomTemplate,
};
use crate::durability::activity_handoff::boss_room::BossRoomDefinition;
use crate::durability::activity_handoff::{ActivityHandoffOwner, ActivityRefusal};
use crate::foundation::MovementLocalPosition;

pub(super) const BOSS: &str = "oteryn:boss.the_primal_menace";
const CREATURE: &str = "oteryn:creature.the_primal_menace";
pub(super) fn is_primal(key: &str) -> bool {
    matches!(key, BOSS | CREATURE)
}

impl ComposedFreshAdmission<'_, '_, '_> {
    fn primal_template(&self) -> Result<QualifiedActivityRoomTemplate, ActivityRefusal> {
        let active = self
            .active_generation
            .ok_or(ActivityRefusal::ContentUnavailable)
            .inspect_err(|_| {
                #[cfg(test)]
                eprintln!("PR3056_PRIMAL_ENTRY stage=active_generation refusal=ContentUnavailable");
            })?;
        let room = self
            .activity_runtimes
            .source_room(PinnedActivitySourceRoom::Primal)
            .ok_or(ActivityRefusal::ContentUnavailable)
            .inspect_err(|_| {
                #[cfg(test)]
                eprintln!(
                    "PR3056_PRIMAL_ENTRY stage=registered_source_room refusal=ContentUnavailable"
                );
            })?;
        let revision = active
            .native_gameplay()
            .and_then(|native| native.training_formula())
            .map(|formula| formula.content_revision())
            .ok_or(ActivityRefusal::ContentUnavailable)
            .inspect_err(|_| {
                #[cfg(test)]
                eprintln!("PR3056_PRIMAL_ENTRY stage=training_revision refusal=ContentUnavailable");
            })?;
        active
            .qualify_primal_activity_template(room, revision)
            .map_err(|_error| {
                #[cfg(test)]
                {
                    let native = active.native_gameplay();
                    eprintln!(
                        "PR3056_PRIMAL_ENTRY stage=qualified_template refusal={_error:?} closure={} callbacks={} pod_present={} corpse_present={}",
                        native.is_some_and(|native| crate::content::bosses::primal_closure::qualify(native).is_some()),
                        native.is_some_and(|native| native.qualifies_primal_callbacks(active.identity().server_artifact_digest())),
                        native.is_some_and(|native| native.item_policy("oteryn:item.tibia.i39176", "definition-r1").is_some()),
                        native.is_some_and(|native| native.reward_table().row(CREATURE).ok().is_some_and(|reward| reward.corpse_item.is_some())),
                    );
                }
                ActivityRefusal::ContentUnavailable
            })
    }
    pub(super) async fn prepare_primal_catalogue(
        &self,
        admitted: AdmittedSession,
    ) -> Result<(), ActivityRefusal> {
        let template = self.primal_template()?;
        let active = self
            .active_generation
            .ok_or(ActivityRefusal::ContentUnavailable)?;
        let origin = self
            .effective_rooms()
            .gate_field_step_ingress()
            .ok_or(ActivityRefusal::ContentUnavailable)
            .inspect_err(|_| {
                #[cfg(test)]
                eprintln!("PR3056_PRIMAL_ENTRY stage=source_lobby_room refusal=ContentUnavailable");
            })?;
        if !active.qualifies_primal_source_lobby(origin) {
            #[cfg(test)]
            eprintln!(
                "PR3056_PRIMAL_ENTRY stage=source_lobby_qualification refusal=ContentUnavailable"
            );
            return Err(ActivityRefusal::ContentUnavailable);
        }
        let fence = admitted
            .item_fence
            .ok_or(ActivityRefusal::AuthorityRejected)?;
        if !ActivityHandoffOwner::new(self.root.clone())
            .activate_primal_boss_catalogue(self.character, self.holder, fence, active, &template)
            .await
            .map_err(|_error| {
                #[cfg(test)]
                eprintln!("PR3056_PRIMAL_ENTRY stage=catalogue_activation refusal={_error:?}");
                ActivityRefusal::AuthorityRejected
            })?
        {
            #[cfg(test)]
            eprintln!("PR3056_PRIMAL_ENTRY stage=catalogue_activation refusal=ContentChanged");
            return Err(ActivityRefusal::ContentChanged);
        }
        #[cfg(test)]
        eprintln!("PR3056_PRIMAL_ENTRY stage=catalogue_activation accepted");
        Ok(())
    }
    pub(super) async fn ensure_primal_definition_runtime(
        &self,
        definition: &BossRoomDefinition,
    ) -> Result<(), ActivityRefusal> {
        let template = self.primal_template()?;
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
            || definition.policy.minimum_level != 500
            || definition.policy.cooldown_ms != 72_000_000
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
        // At12hazard and maximum missingHP, the source's minimum interval is
        // >20s. Twenty minutes admits at most60 waves of6beasts+4Pods. A
        // generous768 fixed slots includes those surviving Fungosaurus and5players.
        self.ensure_pinned_source_runtime(
            PinnedActivitySourceRoom::Primal,
            Some((
                CREATURE,
                MovementLocalPosition {
                    x: boss.x,
                    y: boss.y,
                    floor: boss.floor,
                },
            )),
            768,
        )
        .await
        .inspect_err(|_error| {
            #[cfg(test)]
            eprintln!("PR3056_PRIMAL_ENTRY stage=pinned_runtime refusal={_error:?}");
        })?;
        #[cfg(test)]
        eprintln!("PR3056_PRIMAL_ENTRY stage=pinned_runtime accepted");
        Ok(())
    }
}
