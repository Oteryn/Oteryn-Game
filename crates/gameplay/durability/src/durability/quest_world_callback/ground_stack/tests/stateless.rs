use super::*;

const WINDOW: &str = "oteryn:quest-callback/m02/nomads-land/window";

#[test]
fn m02_nomads_window_postgres_stateless_observer_requires_the_real_stack() -> h::TestResult {
    let Some(admin) = h::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            h::runtime()
                .map_err(h::debug)?
                .block_on(async {
                    let started = std::time::Instant::now();
                    let harness = h::Harness::create(admin, "srvwld_m02_window", true).await?;
                    let result = exercise_window(&harness).await;
                    harness.cleanup().await?;
                    result?;
                    assert!(started.elapsed().as_secs_f64() > 0.01);
                    eprintln!(
                        "SRVWLD_PG_EXECUTED m02_stateless_window duration={:?}",
                        started.elapsed()
                    );
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(h::debug)
        })?
        .join()
        .map_err(|_| "M02 window test thread failed")?
        .map_err(Into::into)
}

async fn exercise_window(harness: &h::Harness) -> h::TestResult {
    let digest = [44_u8; 32];
    let frame = [46_u8; 32];
    // Real control-plane issuance under the fixture's owner role; no guard disabled.
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4) ON CONFLICT DO NOTHING")
        .bind(h::id(h::WORLD).as_slice()).bind(h::id(h::CHANNEL).as_slice()).execute(&harness.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$3,$4)")
        .bind(h::id(h::WORLD).as_slice()).bind(h::id(h::CHANNEL).as_slice())
        .bind(digest.as_slice()).bind(frame.as_slice()).execute(&harness.pool).await?;
    let mut context = context();
    context.content_revision = format!(
        "sha256:{}",
        digest
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    );
    context.placement_context = frame.to_vec();
    let before_quests = (
        harness.count("game_character_quest_tracks").await?,
        harness.count("game_character_quest_receipts").await?,
    );
    assert!(
        !observe(harness, context.clone()).await?,
        "an empty window stack must refuse, without inventing quest progress"
    );
    for (index, x) in [33215, 33216].into_iter().enumerate() {
        let at = Position {
            x,
            y: 32854,
            floor: -9,
        };
        for (offset, item) in [
            "oteryn:item.tibia.i2471",
            "oteryn:item.tibia.i2471",
            "oteryn:item.tibia.i2904",
        ]
        .into_iter()
        .enumerate()
        {
            let ordinal = u32::try_from(index * 3 + offset)?;
            mint_at(harness, 1100 + ordinal, item, 1, at, &context).await?;
        }
        if index == 0 {
            assert!(
                !observe(harness, context.clone()).await?,
                "one pile cannot satisfy both window cells"
            );
        }
    }
    let before_items: Vec<(String, i64)> = sqlx::query_as(
        "SELECT item_instance_id::text,quantity FROM game_item_instances ORDER BY item_instance_id",
    )
    .fetch_all(&harness.pool)
    .await?;
    assert!(
        observe(harness, context.clone()).await?,
        "the real arranged stacks must admit"
    );
    assert!(
        observe(harness, context.clone()).await?,
        "a read-only check is repeatable"
    );
    assert_eq!(
        before_quests,
        (
            harness.count("game_character_quest_tracks").await?,
            harness.count("game_character_quest_receipts").await?
        ),
        "window checks must not fabricate progress or quest receipts"
    );
    assert_eq!(before_items,sqlx::query_as::<_,(String,i64)>("SELECT item_instance_id::text,quantity FROM game_item_instances ORDER BY item_instance_id").fetch_all(&harness.pool).await?);
    let mut stale = context.clone();
    stale.placement_context[0] ^= 1;
    assert!(
        !observe(harness, stale).await?,
        "a retained layout cannot bypass current frame evidence"
    );
    mint_at(
        harness,
        1110,
        "oteryn:item.tibia.i2471",
        1,
        Position {
            x: 33216,
            y: 32854,
            floor: -9,
        },
        &context,
    )
    .await?;
    assert!(
        !observe(harness, context).await?,
        "an extra root must invalidate the exact layout"
    );
    Ok(())
}
async fn observe(harness: &h::Harness, context: QuestWorldContext) -> h::TestResult<bool> {
    let seal = harness.recovery.seal_current().map_err(h::debug)?;
    let authority = harness
        .root
        .open_character_authority(&seal)
        .await
        .map_err(h::debug)?;
    let permit = SpellLanePermit::of_fresh_scope(h::scope()?)
        .await
        .map_err(h::debug)?;
    Ok(harness
        .root
        .observe_world_callback_gate(
            &authority,
            &harness.node,
            &permit,
            h::fence(1)?,
            context,
            WINDOW.into(),
        )
        .await?)
}
