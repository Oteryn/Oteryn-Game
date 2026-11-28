use super::*;
use sqlx::Connection;

type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

const CHARACTER: [u8; 16] = [0, 0, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 1];
const WORLD: [u8; 16] = [0, 0, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 2];

#[test]
fn whisper_moss_replacement_requires_destroyed_issues_and_no_recoverable_custody() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?
                .block_on(exercise(&admin))
        })?
        .join()
        .map_err(|_| "Whisper Moss PostgreSQL test thread failed")?
}

async fn exercise(admin: &str) -> TestResult {
    let mut conn = sqlx::PgConnection::connect(admin).await?;
    let mut tx = conn.begin().await?;
    sqlx::raw_sql(
        "CREATE TEMP TABLE game_item_container_entries(character_id uuid, item_instance_id uuid);
         CREATE TEMP TABLE game_item_container_slots(character_id uuid, item_instance_id uuid);
         CREATE TEMP TABLE game_character_equipment_slots(character_id uuid, item_instance_id uuid);
         CREATE TEMP TABLE game_item_character_inbox_locations(character_id uuid, item_instance_id uuid);
         CREATE TEMP TABLE game_item_character_depot_locations(character_id uuid, item_instance_id uuid);
         CREATE TEMP TABLE game_character_quest_tracks(character_id uuid, quest_key text, track_key text, value bigint);
         CREATE TEMP TABLE game_item_nested_entries(parent_item_instance_id uuid, item_instance_id uuid);
         CREATE TEMP TABLE game_item_instances(item_instance_id uuid, lifecycle smallint, definition_family text, definition_production_key text);
         CREATE TEMP TABLE game_reward_claims(character_id uuid, claim_family text, claim_production_key text, claimed_transaction_id uuid);
         CREATE TEMP TABLE game_reward_claim_mint_receipts(transaction_id uuid, item_instance_id uuid);
         CREATE TEMP TABLE game_item_decay_retire_receipts(item_instance_id uuid);
         CREATE TEMP TABLE game_character_stash_balances(character_id uuid, world_id uuid, definition_key text, quantity bigint);
         CREATE TEMP TABLE game_market_offers(offer_id uuid, character_id uuid, world_id uuid, definition_key text, side smallint, state smallint);
         CREATE TEMP TABLE game_item_market_escrow_locations(offer_id uuid, world_id uuid, item_instance_id uuid);",
    )
    .execute(&mut *tx)
    .await?;

    let character = CharacterId::from_bytes(CHARACTER)?;
    let mut request = super::super::tests::request(None);
    request.item.definition.production_key = ITEM.into();
    request.claim.revision_ref = "reward-claim-r1".into();
    request.claim.production_key = format!("{REPLACEMENT}2");
    request.quantity = 1;
    sqlx::query(
        "INSERT INTO game_character_quest_tracks VALUES
           (encode($1,'hex')::uuid,$2,$3,1),(encode($1,'hex')::uuid,$2,$4,2)",
    )
    .bind(CHARACTER.as_slice())
    .bind(QUEST)
    .bind(MISSION)
    .bind(RECOVERY)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO game_reward_claims VALUES
           (encode($1,'hex')::uuid,'RewardClaim',$2,'00000000-0000-7000-8000-000000000003')",
    )
    .bind(CHARACTER.as_slice())
    .bind(CLAIM)
    .execute(&mut *tx)
    .await?;
    sqlx::raw_sql(
        "INSERT INTO game_reward_claim_mint_receipts VALUES
           ('00000000-0000-7000-8000-000000000003','00000000-0000-7000-8000-000000000004');
         INSERT INTO game_item_instances VALUES
           ('00000000-0000-7000-8000-000000000004',1,'Item','oteryn:item.tibia.i4827'),
           ('00000000-0000-7000-8000-000000000005',1,'Item','oteryn:item.tibia.i4827');
         INSERT INTO game_item_container_entries VALUES
           ('00000000-0000-7000-8000-000000000006','00000000-0000-7000-8000-000000000004');",
    )
    .execute(&mut *tx)
    .await?;

    let cases = [
        ("live issue in peer custody", "", false),
        (
            "retired identity without destruction proof",
            "DELETE FROM game_item_container_entries;
             UPDATE game_item_instances SET lifecycle=2
             WHERE item_instance_id='00000000-0000-7000-8000-000000000004'",
            false,
        ),
        (
            "destroyed issue with positive stash balance",
            "INSERT INTO game_item_decay_retire_receipts VALUES ('00000000-0000-7000-8000-000000000004');
             INSERT INTO game_character_stash_balances VALUES
               ('00000000-0000-7000-8000-000000000001','00000000-0000-7000-8000-000000000002','oteryn:item.tibia.i4827',1)",
            false,
        ),
        (
            "destroyed issue with open sell escrow",
            "UPDATE game_character_stash_balances SET quantity=0;
             INSERT INTO game_market_offers VALUES
               ('00000000-0000-7000-8000-000000000007','00000000-0000-7000-8000-000000000001',
                '00000000-0000-7000-8000-000000000002','oteryn:item.tibia.i4827',2,1);
             INSERT INTO game_item_market_escrow_locations VALUES
               ('00000000-0000-7000-8000-000000000007','00000000-0000-7000-8000-000000000002','00000000-0000-7000-8000-000000000005')",
            false,
        ),
        (
            "destroyed issue with held sell escrow",
            "UPDATE game_market_offers SET state=5",
            false,
        ),
        (
            "destroyed issue with no recoverable custody",
            "DELETE FROM game_item_market_escrow_locations",
            true,
        ),
        (
            "earlier replacement still live",
            "INSERT INTO game_reward_claims SELECT character_id,claim_family,claim_production_key||'/replacement/1',
               '00000000-0000-7000-8000-000000000008' FROM game_reward_claims;
             INSERT INTO game_reward_claim_mint_receipts VALUES
               ('00000000-0000-7000-8000-000000000008','00000000-0000-7000-8000-000000000005')",
            false,
        ),
    ];
    let mut mismatches = Vec::new();
    for (case, setup, expected) in cases {
        if !setup.is_empty() {
            sqlx::raw_sql(setup).execute(&mut *tx).await?;
        }
        let allowed = permits(&mut tx, character, &WORLD, &request).await?;
        if allowed != expected {
            mismatches.push((case, allowed, expected));
        }
    }
    assert!(mismatches.is_empty(), "{mismatches:?}");
    eprintln!("POSTGRES_CASE_COMPLETED whisper_moss_replacement_custody");
    tx.rollback().await?;
    Ok(())
}
