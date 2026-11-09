//! Shared SRX-3 PG preparation using actual pinned maps and existing Character
//! owners. No synthetic boss profile, Instance assignment or handoff is issued.
use super::fixture::{Harness, debug};
use super::{Dependencies, TestResult};
use crate::content::bosses::activity_template::PinnedActivitySourceRoom;
use crate::content::{ContentActivationController, QualifiedNativeEntryRoom};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::{ChannelContentPin, CharacterLease, MovementLocalPosition};
use crate::gameplay_transport::connection::{AdmittedSession, FirstEntryOutcome};
use crate::gameplay_transport::{ComposedFreshAdmission, SeamRooms, item_fence_of};

pub(super) struct SourceScene {
    pub(super) controller: ContentActivationController,
    pub(super) room: QualifiedNativeEntryRoom,
    pub(super) pin: ChannelContentPin,
}
impl SourceScene {
    pub(super) fn new(
        source: PinnedActivitySourceRoom,
        origin: MovementLocalPosition,
    ) -> TestResult<Self> {
        let world = super::fixture::fence(1)?.runtime_scope.world_id();
        let channel = super::fixture::fence(1)?
            .runtime_scope
            .channel_id()
            .ok_or("fixture Channel")?;
        // Only the Character content label differs for this isolated existing
        // fixture. Every Creature/Behavior/Ability source body remains unchanged.
        let (controller, baseline) =
            super::death_ml_loss_composed::production_generation_with_pin(world, "content-1")?;
        let active = controller.active().ok_or("active source content")?;
        let source_room = active
            .qualify_activity_source_room(baseline.qualified_room(), source)
            .map_err(debug)?;
        let room = crate::gameplay_transport::activity_runtime::source_bundle::qualify(
            &source_room,
            source,
            active,
            world,
            channel,
        )
        .map_err(|_| "actual source bundle")?;
        assert_eq!(room.pinned_activity_source_room(), Some(source));
        assert_eq!(
            room.movement_cells().index().lookup(
                room.movement_cells().scope(),
                crate::content::LogicalCell {
                    x: origin.x,
                    y: origin.y,
                    z: i32::from(origin.floor)
                }
            ),
            Ok(crate::content::CollisionClass::Walkable)
        );
        let pin = ChannelContentPin::from_activation(
            world,
            active.activation_sequence(),
            active.identity().server_artifact_digest(),
            active.identity().client_artifact_digest(),
            room.frame_binding().digest(),
            room.map_revision_digest(),
            (origin.x, origin.y, origin.floor),
        );
        Ok(Self {
            controller,
            room,
            pin,
        })
    }
    pub(super) fn dependencies(&self) -> TestResult<Dependencies> {
        Dependencies::with_content(
            self.pin.clone(),
            self.room.clone(),
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )
    }
}

pub(super) async fn prepare_player(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    level: u32,
) -> TestResult {
    prepare_player_vocation(h, owner, admitted, level, "knight").await
}

pub(super) async fn prepare_player_vocation(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    level: u32,
    vocation: &str,
) -> TestResult {
    let fresh = super::fresh_fixture::fresh::Source::new(0)?.request()?;
    prepare_player_with_operation(h, owner, admitted, level, vocation, fresh.operation()).await
}

