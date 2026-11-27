use super::{Harness, TestResult, debug};
use crate::foundation::ExactActorRef;
use crate::gameplay_transport::{
    ComposedFreshAdmission, item_ref_admission,
    item_view::{ItemKey, SessionItemView},
    world_spatial::{SessionVisibility, VisibilityUpdate},
};
use oteryn_gameplay_domain::domain::loot_data::LootDefinitionRef;
use oteryn_protocol_oteryn::{
    world_spatial::{ActorPosition, WorldSpatialError},
    world_spatial_entities::{EntityDetail, EntityKind, EntityRef},
};

pub(super) struct Observer {
    actor: ExactActorRef,
    target: ExactActorRef,
    position: ActorPosition,
    view: SessionVisibility,
    items: SessionItemView,
}

impl Observer {
    pub(super) async fn new(
        owner: &ComposedFreshAdmission<'_, '_, '_>,
        actor: ExactActorRef,
    ) -> TestResult<Self> {
        let target = owner
            .runtime
            .lock()
            .await
            .visible_entities()
            .creatures
            .first()
            .ok_or("live Goblin")?
            .actor;
        let scene = owner
            .observe_channel_entities(actor)
            .await
            .ok_or("initial Goblin scene")?;
        let mut view = SessionVisibility::with_objects(true);
        let mut items = SessionItemView::resume(Default::default());
        let snapshot = view
            .snapshot(&scene, &mut |entities| {
                items
                    .attach_spatial_handles(entities)
                    .map_err(|_| WorldSpatialError::Malformed)
            })
            .map_err(debug)?;
        let live = snapshot
            .entities
            .iter()
            .find(|entry| {
                entry.kind == EntityKind::Creature && entry.entity == creature_ref(target)
            })
            .ok_or("observer snapshot must contain the living Goblin")?;
        Ok(Self {
            actor,
            target,
            position: live.position,
            view,
            items,
        })
    }

    async fn refresh(
        &mut self,
        owner: &ComposedFreshAdmission<'_, '_, '_>,
    ) -> TestResult<VisibilityUpdate> {
        let scene = owner
            .observe_channel_entities(self.actor)
            .await
            .ok_or("observer scene")?;
        Ok(self
            .view
            .refresh(&scene, &mut |entities| {
                self.items
                    .attach_spatial_handles(entities)
                    .map_err(|_| WorldSpatialError::Malformed)
            })
            .map_err(debug)?)
    }

    pub(super) async fn assert_pending(
        &mut self,
        owner: &ComposedFreshAdmission<'_, '_, '_>,
    ) -> TestResult {
        let identity = self.target.corpse_identity();
        assert!(
            !owner
                .runtime
                .lock()
                .await
                .contains_live_creature(self.target)
        );
        assert!(
            !matches!(self.refresh(owner).await?, VisibilityUpdate::Delta(delta)
            if delta.leave.contains(&creature_ref(self.target))
                || delta.enter.iter().any(|entry| entry.entity.identity == identity)),
            "the observer must retain the Goblin until its corpse commits"
        );
        Ok(())
    }

    pub(super) async fn assert_committed(
        &mut self,
        owner: &ComposedFreshAdmission<'_, '_, '_>,
        harness: &Harness,
        body: &LootDefinitionRef,
    ) -> TestResult {
        let identity = self.target.corpse_identity();
        let VisibilityUpdate::Delta(delta) = self.refresh(owner).await? else {
            return Err("expected a Goblin death delta".into());
        };
        assert!(delta.leave.contains(&creature_ref(self.target)));
        let corpse = delta
            .enter
            .iter()
            .find(|entry| {
                entry.kind == EntityKind::Corpse
                    && entry.entity
                        == EntityRef {
                            identity,
                            generation: 0,
                        }
            })
            .ok_or("exact Goblin corpse must enter in the same delta as the Goblin leaves")?;
        assert_eq!(corpse.position, self.position);
        let (EntityDetail::Object {
            item_definition_ref,
            item_handle: Some(handle),
            ..
        }
        | EntityDetail::ObjectWithAppearance {
            item_definition_ref,
            item_handle: Some(handle),
            ..
        }) = corpse.detail
        else {
            return Err("Goblin corpse must carry a usable item handle".into());
        };
        assert_eq!(self.items.resolve(handle), Some(ItemKey::Entity(identity)));

        let (body_ref, binding) = {
            let runtime = owner.runtime.lock().await;
            let index = item_ref_admission::pinned_index(&runtime, owner.active_generation)
                .ok_or("pinned item index")?;
            let body_ref = index
                .definition_ref(&body.family, &body.production_key, &body.revision_ref)
                .ok_or("Goblin corpse definition reference")?;
            let binding = runtime
                .bound_corpse(identity)
                .ok_or("Goblin corpse binding")?;
            (body_ref, binding)
        };
        assert_eq!(item_definition_ref, body_ref.get());
        let committed: (Vec<u8>, String) = sqlx::query_as(
            "SELECT uuid_send(r.item_instance_id), i.definition_production_key \
             FROM game_item_mint_receipts r JOIN game_item_instances i USING (item_instance_id) \
             WHERE r.loot_purpose_key = 'CORPSE_MATERIALIZATION'",
        )
        .fetch_one(&harness.pool)
        .await?;
        assert_eq!(committed.0.as_slice(), binding.item.item_instance_id);
        assert_eq!(committed.1, body.production_key);
        Ok(())
    }
}

fn creature_ref(actor: ExactActorRef) -> EntityRef {
    EntityRef {
        identity: actor.placement_identity(),
        generation: actor.actor_local_generation(),
    }
}
