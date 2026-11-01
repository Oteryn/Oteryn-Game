//! C08: the bed sleep is a logout (Crystal 201189f8 `bed.cpp:206`), so its terminal write
//! stores the persistent conditions with the final capture, under the same order key, and a
//! replay of the sleep cannot replace them.
use super::bed_sleep_fixture::{FixturePresence, fixture, presence, request};
use crate::bestiary_postgres_harness::{Harness, TestResult, debug, runtime};
use crate::durability::bed_sleep::{BedSleepOwner, BedSleepRequest};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::character_last_position::{
    FinalLastPosition, LastPositionCell, LastPositionRecord, LastVitals, SavedConditions,
};
use crate::foundation::ExactActorRef;
use crate::gameplay_transport::connection::{
    AdmissionRefusal, AdmittedSession, FreshAdmissionAttempt, FreshAdmissionAuthority, UseCommand,
};
use crate::gameplay_transport::item_view::ItemKey;
use oteryn_protocol_oteryn::bed::BedSleepIntent;
use oteryn_protocol_oteryn::item_view::ItemHandle;
use oteryn_protocol_oteryn::world_object::UseDisposition;

const SAVED: &str = r#"[{"key":"drunk","revision":1,"values":{"type":"timed_status","kind":"drunk","duration_ms":4000}},{"key":"exura","revision":0,"values":{"type":"spell_cooldown","remaining_us":6000000}}]"#;

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
fn c08_bed_sleep_stores_the_persistent_conditions_postgres() -> TestResult {
    let Some(admin) = crate::bestiary_postgres_harness::configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async {
        let h = fixture("c08_bed_sleep_conditions", &admin).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let recovery = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut request = request()?;
        request.final_position = Some(FinalLastPosition {
            write_sequence: 1,
            record: LastPositionRecord {
                world_id: request.house.world_id(),
                cell: LastPositionCell {
                    x: 100,
                    y: 200,
                    floor: 7,
                },
                coordinate_frame_digest: [7; 32],
                map_revision_digest: [7; 32],
                server_artifact_digest: [7; 32],
                vitals: LastVitals::new(27, 9),
                rest: Some([5, 31_567, 1234, 567]),
            },
        });
        request.final_conditions = Some(SavedConditions::new(SAVED.to_owned()).ok_or("saved")?);
        let actor = ExactActorRef::transport_fixture(
            request.house.world_id(),
            crate::foundation::ChannelId::decode(&crate::bestiary_postgres_harness::id(43))
                .map_err(debug)?,
        );
        let command = UseCommand {
            game_session_id: request.fence.game_session_id,
            command_id: request.command_id,
            item_fence: Some(request.fence),
        };
        let intent = BedSleepIntent {
            target: ItemHandle::new(1).ok_or("handle")?,
            skill: request.skill,
        };
        let target = ItemKey::MapBase {
            bundle_digest: [7; 32],
            placement_key: 123,
            reset_epoch: 1,
        };
        let mut owner = SleepOwner {
            harness: &h,
            recovery: &recovery,
            request,
            presence: presence()?,
        };
        assert_eq!(
            owner
                .sleep_in_bed(actor, command, intent, target)
                .await
                .map_err(debug)?
                .disposition,
            UseDisposition::OfflineTraining
        );
        // A replay carries a later capture without conditions but replaces nothing.
        let snapshot = owner.request.final_position.as_mut().ok_or("capture")?;
        snapshot.write_sequence = 2;
        owner.request.final_conditions = None;
        assert_eq!(
            owner
                .sleep_in_bed(actor, command, intent, target)
                .await
                .map_err(debug)?
                .disposition,
            UseDisposition::OfflineTraining
        );
        let (conditions, sequence): (Option<String>, String) = sqlx::query_as(
            "SELECT conditions::text, write_sequence::text FROM game_character_last_positions \
              WHERE character_id=encode($1,'hex')::uuid",
        )
        .bind(owner.request.fence.character_id.as_bytes().as_slice())
        .fetch_one(&h.pool)
        .await?;
        let conditions = conditions.ok_or("the bed sleep stored no conditions")?;
        let stored: serde_json::Value = serde_json::from_str(&conditions)?;
        let expected: serde_json::Value = serde_json::from_str(SAVED)?;
        assert_eq!(stored, expected);
        assert_eq!(sequence, "1");
        eprintln!(
            "PG_MARKER c08-bed-sleep-saved-conditions sequence={sequence} entries={}",
            expected.as_array().map_or(0, Vec::len)
        );
        drop(owner);
        drop(recovery);
        drop(seal);
        h.cleanup().await
    })
}
