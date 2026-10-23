//! Durable participant qualification. Admission uses the existing accepted §10
//! House fixture; sleep authorization uses native ownership/ACL tables, never
//! test_house_properties. The sex/head/skill are server fixture intent, not
//! evidence for the still-unimplemented active map/reservation/OFFLINE owners.
#[path = "../../../tests/support/house_scope_handoff_fixture.rs"]
mod house_fixture;
use crate::bestiary_postgres_harness::{CHARACTER, Harness, TestResult, WORLD, debug, id, runtime};
use crate::content::LogicalCell;
use crate::domain::appearance::Sex;
use crate::durability::bed_sleep::{BedSleepOutcome, BedSleepOwner, BedSleepRequest};
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::foundation::{AuthenticatedTransportRefV1, ConnectionGeneration, GameSessionId};

/// Fixed server-owner fixture facts, independently of each request under test.
pub(crate) struct FixturePresence {
    world: crate::foundation::WorldId,
    session: GameSessionId,
}
pub(crate) fn presence() -> TestResult<FixturePresence> {
    Ok(FixturePresence {
        world: crate::foundation::WorldId::decode(&id(WORLD)).map_err(debug)?,
        session: GameSessionId::decode(&id(61)).map_err(debug)?,
    })
}
impl crate::durability::spell_house_abi::house_presence_seal::Sealed for FixturePresence {}
impl crate::durability::spell_house_abi::HousePresenceProof for FixturePresence {
    fn world(&self) -> crate::foundation::WorldId {
        self.world
    }
    fn actor_session(&self) -> GameSessionId {
        self.session
    }
    fn scope_generation(&self) -> u64 {
        1
    }
    fn content_digest(&self) -> [u8; 32] {
        [7; 32]
    }
    fn house_key(&self) -> &str {
        HOUSE_KEY
    }
    fn has_door(&self, _: u32) -> bool {
        false
    }
}
use house_fixture::{HOUSE_KEY, admit, house, install_test_house, seed};

pub(crate) async fn fixture(tag: &str, admin: &str) -> TestResult<Harness> {
    let h = Harness::create(admin.to_owned(), tag, false).await?;
    seed(&h).await?;
    install_test_house(&h).await?;
    admit(&h).await?;
    sqlx::query("INSERT INTO game_house_ownership(world_id,house_key,owner_character_id,ownership_revision,acquisition_receipt_ref) VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,1,game_character_uuid_v7())")
        .bind(id(WORLD).as_slice()).bind(HOUSE_KEY).bind(id(CHARACTER).as_slice()).execute(&h.pool).await?;
    sqlx::query("UPDATE game_durability_reconnect_sessions SET current_transport_ref=$1,session_state=1 WHERE game_session_id=encode($2,'hex')::uuid")
        .bind(id(64).as_slice()).bind(id(61).as_slice()).execute(&h.pool).await?;
    Ok(h)
}

pub(crate) fn request() -> TestResult<BedSleepRequest> {
    let current = crate::bestiary_postgres_harness::fence(1)?;
    Ok(BedSleepRequest {
        fence: CurrentCharacterItemFence {
            character_id: current.character_id,
            game_session_id: GameSessionId::decode(&id(61)).map_err(debug)?,
            connection_generation: current.connection_generation,
            character_lease_generation: 2,
            runtime_scope: house()?.runtime_scope().map_err(debug)?,
            scope_ownership_generation: current.scope_ownership_generation,
        },
        transport: AuthenticatedTransportRefV1::decode(&id(64)).map_err(debug)?,
        command_id: 1,
        house: house()?,
        head: LogicalCell {
            x: 100,
            y: 200,
            z: -7,
        },
        sex: Sex::Female,
        skill: 6,
    })
}

pub(crate) async fn commit(h: &Harness, request: BedSleepRequest) -> TestResult<BedSleepOutcome> {
    let seal = h.recovery.seal_current().map_err(debug)?;
    let recovery = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    commit_authorized(h, &recovery, request).await
}

async fn commit_authorized(
    h: &Harness,
    recovery: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    request: BedSleepRequest,
) -> TestResult<BedSleepOutcome> {
    let presence = presence()?;
    Ok(BedSleepOwner {
        root: &h.root,
        recovery,
        node: &h.node,
        presence: &presence,
        request,
    }
    .commit()
    .await
    .map_err(debug)?)
}

async fn footprint(h: &Harness) -> TestResult<(i16, i64, i64, String)> {
    Ok(sqlx::query_as("SELECT (SELECT session_state FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid),(SELECT count(*) FROM game_house_bed_sleepers),(SELECT count(*) FROM game_house_bed_sleep_receipts),(SELECT revision::text FROM game_house_bed_revisions WHERE world_id=encode($2,'hex')::uuid AND house_key=$3)")
        .bind(id(61).as_slice()).bind(id(WORLD).as_slice()).bind(HOUSE_KEY).fetch_one(&h.pool).await?)
}

