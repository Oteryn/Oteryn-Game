//! Actual source death qualification; corrected rewardroom/knife caller is separate.
use super::{
    TestResult,
    fixture::{Harness, id},
};
use crate::gameplay_transport::{
    ComposedFreshAdmission,
    connection::{AdmittedSession, FreshAdmissionAuthority},
};
use std::cell::Cell;
thread_local! {static ACTIVE:Cell<bool>=const{Cell::new(false)};static PORTAL:Cell<bool>=const{Cell::new(false)};static CROSS_CHANNEL:Cell<bool>=const{Cell::new(false)};}
struct Guard;
impl Drop for Guard {
    fn drop(&mut self) {
        ACTIVE.set(false);
        PORTAL.set(false);
        CROSS_CHANNEL.set(false);
    }
}
pub(super) fn portal_active() -> bool {
    PORTAL.get()
}
#[test]
fn srv7_pumpkin_real_death_cannot_open_independent_current_channel_b() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("srv7-pumpkin-cross-channel", || {
        ACTIVE.set(true);
        CROSS_CHANNEL.set(true);
        let _guard = Guard;
        super::kill_reward_e2e::scenario()
    })
}
#[test]
fn srv7_pumpkin_actual_fight_portal_visit_and_static_reward_body_use() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("srv7-pumpkin-room", || {
        ACTIVE.set(true);
        PORTAL.set(true);
        let _guard = Guard;
        super::kill_reward_e2e::scenario()
    })
}
pub(super) fn active() -> bool {
    ACTIVE.get()
}
#[test]
fn srv7_pumpkin_actual_native_death_is_8133_and_records_fight_participants() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("srv7-pumpkin-death-source", || {
        ACTIVE.set(true);
        let _guard = Guard;
        super::kill_reward_e2e::scenario()
    })
}
pub(super) async fn qualify(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    mut admitted: AdmittedSession,
) -> TestResult {
    let corpse = owner
        .runtime
        .lock()
        .await
        .visible_corpses()
        .into_iter()
        .find(|c| c.item.production_key == "oteryn:item.tibia.i8133")
        .ok_or("actual native MutatedPumpkin corpse8133")?;
    let source = corpse.item.item_instance_id;
    let source_mint:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_mint_receipts WHERE item_instance_id=encode($1,'hex')::uuid AND loot_purpose_key='CORPSE_MATERIALIZATION' AND loot_table_family='Item' AND loot_table_production_key='oteryn:item.tibia.i8133'").bind(source.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        source_mint, 1,
        "ordinary8133 corpse is distinct from static12816 reward object"
    );
    let participants:i64=sqlx::query_scalar("SELECT count(*) FROM game_boss_reward_contributors WHERE character_id=encode($1,'hex')::uuid AND(damage_dealt>0 OR damage_received>0 OR healing_others>0)").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        participants, 1,
        "source player genuinely participated in the native fight"
    );
    eprintln!(
        "SRV7_PUMPKIN_SOURCE_DEATH_POSTGRES_EXECUTED actual_corpse=8133 reward_object_is_separate=true genuine_fight_participant=true"
    );
    if CROSS_CHANNEL.get() {
        return super::pumpkin_cross_channel::qualify(h, owner, admitted).await;
    }
    if portal_active() {
        let current = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(super::fixture::debug)?
            .ok_or("genuine current Pumpkin item fence")?;
        admitted.item_fence = Some(current.item_fence());
        admitted.first_entry = crate::gameplay_transport::connection::FirstEntryOutcome::Positioned;
        admitted.continuity.selected_capabilities =
            crate::gameplay_transport::capabilities::SelectedCapabilities::select(
                owner.offered_capabilities(),
                &[
                    4,
                    6,
                    7,
                    11,
                    14,
                    15,
                    18,
                    385,
                    oteryn_protocol_oteryn::chat::CAPABILITY_SCRIPTED_ITEM_SPEECH_V1,
                ],
            )
            .ok_or("current composed Pumpkin movement/map/USE offers")?;
        assert!(
            owner
                .offered_capabilities()
                .iter()
                .any(|offer| offer.id == oteryn_protocol_oteryn::chat::CAPABILITY_SCRIPTED_ITEM_SPEECH_V1)
        );
        assert!(admitted.continuity.selected_capabilities.contains(7));
        assert!(
            admitted
                .continuity
                .selected_capabilities
                .contains(oteryn_protocol_oteryn::chat::CAPABILITY_SCRIPTED_ITEM_SPEECH_V1)
        );
        assert!(
            !admitted.continuity.selected_capabilities.contains(288),
            "scripted item speech is independent of optional mount sounds"
        );
        let actor = admitted
            .runtime_actor
            .ok_or("current Pumpkin participant actor")?;
        portal_presentation(owner, actor).await?;
        let (step, _) = owner
            .ordinary_paced_step(
                actor,
                admitted.game_session_id,
                crate::gameplay_transport::world_spatial::StepDirection::East,
                None,
                None,
            )
            .await;
        assert_eq!(
            step.disposition,
            crate::gameplay_transport::world_spatial::StepDisposition::Moved,
            "actual source portal step"
        );
        let at = owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(super::fixture::debug)?
            .position();
        assert_eq!(
            (at.x, at.y, at.floor),
            (33317, 32417, 15),
            "genuine participant must enter the actual derived reward room through STEP"
        );
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(super::fixture::debug)?
            .ok_or("current participant fence")?;
        let character = crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
            .map_err(super::fixture::debug)?;
        let visit = owner
            .root
            .current_pumpkin_reward_visit(owner.character, character, owner.world_id)
            .await?
            .ok_or("real immutable first reward-room visit")?;
        assert_eq!(visit.deadline_ms - visit.first_entry_ms, 300000);
        super::pumpkin_skinning_use::qualify(h, owner, admitted, &visit).await?;
    }
    Ok(())
}

