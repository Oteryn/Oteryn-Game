//! MAIL enters the shared admission relation order before reading its session fence.
use crate::bestiary_postgres_harness::{TestResult, configured_admin, runtime};
use crate::durability::item_text::ItemTextWriteRequest;
use crate::durability::item_transfer::{
    ItemStackClass, ItemTransferOutcome, ItemWeightLookup, ItemWeightRefused,
};
use crate::durability::mail::MailPostOutcome;
use crate::durability::spell_owner_commit::SpellLanePermit;
use crate::item_transfer_postgres_cases as native;
use std::time::Duration;

const LETTER: &str = "oteryn:item.tibia.i3505";

// Explicit fixture extension: the shared synthetic catalogue has no letter.
struct MailWeights;
impl ItemWeightLookup for MailWeights {
    fn generation_digest(&self) -> [u8; 32] {
        crate::durability::item_weight_lookup::TEST_WEIGHTS.generation_digest()
    }

    fn weight_of(
        &self,
        definition: &crate::durability::item_mint::TypedDefinitionRef,
    ) -> Result<u32, ItemWeightRefused> {
        if definition == &native::definition("Item", LETTER) {
            Ok(1)
        } else {
            crate::durability::item_weight_lookup::TEST_WEIGHTS.weight_of(definition)
        }
    }
}

#[test]
fn mail_waits_for_admission_relations_before_session_reads_and_replays_once() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    let result = std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || run(admin).map_err(|error| error.to_string()))?
        .join();
    match result {
        Ok(result) => result.map_err(Into::into),
        Err(panic) => std::panic::resume_unwind(panic),
    }
}

fn run(admin: String) -> TestResult {
    runtime()?.block_on(async {
        let h = native::Harness::create(admin, "mail_admission_order").await?;
        let result = post_after_relation_wait(&h).await;
        h.cleanup().await?;
        result
    })
}

async fn post_after_relation_wait(h: &native::Harness) -> TestResult {
    let seal = h.recovery.seal_current().map_err(native::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(native::debug)?;
    let backpack = h.mint(&authority, native::BACKPACK, 1).await?;
    h.committed(
        &authority,
        native::to_slot(native::command(11)?, backpack, native::backpack_facts()),
    )
    .await?;
    let letter = h.mint(&authority, LETTER, 1).await?;
    let fence = native::fence()?;
    let mut transfer = h
        .root
        .freeze_item_transfer(
            &authority,
            &h.node,
            fence,
            native::to_backpack(
                native::command(12)?,
                letter,
                native::facts(LETTER, ItemStackClass::NonStackable),
            ),
            &MailWeights,
        )
        .await
        .map_err(native::debug)?;
    assert!(matches!(
        h.root
            .commit_item_transfer(
                &SpellLanePermit::of_fresh_scope(fence.runtime_scope).await?,
                &authority,
                &h.node,
                fence,
                &mut transfer,
                &MailWeights,
            )
            .await
            .map_err(native::debug)?,
        ItemTransferOutcome::Committed(_)
    ));
    // Existing storage fixture writes the address under the runtime role. The actual
    // MAIL call below still proves the current session, sender and recipient itself.
    let text = crate::durability::item_text::tests::store_fixture_request(
        &h.pool,
        &native::id(native::CHARACTER),
        ItemTextWriteRequest {
            command: native::command(13)?,
            item_instance_id: letter,
            definition: native::definition("Item", LETTER),
            text: "Fixture Hero".into(),
        },
    )
    .await?;
    assert!(text.changed);
    let before = footprint(h, letter).await?;

    // A separate transaction stands for another node's admitted writer. Each
    // DurabilityRoot has one holder, so clones of h.root cannot create this overlap.
    let mut blocker = h.pool.begin().await?;
    sqlx::query("SET LOCAL lock_timeout = '2s'")
        .execute(&mut *blocker)
        .await?;
    let blocker_pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *blocker)
        .await?;
    sqlx::query("LOCK TABLE game_durability_admission_runtime_guards IN EXCLUSIVE MODE")
        .execute(&mut *blocker)
        .await?;

    let post = h
        .root
        .post_admitted_mail(&authority, &h.node, native::fence()?, 31, letter, 1);
    let observe_and_release = async {
        let observed = tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                let waiting: Option<(String, bool)> = sqlx::query_as(
                    "SELECT waiting.mode, EXISTS(SELECT 1 FROM pg_locks held \
                     WHERE held.pid=waiting.pid AND held.locktype='relation' \
                     AND held.database=waiting.database \
                     AND held.relation='game_durability_reconnect_sessions'::regclass \
                     AND held.mode='RowShareLock' AND held.granted) \
                     FROM pg_locks waiting WHERE waiting.locktype='relation' \
                     AND waiting.database=(SELECT oid FROM pg_database WHERE datname=current_database()) \
                     AND waiting.relation='game_durability_admission_runtime_guards'::regclass \
                     AND NOT waiting.granted AND $1=ANY(pg_blocking_pids(waiting.pid)) LIMIT 1",
                )
                .bind(blocker_pid)
                .fetch_optional(&h.pool)
                .await?;
                if let Some(waiting) = waiting {
                    return Ok::<_, Box<dyn std::error::Error>>((waiting, footprint(h, letter).await?));
                }
                tokio::task::yield_now().await;
            }
        })
        .await;
        // Release even when observation failed, before checking any assertion.
        blocker.rollback().await?;
        observed.map_err(|_| "MAIL did not reach the controlled admission relation wait")?
    };
    let (posted, observed) = tokio::join!(post, observe_and_release);
    let ((lock_mode, held_session), while_waiting) = observed?;
    assert_eq!(lock_mode, "ExclusiveLock");
    assert!(
        !held_session,
        "MAIL must not retain the session lock while entering admission order"
    );
    assert_eq!(while_waiting, before);
    assert_eq!(posted?, MailPostOutcome::Posted);

    let committed = footprint(h, letter).await?;
    assert_eq!(
        h.root
            .post_admitted_mail(&authority, &h.node, native::fence()?, 31, letter, 1)
            .await?,
        MailPostOutcome::AlreadyPosted
    );
    assert_eq!(footprint(h, letter).await?, committed);
    assert_eq!(h.count("game_mail_operations").await?, 1);
    assert_eq!(h.count("game_mail_outbox").await?, 1);
    let stamp: String = sqlx::query_scalar(
        "SELECT definition_production_key FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(letter.as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(stamp, "oteryn:item.tibia.i3506");
    Ok(())
}

async fn footprint(h: &native::Harness, item: [u8; 16]) -> TestResult<serde_json::Value> {
    Ok(sqlx::query_scalar(
        "SELECT jsonb_build_array( \
         (SELECT to_jsonb(i) FROM game_item_instances i WHERE item_instance_id=encode($1,'hex')::uuid), \
         (SELECT to_jsonb(e) FROM game_item_container_entries e WHERE item_instance_id=encode($1,'hex')::uuid), \
         (SELECT to_jsonb(c) FROM game_character_inbox_counters c WHERE character_id=encode($2,'hex')::uuid), \
         (SELECT count(*) FROM game_mail_operations), (SELECT count(*) FROM game_mail_outbox), \
         (SELECT count(*) FROM game_character_inbox_deliveries WHERE item_instance_id=encode($1,'hex')::uuid), \
         (SELECT count(*) FROM game_item_character_inbox_locations WHERE item_instance_id=encode($1,'hex')::uuid))",
    )
    .bind(item.as_slice())
    .bind(native::id(native::CHARACTER).as_slice())
    .fetch_one(&h.pool)
    .await?)
}