#[test]
fn bed_sleep_commits_logout_receipt_and_sleeper_once() -> TestResult {
    let Some(admin) = crate::bestiary_postgres_harness::configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async {
        let h = fixture("bed_sleep_commit", &admin).await?;
        let r = request()?;
        assert_eq!(commit(&h, r.clone()).await?, BedSleepOutcome::Slept);
        assert_eq!(footprint(&h).await?, (3, 1, 1, "2".into()));
        let stored: (String, String, i32, bool, bool) = sqlx::query_as("SELECT s.lease_generation::text,s.occupied_variant,s.bed_x,s.slept_at=r.slept_at,d.current_transport_ref IS NULL FROM game_house_bed_sleepers s JOIN game_house_bed_sleep_receipts r ON r.game_session_id=encode($1,'hex')::uuid JOIN game_durability_reconnect_sessions d USING(game_session_id) WHERE s.character_id=encode($2,'hex')::uuid")
            .bind(id(61).as_slice()).bind(id(CHARACTER).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(stored, ("2".into(), "female".into(), 100, true, true));
        assert_eq!(commit(&h, r.clone()).await?, BedSleepOutcome::Slept);
        let mut changed = r;
        changed.head.x += 1;
        assert_eq!(commit(&h, changed).await?, BedSleepOutcome::Conflict);
        assert_eq!(footprint(&h).await?, (3, 1, 1, "2".into()));
        h.cleanup().await
    })
}

#[test]
fn bed_sleep_replaces_its_older_row_and_protects_the_receipt() -> TestResult {
    let Some(admin) = crate::bestiary_postgres_harness::configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async {
        let h = fixture("bed_sleep_old_row", &admin).await?;
        sqlx::query("INSERT INTO game_house_bed_sleepers VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,100,200,-7,1,'male',1000,NULL,NULL)")
            .bind(id(CHARACTER).as_slice()).bind(id(WORLD).as_slice()).bind(HOUSE_KEY).execute(&h.pool).await?;
        assert_eq!(commit(&h, request()?).await?, BedSleepOutcome::Slept);
        assert_eq!(footprint(&h).await?, (3, 1, 1, "4".into()));
        let lease: String = sqlx::query_scalar("SELECT lease_generation::text FROM game_house_bed_sleepers WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(CHARACTER).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(lease, "2");
        for statement in [
            "DELETE FROM game_house_bed_sleep_receipts",
            "UPDATE game_house_bed_sleep_receipts SET slept_at=slept_at+1",
            "TRUNCATE game_house_bed_sleep_receipts",
        ] {
            let result = sqlx::query(statement).execute(&h.pool).await;
            assert!(result.is_err());
            if let Err(error) = result {
                assert_eq!(error.as_database_error().and_then(|e| e.code()).as_deref(), Some("23514"));
            }
        }
        for role in ["oteryn_game_runtime", "oteryn_game_control"] {
            for table in ["game_house_bed_sleep_receipts", "game_house_bed_sleepers", "game_house_bed_revisions"] {
                let granted: bool = sqlx::query_scalar("SELECT has_table_privilege($1,$2,'INSERT,UPDATE,DELETE,TRUNCATE')")
                    .bind(role).bind(table).fetch_one(&h.pool).await?;
                assert!(!granted, "direct mutation grant: {role} on {table}");
            }
        }
        assert_eq!(commit(&h, request()?).await?, BedSleepOutcome::Slept);
        assert_eq!(footprint(&h).await?, (3, 1, 1, "4".into()));
        h.cleanup().await
    })
}

#[test]
fn bed_sleep_stale_fences_and_channel_scope_write_nothing() -> TestResult {
    let Some(admin) = crate::bestiary_postgres_harness::configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async {
        let h = fixture("bed_sleep_fences", &admin).await?;
        for operator in 0..7 {
            let mut r = request()?;
            match operator {
                0 => r.fence.connection_generation = ConnectionGeneration::new(2).map_err(debug)?,
                1 => r.fence.character_lease_generation = 1,
                2 => {
                    r.fence.scope_ownership_generation =
                        crate::foundation::ScopeOwnershipGeneration::new(2).map_err(debug)?
                }
                3 => r.transport = AuthenticatedTransportRefV1::decode(&id(65)).map_err(debug)?,
                4 => r.fence.runtime_scope = crate::bestiary_postgres_harness::scope()?,
                5 => {
                    r.house = crate::durability::house_scope_handoff::HouseId::new(
                        r.house.world_id(),
                        "oteryn:content.house.other",
                    )
                    .ok_or("house")?
                }
                _ => {
                    r.fence.character_id =
                        crate::domain::CharacterId::from_bytes(id(66)).map_err(debug)?
                }
            }
            assert!(matches!(
                commit(&h, r).await?,
                BedSleepOutcome::Stale | BedSleepOutcome::Denied
            ));
            assert_eq!(footprint(&h).await?, (1, 0, 0, "1".into()));
        }
        h.cleanup().await
    })
}