pub(super) fn source_room(
    original: &crate::content::QualifiedNativeEntryRoom,
    native: &crate::content::native_gameplay::NativeGameplayState,
    world: crate::foundation::WorldId,
    channel: crate::foundation::ChannelId,
) -> TestResult<crate::content::QualifiedNativeEntryRoom> {
    use super::fixture::debug;
    let index = native.item_index().ok_or("Item index")?;
    let bytes = std::fs::read(
        std::env::var_os("OTERYN_SRV7_PUMPKIN_BUNDLE").ok_or("actual compiler Bundle path")?,
    )?;
    let bundle = oteryn_world_bundle::bundle::read(&bytes).map_err(debug)?;
    let pins = crate::map::BundlePins {
        digest: bundle.digest,
        project_format_version: bundle.manifest.identity.project_format_version.clone(),
        world_schema_version: bundle.manifest.identity.world_schema_version.clone(),
        content_revision: bundle.manifest.identity.content_revision.clone(),
        production: false,
    };
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    let start = crate::map::overlay::TilePos {
        x: 33316,
        y: 32415,
        floor: -7,
    };
    let map=crate::map::boot::boot(&bytes,&crate::map::boot::BootPins{bundle:pins,map_revision:crate::map::overlay::map_revision(&base),start},world,channel,|key|{
            let revision=index.revision(key)?;
            let p=native.item_policy(key,revision).map(|p|p.record());
            Some(crate::map::facts::ItemDefinition{
                reference:index.definition_ref("Item",key,revision)?,
                solid:p.and_then(|p|p.attributes.blocks_movement),
                blocks_projectile:p.and_then(|p|p.attributes.blocks_projectile).unwrap_or(true),
                pickupable:p.is_some_and(|p|matches!(&p.semantics.physical,crate::content::ReferenceItemField::Known(p) if p.pickupable==crate::content::ReferenceItemField::Known(true))),door:false,
            })
        }).map_err(debug)?;
    let room = original
        .bundle_view(
            map.movement_cells(original.movement_cells())
                .map_err(debug)?,
            map.frame_binding_digest(world),
            map.map_revision_digest(),
            crate::content::NativeEntryStart {
                x: 33316,
                y: 32415,
                floor: 7,
            },
        )
        .ok_or("real Pumpkin source bundle room view")?;

    Ok(room)
}

