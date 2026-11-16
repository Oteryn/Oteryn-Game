//! Exercise the shared GUI/Aleta durable editor under the production runtime grants.
use crate::bestiary_postgres_harness::{Harness, TestResult, debug, fence, id, runtime};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::house_spell_acl::{self as acl, HouseEditor};
use crate::durability::spell_house_abi::HouseList;
use crate::durability::spell_item_transaction::{self as items, SpellItemError};
use crate::durability::spell_owner_commit::SpellLanePermit;
use crate::foundation::{CommandId, CommandRef};

const HOUSE: &str = "oteryn:content.house.runtime_acl_test";

#[test]
fn house_spell_acl_runtime_role_edit_and_ownership_revision_fence_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let admin = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL")?;
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            (|| -> TestResult { runtime()?.block_on(run_case(admin)) })()
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "House ACL PostgreSQL worker failed")?
        .map_err(Into::into)
}

async fn run_case(admin: String) -> TestResult {
    let started = std::time::Instant::now();
    let h = Harness::create(admin, "house_spell_acl_runtime", true).await?;
    sqlx::query(include_str!("../owned_house_fixture.sql"))
        .bind(id(42).as_slice())
        .bind(HOUSE)
        .bind(id(41).as_slice())
        .execute(&h.pool)
        .await?;
    let grants: (bool, bool, bool, bool) = sqlx::query_as("SELECT has_table_privilege('oteryn_game_runtime','game_house_ownership','SELECT'),has_table_privilege('oteryn_game_runtime','game_house_ownership','UPDATE'),has_table_privilege('oteryn_game_runtime','game_house_acl','UPDATE'),has_table_privilege('oteryn_game_runtime','game_house_editors','UPDATE')").fetch_one(&h.pool).await?;
    assert_eq!(grants, (true, false, true, true));
    {
        let seal = h.recovery.seal_current().map_err(debug)?;
        let recovery = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let editor = edit(&h, &recovery, 1, None).await?.map_err(debug)?;
        let sibling = edit(&h, &recovery, 2, None).await?.map_err(debug)?;
        let name: String = sqlx::query_scalar(
            "SELECT name FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
        )
        .bind(id(41).as_slice())
        .fetch_one(&h.pool)
        .await?;
        edit(&h, &recovery, 3, Some((&editor, &name)))
            .await?
            .map_err(debug)?;
        let stale_acl = edit(&h, &recovery, 4, Some((&sibling, "*"))).await?;
        assert!(
            matches!(
                stale_acl,
                Err(SpellItemError::Rejected("stale House ACL revision"))
            ),
            "{stale_acl:?}"
        );
        let editor = edit(&h, &recovery, 5, None).await?.map_err(debug)?;
        assert_eq!(editor.acl_revision, 1);
        assert_eq!(editor.members, vec![id(41)]);
        assert_eq!(editor.text, format!("{name}\n"));
        // A separate ownership transaction commits after the editor's read.
        // Keep the owner identical so only the revision fence can reject it.
        let mut transfer = h.pool.begin().await?;
        sqlx::query("WITH property AS (UPDATE game_house_properties SET state=1,owner_character_id=NULL,owner_account_id=NULL,ownership_generation=ownership_generation+1,revision=revision+1 WHERE house_key=$1 RETURNING *), slot AS (DELETE FROM game_account_housing_slots s USING property p WHERE (s.world_id,s.house_key)=(p.world_id,p.house_key)) UPDATE game_house_ownership a SET owner_character_id=NULL,ownership_revision=p.ownership_generation FROM property p WHERE (a.world_id,a.house_key)=(p.world_id,p.house_key)")
            .bind(HOUSE).execute(&mut *transfer).await?;
        sqlx::query("WITH property AS (UPDATE game_house_properties p SET state=3,owner_character_id=r.character_id,owner_account_id=r.account_id,ownership_generation=p.ownership_generation+1,revision=p.revision+1,acquisition_receipt_ref=game_character_uuid_v7() FROM game_character_roots r WHERE r.character_id=encode($2,'hex')::uuid AND p.house_key=$1 RETURNING p.*), slot AS (INSERT INTO game_account_housing_slots(account_id,world_id,house_key) SELECT owner_account_id,world_id,house_key FROM property) UPDATE game_house_ownership a SET owner_character_id=p.owner_character_id,ownership_revision=p.ownership_generation,acquisition_receipt_ref=p.acquisition_receipt_ref FROM property p WHERE (a.world_id,a.house_key)=(p.world_id,p.house_key)")
            .bind(HOUSE).bind(id(41).as_slice()).execute(&mut *transfer).await?;
        transfer.commit().await?;
        let result = edit(&h, &recovery, 6, Some((&editor, "*"))).await?;
        assert!(
            matches!(
                result,
                Err(SpellItemError::Rejected(
                    "House access changed after editor opening"
                ))
            ),
            "{result:?}"
        );
        let state: (String, i64) = sqlx::query_as("SELECT revision::text,(SELECT count(*) FROM game_house_acl_receipts WHERE house_key=$1) FROM game_house_acl WHERE house_key=$1 AND list_id=-1").bind(HOUSE).fetch_one(&h.pool).await?;
        assert_eq!(state, ("1".into(), 1));
    }
    h.cleanup().await?;
    eprintln!(
        "HOUSE_ACL_RUNTIME_PG_EXECUTED edit_and_revision_fence elapsed_ms={}",
        started.elapsed().as_millis()
    );
    Ok(())
}