#[test]
fn bed_sleep_native_acl_denies_uninvited_and_accepts_guest_and_subowner() -> TestResult {
    let Some(admin) = crate::bestiary_postgres_harness::configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async {
        for list in [-1, -2] {
            let h = fixture("bed_sleep_acl", &admin).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let recovery = h.root.open_character_authority(&seal).await.map_err(debug)?;
            sqlx::query("INSERT INTO game_character_roots(character_id,account_id,world_id,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,name) SELECT encode($1,'hex')::uuid,account_id,world_id,lifecycle,1,profile_revision,ruleset_revision,content_revision,starter_template_revision,'Bed Owner' FROM game_character_roots WHERE character_id=encode($2,'hex')::uuid")
                .bind(id(67).as_slice()).bind(id(CHARACTER).as_slice()).execute(&h.pool).await?;
            sqlx::query("UPDATE game_house_ownership SET owner_character_id=encode($1,'hex')::uuid,ownership_revision=2,acquisition_receipt_ref=game_character_uuid_v7()")
                .bind(id(67).as_slice()).execute(&h.pool).await?;
            assert_eq!(commit_authorized(&h, &recovery, request()?).await?, BedSleepOutcome::Denied);
            assert_eq!(footprint(&h).await?, (1, 0, 0, "1".into()));
            let mut acl_tx = h.pool.begin().await?;
            sqlx::query("INSERT INTO game_house_acl_receipts(receipt_id,world_id,house_key,list_id,character_id,game_session_id,command_id,ownership_revision,revision_before,revision_after,allow_everyone,members) VALUES(game_character_uuid_v7(),encode($1,'hex')::uuid,$2,$3,encode($4,'hex')::uuid,encode($5,'hex')::uuid,1,2,0,1,false,ARRAY[encode($6,'hex')::uuid])")
                .bind(id(WORLD).as_slice()).bind(HOUSE_KEY).bind(list).bind(id(67).as_slice()).bind(id(61).as_slice()).bind(id(CHARACTER).as_slice()).execute(&mut *acl_tx).await?;
            sqlx::query("INSERT INTO game_house_acl SELECT world_id,house_key,list_id,revision_after,allow_everyone,members,receipt_id FROM game_house_acl_receipts")
                .execute(&mut *acl_tx).await?;
            acl_tx.commit().await?;
            assert_eq!(commit_authorized(&h, &recovery, request()?).await?, BedSleepOutcome::Slept);
            drop(recovery);
            drop(seal);
            h.cleanup().await?;
        }
        Ok(())
    })
}

#[test]
fn bed_sleep_occupied_bed_and_overflow_preserve_live_session() -> TestResult {
    let Some(admin) = crate::bestiary_postgres_harness::configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async {
        let h = fixture("bed_sleep_occupied", &admin).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let recovery = h.root.open_character_authority(&seal).await.map_err(debug)?;
        sqlx::query("INSERT INTO game_character_roots(character_id,account_id,world_id,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,name) SELECT encode($1,'hex')::uuid,account_id,world_id,lifecycle,1,profile_revision,ruleset_revision,content_revision,starter_template_revision,'Bed Sleeper' FROM game_character_roots WHERE character_id=encode($2,'hex')::uuid")
            .bind(id(68).as_slice()).bind(id(CHARACTER).as_slice()).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_house_bed_sleepers VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,100,200,-7,1,'male',1000,NULL,NULL)")
            .bind(id(68).as_slice()).bind(id(WORLD).as_slice()).bind(HOUSE_KEY).execute(&h.pool).await?;
        assert_eq!(commit_authorized(&h, &recovery, request()?).await?, BedSleepOutcome::Occupied);
        assert_eq!(footprint(&h).await?, (1, 1, 0, "2".into()));
        sqlx::query("UPDATE game_house_bed_revisions SET revision=18446744073709551615").execute(&h.pool).await?;
        let mut r = request()?;
        r.head.x += 1;
        assert!(commit_authorized(&h, &recovery, r).await.is_err());
        assert_eq!(footprint(&h).await?, (1, 1, 0, "18446744073709551615".into()));
        drop(recovery);
        drop(seal);
        h.cleanup().await
    })
}
