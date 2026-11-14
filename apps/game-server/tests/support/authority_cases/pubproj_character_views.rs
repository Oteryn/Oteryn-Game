//! P3 committed historical cuts and the actual v2 source/outbox/read/ACK owners.
use crate::bestiary_postgres_harness::{CHARACTER, Harness, TestResult, configured_admin, debug, id, runtime};
use crate::native_admission_source::public_projections::v2::{Body, Family};
use sqlx::{Executor, Row};

const FAMILIES: [&str; 4] = ["character_skills", "character_achievements", "character_equipment", "character_outfit"];

// Historical fixtures have no replayable gameplay receipts. Only the NEW projection triggers
// are strengthened to ALWAYS in the isolated database, so their real deferred capture runs
// while importing historical source rows. No existing production guard is changed or removed.
async fn prepare(h: &Harness) -> TestResult {
    for (table, trigger) in [("game_character_roots","public_v2_character_root"),
        ("game_character_progression_state","public_v2_skills_progression"),
        ("game_character_build_state","public_v2_skills_build"),
        ("game_account_achievements","public_v2_earned"),
        ("game_character_equipment_slots","public_v2_equipment_slots"),
        ("game_character_equipment_state","public_v2_equipment_state"),
        ("game_item_instances","public_v2_item"),
        ("game_item_container_slots","public_v2_container"),
        ("game_character_appearance","public_v2_appearance"),
        ("game_public_v2_outfit_proofs","public_v2_appearance_proof")] {
        sqlx::query(sqlx::AssertSqlSafe(format!("ALTER TABLE {table} ENABLE ALWAYS TRIGGER {trigger}")))
            .execute(&h.pool).await?;
    }
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
    h.root.sync_public_achievement_catalogue(&authority).await.map_err(debug)?;
    for family in FAMILIES {
        sqlx::query("SELECT game_public_projection_resync($1,false)").bind(family).execute(&h.pool).await?;
    }
    Ok(())
}
async fn history(h: &Harness, statements: &[&'static str]) -> TestResult {
    let mut tx = h.pool.begin().await?;
    tx.execute("SET LOCAL session_replication_role=replica").await?;
    for statement in statements {
        sqlx::query(sqlx::AssertSqlSafe(*statement)).bind(id(CHARACTER).as_slice()).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(())
}
async fn head(h: &Harness, family: &str) -> TestResult<(i64, serde_json::Value)> {
    let row = sqlx::query("SELECT revision,payload FROM game_public_v2_heads WHERE family=$1 AND subject=encode($2,'hex')::uuid::text AND world_id=''")
        .bind(family).bind(id(CHARACTER).as_slice()).fetch_one(&h.pool).await?;
    Ok((row.try_get("revision")?, row.try_get("payload")?))
}
async fn snapshot(h: &Harness, family: Family) -> TestResult<Option<crate::native_admission_source::public_projections::v2::Snapshot>> {
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
    Ok(h.root.next_public_v2(&authority, family, &[]).await.map_err(debug)?)
}
const XP: &str = "UPDATE game_character_progression_state SET total_experience=total_experience+1 WHERE character_id=encode($1,'hex')::uuid";
const BUILD: &str = "INSERT INTO game_character_build_state SELECT character_id,'none',1,0,10,0,10,0,10,0,10,0,10,0,10,0,10,0,2,'019a0000-0000-7000-8000-000000000071'::uuid FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid";
const EQUIPMENT: &str = "INSERT INTO game_character_equipment_state(character_id,revision) SELECT character_id,1 FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid";
const ITEM: &str = "INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) SELECT '019a0000-0000-7000-8000-000000000072',world_id,'Item','fixture:two_handed_sword','fixture-v1',1,1,'019a0000-0000-7000-8000-000000000073' FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid";
const SLOT: &str = "INSERT INTO game_character_equipment_slots(character_id,slot,item_instance_id,world_id,placed_transaction_id) SELECT character_id,5,'019a0000-0000-7000-8000-000000000072',world_id,'019a0000-0000-7000-8000-000000000073' FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid";

// A bounded test content profile, with an actual earned account mount fact below.
static CONTENT_FALLBACK_READ: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
struct LookContent;
impl crate::domain::appearance::AppearanceContent<String> for LookContent {
    fn palette_size(&self)->u16 {133}
    fn projection_fallback(&self,_:crate::domain::appearance::Sex)->Option<crate::domain::appearance::AppearanceSelection<String>> {
        CONTENT_FALLBACK_READ.store(true,std::sync::atomic::Ordering::SeqCst);
        Some(crate::domain::appearance::AppearanceSelection {outfit_key:"outfit.fixture".into(),colours:[0;4],addons:0,mount_key:None})
    }
    fn earned_grant_keys(&self)->Vec<String> {vec!["fixture:mount".into()]}
    fn earned_unlock(&self,key:&str,provenance:&str,target:crate::domain::appearance::EarnedCosmetic<'_>)->crate::domain::appearance::UnlockFact {
        use crate::domain::appearance::{EarnedCosmetic,UnlockFact};
        if key=="fixture:mount" && provenance=="content-1" && matches!(target,EarnedCosmetic::Mount("mount.fixture")) {
            UnlockFact::Compatible
        } else {UnlockFact::Absent}
    }
    fn outfit_addon_count(&self,key:&String)->Option<u8> {(key=="outfit.fixture").then_some(0)}
    fn outfit_is_starter(&self,key:&String,sex:crate::domain::appearance::Sex)->bool {key=="outfit.fixture" && sex==crate::domain::appearance::Sex::Male}
    fn addon_is_starter(&self,_:&String,_:crate::domain::appearance::Sex,_:u8)->bool {false}
    fn mount_exists(&self,key:&String)->bool {key=="mount.fixture"}
}

async fn exercise(h: &Harness, case: u8) -> TestResult {
    prepare(h).await?;
    match case {
        0 => {
            let before = head(h,"character_skills").await?;
            assert_eq!(before.1[0]["magic_level"],0);
            history(h,&[XP]).await?;
            let xp = head(h,"character_skills").await?;
            assert!(xp.0>before.0);
            assert_ne!(xp.1[0]["total_experience"],before.1[0]["total_experience"]);
            history(h,&["UPDATE game_character_roots SET character_revision=2 WHERE character_id=encode($1,'hex')::uuid", "UPDATE game_character_progression_state SET character_revision=2 WHERE character_id=encode($1,'hex')::uuid",BUILD]).await?;
            let built = head(h,"character_skills").await?;
            assert!(built.0>xp.0); assert_eq!(built.1[0]["magic_level"],1);
        }
        1 => {
            history(h,&["DELETE FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid"]).await?;
            let s = snapshot(h,Family::CharacterSkills).await?.ok_or("suppression")?;
            assert!(matches!(s.body,Body::Skills(rows) if rows.is_empty()));
        }
        2 => {
            history(h,&["INSERT INTO game_account_achievement_grant_requests SELECT 'fixture',decode('01','hex'),k.key,k.value->>'revision',r.account_id,r.character_id,1790000000000 FROM game_character_roots r CROSS JOIN game_public_v2_achievement_catalogue c CROSS JOIN LATERAL (SELECT key,value FROM jsonb_each(c.entries) ORDER BY key LIMIT 1) k WHERE r.character_id=encode($1,'hex')::uuid", "INSERT INTO game_account_achievements SELECT account_id,achievement_key,source_kind,source_event_id FROM game_account_achievement_grant_requests WHERE character_id=encode($1,'hex')::uuid"]).await?;
            let old = snapshot(h,Family::CharacterAchievements).await?.ok_or("earned")?;
            assert!(matches!(old.body,Body::Achievements(rows) if rows.len()==1 && rows[0].earned.len()==1));
            history(h,&["UPDATE game_character_roots SET account_id='019a0000-0000-7000-8000-000000000088',character_revision=character_revision+1 WHERE character_id=encode($1,'hex')::uuid"]).await?;
            // Inspect the committed ownership projection directly: importing this historical
            // owner relation is not a fabricated Bazaar receipt or a new recovery proof.
            let (revision, payload) = head(h,"character_achievements").await?;
            assert!(u64::try_from(revision)?>old.projection_revision);
            let rows: Vec<crate::native_admission_source::public_projections::v2::character_views::Achievements> = serde_json::from_value(payload)?;
            assert!(rows.len()==1 && rows[0].earned.is_empty() && rows[0].character_revision=="2");
        }
        3 => {
            // Synthetic, fully compatible 1025-key catalogue stresses the accepted wire bound.
            history(h,&["UPDATE game_public_v2_achievement_catalogue SET entries=(SELECT jsonb_object_agg('oteryn:achievement/fixture_'||n,jsonb_build_object('revision','1','retired',false)) FROM generate_series(1,1025) n) WHERE singleton AND EXISTS(SELECT 1 FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid)", "INSERT INTO game_account_achievement_grant_requests SELECT 'fixture',decode('01','hex'),'oteryn:achievement/fixture_'||n,'1',r.account_id,r.character_id,1790000000000 FROM game_character_roots r CROSS JOIN generate_series(1,1024) n WHERE r.character_id=encode($1,'hex')::uuid", "INSERT INTO game_account_achievements SELECT account_id,achievement_key,source_kind,source_event_id FROM game_account_achievement_grant_requests WHERE character_id=encode($1,'hex')::uuid"]).await?;
            assert_eq!(head(h,"character_achievements").await?.1[0]["earned"].as_array().ok_or("earned array")?.len(),1024);
            history(h,&["INSERT INTO game_account_achievement_grant_requests SELECT 'fixture',decode('01','hex'),'oteryn:achievement/fixture_1025','1',account_id,character_id,1790000000000 FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid", "INSERT INTO game_account_achievements SELECT account_id,achievement_key,source_kind,source_event_id FROM game_account_achievement_grant_requests WHERE character_id=encode($1,'hex')::uuid ON CONFLICT DO NOTHING"]).await?;
            assert!(head(h,"character_achievements").await?.1.is_null());
            let ready: bool=sqlx::query_scalar("SELECT game_public_v2_ready('character_achievements','',encode($1,'hex')::uuid::text)").bind(id(CHARACTER).as_slice()).fetch_one(&h.pool).await?;
            assert!(!ready);
            assert!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_public_v2_outbox WHERE family='character_achievements'").fetch_one(&h.pool).await?>0);
        }
        4 | 5 => {
            history(h,&[EQUIPMENT,ITEM,SLOT]).await?;
            let first=snapshot(h,Family::CharacterEquipment).await?.ok_or("equipment")?;
            let Body::Equipment(ref rows)=first.body else { return Err("equipment variant".into()); };
            assert_eq!(rows[0].slots.len(),1);
            assert_eq!(rows[0].slots[0].slot,crate::native_admission_source::public_projections::v2::character_views::Slot::RightHand);
            assert_eq!(rows[0].slots[0].item_key,"fixture:two_handed_sword");
            assert!(!rows[0].slots.iter().any(|s|s.slot==crate::native_admission_source::public_projections::v2::character_views::Slot::LeftHand),"reader must not duplicate the two-handed primary item into LeftHand");
            if case==4 {
                let mut pending=h.pool.begin().await?;
                pending.execute("SET LOCAL session_replication_role=replica").await?;
                sqlx::query("DELETE FROM game_character_equipment_slots WHERE character_id=encode($1,'hex')::uuid").bind(id(CHARACTER).as_slice()).execute(&mut *pending).await?;
                assert_eq!(snapshot(h,Family::CharacterEquipment).await?.ok_or("committed cut")?,first);
                pending.rollback().await?;
                assert_eq!(snapshot(h,Family::CharacterEquipment).await?.ok_or("rolled back cut")?,first);
            }
        }
        6 => {
            CONTENT_FALLBACK_READ.store(false,std::sync::atomic::Ordering::SeqCst);
            use crate::domain::appearance::{AppearanceSelection, Sex};
            use crate::durability::character_appearance::{AppearanceChangeOccurrence, AppearanceChangeRequest,
                AppearanceValidationContext, AppearanceChangeOutcome};
            history(h,&["INSERT INTO game_account_appearance_unlock_receipts SELECT '019a0000-0000-7000-8000-000000000075',account_id,'fixture:mount','content-1',character_id,'mount_taming',decode('01','hex') FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid", "INSERT INTO game_account_appearance_unlocks SELECT account_id,unlock_key,grant_occurrence_id FROM game_account_appearance_unlock_receipts WHERE granting_character_id=encode($1,'hex')::uuid"]).await?;
            let seal=h.recovery.seal_current().map_err(debug)?;
            let authority=h.root.open_character_authority(&seal).await.map_err(debug)?;
            let outcome=h.root.commit_character_appearance(&authority,&h.node,
                crate::bestiary_postgres_harness::fence(1)?, AppearanceChangeRequest {
                    transport_binding: None,
                    occurrence: AppearanceChangeOccurrence::from_bytes(id(177)).map_err(debug)?,
                    before: None, after: AppearanceSelection {outfit_key:"outfit.fixture".into(),
                        colours:[1,2,3,4], addons:0, mount_key:Some("mount.fixture".into())},
                    content_revision:"content-1".into(),
                }, AppearanceValidationContext {sex:Sex::Male,content:LookContent,content_revision:"content-1".into()})
                .await.map_err(debug)?;
            assert!(matches!(outcome,AppearanceChangeOutcome::Committed(_)));
            assert!(CONTENT_FALLBACK_READ.load(std::sync::atomic::Ordering::SeqCst),"DisplayedLook must consult the authored content default");
            let s=snapshot(h,Family::CharacterOutfit).await?.ok_or("displayed look")?;
            assert!(matches!(s.body,Body::Outfit(rows) if rows.len()==1 && rows[0].mount.is_empty()));
            let stored:Option<String>=sqlx::query_scalar("SELECT mount_key FROM game_character_appearance WHERE character_id=encode($1,'hex')::uuid")
                .bind(id(CHARACTER).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(stored.as_deref(),Some("mount.fixture"));
        }
        7 => {
            let clock: String=sqlx::query_scalar("SELECT pg_get_functiondef('game_public_v2_force_touch(text,text,text,jsonb)'::regprocedure)").fetch_one(&h.pool).await?;
            assert!(clock.contains("clock_timestamp()"));
            sqlx::query(sqlx::AssertSqlSafe(clock.replace("clock_timestamp()","to_timestamp(2000000000)"))).execute(&h.pool).await?;
            let before=head(h,"character_skills").await?.0;
            history(h,&[XP]).await?;
            let middle=head(h,"character_skills").await?.0;
            history(h,&[XP]).await?;
            let after=head(h,"character_skills").await?.0;
            assert!(middle>before && after>middle);
            let observed: i64=sqlx::query_scalar("SELECT observed_ms FROM game_public_v2_heads WHERE family='character_skills' AND subject=encode($1,'hex')::uuid::text").bind(id(CHARACTER).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(observed,2000000000000);
            let mut rolled_back=h.pool.begin().await?;
            rolled_back.execute("SET LOCAL session_replication_role=replica").await?;
            sqlx::query(XP).bind(id(CHARACTER).as_slice()).execute(&mut *rolled_back).await?;
            rolled_back.execute("SET CONSTRAINTS ALL IMMEDIATE").await?;
            let pending: i64=sqlx::query_scalar("SELECT revision FROM game_public_v2_heads WHERE family='character_skills' AND subject=encode($1,'hex')::uuid::text").bind(id(CHARACTER).as_slice()).fetch_one(&mut *rolled_back).await?;
            assert!(pending>after);
            rolled_back.rollback().await?;
            assert_eq!(head(h,"character_skills").await?.0,after);
        }
        8 => {
            let first=snapshot(h,Family::CharacterSkills).await?.ok_or("first")?;
            history(h,&[XP]).await?;
            let seal=h.recovery.seal_current().map_err(debug)?;
            let authority=h.root.open_character_authority(&seal).await.map_err(debug)?;
            h.root.ack_public_v2(&authority,&first).await.map_err(debug)?;
            let second=snapshot(h,Family::CharacterSkills).await?.ok_or("successor remains")?;
            assert!(second.projection_revision>first.projection_revision);
            h.root.ack_public_v2(&authority,&second).await.map_err(debug)?;
            assert!(snapshot(h,Family::CharacterSkills).await?.is_none());
        }
        9 => {
            history(h,&["DELETE FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid"]).await?;
            sqlx::query("DELETE FROM game_public_v2_outbox WHERE family=ANY($1)").bind(FAMILIES.to_vec()).execute(&h.pool).await?;
            sqlx::query("SELECT game_public_projection_resync('all',true)").execute(&h.pool).await?;
            for family in [Family::CharacterSkills,Family::CharacterAchievements,Family::CharacterEquipment,Family::CharacterOutfit] {
                let s=snapshot(h,family).await?.ok_or("baseline includes subject")?;
                assert!(s.projection_epoch>1);
                if family==Family::CharacterSkills { assert!(matches!(s.body,Body::Skills(rows) if rows.is_empty())); }
            }
        }
        _ => return Err("unknown projection case".into()),
    }
    Ok(())
}
fn run(tag: &'static str,case:u8)->TestResult {
    let Some(admin)=configured_admin() else { return Ok(()); };
    std::thread::Builder::new().stack_size(64<<20).spawn(move || -> Result<(),String> {
        runtime().map_err(debug)?.block_on(async move {
            let started=std::time::Instant::now();
            // Red-before-green control on real PostgreSQL: a disabled new projector must
            // fail this same case for its missing output, not for unrelated eligibility.
            let negative=Harness::create(admin.clone(),&format!("p3_negative_{case}"),true).await?;
            let mut absence=negative.pool.begin().await?;
            absence.execute("SET LOCAL session_replication_role=replica").await?;
            sqlx::query("DELETE FROM game_public_v2_outbox WHERE family=ANY($1)").bind(FAMILIES.to_vec()).execute(&mut *absence).await?;
            sqlx::query("DELETE FROM game_public_v2_heads WHERE family=ANY($1)").bind(FAMILIES.to_vec()).execute(&mut *absence).await?;
            absence.commit().await?;
            negative.pool.execute("CREATE OR REPLACE FUNCTION game_public_v2_character_capture(c UUID,f TEXT) RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER AS $$ BEGIN RETURN; END $$").await?;
            let missing=exercise(&negative,case).await;
            negative.cleanup().await?;
            let error=missing.err().ok_or("capture-disabled negative control unexpectedly passed")?;
            let expected=match case {
                0|3|7 => matches!(error.downcast_ref::<sqlx::Error>(),Some(sqlx::Error::RowNotFound)),
                1=>error.to_string()=="suppression", 2=>error.to_string()=="earned",
                4|5=>error.to_string()=="equipment", 6=>error.to_string()=="displayed look",
                8=>error.to_string()=="first", 9=>error.to_string()=="baseline includes subject",
                _=>false,
            };
            if !expected { return Err(format!("wrong negative-control failure: {error}").into()); }
            println!("P3_CHARACTER_VIEWS_RED case={tag} result=missing_capture");
            let h=Harness::create(admin,&format!("p3_{case}"),true).await?;
            let result=exercise(&h,case).await;
            h.cleanup().await?;
            if result.is_ok() { println!("P3_CHARACTER_VIEWS_PG case={tag} elapsed_ms={}",started.elapsed().as_millis()); }
            result
        }).map_err(debug)
    })?.join().map_err(|_|"projection test thread failed")?.map_err(Into::into)
}
macro_rules! pg_case { ($name:ident,$case:expr) => { #[test] fn $name()->TestResult { run(stringify!($name),$case) } }; }
pg_case!(public_v2_skills_revision_bumps_on_progression_and_build_change,0);
pg_case!(public_v2_skills_missing_progression_is_suppression,1);
pg_case!(public_v2_achievements_transfer_drops_previous_owner_facts,2);
pg_case!(public_v2_achievements_over_1024_held_not_truncated,3);
pg_case!(public_v2_equipment_reads_committed_custody_only,4);
pg_case!(public_v2_equipment_two_handed_not_duplicated,5);
pg_case!(public_v2_outfit_mount_always_empty,6);
pg_case!(public_v2_same_second_changes_bump_revision,7);
pg_case!(public_v2_ack_clears_only_at_or_below_revision,8);
pg_case!(public_v2_resync_includes_new_families_and_suppressions,9);

#[test]
fn public_v2_review_source_isolation_and_missing_evidence() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new().stack_size(64<<20).spawn(move || -> Result<(),String> {
 runtime().map_err(debug)?.block_on(async move {
 let h=Harness::create(admin,"p3_review_sources",true).await?;
 let result:TestResult=async {
 prepare(&h).await?;
 let seal=h.recovery.seal_current().map_err(debug)?;
 let authority=h.root.open_character_authority(&seal).await.map_err(debug)?;
 sqlx::query("UPDATE game_public_v2_achievement_catalogue SET revision='incompatible' WHERE singleton").execute(&h.pool).await?;
 assert!(h.root.next_public_v2(&authority,Family::CharacterAchievements,&[]).await.is_err(),"catalogue mismatch must hold reader");
 assert!(h.root.public_v2_facts(&authority,Family::CharacterAchievements).await.is_err(),"catalogue mismatch must hold watermark");
 h.root.sync_public_achievement_catalogue(&authority).await.map_err(debug)?;
 let before=[head(&h,"character_achievements").await?.0,head(&h,"character_equipment").await?.0,head(&h,"character_outfit").await?.0];
 history(&h,&[XP]).await?;
 let after=[head(&h,"character_achievements").await?.0,head(&h,"character_equipment").await?.0,head(&h,"character_outfit").await?.0];
 assert_eq!(before,after,"progression must not touch unrelated families");
 // Exercise the deferred revision-only trigger inside a rollback-only probe:
 // historical projection fixtures cannot fabricate a committed gameplay receipt chain.
 let mut probe=h.pool.begin().await?;
 probe.execute("SET LOCAL session_replication_role=replica").await?;
 sqlx::query("UPDATE game_character_roots SET character_revision=character_revision+1 WHERE character_id=encode($1,'hex')::uuid").bind(id(CHARACTER).as_slice()).execute(&mut *probe).await?;
 probe.execute("SET CONSTRAINTS ALL IMMEDIATE").await?;
 let probed:i64=sqlx::query_scalar("SELECT revision FROM game_public_v2_heads WHERE family='character_achievements' AND subject=encode($1,'hex')::uuid::text").bind(id(CHARACTER).as_slice()).fetch_one(&mut *probe).await?;
 assert_eq!(probed,before[0],"revision-only writes must not scan achievements");
 probe.rollback().await?;
 history(&h,&["DELETE FROM game_public_v2_achievement_catalogue WHERE EXISTS(SELECT 1 FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid)"]).await?;
 sqlx::query("SELECT game_public_v2_character_capture(encode($1,'hex')::uuid,'character_achievements')").bind(id(CHARACTER).as_slice()).execute(&h.pool).await?;
 assert!(head(&h,"character_achievements").await?.1.is_null(),"missing catalogue must hold");
 let ready:bool=sqlx::query_scalar("SELECT game_public_v2_ready('character_achievements','',encode($1,'hex')::uuid::text)").bind(id(CHARACTER).as_slice()).fetch_one(&h.pool).await?;
 assert!(!ready);
 let may_write:bool=sqlx::query_scalar("SELECT has_table_privilege('oteryn_game_runtime','game_public_v2_outfit_proofs','INSERT,UPDATE')").fetch_one(&h.pool).await?;
 assert!(!may_write,"runtime cannot forge outfit proofs");
 assert!(matches!(snapshot(&h,Family::CharacterEquipment).await?.ok_or("custody suppression")?.body,Body::Equipment(rows) if rows.is_empty()));
 assert!(matches!(snapshot(&h,Family::CharacterOutfit).await?.ok_or("proof suppression")?.body,Body::Outfit(rows) if rows.is_empty()));
 history(&h,&["UPDATE game_character_roots SET character_revision=2 WHERE character_id=encode($1,'hex')::uuid", "UPDATE game_character_progression_state SET character_revision=2 WHERE character_id=encode($1,'hex')::uuid", "INSERT INTO game_character_appearance(character_id,outfit_key,colours,addons,mount_key,content_revision,committed_character_revision,last_appearance_occurrence_id) SELECT character_id,'outfit.fixture',ARRAY[1,2,3,4],0,NULL,content_revision,character_revision,'019a0000-0000-7000-8000-000000000099' FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid"]).await?;
 assert_eq!(head(&h,"character_outfit").await?.1,serde_json::json!([]),"raw selection without DisplayedLook proof must suppress");
 // Cross a full keyset page, including roots with no progression/equipment/proof.
 history(&h,&["INSERT INTO game_character_roots(character_id,account_id,world_id,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,name,sex) SELECT ('019a0000-0000-7000-8000-'||lpad(to_hex(n),12,'0'))::uuid,account_id,world_id,1,1,profile_revision,ruleset_revision,content_revision,starter_template_revision,'Fixture Batch '||chr(65+n/26)||chr(65+n%26),sex FROM game_character_roots CROSS JOIN generate_series(1000,1128) n WHERE character_id=encode($1,'hex')::uuid"]).await?;
 for family in FAMILIES {
  sqlx::query("SELECT game_public_projection_resync($1,false)").bind(family).execute(&h.pool).await?;
  let covered:i64=sqlx::query_scalar("SELECT count(*) FROM game_public_v2_heads WHERE family=$1").bind(family).fetch_one(&h.pool).await?;
  assert_eq!(covered,130,"bounded resync must cover every subject");
 }


 Ok(())
 }.await;
 h.cleanup().await?;
 result
 }).map_err(debug)
 })?.join().map_err(|_|"review case failed")?.map_err(Into::into)
}

pg_case!(public_v2_resync_covers_new_families, 9);


async fn catalogue_lock_race(h:&Harness)->TestResult {
                // Pause the real fenced kill immediately after its deferred skills capture.
                // The observer controls a lock, not an elapsed-time guess.
                h.pool.execute("ALTER FUNCTION game_public_v2_character_capture(UUID,TEXT) RENAME TO p3_capture_actual; CREATE FUNCTION game_public_v2_character_capture(c UUID,f TEXT) RETURNS VOID LANGUAGE plpgsql AS $$ BEGIN PERFORM p3_capture_actual(c,f); IF f='character_skills' THEN PERFORM pg_advisory_xact_lock(3185001); END IF; END $$").await?;
                let mut barrier=h.pool.acquire().await?;
                sqlx::query("SELECT pg_advisory_lock(3185001)").execute(&mut *barrier).await?;
                let barrier_pid:i32=sqlx::query_scalar("SELECT pg_backend_pid()").fetch_one(&mut *barrier).await?;
                let seal=h.recovery.seal_current().map_err(debug)?;
                let authority=h.root.open_character_authority(&seal).await.map_err(debug)?;
                // Allow the negative control to reach PostgreSQL's deadlock detector.
                let _test_deadline=h.root.boot_pass_deadline(std::time::Duration::from_secs(15));
                let kill=async {
                    use crate::durability::bestiary_progress::{BestiaryKillRequest,BestiaryKillOccurrence,BestiaryKillOutcome};
                    let request=BestiaryKillRequest {
                        occurrence:BestiaryKillOccurrence::from_bytes(id(201)).map_err(debug)?,
                        race:crate::domain::bestiary::BestiaryRace::new("oteryn:creature.rat","definition-r1",vec![1]).map_err(debug)?,
                        context:crate::bestiary_postgres_harness::context(),
                        policy_revision:"policy-1".into(), reward_revision:"reward-1".into(),
                    };
                    let outcome=h.root.commit_bestiary_kill(&authority,&h.node,crate::bestiary_postgres_harness::fence(1)?,request).await.map_err(debug)?;
                    assert!(matches!(outcome,BestiaryKillOutcome::Committed(_)));
                    Ok::<_,Box<dyn std::error::Error>>(())
                };
                let refresh=async {
                    tokio::time::timeout(std::time::Duration::from_secs(5),async {
                        loop {
                            let waiting:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE $1=ANY(pg_blocking_pids(pid)))").bind(barrier_pid).fetch_one(&mut *barrier).await?;
                            if waiting { break; } tokio::task::yield_now().await;
                        }
                        Ok::<_,sqlx::Error>(())
                    }).await??;
                    let mut catalogue=h.pool.acquire().await?;
                    let catalogue_pid:i32=sqlx::query_scalar("SELECT pg_backend_pid()").fetch_one(&mut *catalogue).await?;
                    let update=async {
                        sqlx::query("UPDATE game_public_v2_achievement_catalogue SET revision=revision||'.refresh' WHERE singleton").execute(&mut *catalogue).await?;
                        Ok::<_,sqlx::Error>(())
                    };
                    let release=async {
                        tokio::time::timeout(std::time::Duration::from_secs(5),async {
                            loop {
                                let blocked:bool=sqlx::query_scalar("SELECT cardinality(pg_blocking_pids($1))>0").bind(catalogue_pid).fetch_one(&mut *barrier).await?;
                                if blocked { break; } tokio::task::yield_now().await;
                            }
                            Ok::<_,sqlx::Error>(())
                        }).await??;
                        sqlx::query("SELECT pg_advisory_unlock(3185001)").execute(&mut *barrier).await?;
                        Ok::<_,Box<dyn std::error::Error>>(())
                    };
                    let (updated,released)=tokio::join!(update,release); released?; updated?;
                    Ok::<_,Box<dyn std::error::Error>>(())
                };
                let (kill,refresh)=tokio::join!(kill,refresh);
                refresh?; kill?;
                let earned:i64=sqlx::query_scalar("SELECT count(*) FROM game_account_achievements WHERE achievement_key='oteryn:achievement/hunting_permit'").fetch_one(&h.pool).await?;
                assert_eq!(earned,1,"the competing real kill must actually grant an achievement");
    Ok(())
}
#[test]
fn public_v2_catalogue_refresh_and_bestiary_do_not_deadlock() -> TestResult {
    let Some(admin)=configured_admin() else { return Ok(()); };
    std::thread::Builder::new().stack_size(64<<20).spawn(move || -> Result<(),String> {
        runtime().map_err(debug)?.block_on(async move {
            for negative in [true,false] {
                let h=Harness::create(admin.clone(),if negative {"p3_old_lock_cycle"} else {"p3_catalogue_lock_order"},true).await?;
                prepare(&h).await?;
                if negative {
                    // Restore the two reviewed lock-order defects in this isolated DB.
                    let definition:String=sqlx::query_scalar("SELECT pg_get_functiondef('game_public_v2_character_capture(uuid,text)'::regprocedure)").fetch_one(&h.pool).await?;
                    let old=definition.replace("PERFORM 1 FROM game_public_v2_achievement_catalogue WHERE singleton FOR SHARE;", "IF f='character_achievements' THEN PERFORM 1 FROM game_public_v2_achievement_catalogue WHERE singleton FOR SHARE; END IF;");
                    assert_ne!(old,definition,"negative control must restore the reviewed lock order");
                    sqlx::query(sqlx::AssertSqlSafe(old)).execute(&h.pool).await?;
                    h.pool.execute("CREATE OR REPLACE FUNCTION game_public_v2_catalogue_source() RETURNS TRIGGER LANGUAGE plpgsql SECURITY DEFINER AS $$ DECLARE c RECORD; BEGIN FOR c IN SELECT character_id FROM game_character_roots WHERE lifecycle=1 ORDER BY character_id LOOP PERFORM game_public_v2_character_capture(c.character_id,'character_achievements'); END LOOP; RETURN NULL; END $$").await?;
                }
                let result=catalogue_lock_race(&h).await;
                h.cleanup().await?;
                if negative {
                    let error=result.err().ok_or("reviewed lock cycle unexpectedly succeeded")?;
                    assert!(error.to_string().contains("40P01") || error.to_string().contains("deadlock detected"),"wrong negative-control failure: {error}");
                    println!("P3_REVIEW_LOCK_ORDER_RED: {error}");
                } else { result?; }
            }
            Ok::<_,Box<dyn std::error::Error>>(())
        }).map_err(debug)
    })?.join().map_err(|_|"catalogue concurrency case failed")?.map_err(Into::into)
}

#[test]
fn public_v2_catalogue_refresh_is_bounded_and_achievement_grants_coalesce() -> TestResult {
    let Some(admin)=configured_admin() else { return Ok(()); };
    std::thread::Builder::new().stack_size(64<<20).spawn(move || -> Result<(),String> {
        runtime().map_err(debug)?.block_on(async move {
            let h=Harness::create(admin,"p3_catalogue_pages",true).await?;
            let result:TestResult=async {
                prepare(&h).await?;
                let seal=h.recovery.seal_current().map_err(debug)?;
                let authority=h.root.open_character_authority(&seal).await.map_err(debug)?;
                history(&h,&["INSERT INTO game_character_roots(character_id,account_id,world_id,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,name,sex) SELECT ('019a0000-0000-7000-8000-'||lpad(to_hex(n),12,'0'))::uuid,account_id,world_id,1,1,profile_revision,ruleset_revision,content_revision,starter_template_revision,'Catalogue Page '||chr(65+n/26)||chr(65+n%26),sex FROM game_character_roots CROSS JOIN generate_series(1000,1031) n WHERE character_id=encode($1,'hex')::uuid"]).await?;
                for function in ["game_public_projection_resync(text,boolean)","game_public_projection_resync_before_character_views(text,boolean)"] {
                    for role in ["oteryn_game_runtime","oteryn_game_control"] {
                        let allowed:bool=sqlx::query_scalar("SELECT has_function_privilege($1,$2,'EXECUTE')").bind(role).bind(function).fetch_one(&h.pool).await?;
                        assert!(!allowed,"{function} must remain owner-only for {role}");
                    }
                }
                let before=head(&h,"character_achievements").await?.0;
                sqlx::query("UPDATE game_public_v2_achievement_catalogue SET revision='refresh-required' WHERE singleton").execute(&h.pool).await?;
                assert_eq!(head(&h,"character_achievements").await?.0,before,"catalogue trigger must never synchronously scan subjects");
                h.pool.execute("CREATE FUNCTION p3_runtime_deadline() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF current_setting('statement_timeout')::interval>interval '2 seconds' THEN RAISE EXCEPTION 'runtime used the operator deadline'; END IF; RETURN NEW; END $$; CREATE TRIGGER p3_runtime_deadline BEFORE INSERT ON game_public_v2_outbox FOR EACH ROW EXECUTE FUNCTION p3_runtime_deadline()").await?;
                assert!(h.root.sync_public_achievement_catalogue(&authority).await.is_err(),"one ordinary pass cannot finish 33 subjects");
                let complete:bool=sqlx::query_scalar("SELECT complete FROM game_public_v2_character_baselines WHERE family='character_achievements'").fetch_one(&h.pool).await?;
                assert!(!complete);
                let pending:i64=sqlx::query_scalar("SELECT game_public_v2_source_pending('character_achievements')").fetch_one(&h.pool).await?;
                assert_eq!(pending,0,"coverage must hold the watermark until the last page");
                let mut completed=false;
                for _ in 0..4 { if h.root.sync_public_achievement_catalogue(&authority).await.is_ok() {completed=true;break;} }
                assert!(completed,"bounded pages eventually cover every subject");
                h.pool.execute("DROP TRIGGER p3_runtime_deadline ON game_public_v2_outbox; DROP FUNCTION p3_runtime_deadline()").await?;
                let before=head(&h,"character_achievements").await?.0;
                history(&h,&["INSERT INTO game_account_achievement_grant_requests SELECT 'fixture',decode('01','hex'),k.key,k.value->>'revision',r.account_id,r.character_id,1790000000000 FROM game_character_roots r CROSS JOIN game_public_v2_achievement_catalogue c CROSS JOIN LATERAL (SELECT key,value FROM jsonb_each(c.entries) ORDER BY key LIMIT 3) k WHERE r.character_id=encode($1,'hex')::uuid", "INSERT INTO game_account_achievements SELECT account_id,achievement_key,source_kind,source_event_id FROM game_account_achievement_grant_requests WHERE character_id=encode($1,'hex')::uuid"]).await?;
                assert_eq!(head(&h,"character_achievements").await?.0,before+1,"three deferred grants produce one account/subject capture");
                assert_eq!(head(&h,"character_achievements").await?.1[0]["earned"].as_array().ok_or("earned")?.len(),3);
                let indexed:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_indexes WHERE indexname='game_public_v2_character_account' AND indexdef LIKE '%(account_id, character_id)%')").fetch_one(&h.pool).await?;
                assert!(indexed);
                Ok(())
            }.await;
            h.cleanup().await?;result
        }).map_err(debug)
    })?.join().map_err(|_|"bounded catalogue case failed")?.map_err(Into::into)
}