async fn edit(
    h: &Harness,
    recovery: &ReconciledCharacterAuthority<'_, '_>,
    command: u64,
    save: Option<(&HouseEditor, &str)>,
) -> TestResult<Result<HouseEditor, SpellItemError>> {
    let session = crate::foundation::GameSessionId::decode(&id(50)).map_err(debug)?;
    let fence = fence(1)?.item_fence();
    let permit = SpellLanePermit::of_fresh_scope(fence.runtime_scope).await?;
    let command = CommandRef::new(session, CommandId::new(command).map_err(debug)?);
    let mut context = (h, recovery, permit, save);
    Ok(h.root.try_issue_semantic_pass()?.run_with_context(&mut context, move |holder, deadline, (h, recovery, permit, save)| {
        let save = *save;
        let pool = &h.pool;
        Box::pin(async move {
            let mut tx = items::begin_spell_owner_transaction(holder, deadline).await?;
            sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
                .execute(&mut *tx).await?;
            let role: String = sqlx::query_scalar("SELECT current_user::text").fetch_one(&mut *tx).await?;
            assert_eq!(role, "oteryn_game_runtime");
            let result = async {
                let authority = items::assert_spell_item_authority_in_transaction(&mut tx, permit, &h.root, recovery, &h.node, &fence, command, [1;32]).await?;
                if let Some((editor, text)) = save {
                    let (_, _, revision) = acl::save_editor_in_transaction(&mut tx, &authority, editor.editor_id, text).await?;
                    assert_eq!(revision, editor.acl_revision + 1);
                    Ok(editor.clone())
                } else {
                    acl::open_house_gui_in_transaction(&mut tx, &authority, HOUSE, HouseList::Guest).await
                }
            }.await;
            if result.is_ok() {
                // A concurrent owner writer cannot pass the property lock between
                // our ownership read and commit (NOWAIT proves this without sleeps).
                let mut competing = pool.begin().await?;
                let blocked = sqlx::query("SELECT house_key FROM game_house_properties WHERE house_key=$1 FOR UPDATE NOWAIT")
                    .bind(HOUSE).fetch_all(&mut *competing).await;
                assert_eq!(blocked.as_ref().err().and_then(sqlx::Error::as_database_error).and_then(|e| e.code()).as_deref(), Some("55P03"), "concurrent property mutation must be fenced: {blocked:?}");
                competing.rollback().await?;
                tx.commit().await?;
            } else { tx.rollback().await?; }
            Ok(result)
        })
    }).await?)
}

const OTHER: &str = "oteryn:content.house.runtime_acl_z";

#[test]
fn house_spell_acl_opposite_house_lock_order_postgres() -> TestResult {
    lock_order_case(false)
}

#[test]
fn house_spell_acl_property_before_editor_lock_postgres() -> TestResult {
    lock_order_case(true)
}

fn lock_order_case(editor: bool) -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            (|| -> TestResult {
                runtime()?.block_on(async {
                    let h = Harness::create(
                        admin,
                        if editor {
                            "acl_editor_order"
                        } else {
                            "acl_house_order"
                        },
                        true,
                    )
                    .await?;
                    sqlx::query(include_str!("../owned_house_fixture.sql"))
                        .bind(id(42).as_slice())
                        .bind(HOUSE)
                        .bind(id(41).as_slice())
                        .execute(&h.pool)
                        .await?;
                    let result = if editor {
                        editor_order(&h).await
                    } else {
                        property_order(&h).await
                    };
                    h.cleanup().await?;
                    result
                })
            })()
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "House lock-order PostgreSQL worker failed")?
        .map_err(Into::into)
}

