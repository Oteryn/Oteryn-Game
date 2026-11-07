//! The BED-1 producer includes its final living snapshot in the sleep transaction.
use super::bed_sleep_fixture::{FixturePresence, fixture, presence, request};
use crate::bestiary_postgres_harness::{Harness, TestResult, debug, runtime};
use crate::durability::bed_sleep::{BedSleepOwner, BedSleepRequest};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::character_last_position::{
    FinalLastPosition, LastPositionCell, LastPositionRecord, LastVitals,
};
use crate::foundation::ExactActorRef;
use crate::gameplay_transport::connection::{
    AdmissionRefusal, AdmittedSession, FreshAdmissionAttempt, FreshAdmissionAuthority, UseCommand,
};
use crate::gameplay_transport::item_view::ItemKey;
use oteryn_protocol_oteryn::bed::BedSleepIntent;
use oteryn_protocol_oteryn::item_view::ItemHandle;
use oteryn_protocol_oteryn::world_object::UseDisposition;

struct SleepOwner<'a, 'f, 's> {
    harness: &'a Harness,
    recovery: &'a ReconciledCharacterAuthority<'f, 's>,
    request: BedSleepRequest,
    presence: FixturePresence,
}
impl FreshAdmissionAuthority for SleepOwner<'_, '_, '_> {
    async fn admit(
        &self,
        _: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        Err(AdmissionRefusal::Rejected)
    }
    async fn prepare_bed_sleep(
        &self,
        _: ExactActorRef,
        _: UseCommand,
        _: BedSleepIntent,
        _: ItemKey,
    ) -> Option<BedSleepOwner<'_, '_, '_>> {
        Some(BedSleepOwner {
            root: &self.harness.root,
            recovery: self.recovery,
            node: &self.harness.node,
            presence: &self.presence,
            request: self.request.clone(),
        })
    }
}

#[test]
fn bed_regen_sleep_dispatch_captures_food_soul_and_vitals_atomically_postgres() -> TestResult {
    let Some(admin) = crate::bestiary_postgres_harness::configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async {
        let h=fixture("bed_regen_sleep_snapshot",&admin).await?;
        let seal=h.recovery.seal_current().map_err(debug)?;
        let recovery=h.root.open_character_authority(&seal).await.map_err(debug)?;
        let mut request=request()?;
        request.final_position=Some(FinalLastPosition { write_sequence:1, record:LastPositionRecord {
            world_id:request.house.world_id(),cell:LastPositionCell { x:100,y:200,floor:7 },
            coordinate_frame_digest:[7;32],map_revision_digest:[7;32],server_artifact_digest:[7;32],
            vitals:LastVitals::new(27,9),rest:Some([5,31_567,1234,567]),
        } });
        let actor=ExactActorRef::transport_fixture(request.house.world_id(),crate::foundation::ChannelId::decode(&crate::bestiary_postgres_harness::id(43)).map_err(debug)?);
        let command=UseCommand { game_session_id:request.fence.game_session_id,command_id:request.command_id,item_fence:Some(request.fence) };
        let intent=BedSleepIntent { target:ItemHandle::new(1).ok_or("handle")?,skill:request.skill };
        let target=ItemKey::MapBase { bundle_digest:[7;32],placement_key:123,reset_epoch:1 };
        let mut owner=SleepOwner { harness:&h,recovery:&recovery,request,presence:presence()? };
        assert_eq!(owner.sleep_in_bed(actor,command,intent,target).await.map_err(debug)?.disposition,UseDisposition::OfflineTraining);
        // A replay carries a later capture but cannot replace the first terminal snapshot.
        let snapshot=owner.request.final_position.as_mut().ok_or("capture")?;
        snapshot.write_sequence=2;
        snapshot.record.rest=Some([100,100_000,0,0]);
        assert_eq!(owner.sleep_in_bed(actor,command,intent,target).await.map_err(debug)?.disposition,UseDisposition::OfflineTraining);
        let stored:(i32,i32,Vec<i32>,String)=sqlx::query_as("SELECT health,mana,rest,character_lease_generation::text FROM game_character_last_positions WHERE character_id=encode($1,'hex')::uuid")
            .bind(owner.request.fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(stored,(27,9,vec![5,31567,1234,567],"2".to_owned()));
        drop(owner);drop(recovery);drop(seal);
        h.cleanup().await
    })
}
