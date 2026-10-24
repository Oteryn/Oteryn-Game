//! MINING-1 PostgreSQL qualification through the production USE dispatcher.
use crate::bestiary_postgres_harness as postgres_fixture;
use crate::content::item_ref::ItemDefinitionIndex;
use crate::foundation::{
    ChannelId, ChannelRuntimeV1, ExactActorRef, GameSessionId, NodeId, WorldId,
};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::connection::{
    AdmittedSession, FirstEntryOutcome, FreshAdmissionAuthority, SessionContinuity, UseCommand,
    UseOutcome,
};
use crate::gameplay_transport::item_view::ItemKey;
use oteryn_protocol_oteryn::world_object::{UseDisposition, UseIntent};
use postgres_fixture::{Harness, TestResult, configured_admin, debug, id, item_fence};
use std::time::Duration;
use tokio::sync::Mutex;

#[path = "mining/fixture.rs"]
mod fixture;
#[path = "mining/full_backpack.rs"]
mod full_backpack;
#[path = "mining/restart.rs"]
mod restart;
#[path = "mining/wire.rs"]
mod wire;

#[test]
fn mining_pick_refining_commits_and_replays_through_real_wire_on_postgres() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            postgres_fixture::runtime()
                .map_err(|e| e.to_string())?
                .block_on(fixture::scenario(admin))
                .map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "mining e2e panicked")?
        .map_err(Into::into)
}