async fn portal_presentation(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
) -> TestResult {
    use super::fixture::debug;
    use crate::gameplay_transport::{
        item_view::{ItemViewContinuity, SessionItemView},
        world_map::{MapUpdate, SessionMapView},
    };
    let mut view = SessionMapView::default();
    let mut items = SessionItemView::resume(ItemViewContinuity::default()).with_map_view();
    let closed = owner
        .observe_world_map(actor, &mut view, &mut items)
        .await
        .ok_or("closed portal observation")?
        .map_err(debug)?
        .ok_or("initial closed portal snapshot")?;
    let MapUpdate::Snapshot(closed) = closed else {
        return Err("closed portal native snapshot".into());
    };
    let live = owner.bundle_map().ok_or("real Pumpkin map")?;
    let pos = crate::map::overlay::TilePos {
        x: 33317,
        y: 32415,
        floor: -7,
    };
    let tile = crate::map::view::compose_tile(&live.overlay, &live.facts.pinned(), pos)
        .map_err(debug)?
        .ok_or("real surface portal tile")?;
    assert!(
        !tile.entries.iter().any(|e| e.facts.appearance_id == 1949),
        "initial presentation must be hidden before the first snapshot"
    );
    owner.pumpkin_portal_pass().await.map_err(debug)?;
    let tile = crate::map::view::compose_tile(&live.overlay, &live.facts.pinned(), pos)
        .map_err(debug)?
        .ok_or("open source portal tile")?;
    assert!(
        tile.entries.iter().any(|e| e.facts.appearance_id == 1949),
        "actual settled death must open the source portal presentation"
    );
    let open = owner
        .observe_world_map(actor, &mut view, &mut items)
        .await
        .ok_or("open portal observation")?
        .map_err(debug)?
        .ok_or("open portal native delta")?;
    let MapUpdate::Delta(open) = open else {
        return Err("open portal native delta".into());
    };
    if let Some(path) = std::env::var_os("OTERYN_SRV7_PUMPKIN_CAPTURE") {
        fn hex(bytes: &[u8]) -> String {
            bytes.iter().map(|b| format!("{b:02x}")).collect()
        }
        let observer = owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position();
        let bytes = std::fs::read(
            std::env::var_os("OTERYN_SRV7_PUMPKIN_BUNDLE").ok_or("real Pumpkin Bundle source")?,
        )?;
        let bundle = oteryn_world_bundle::bundle::read(&bytes).map_err(debug)?;
        use sha2::Digest;
        std::fs::write(
            path,
            serde_json::to_vec_pretty(&serde_json::json!({
                "closed_snapshot_hex":hex(&closed.payload),"open_delta_hex":hex(&open.payload),
                "source":{
                    "kind":"PumpkinPortal", "producer_test":"srv7_pumpkin_actual_fight_portal_visit_and_static_reward_body_use",
                    "source_item":1949,"entry":[33317,32415,7],"layout":"DERIVED_NATIVE_ROOM",
                    "reward_body":12816,"ordinary_corpse":8133,
                    "observer_runtime":{"x":observer.x,"y":observer.y,"floor":observer.floor},
                    "observer_native":{"x":observer.x,"y":observer.y,"floor":-i32::from(observer.floor)},
                    "observer_actor_hex":hex(&actor.placement_identity()),
                    "world_id_hex":hex(owner.world_id.as_bytes()),
                    "bundle_digest_hex":hex(&bundle.digest),
                    "bundle_sha256":hex(&sha2::Sha256::digest(&bytes)),
                    "derived_spec_sha256":"48b645d0d99acc8e7424ab44407ce43fa0370bf4108785b7433941c5f8476c86"
                }
            }))?,
        )?;
    }
    Ok(())
}
