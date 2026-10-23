// BED-1d: real dispatch adapter -> native durable owner -> PostgreSQL.
// The server fixture supplies the House owner's resolved intent. This proves
// the durable dispatch connection, not active-bundle/reservation qualification.
use crate::bed_sleep_postgres_cases::{FixturePresence, fixture, presence, request};
use crate::bestiary_postgres_harness::{Harness, TestResult, debug, runtime};
use crate::durability::bed_sleep::{BedSleepOwner, BedSleepRequest};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::foundation::ExactActorRef;
use crate::gameplay_transport::connection::{
    AdmissionRefusal, AdmittedSession, FreshAdmissionAttempt, FreshAdmissionAuthority, UseCommand,
};
use crate::gameplay_transport::item_view::ItemViewContinuity;
use crate::gameplay_transport::item_view::{ItemKey, SessionItemView};
use oteryn_protocol_oteryn::bed::BedSleepIntent;
use oteryn_protocol_oteryn::item_view::ItemHandle;
use oteryn_protocol_oteryn::world_object::UseDisposition;

struct HouseOwner<'a, 'f, 's> {
    harness: &'a Harness,
    recovery: &'a ReconciledCharacterAuthority<'f, 's>,
    request: BedSleepRequest,
    target: ItemKey,
    presence: FixturePresence,
}

impl FreshAdmissionAuthority for HouseOwner<'_, '_, '_> {
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
        target: ItemKey,
    ) -> Option<BedSleepOwner<'_, '_, '_>> {
        (target == self.target).then(|| BedSleepOwner {
            root: &self.harness.root,
            recovery: self.recovery,
            node: &self.harness.node,
            presence: &self.presence,
            request: self.request.clone(),
        })
    }
}

#[test]
fn bed_sleep_successful_real_dispatch_commits_the_native_owner() -> TestResult {
    let Some(admin) = crate::bestiary_postgres_harness::configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async {
        let h = fixture("bed_sleep_dispatch", &admin).await?;
        let request = request()?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let recovery = h.root.open_character_authority(&seal).await.map_err(debug)?;
        let target = ItemKey::MapBase {
            bundle_digest: [7; 32], placement_key: 123, reset_epoch: 1,
        };
        let owner = HouseOwner { harness: &h, recovery: &recovery, request: request.clone(), target, presence: presence()? };
        let actor = ExactActorRef::transport_fixture(
            request.house.world_id(),
            crate::foundation::ChannelId::decode(&crate::bestiary_postgres_harness::id(43)).map_err(debug)?,
        );
        let mut view = SessionItemView::resume(ItemViewContinuity::default()).with_map_view();
        let handle = view.map_view(&[target], |table| {
            table.handle(&target).ok_or(crate::gameplay_transport::item_view::ItemViewError::LimitExceeded)
        }).map_err(debug)?;
        let command = UseCommand { game_session_id: request.fence.game_session_id,
            command_id: request.command_id, item_fence: Some(request.fence) };
        let intent = BedSleepIntent { target: handle, skill: request.skill };
        // Bind the server-prepared intent to the sequenced command before any write.
        let wrong = UseCommand { command_id: 2, ..command };
        assert_eq!(crate::gameplay_transport::bed_sleep::dispatch(&owner, actor, wrong, Ok(intent), Some(&view)).await.map_err(debug)?.disposition,
            UseDisposition::Rejected);
        assert_eq!(h.count("game_house_bed_sleep_receipts").await?, 0);
        let stale = BedSleepIntent { target: ItemHandle::new(handle.get()+1).ok_or("handle")?, ..intent };
        assert_eq!(crate::gameplay_transport::bed_sleep::dispatch(&owner, actor, command, Ok(stale), Some(&view)).await.map_err(debug)?.disposition,
            UseDisposition::StaleState);
        let result = crate::gameplay_transport::bed_sleep::dispatch(&owner, actor, command, Ok(intent), Some(&view)).await.map_err(debug)?;
        assert_eq!(result.disposition, UseDisposition::OfflineTraining);
        let persisted: (i16, i64, i64) = sqlx::query_as("SELECT (SELECT session_state FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid),(SELECT count(*) FROM game_house_bed_sleepers),(SELECT count(*) FROM game_house_bed_sleep_receipts)")
            .bind(request.fence.game_session_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(persisted, (3,1,1));
        drop(owner);
        drop(recovery);
        drop(seal);
        h.cleanup().await
    })
}
