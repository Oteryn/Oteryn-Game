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
struct LookContent;
impl crate::domain::appearance::AppearanceContent<String> for LookContent {
    fn palette_size(&self)->u16 {133}
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
