use oteryn_gameplay_entities::carrier::{ActorState, LocalPosition};
use oteryn_gameplay_entities::*;
#[allow(clippy::expect_used)]
pub trait CrystalRouterFixtureExt {
    #[allow(clippy::too_many_arguments)]
    fn crystal_router_fixture_actor(
        &mut self,
        key: &str,
        health: i64,
        at: MovementLocalPosition,
    ) -> ExactActorRef;
    #[allow(clippy::too_many_arguments)]
    fn crystal_router_fixture_project_death(
        &mut self,
        actor: ExactActorRef,
        key: &str,
        health: i64,
    );
    #[allow(clippy::too_many_arguments)]
    fn crystal_router_fixture_health(&self, actor: ExactActorRef) -> i64;
    #[allow(clippy::too_many_arguments)]
    fn crystal_router_fixture_commit_lethal(
        &mut self,
        actor: ExactActorRef,
        key: &str,
        health: i64,
    );
}
// Append in physical Foundation carrier. Pure test-only owner construction; no production grants.
#[cfg(any(test, feature = "test-support"))]
#[allow(
    clippy::expect_used,
    reason = "test-only native fixture assertions retain descriptive failures"
)]
pub fn crystal_death_router_fixture(world: WorldId) -> ChannelRuntimeV1 {
    crystal_death_router_fixture_digest(world, [1; 32])
}
#[cfg(any(test, feature = "test-support"))]
pub fn crystal_death_router_fixture_with_artifact(
    world: WorldId,
    digest: [u8; 32],
) -> ChannelRuntimeV1 {
    crystal_death_router_fixture_digest(world, digest)
}
#[cfg(any(test, feature = "test-support"))]
#[allow(clippy::expect_used)]
fn crystal_death_router_fixture_digest(world: WorldId, digest: [u8; 32]) -> ChannelRuntimeV1 {
    let mut channel = [0u8; 16];
    channel[6] = 0x70;
    channel[8] = 0x80;
    channel[15] = 92;
    let mut node = channel;
    node[15] = 93;
    ChannelRuntimeV1::from_committed_assignment(
        world,
        ChannelId::decode(&channel).expect("valid native fixture operation must succeed"),
        NodeId::decode(&node).expect("valid native fixture operation must succeed"),
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        16,
        ChannelContentPin::from_activation(
            world,
            1,
            digest,
            [2; 32],
            [3; 32],
            [4; 32],
            (100, 100, 7),
        ),
    )
    .expect("valid native fixture operation must succeed")
}
#[cfg(any(test, feature = "test-support"))]
#[allow(
    clippy::expect_used,
    reason = "test-only native fixture assertions retain descriptive failures"
)]
impl CrystalRouterFixtureExt for ChannelRuntimeV1 {
    fn crystal_router_fixture_actor(
        &mut self,
        key: &str,
        health: i64,
        at: MovementLocalPosition,
    ) -> ExactActorRef {
        ExactActorRef(
            self.admit_encounter_fixture_actor(
                ActorState(0),
                key,
                health,
                Some(LocalPosition {
                    x: at.x,
                    y: at.y,
                    floor: at.floor,
                }),
            )
            .expect("valid native fixture operation must succeed"),
        )
    }
    fn crystal_router_fixture_project_death(
        &mut self,
        actor: ExactActorRef,
        key: &str,
        health: i64,
    ) {
        self.borrow_exact_actor_commit()
            .commit_damage(
                actor,
                OwnerDamageCommand {
                    target: key.as_bytes(),
                    occurrence: b"router-fixture-death",
                    binding: b"router-fixture-death\0fixture",
                    damage: health,
                },
            )
            .expect("valid native fixture operation must succeed");
        let mut owner = self.borrow_combat_death();
        let receipt = owner
            .committed_lethal_receipt(actor)
            .expect("valid native fixture operation must succeed");
        owner
            .project_committed_lethal(receipt)
            .expect("valid native fixture operation must succeed");
    }
    fn crystal_router_fixture_health(&self, actor: ExactActorRef) -> i64 {
        self.read_encounter_fixture_health(actor)
    }
    fn crystal_router_fixture_commit_lethal(
        &mut self,
        actor: ExactActorRef,
        key: &str,
        health: i64,
    ) {
        self.borrow_exact_actor_commit()
            .commit_damage(
                actor,
                OwnerDamageCommand {
                    target: key.as_bytes(),
                    occurrence: b"router-hook-fixture-death",
                    binding: b"router-hook-fixture-death\0fixture",
                    damage: health,
                },
            )
            .expect("valid native fixture operation must succeed");
    }
}
