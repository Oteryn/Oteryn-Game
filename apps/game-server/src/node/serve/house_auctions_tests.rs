use super::*;
use crate::bestiary_postgres_harness::{Harness, TestResult, debug, runtime, scope};

#[test]
fn house_auction_worker_retries_listing_after_epoch_initialization() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || scenario(admin).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "house auction worker PG thread panicked")??;
    Ok(())
}

fn scenario(admin: String) -> TestResult {
    runtime()?.block_on(async {
        let h = Harness::create(admin, "auction_worker_epoch", false).await?;
        let result: TestResult = async {
            let scope = scope()?;
            let digest = [1; 32];
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
            let generation = h.root.read_runtime_scope_assignment(scope).await.map_err(debug)?
                .ok_or("channel assignment")?.ownership_generation;
            let worker = HouseAuctions {
                root: &h.root,
                authority: &authority,
                node: &h.node,
                catalogue: HouseAuctionCatalogue {
                    scope, generation, reset_epoch: 0, map_revision: "house-epoch-map", content_digest: digest,
                },
            };
            let stop = CancellationToken::new();
            let house = "oteryn:content.house.worker_epoch";
            let pending = vec![(house.into(), "private_house", 100, vec![[42, 42, -7]])];
            let retry = DurabilityRoot::with_background_holder(worker.qualify_listings(pending, &stop))
                .await.ok_or("worker stopped")?;
            assert_eq!(retry.len(), 1, "missing epoch must retain the property");
            sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month)
              VALUES(encode($1,'hex')::uuid,0,$2,'house-epoch-map',$2,$2,$2,'TIBIA_SERVER_SAVE_BERLIN_06',1,to_timestamp(123),'1970-01')")
                .bind(scope.world_id().as_bytes().as_slice()).bind(digest.as_slice()).execute(&h.pool).await?;
            sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,0)")
                .bind(scope.world_id().as_bytes().as_slice()).execute(&h.pool).await?;
            let retry = DurabilityRoot::with_background_holder(worker.qualify_listings(retry, &stop))
                .await.ok_or("worker stopped")?;
            assert!(retry.is_empty());
            let listed: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_house_current_auction_catalogue WHERE house_key=$1)")
                .bind(house).fetch_one(&h.pool).await?;
            assert!(listed);
            Ok(())
        }.await;
        let cleanup = h.cleanup().await;
        result?;
        cleanup
    })
}
