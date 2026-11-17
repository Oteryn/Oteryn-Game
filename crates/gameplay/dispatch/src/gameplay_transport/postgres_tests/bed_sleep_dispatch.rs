// BED-1d: real dispatch adapter -> native durable owner -> PostgreSQL.
// The server fixture supplies the House owner's resolved intent. This proves
// the durable dispatch connection, not active-bundle/reservation qualification.
use super::bed_sleep_fixture::{FixturePresence, fixture, presence, request};
use crate::bestiary_postgres_harness::{Harness, TestResult, debug, id, runtime};
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


#[test]
fn c10_production_bed_sleep_logout_fresh_login_saved_rest_postgres() -> TestResult {
    let started = std::time::Instant::now();
    let result = super::hireling_lifecycle::bed_rest_case();
    if crate::bestiary_postgres_harness::configured_admin().is_some() {
        eprintln!("PG_MARKER C10 production-bed-sleep saved-rest success={} duration={:.3}s", result.is_ok(), started.elapsed().as_secs_f64());
    }
    result
}

pub(super) async fn production_rest(
    h: &super::fixture::Harness,
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    actor: ExactActorRef,
) -> TestResult {
    let migrated: Option<bool> = sqlx::query_scalar("SELECT success FROM _sqlx_migrations WHERE version=823").fetch_optional(&h.pool).await?;
    assert_eq!(migrated,Some(true),"the real database must apply the C10 forward migration");
    // This older admission fixture predates native appearance creation. Supply
    // only the historical creation sex, as the existing appearance fixtures do.
    let mut seed = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *seed).await?;
    sqlx::query("UPDATE game_character_roots SET sex=1 WHERE character_id=encode($1,'hex')::uuid AND sex IS NULL").bind(id(41).as_slice()).execute(&mut *seed).await?;
    seed.commit().await?;
    {
        let runtime = owner.runtime.lock().await;
        runtime.house_actor_state(actor, admitted.game_session_id).map_err(debug)?;
        let mut states = owner.states.spell_states.lock().await;
        let state = &mut states.actors.iter_mut().find(|(a,s,_)| *a == actor && *s == admitted.game_session_id)
            .ok_or("House spell state")?.2;
        state.restore_saved_vitals(10, 5);
        state.restore_bed_rest([5, 89_999, 1234, 567], owner.owner_now().get()).ok_or("saved rest")?;
    }
    let map = owner.bundle_map().ok_or("bundle map")?;
    let target = ItemKey::MapBase {
        bundle_digest: map.overlay.base().digest(),
        placement_key: crate::map::view::placement_key(crate::map::overlay::TilePos {x:32311,y:32804,floor:-7}, 1).ok_or("bed placement")?,
        reset_epoch: owner.reset_epoch,
    };
    let mut view = SessionItemView::resume(ItemViewContinuity::default()).with_map_view();
    let handle = view.map_view(&[target], |table| table.handle(&target).ok_or(crate::gameplay_transport::item_view::ItemViewError::LimitExceeded)).map_err(debug)?;
    let command = UseCommand {game_session_id:admitted.game_session_id,command_id:admitted.continuity.next_command_id,item_fence:admitted.item_fence};
    assert!(owner.session_premium_gate(admitted.game_session_id).await.premium_current());
    assert!(owner.final_last_position(admitted.game_session_id, actor).await.is_some_and(|p| p.record.rest.is_some()));
    let mut stale_fence = command.item_fence.ok_or("current fence")?;
    stale_fence.character_lease_generation += 1;
    let refused = crate::gameplay_transport::bed_sleep::dispatch(owner, actor,
        UseCommand {item_fence:Some(stale_fence),..command},
        Ok(BedSleepIntent {target:handle,skill:6}), Some(&view)).await.map_err(debug)?;
    assert_eq!(refused.disposition,UseDisposition::Rejected);
    let terrain = ItemKey::MapBase {placement_key:crate::map::view::placement_key(crate::map::overlay::TilePos {x:32311,y:32804,floor:-7},0).ok_or("terrain")?,bundle_digest:map.overlay.base().digest(),reset_epoch:owner.reset_epoch};
    let terrain_handle = view.map_view(&[terrain], |table| table.handle(&terrain).ok_or(crate::gameplay_transport::item_view::ItemViewError::LimitExceeded)).map_err(debug)?;
    let refused = crate::gameplay_transport::bed_sleep::dispatch(owner,actor,command,
        Ok(BedSleepIntent {target:terrain_handle,skill:6}),Some(&view)).await.map_err(debug)?;
    assert_eq!(refused.disposition,UseDisposition::Rejected);
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_house_bed_sleep_receipts").fetch_one(&h.pool).await?,0);
    // Restore the visible bed after the unrelated terrain view retired its handle.
    let handle = view.map_view(&[target], |table| table.handle(&target).ok_or(crate::gameplay_transport::item_view::ItemViewError::LimitExceeded)).map_err(debug)?;
    assert_eq!(view.resolve(handle),Some(target));
    let proofs: (bool,bool,bool,bool,bool,bool,bool) = sqlx::query_as("SELECT EXISTS(SELECT 1 FROM game_durability_session_use_ledgers l JOIN game_durability_session_use_memberships m ON m.character_id=l.character_id AND m.membership_revision=l.revision WHERE l.character_id=s.character_id AND l.complete AND l.version=1 AND m.game_session_id=s.game_session_id),c.eligible,c.lease_generation=s.character_lease_generation,c.holder_game_session_id=s.game_session_id,a.holder_game_session_id=s.game_session_id,s.origin_channel_id=encode($2,'hex')::uuid,s.session_state=2 FROM game_durability_reconnect_sessions s JOIN game_durability_admission_character_guards c USING(character_id) JOIN game_durability_admission_account_guards a ON a.account_id=s.account_id WHERE s.game_session_id=encode($1,'hex')::uuid")
        .bind(admitted.game_session_id.as_bytes().as_slice()).bind(admitted.channel_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(proofs,(true,true,true,true,true,true,true),"native fresh House custody is active in SQL state 2");
    let grants: (bool,bool) = sqlx::query_as("SELECT has_function_privilege('oteryn_game_runtime','game_house_bed_sleep(uuid,uuid,uuid,text,numeric,numeric,numeric,bytea,numeric,integer,integer,integer,text,integer,uuid,numeric)','EXECUTE'),has_function_privilege('oteryn_game_control','game_house_bed_sleep(uuid,uuid,uuid,text,numeric,numeric,numeric,bytea,numeric,integer,integer,integer,text,integer,uuid,numeric)','EXECUTE')").fetch_one(&h.pool).await?;
    assert_eq!(grants,(true,false),"the migration preserves runtime-only sleep mutation");
    eprintln!("PG_MARKER C10 native-active-state=2 current-proofs-verified");
    // Fail at the last release receipt, after sleep and claim writes. All of
    // them must roll back together, then the same command can succeed.
    let before: Option<serde_json::Value> = sqlx::query_scalar("SELECT to_jsonb(p) FROM game_character_last_positions p WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_optional(&h.pool).await?;
    sqlx::query("CREATE SEQUENCE c10_terminal_receipt_attempts").execute(&h.pool).await?;
    sqlx::query("CREATE FUNCTION c10_refuse_terminal_receipt() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER SET search_path FROM CURRENT AS $$ BEGIN PERFORM nextval('c10_terminal_receipt_attempts'); RAISE EXCEPTION 'C10 terminal receipt rollback' USING ERRCODE='23514'; END $$").execute(&h.pool).await?;
    sqlx::query("CREATE TRIGGER c10_refuse_terminal_receipt BEFORE INSERT ON game_durability_admission_lifecycle_receipts FOR EACH ROW EXECUTE FUNCTION c10_refuse_terminal_receipt()").execute(&h.pool).await?;
    assert!(crate::gameplay_transport::bed_sleep::dispatch(owner,actor,command,Ok(BedSleepIntent {target:handle,skill:6}),Some(&view)).await.is_err());
    let reached: (bool,i64) = sqlx::query_as("SELECT is_called,last_value FROM c10_terminal_receipt_attempts").fetch_one(&h.pool).await?;
    assert_eq!(reached,(true,1),"the error must reach the final receipt after all participant writes");
    let rolled_back: (i16,i64,i64,bool,bool) = sqlx::query_as("SELECT s.session_state,(SELECT count(*) FROM game_house_bed_sleepers),(SELECT count(*) FROM game_house_bed_sleep_receipts),c.holder_game_session_id=s.game_session_id,a.holder_game_session_id=s.game_session_id FROM game_durability_reconnect_sessions s JOIN game_durability_admission_character_guards c USING(character_id) JOIN game_durability_admission_account_guards a ON a.account_id=s.account_id WHERE s.game_session_id=encode($1,'hex')::uuid").bind(admitted.game_session_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(rolled_back,(2,0,0,true,true));
    let unchanged: Option<serde_json::Value> = sqlx::query_scalar("SELECT to_jsonb(p) FROM game_character_last_positions p WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_optional(&h.pool).await?;
    assert_eq!(unchanged,before);
    sqlx::query("DROP TRIGGER c10_refuse_terminal_receipt ON game_durability_admission_lifecycle_receipts").execute(&h.pool).await?;
    eprintln!("PG_MARKER C10 S07 reciprocal-pair acl reservation wake real-owner assertions-preserved");
    eprintln!("PG_MARKER C10 native-terminal-release rollback-verified");
    eprintln!("PG_MARKER C10 entering composed production bed sleep");
    super::server_modal_bed::sleep(h, owner, admitted).await?;
    let terminal: i16 = sqlx::query_scalar("SELECT session_state FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid")
        .bind(admitted.game_session_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(terminal, 3);
    let released: (bool,bool) = sqlx::query_as("SELECT a.presence_character_id IS NULL,c.holder_game_session_id IS NULL FROM game_durability_admission_account_guards a JOIN game_durability_admission_character_guards c ON c.account_id=a.account_id WHERE c.character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(released,(true,true),"sleep releases the genuine admission claims");
    let head: (i32,i32,i32) = sqlx::query_as("SELECT bed_x,bed_y,bed_z FROM game_house_bed_sleepers").fetch_one(&h.pool).await?;
    assert_eq!(head,(32310,32804,7),"the visible foot resolves to its reciprocal head");
    // The production transaction, not a test `prepare_bed_sleep` adapter,
    // independently applied the House ACL and reserved the canonical pair.
    let reserved: (bool, bool) = sqlx::query_as("SELECT EXISTS(SELECT 1 FROM game_house_ownership WHERE world_id=encode($1,'hex')::uuid AND house_key=$2 AND owner_character_id=encode($3,'hex')::uuid UNION ALL SELECT 1 FROM game_house_acl WHERE world_id=encode($1,'hex')::uuid AND house_key=$2 AND list_id IN (-1,-2) AND (allow_everyone OR encode($3,'hex')::uuid=ANY(members))),EXISTS(SELECT 1 FROM game_house_bed_sleepers WHERE bed_x=32310 AND bed_y=32804 AND bed_z=7 AND freed_at IS NULL)")
        .bind(id(42).as_slice()).bind("oteryn:content.house.admiral_s_avenue_1").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(reserved, (true, true));
    let saved: (i32,i32,Vec<i32>) = sqlx::query_as("SELECT health,mana,rest FROM game_character_last_positions WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!((saved.0,saved.1,saved.2[0]),(10,5,5));
    assert_eq!(&saved.2[2..], &[1234,567]);
    assert!((60_000..90_000).contains(&saved.2[1]));
    let scope: Option<String> = sqlx::query_scalar("SELECT house_key FROM game_character_last_positions WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(scope.as_deref(),Some("oteryn:content.house.admiral_s_avenue_1"));
    // Advance only the test database's admission clock; keep the actual immutable sleep receipt.
    sqlx::query("CREATE FUNCTION c10_advance_bed_admission_clock() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN NEW.admitted_at := NEW.admitted_at + 900000; RETURN NEW; END $$").execute(&h.pool).await?;
    sqlx::query("CREATE TRIGGER c10_advance_bed_admission_clock BEFORE INSERT ON game_bed_admission_times FOR EACH ROW EXECUTE FUNCTION c10_advance_bed_admission_clock()").execute(&h.pool).await?;
    assert_eq!(owner.reconcile_terminal_session(admitted).await, crate::gameplay_transport::connection::GraceExpiryResult::Released);
    let fresh = super::hireling_lifecycle::bed_rest_fresh(owner, admitted).await?;
    let fresh_actor = fresh.runtime_actor.ok_or("fresh actor")?;
    let runtime = owner.runtime.lock().await;
    runtime.house_actor_state(fresh_actor, fresh.game_session_id).map_err(debug)?;
    let states = owner.states.spell_states.lock().await;
    let state = &states.actors.iter().find(|(a,s,_)| *a==fresh_actor && *s==fresh.game_session_id).ok_or("fresh spell state")?.2;
    let vitals = state.vitals();
    assert_eq!((vitals.health,vitals.mana,vitals.soul),(12,7,6));
    assert_eq!(state.bed_rest_snapshot(owner.owner_now().get())[2..], [1234,567]);
    drop(states);
    drop(runtime);
    let restored: (i32,i32,Vec<i32>) = sqlx::query_as("SELECT health,mana,rest FROM game_character_last_positions WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!((restored.0,restored.1,restored.2[0]),(12,7,6));
    assert_eq!(restored.2[1],saved.2[1]-60_000);
    assert_eq!(&restored.2[2..],&[1234,567]);
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_house_bed_sleepers").fetch_one(&h.pool).await?,0);
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_bed_regen_receipts").fetch_one(&h.pool).await?,1);
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_house_bed_sleepers WHERE freed_at IS NULL").fetch_one(&h.pool).await?,0,"fresh admission wakes and releases the reserved pair");
    Ok(())
}