struct WireOwner<'a, 'b, 'f, 's>(&'a ComposedFreshAdmission<'b, 'f, 's>);
impl FreshAdmissionAuthority for WireOwner<'_, '_, '_, '_> {
    async fn observe(
        &self,
        actor: ExactActorRef,
    ) -> Option<crate::gameplay_transport::world_spatial::WorldSpatialObservation> {
        self.0.observe(actor).await
    }
    async fn admit(
        &self,
        _: crate::gameplay_transport::connection::FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, crate::gameplay_transport::connection::AdmissionRefusal> {
        Err(crate::gameplay_transport::connection::AdmissionRefusal::Unavailable)
    }
    async fn observe_character_inventory(
        &self,
        _: ExactActorRef,
        _: GameSessionId,
    ) -> Option<crate::gameplay_transport::item_view::InventoryItems> {
        let bag = self
            .0
            .root
            .read_character_backpack(self.0.character, item_fence().ok()?.character_id)
            .await
            .ok()?;
        let runtime = self.0.runtime.lock().await;
        let index = crate::gameplay_transport::item_ref_admission::pinned_index(
            &runtime,
            self.0.active_generation,
        )?;
        crate::gameplay_transport::item_view::inventory_from_backpack(bag.as_ref(), |d| {
            crate::gameplay_transport::item_ref_admission::definition_ref(index, d)
        })
    }
    async fn observe_container(
        &self,
        actor: ExactActorRef,
        fence: Option<crate::durability::item_transfer::CurrentCharacterItemFence>,
        target: ItemKey,
    ) -> Option<crate::gameplay_transport::container_view::ContainerObservation> {
        self.0.observe_container(actor, fence, target).await
    }
    async fn use_pick(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: UseIntent,
        source: Option<ItemKey>,
        target: Option<ItemKey>,
    ) -> Option<UseOutcome> {
        self.0
            .use_pick(actor, command, intent, source, target)
            .await
    }
}

async fn run(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    harness: &Harness,
    actor: ExactActorRef,
    session: GameSessionId,
    index: &ItemDefinitionIndex,
) -> TestResult {
    use oteryn_protocol_oteryn::world_object::{
        MapUseTarget, UseTarget, encode_item_use_intent, encode_use_item_intent,
    };
    use std::num::NonZeroU64;
    let selected = crate::gameplay_transport::capabilities::SelectedCapabilities::select(
        crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,
        &[4, 6, 11, 12, 14, 15, 18],
    )
    .expect("bundle capabilities");
    assert!(selected.contains(11) && selected.contains(15));
    // This fixture observes inventory only; the real offer closure above is asserted
    // independently, while this wire does not request spatial/map domain snapshots.
    let offered = [
        crate::gameplay_transport::capabilities::OfferedCapability {
            id: 4,
            requires: &[],
        },
        crate::gameplay_transport::capabilities::OfferedCapability {
            id: 11,
            requires: &[15],
        },
        crate::gameplay_transport::capabilities::OfferedCapability {
            id: 14,
            requires: &[4],
        },
        crate::gameplay_transport::capabilities::OfferedCapability {
            id: 15,
            requires: &[4],
        },
    ];
    let admitted = AdmittedSession {
        game_session_id: session,
        world_id: owner.world_id,
        channel_id: owner.channel_id,
        runtime_actor: Some(actor),
        first_entry: FirstEntryOutcome::Positioned,
        controller: None,
        continuity: SessionContinuity {
            selected_capabilities:
                crate::gameplay_transport::capabilities::SelectedCapabilities::select(
                    &offered,
                    &[4, 11, 14, 15],
                )
                .expect("selected"),
            ..SessionContinuity::FRESH
        },
        item_fence: Some(item_fence()?),
    };
    let pick = index
        .definition_ref(
            "Item",
            "oteryn:item.tibia.i3456",
            index
                .revision("oteryn:item.tibia.i3456")
                .expect("pick revision"),
        )
        .expect("pick index");
    let intent = UseIntent {
        target: UseTarget::ItemByDefinition(pick),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Item(
            NonZeroU64::new(4).expect("stone handle"),
        )),
        use_with_position: None,
    };
    let bag_index = index
        .definition_ref(
            "Item",
            "oteryn:item.tibia.i2854",
            index
                .revision("oteryn:item.tibia.i2854")
                .expect("bag revision"),
        )
        .expect("bag index");
    let wrong_tool = UseIntent {
        target: UseTarget::ItemByDefinition(bag_index),
        ..intent.clone()
    };
    let wrong_target = UseIntent {
        use_with_map_item: Some(MapUseTarget::Item(NonZeroU64::new(5).expect("pick target"))),
        ..intent.clone()
    };
    // main=1, outer=2, inner=3, stone=4, pick=5 (newest entry first).
    let payloads = [
        encode_use_item_intent(NonZeroU64::new(2).expect("outer")),
        encode_use_item_intent(NonZeroU64::new(3).expect("inner")),
        encode_item_use_intent(&wrong_tool).map_err(debug)?,
        encode_item_use_intent(&wrong_target).map_err(debug)?,
        encode_item_use_intent(&intent).map_err(debug)?,
    ];
    full_backpack::set(harness, index, true).await?;
    let full = owner
        .use_pick(
            actor,
            UseCommand {
                game_session_id: session,
                command_id: 1,
                item_fence: Some(item_fence()?),
            },
            intent.clone(),
            Some(ItemKey::Instance(id(100))),
            Some(ItemKey::Instance(id(110))),
        )
        .await
        .expect("full refusal");
    assert_eq!(full.disposition, UseDisposition::Full);
    assert_eq!(harness.count("game_pick_refinements").await?, 0);
    full_backpack::set(harness, index, false).await?;
    crate::gameplay_transport::mining::test_faults::lose_next_ack();
    let (results, retained) = wire::exchange(owner, admitted, &payloads).await?;
    assert_eq!(
        results,
        [
            UseDisposition::Committed,
            UseDisposition::Committed,
            UseDisposition::NothingToUse,
            UseDisposition::NothingToUse
        ]
    );
    assert_eq!(retained.continuity.next_command_id, 5);
    assert_eq!(harness.count("game_pick_refinements").await?, 1);
    let (results, _) = wire::exchange(owner, retained, &payloads[4..]).await?;
    assert_eq!(results, vec![UseDisposition::Committed]);
    assert_eq!(harness.count("game_pick_refinements").await?, 1);
    assert_eq!(harness.count("game_item_use_records").await?, 1);
    let quantity: i64 = sqlx::query_scalar(
        "SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(id(110).as_slice())
    .fetch_one(&harness.pool)
    .await?;
    assert_eq!(quantity, 1);
    let tool_quantity: i64 = sqlx::query_scalar(
        "SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(id(100).as_slice())
    .fetch_one(&harness.pool)
    .await?;
    assert_eq!(tool_quantity, 1);
    // A committed receipt resolves even when the consumed target's view no longer exists.
    let replay = owner
        .use_pick(
            actor,
            UseCommand {
                game_session_id: session,
                command_id: 5,
                item_fence: Some(item_fence()?),
            },
            intent.clone(),
            None,
            None,
        )
        .await
        .expect("known replay");
    assert_eq!(replay.disposition, UseDisposition::Committed);
    assert_eq!(harness.count("game_pick_refinements").await?, 1);
    restart::reload(harness, session, &intent).await?;
    let mut conflict = intent.clone();
    conflict.use_with_map_item = Some(MapUseTarget::Item(NonZeroU64::new(5).expect("pick handle")));
    assert_eq!(
        owner
            .use_pick(
                actor,
                UseCommand {
                    game_session_id: session,
                    command_id: 5,
                    item_fence: Some(item_fence()?)
                },
                conflict,
                None,
                None
            )
            .await
            .expect("conflict")
            .disposition,
        UseDisposition::Rejected
    );
    let inventory = owner
        .root
        .read_item_use_inventory(owner.character, owner.holder, item_fence()?)
        .await
        .map_err(debug)?;
    assert_eq!(inventory.backpack.expect("backpack").entries.len(), 2);
    let mut stale = item_fence()?;
    stale.connection_generation = crate::foundation::ConnectionGeneration::new(2).map_err(debug)?;
    assert_eq!(
        owner
            .use_pick(
                actor,
                UseCommand {
                    game_session_id: session,
                    command_id: 6,
                    item_fence: Some(stale)
                },
                intent.clone(),
                Some(ItemKey::Instance(id(100))),
                Some(ItemKey::Instance(id(110)))
            )
            .await
            .expect("stale fence")
            .disposition,
        UseDisposition::Rejected
    );
    assert_eq!(harness.count("game_pick_refinements").await?, 1);
    let mut no_map_use = admitted;
    no_map_use.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            &offered,
            &[4, 14, 15],
        )
        .expect("without interaction");
    let (results, _) = wire::exchange(
        owner,
        no_map_use,
        &[encode_item_use_intent(&intent).map_err(debug)?],
    )
    .await?;
    assert_eq!(results, [UseDisposition::Rejected]);
    assert_eq!(harness.count("game_pick_refinements").await?, 1);
    // Two live commands compete for the final unit. The real Channel permit and
    // writer fences admit one conversion; the retired source cannot be refined.
    let attempt = |command_id| {
        owner.use_pick(
            actor,
            UseCommand {
                game_session_id: session,
                command_id,
                item_fence: Some(item_fence().expect("fence")),
            },
            intent.clone(),
            Some(ItemKey::Instance(id(100))),
            Some(ItemKey::Instance(id(110))),
        )
    };
    let mut first = std::pin::pin!(attempt(7));
    let mut second = std::pin::pin!(attempt(8));
    let mut a = None;
    let mut b = None;
    let (first, second) = std::future::poll_fn(|cx| {
        use std::future::Future;
        if a.is_none()
            && let std::task::Poll::Ready(value) = first.as_mut().poll(cx)
        {
            a = Some(value);
        }
        if b.is_none()
            && let std::task::Poll::Ready(value) = second.as_mut().poll(cx)
        {
            b = Some(value);
        }
        match (a.take(), b.take()) {
            (Some(first), Some(second)) => std::task::Poll::Ready((first, second)),
            (first, second) => {
                a = first;
                b = second;
                std::task::Poll::Pending
            }
        }
    })
    .await;
    let outcomes = [
        first.expect("first decided"),
        second.expect("second decided"),
    ];
    assert_eq!(
        outcomes
            .iter()
            .filter(|o| o.disposition == UseDisposition::Committed)
            .count(),
        1
    );
    assert_eq!(harness.count("game_pick_refinements").await?, 2);
    let final_state: (i64,i16) = sqlx::query_as("SELECT quantity,lifecycle FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(id(110).as_slice()).fetch_one(&harness.pool).await?;
    assert_eq!(final_state, (0, 2));
    assert_eq!(
        attempt(9).await.expect("depleted refusal").disposition,
        UseDisposition::StaleState
    );
    assert_eq!(harness.count("game_pick_refinements").await?, 2);
    Ok(())
}