pub(super) async fn prepare_player_with_operation(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    level: u32,
    vocation: &str,
    operation: &crate::foundation::fresh_admission_durability::FreshAdmissionOperationV1,
) -> TestResult {
    use crate::durability::character_build::{
        BuildCause, BuildChangeRequest, BuildOccurrence, convert_vocation,
    };
    use crate::durability::character_progression::{
        ExperienceAwardRequest, ExperienceRewardOccurrence,
    };
    use crate::gameplay_transport::spell_character_facts::{self, CastFactsLoad};
    admitted.first_entry = FirstEntryOutcome::Positioned;
    admitted.item_fence = item_fence_of(
        FreshAdmissionStore::from_root(h.root.clone())
            .current_session_at(admitted.game_session_id)
            .await?
            .0,
    );
    let actor = admitted.runtime_actor.ok_or("fixture player actor")?;
    let fence = admitted.item_fence.ok_or("fixture player fence")?;
    {
        let mut runtime = owner.runtime.lock().await;
        runtime
            .bind_attacker_lease(
                actor,
                admitted.game_session_id,
                CharacterLease::new(
                    crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                        .map_err(debug)?,
                    fence.character_lease_generation,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        runtime
            .initialize_first_entry_position(actor)
            .map_err(debug)?;
    }
    Box::pin(owner.admit_progression_session(admitted)).await;
    let native = owner
        .active_generation
        .and_then(|active| active.native_gameplay())
        .ok_or("actual native gameplay")?;
    // This existing-character fixture skips the transport's first-entry
    // bootstrap. Initialize only its declared empty equipment state through
    // the real committed FreshAdmission owner; infer or mint no starting gear.
    h.root
        .initialize_admission_equipment(
            owner.character,
            owner.holder,
            &fence,
            native.source_digest(),
            operation,
        )
        .await
        .map_err(debug)?;
    let formula = native.training_formula().ok_or("actual training formula")?;
    let before = h
        .root
        .read_character_build_state(owner.character, fence.character_id)
        .await
        .map_err(debug)?;
    let after = convert_vocation(formula, &before, vocation).map_err(debug)?;
    let quest = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "Character fence")?
        .ok_or("current Character")?;
    // Receipt occurrence identities are global, including independently
    // admitted Party members prepared by this same fixture.
    let mut build_occurrence = *fence.character_id.as_bytes();
    build_occurrence[14] = 211;
    h.root
        .commit_character_build(
            owner.character,
            owner.holder,
            quest,
            BuildChangeRequest {
                occurrence: BuildOccurrence::from_bytes(build_occurrence).map_err(debug)?,
                cause: BuildCause::VocationChoice,
                before,
                after,
                pruned_stance: None,
            },
            formula,
        )
        .await
        .map_err(debug)?;
    let progression = owner
        .progression_binding(admitted.game_session_id)
        .ok_or("native progression binding")?;
    let threshold = progression
        .policy
        .thresholds
        .iter()
        .find(|threshold| threshold.level == level)
        .ok_or("source-supported test level")?
        .minimum_experience;
    let current = h
        .root
        .read_character_progression(owner.character, fence.character_id)
        .await
        .map_err(debug)?
        .ok_or("owned progression")?;
    let quest = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "Character XP fence")?
        .ok_or("current Character")?;
    let mut experience_occurrence = *fence.character_id.as_bytes();
    experience_occurrence[14] = 212;
    h.root
        .commit_character_experience(
            owner.character,
            owner.holder,
            quest,
            ExperienceAwardRequest {
                occurrence: ExperienceRewardOccurrence::from_bytes(experience_occurrence)
                    .map_err(debug)?,
                amount: oteryn_simulation_determinism::ExactI64::new(
                    threshold.get() - current.total_experience.get(),
                ),
                context: progression.context.clone(),
                policy_revision: progression.policy_revision.clone(),
                reward_revision: progression.reward_revision.clone(),
                policy: progression.policy.clone(),
            },
        )
        .await
        .map_err(debug)?;
    admitted.item_fence = item_fence_of(
        FreshAdmissionStore::from_root(h.root.clone())
            .current_session_at(admitted.game_session_id)
            .await?
            .0,
    );
    let facts = match spell_character_facts::load_character_cast_facts(
        &h.root,
        owner.character,
        fence.character_id,
    )
    .await
    {
        CastFactsLoad::Ready { facts, .. } => facts,
        _ => return Err("classed existing Character facts".into()),
    };
    assert_eq!(facts.level, level);
    let monk = owner
        .load_monk_state(fence.character_id)
        .await
        .ok_or("owned monk defaults")?;
    {
        let runtime = owner.runtime.lock().await;
        owner
            .states
            .spell_states
            .lock()
            .await
            .initialize_admitted(
                &runtime,
                actor,
                admitted.game_session_id,
                facts,
                monk,
                owner.owner_now(),
                None,
                None,
            )
            .ok_or("native Player admission")?;
    }
    owner
        .register_current_runtime()
        .await
        .map_err(|_| "register actual source runtime")?;
    owner
        .activity_runtimes
        .remember(*admitted)
        .map_err(|_| "remember actual source connection")?;
    assert!(owner.admit_hazard(*admitted).await);
    Ok(())
}

pub(super) fn install_scene<'a, 'f, 's>(
    owner: &mut ComposedFreshAdmission<'a, 'f, 's>,
    scene: &'a SourceScene,
    source: PinnedActivitySourceRoom,
) -> TestResult {
    owner.active_generation = scene.controller.active();
    owner.rooms = SeamRooms::new(Some(&scene.room), Some(&scene.room));
    owner
        .activity_runtimes
        .register_source_room(source, &scene.room)
        .map_err(|_| "register pinned source room")?;
    Ok(())
}