// Observe the actual database lock wait, rather than relying on scheduler timing.
async fn property_wait(
    pool: &sqlx::PgPool,
    completed: &mut tokio::sync::oneshot::Receiver<()>,
) -> TestResult {
    loop {
        let waiting: bool = tokio::select! {
            waiting = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE datname=current_database() AND state='active' AND wait_event_type='Lock' AND query LIKE '%FROM game_house_property_lock(%')").fetch_one(pool) => waiting?,
            _ = &mut *completed => return Err("operation ended before the property lock wait".into()),
        };
        if waiting {
            return Ok(());
        }
        tokio::task::yield_now().await;
    }
}

async fn property_order(h: &Harness) -> TestResult {
    sqlx::query("WITH property AS (INSERT INTO game_house_properties(world_id,house_key,kind,state,rent_gold,content_digest) VALUES(encode($1,'hex')::uuid,$2,'private_house',1,100,decode(repeat('01',32),'hex')) RETURNING *) INSERT INTO game_house_ownership(world_id,house_key,owner_character_id,ownership_revision,acquisition_receipt_ref) SELECT world_id,house_key,NULL,ownership_generation,game_character_uuid_v7() FROM property")
        .bind(id(42).as_slice()).bind(OTHER).execute(&h.pool).await?;
    let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
    let mut blocker = h.pool.begin().await?;
    // One role starts from H1; the other requests H2 then H1. Once it is
    // observed waiting for H1, it must not hold H2. The duplicate H1 also
    // exercises the same-House request supported by the production kick path.
    acl::lock_house_properties(&mut blocker, world, [HOUSE])
        .await
        .map_err(debug)?;
    let (completed, mut completion) = tokio::sync::oneshot::channel();
    let opposite = async {
        let result = async {
            let mut tx = h.pool.begin().await?;
            sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
                .execute(&mut *tx)
                .await?;
            acl::lock_house_properties(&mut tx, world, [OTHER, HOUSE, HOUSE])
                .await
                .map_err(debug)?;
            tx.rollback().await?;
            Ok::<_, Box<dyn std::error::Error>>(())
        }
        .await;
        let _ = completed.send(());
        result
    };
    let probe = async {
        property_wait(&h.pool, &mut completion).await?;
        let mut tx = h.pool.begin().await?;
        let free = sqlx::query(
            "SELECT house_key FROM game_house_properties WHERE house_key=$1 FOR UPDATE NOWAIT",
        )
        .bind(OTHER)
        .fetch_one(&mut *tx)
        .await;
        tx.rollback().await?;
        blocker.rollback().await?;
        Ok::<_, Box<dyn std::error::Error>>(free.is_ok())
    };
    let (opposite, probe) = tokio::join!(opposite, probe);
    opposite?;
    assert!(
        probe?,
        "opposite caster/target order locked H2 before waiting for H1"
    );
    eprintln!("HOUSE_ACL_PG_EXECUTED opposite_house_lock_order PASS");
    Ok(())
}

async fn editor_order(h: &Harness) -> TestResult {
    let seal = h.recovery.seal_current().map_err(debug)?;
    let recovery = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let editor = edit(h, &recovery, 1, None).await?.map_err(debug)?;
    let mut blocker = h.pool.begin().await?;
    sqlx::query("SELECT house_key FROM game_house_properties WHERE house_key=$1 FOR UPDATE")
        .bind(HOUSE)
        .fetch_one(&mut *blocker)
        .await?;
    let (completed, mut completion) = tokio::sync::oneshot::channel();
    let save = async {
        let result = edit(h, &recovery, 2, Some((&editor, "*"))).await;
        let _ = completed.send(());
        result
    };
    let probe = async {
        property_wait(&h.pool, &mut completion).await?;
        let mut tx = h.pool.begin().await?;
        let free = sqlx::query("SELECT editor_id FROM game_house_editors WHERE editor_id=encode($1,'hex')::uuid FOR UPDATE NOWAIT").bind(editor.editor_id.as_slice()).fetch_one(&mut *tx).await;
        let unlocked = free.is_ok();
        if unlocked {
            // Model release deleting the token while it holds the property.
            sqlx::query("DELETE FROM game_house_editors WHERE editor_id=encode($1,'hex')::uuid")
                .bind(editor.editor_id.as_slice())
                .execute(&mut *tx)
                .await?;
            tx.commit().await?;
        } else {
            tx.rollback().await?;
        }
        blocker.rollback().await?;
        Ok::<_, Box<dyn std::error::Error>>(unlocked)
    };
    let (save, probe) = tokio::join!(save, probe);
    let save = save?;
    assert!(
        probe?,
        "save locked the editor before waiting for its property"
    );
    assert!(
        matches!(
            save,
            Err(SpellItemError::Rejected("House editor/session expired"))
        ),
        "deleted editor was not revalidated: {save:?}"
    );
    eprintln!("HOUSE_ACL_PG_EXECUTED property_before_editor_lock PASS");
    Ok(())
}
