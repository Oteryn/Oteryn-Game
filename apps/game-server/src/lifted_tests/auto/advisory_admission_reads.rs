//! Advisory reads use MVCC while an admission writer holds the relation locks.
use crate::bestiary_postgres_harness::{self as fixture, Harness, TestResult, debug};

#[test]
fn advisory_channel_reads_complete_under_exclusive_admission_locks() -> TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new().stack_size(64 << 20).spawn(move || -> Result<(), String> {
        fixture::runtime().map_err(debug)?.block_on(async {
            let h = Harness::create(admin, "advisory_admission", true).await?;
            let result: TestResult = async {
                let seal = h.recovery.seal_current().map_err(debug)?;
                let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
                let fence = fixture::fence(1)?;
                let mut admission = h.pool.begin().await?;
                sqlx::query("LOCK TABLE game_durability_admission_account_guards, game_durability_reconnect_sessions IN EXCLUSIVE MODE")
                    .execute(&mut *admission).await?;

                // AccessShare readers can proceed while the exclusive lock is still held.
                h.root.await_ready().await?;
                assert_eq!(h.root.read_guild_emblems(&authority, &h.node, fence.item_fence(), None, Vec::new()).await?, Some(Vec::new()));
                h.root.await_ready().await?;
                h.root.read_blessings_dialog(&authority, &h.node, fence, false).await.map_err(debug)?;
                h.root.await_ready().await?;
                let holdings = h.root.read_cyclopedia_item_summary(&authority, &h.node, fence.item_fence(), 0).await.map_err(debug)?;
                assert!(holdings.rows.is_empty());
                admission.rollback().await?;
                Ok(())
            }.await;
            h.cleanup().await?;
            result
        }).map_err(debug)
    })?.join().map_err(|_| "advisory reads thread failed")?.map_err(Into::into)
}

struct EmptyInventory;
impl crate::durability::item_transfer::ItemWeightLookup for EmptyInventory {
    fn generation_digest(&self) -> [u8; 32] {
        [1; 32]
    }

    fn weight_of(
        &self,
        _: &crate::durability::item_mint::TypedDefinitionRef,
    ) -> Result<u32, crate::durability::item_transfer::ItemWeightRefused> {
        Err(crate::durability::item_transfer::ItemWeightRefused::Unknown)
    }
}

#[test]
fn advisory_house_reads_complete_under_exclusive_admission_locks() -> TestResult {
    use super::bed_sleep_postgres_cases as house;
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new().stack_size(64 << 20).spawn(move || -> Result<(), String> {
        fixture::runtime().map_err(debug)?.block_on(async {
            let h = house::fixture("advisory_house", &admin).await?;
            let result: TestResult = async {
                sqlx::query("INSERT INTO game_character_progression_state VALUES(encode($1,'hex')::uuid,1,50,1000,'profile-1','ruleset-1','content-1','simulation-1','evidence-1','declaration-1','policy-1','reward-1')")
                    .bind(fixture::id(fixture::CHARACTER).as_slice()).execute(&h.pool).await?;
                let seal = h.recovery.seal_current().map_err(debug)?;
                let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
                let current = house::request()?.fence;
                let origin = match fixture::scope()? {
                    crate::foundation::RuntimeScopeRefV1::Channel { channel_id, .. } => *channel_id.as_bytes(),
                    _ => return Err("expected a Channel origin".into()),
                };
                let mut admission = h.pool.begin().await?;
                sqlx::query("LOCK TABLE game_durability_admission_account_guards, game_durability_reconnect_sessions IN EXCLUSIVE MODE")
                    .execute(&mut *admission).await?;
                h.root.await_ready().await?;
                let capacity = h.root.read_inventory_capacity(&authority, &h.node, current, Some(origin), &EmptyInventory).await?;
                assert!(capacity.is_some_and(|(capacity, carried)| capacity > 0 && carried == 0));
                h.root.await_ready().await?;
                let emblems = h.root.read_guild_emblems(&authority, &h.node, current, Some(origin), Vec::new()).await?;
                assert_eq!(emblems, Some(Vec::new()));
                h.root.await_ready().await?;
                assert_eq!(h.root.read_guild_emblems(&authority, &h.node, current, Some([0; 16]), Vec::new()).await?, None);
                admission.rollback().await?;
                Ok(())
            }.await;
            h.cleanup().await?;
            result
        }).map_err(debug)
    })?.join().map_err(|_| "House advisory reads thread failed")?.map_err(Into::into)
}
