//! MOUNTTAME PostgreSQL qualification through serve_admitted and the real composed owner.
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

#[path = "mount_taming/fixture.rs"]
mod fixture;
#[path = "mount_taming/outcomes.rs"]
mod outcomes;
#[path = "mount_taming/pairs.rs"]
mod pairs;
#[path = "mount_taming/wire.rs"]
mod wire;

#[test]
fn mount_taming_postgres_real_dispatch_outcomes_refusals_replay_and_reload() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            postgres_fixture::runtime()
                .map_err(|e| e.to_string())?
                .block_on(fixture::scenario(admin, Cases::Outcomes))
                .map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "mount taming e2e panicked")?
        .map_err(Into::into)
}

struct WireOwner<'a, 'b, 'f, 's>(&'a ComposedFreshAdmission<'b, 'f, 's>);
impl FreshAdmissionAuthority for WireOwner<'_, '_, '_, '_> {
    async fn observe_visible_entities(
        &self,
        actor: ExactActorRef,
    ) -> Option<crate::gameplay_transport::world_spatial::ChannelEntities> {
        self.0.observe_visible_entities(actor).await
    }
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
    async fn tame_mount(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: UseIntent,
        source: Option<ItemKey>,
        visible: bool,
    ) -> Result<Option<UseOutcome>, ()> {
        self.0
            .tame_mount(actor, command, intent, source, visible)
            .await
    }
}

fn cosmetics() -> TestResult<Vec<u8>> {
    let mut packet =
        crate::content::native_gameplay::test_support::cosmetics_fixture::cosmetics_test_packet();
    let author = &mut packet["authoring"];
    author["document"]["content_revision"] = "content-1".into();
    let template = author["document"]["mounts"][0].clone();
    let mounts: std::collections::BTreeSet<_> =
        crate::content::native_gameplay::mount_taming::rules()
            .iter()
            .map(|r| &r.mount_key)
            .collect();
    let mount_rows: Vec<_> = mounts
        .iter()
        .map(|key| {
            let mut row = template.clone();
            row["key"] = (**key).clone().into();
            row
        })
        .collect();
    author["document"]["mounts"] = mount_rows.into();
    let mappings: Vec<_> = mounts.iter().map(|key| {
        let rule = crate::content::native_gameplay::mount_taming::rules().iter().find(|r| r.mount_key==**key).expect("rule");
        serde_json::json!({"grant_key":rule.grant_key,"target":{"kind":"Mount","mount_key":rule.mount_key},"compatible_provenance":[crate::content::native_gameplay::mount_taming::PROVENANCE]})
    }).collect();
    author["document"]["grant_mappings"] = mappings.into();
    let mut template_promotion = author["promotions"][0].clone();
    let mount_promotions: Vec<_> = author["promotions"]
        .as_array()
        .ok_or("promotions")?
        .iter()
        .filter(|p| {
            p["path"]
                .as_str()
                .is_some_and(|s| s.starts_with("/mounts/0/"))
        })
        .cloned()
        .collect();
    author["promotions"]
        .as_array_mut()
        .ok_or("promotions")?
        .retain(|p| {
            !p["path"]
                .as_str()
                .is_some_and(|s| s.starts_with("/mounts/") || s == "/grant_mappings")
        });
    for (i, mount) in author["document"]["mounts"]
        .as_array()
        .ok_or("mounts")?
        .clone()
        .iter()
        .enumerate()
    {
        for original in &mount_promotions {
            let field = original["path"]
                .as_str()
                .ok_or("path")?
                .rsplit('/')
                .next()
                .ok_or("field")?;
            let mut promotion = original.clone();
            promotion["path"] = format!("/mounts/{i}/{field}").into();
            promotion["value"] = mount[field].clone();
            author["promotions"]
                .as_array_mut()
                .ok_or("promotions")?
                .push(promotion);
        }
    }
    template_promotion["path"] = "/grant_mappings".into();
    template_promotion["value"] = author["document"]["grant_mappings"].clone();
    author["promotions"]
        .as_array_mut()
        .ok_or("promotions")?
        .push(template_promotion);
    for promotion in author["promotions"].as_array_mut().ok_or("promotions")? {
        if promotion["path"] == "/content_revision" {
            promotion["value"] = "content-1".into();
        }
    }
    Ok(serde_json::to_vec(&packet)?)
}

async fn runtime_role(root: &crate::durability::DurabilityRoot) -> TestResult {
    let mut connection = root.try_acquire_ready()?;
    sqlx::query("SET ROLE oteryn_game_runtime")
        .execute(&mut *connection)
        .await?;
    assert!(matches!(
        connection
            .return_to_pool_observed_until(std::time::Instant::now() + Duration::from_secs(5))
            .await,
        sqlx::pool::PoolConnectionReturnDisposition::ReturnedToIdle
    ));
    Ok(())
}

fn admitted(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
) -> TestResult<AdmittedSession> {
    use crate::gameplay_transport::capabilities::{OfferedCapability, SelectedCapabilities};
    let offered = [
        OfferedCapability {
            id: 6,
            requires: &[],
        },
        OfferedCapability {
            id: 4,
            requires: &[],
        },
        OfferedCapability {
            id: 14,
            requires: &[4],
        },
        OfferedCapability {
            id: 15,
            requires: &[4],
        },
    ];
    Ok(AdmittedSession {
        game_session_id: session,
        world_id: owner.world_id,
        channel_id: owner.channel_id,
        runtime_actor: Some(actor),
        first_entry: FirstEntryOutcome::Positioned,
        controller: None,
        continuity: SessionContinuity {
            selected_capabilities: SelectedCapabilities::select(&offered, &[4, 6, 14, 15])
                .ok_or("caps")?,
            ..SessionContinuity::FRESH
        },
        item_fence: Some(item_fence()?),
    })
}

async fn spawn(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    key: &str,
    dx: i32,
) -> TestResult<(
    ExactActorRef,
    oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
)> {
    let mut runtime = owner.runtime.lock().await;
    let from = runtime
        .read_actor_position(actor)
        .map_err(debug)?
        .position();
    let target = runtime
        .admit_source_pinned_lab_creature(
            crate::foundation::MovementLocalPosition {
                x: from.x + dx,
                y: from.y,
                floor: from.floor,
            },
            key,
            100,
        )
        .map_err(debug)?;
    let visible = runtime.visible_entities();
    let v = visible
        .creatures
        .iter()
        .find(|c| c.actor == target)
        .ok_or("visible")?;
    Ok((
        target,
        oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
            identity: target.placement_identity(),
            generation: v.generation,
        },
    ))
}

async fn quantity(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(id(100).as_slice())
    .fetch_one(&h.pool)
    .await?)
}

#[derive(Clone, Copy)]
enum Cases {
    Outcomes,
    Items,
    MusicBox,
}

#[test]
fn mount_taming_postgres_all_authored_item_pairs() -> TestResult {
    run_pairs(Cases::Items)
}

#[test]
fn mount_taming_postgres_music_box_guaranteed_pairs() -> TestResult {
    run_pairs(Cases::MusicBox)
}

fn run_pairs(cases: Cases) -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            postgres_fixture::runtime()
                .map_err(|e| e.to_string())?
                .block_on(fixture::scenario(admin, cases))
                .map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "mount taming e2e panicked")?
        .map_err(Into::into)
}
