//! The deferred NPC MINT proof uses the producer's exact command occurrence.
use crate::bestiary_postgres_harness as fixture;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

#[test]
fn daily_npc_mint_proof_matches_command_identity() -> fixture::TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            fixture::runtime()
                .map_err(fixture::debug)?
                .block_on(async {
                    let harness =
                        fixture::Harness::create(admin, "daily_npc_mint_proof", false).await?;
                    let result = exercise(&harness).await;
                    harness.cleanup().await?;
                    result?;
                    eprintln!("DAILY_NPC_MINT_PG_EXECUTED claim_identity");
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(fixture::debug)
        })?
        .join()
        .map_err(|_| "daily NPC MINT proof worker panicked")?
        .map_err(Into::into)
}

async fn exercise(harness: &fixture::Harness) -> fixture::TestResult {
    const DANGEROUS_DEPTHS: &str = "oteryn:quest.dangerous_depths_quest";
    const SPIKE: &str = "oteryn:quest.spike_task";
    let first = fixture::id(fixture::SESSION);
    let second = fixture::id(fixture::SESSION + 1);
    for (quest, rule) in [
        (
            DANGEROUS_DEPTHS,
            "oteryn:quest-npc/dangerous_depths/gnomus/measurements-report",
        ),
        (
            DANGEROUS_DEPTHS,
            "oteryn:quest-npc/dangerous_depths/gnomus/charting-report-1",
        ),
        (
            DANGEROUS_DEPTHS,
            "oteryn:quest-npc/dangerous_depths/gnomus/charting-report-2",
        ),
        (
            DANGEROUS_DEPTHS,
            "oteryn:quest-npc/dangerous_depths/klom_stonecutter/subterranean-report",
        ),
        (
            SPIKE,
            "oteryn:quest-npc/spike/gnomilly/upper-pacifier-accept",
        ),
        (
            SPIKE,
            "oteryn:quest-npc/spike/gnombold/middle-mushroom-accept",
        ),
        (
            SPIKE,
            "oteryn:quest-npc/spike/gnombold/middle-charge-accept",
        ),
    ] {
        let plan = json!({"quest": quest});
        for (session, command) in [
            (first, 1),
            (first, 2),
            (second, 1),
            (first, 1 << 63),
            (first, u64::MAX),
        ] {
            let digest = Sha256::new()
                .chain_update(rule)
                .chain_update(b"/daily/")
                .chain_update(session)
                .chain_update(command.to_be_bytes())
                .finalize();
            assert_eq!(
                proof_key(harness, &plan, rule, session, command).await?,
                format!("oteryn:reward-claim.npc-program.{digest:x}"),
                "{rule}, command {command}"
            );
        }
    }

    let permanent = "oteryn:quest-npc/a_father_s_burden_quest/tereban_functions/mission-complete";
    let plan = json!({"quest": "oteryn:quest.a_father_s_burden_quest"});
    let permanent_digest = Sha256::digest(permanent);
    for (session, command) in [(first, 1), (second, 2)] {
        assert_eq!(
            proof_key(harness, &plan, permanent, session, command).await?,
            format!("oteryn:reward-claim.npc-program.{permanent_digest:x}")
        );
    }

    let cooking = "oteryn:quest-npc/hot_cuisine_quest/jean_pierre/native-cook-1";
    let plan = json!({
        "quest": "oteryn:quest.hot_cuisine_quest",
        "tracks": [{
            "key": "oteryn:quest-progress/quest/u8_5/hot_cuisine_quest/kv/cookcount-2",
            "before": 1,
            "after": 2,
        }],
    });
    let cooking_digest = Sha256::digest(format!("{cooking}/2"));
    assert_eq!(
        proof_key(harness, &plan, cooking, first, 1).await?,
        format!("oteryn:reward-claim.npc-program.{cooking_digest:x}")
    );

    let bigfoot = "oteryn:quest-npc/bigfoot_s_burden_quest/commander_stone/branch-73-4-1-1";
    let plan = json!({"quest": "oteryn:quest.bigfoot_s_burden_quest"});
    let digest = Sha256::new()
        .chain_update(b"oteryn:quest-exchange:v1\0")
        .chain_update(fixture::id(fixture::CHARACTER))
        .chain_update(first)
        .chain_update(format!("1:{bigfoot}"))
        .finalize();
    assert_eq!(
        proof_key(harness, &plan, bigfoot, first, 1).await?,
        format!("oteryn:quest-exchange/{digest:x}")
    );
    Ok(())
}

async fn proof_key(
    harness: &fixture::Harness,
    plan: &Value,
    rule: &str,
    session: [u8; 16],
    command: u64,
) -> Result<String, sqlx::Error> {
    sqlx::query_scalar("SELECT game_quest_npc_reward_claim_key($1,$2,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5::text::numeric)")
        .bind(plan)
        .bind(rule)
        .bind(fixture::id(fixture::CHARACTER).as_slice())
        .bind(session.as_slice())
        .bind(command.to_string())
        .fetch_one(&harness.pool)
        .await
}
