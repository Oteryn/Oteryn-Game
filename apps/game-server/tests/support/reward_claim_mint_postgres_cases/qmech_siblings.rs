//! QMECH independent outputs: one fenced receipt, exact custody and rollback.
use super::*;
use crate::durability::reward_claim_mint::RewardClaimMintContent;

fn reward(command_id: u64) -> TestResult<RewardClaimMintRequest> {
    let mut request = chest(
        command(command_id)?,
        CLAIM,
        facts(STONE, ItemStackClass::NonStackable),
        1,
    );
    request.additional_items.push(RewardClaimMintContent {
        charges_remaining: None,
        item: facts(COIN, STACKABLE),
        quantity: 7,
        key_number: None,
        text: None,
        fluid_subtype: None,
    });
    Ok(request)
}

pub(crate) fn qmech_siblings_exact_once_replay_and_changed_intent() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    let started = std::time::Instant::now();
    runtime()?.block_on(async move {
        let h = Harness::create(admin, "qmech_siblings_once").await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
        let backpack = h.equip_backpack(&authority, 1).await?;
        let first = h.claimed(&authority, reward(10)?).await?;
        let items: Vec<(String,i64,String,String)> = sqlx::query_as(
            "SELECT i.definition_production_key,i.quantity,c.parent_item_instance_id::text,c.placement_ordinal::text FROM game_item_instances i JOIN game_item_container_entries c USING(item_instance_id) ORDER BY c.placement_ordinal")
            .fetch_all(&h.pool).await?;
        assert_eq!(items, vec![(STONE.into(),1,uuid_text(backpack),"1".into()), (COIN.into(),7,uuid_text(backpack),"2".into())]);
        assert_eq!(h.count("game_reward_claim_mint_receipts").await?,1);
        assert_eq!(h.count("game_item_nested_entries").await?,0);
        let before = h.footprint().await?;
        assert!(matches!(h.claim(&authority,fence()?,reward(10)?).await.map_err(debug)?,
            RewardClaimMintOutcome::AlreadyCommitted(ref result) if result == &first));
        refused(h.claim(&authority,fence()?,reward(11)?).await,RewardClaimRefusal::AlreadyClaimed)?;
        let mut changed = reward(10)?; changed.additional_items[0].quantity=8;
        assert!(h.root.freeze_reward_claim_mint(&authority,&h.node,fence()?,changed).await.is_err());
        assert_eq!(h.footprint().await?,before);
        drop(authority);
        drop(seal); h.cleanup().await
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED qmech_siblings_exact_once duration={:?}",
        started.elapsed()
    );
    Ok(())
}

pub(crate) fn qmech_siblings_full_bag_writes_no_partial_reward() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    let started = std::time::Instant::now();
    runtime()?.block_on(async move {
        let h = Harness::create(admin, "qmech_siblings_full").await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        h.equip_backpack(&authority, 1).await?;
        h.fill_with_stones(&authority, 2, 19).await?;
        let before = h.footprint().await?;
        refused(
            h.claim(&authority, fence()?, reward(30)?).await,
            RewardClaimRefusal::MainBackpackFull,
        )?;
        assert_eq!(h.footprint().await?, before);
        assert_eq!(h.count("game_reward_claim_mint_reservations").await?, 0);
        assert_eq!(h.count("game_reward_claim_mint_receipts").await?, 0);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED qmech_siblings_full_bag duration={:?}",
        started.elapsed()
    );
    Ok(())
}

pub(crate) fn qmech_siblings_weight_counts_all_outputs_before_claim() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    let started = std::time::Instant::now();
    runtime()?.block_on(async move {
        let h = Harness::create(admin, "qmech_siblings_weight").await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        h.equip_backpack(&authority, 1).await?;
        let mut request = reward(10)?;
        request.item = facts("fixture:capacity.exact", ItemStackClass::NonStackable);
        request.item.weight = Some(155999);
        request.content_revision = format!("sha256:{}", "d1".repeat(32));
        let before = h.footprint().await?;
        assert!(matches!(
            h.root
                .freeze_reward_claim_mint_weighted(
                    &authority,
                    &h.node,
                    fence()?,
                    request.clone(),
                    &TEST_WEIGHTS
                )
                .await,
            Err(RewardClaimMintError::Refused(
                RewardClaimRefusal::CarryingCapacity
            ))
        ));
        assert_eq!(h.footprint().await?, before);
        request.additional_items.clear();
        let mut candidate = h
            .root
            .freeze_reward_claim_mint_weighted(
                &authority,
                &h.node,
                fence()?,
                request,
                &TEST_WEIGHTS,
            )
            .await
            .map_err(debug)?;
        assert!(matches!(
            h.root
                .commit_reward_claim_mint_weighted_noticed(
                    &authority,
                    &h.node,
                    fence()?,
                    &mut candidate,
                    None,
                    &TEST_WEIGHTS
                )
                .await
                .map_err(debug)?,
            (RewardClaimMintOutcome::Committed(_), _)
        ));
        assert_eq!(h.count("game_reward_claim_mint_receipts").await?, 1);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED qmech_siblings_weight duration={:?}",
        started.elapsed()
    );
    Ok(())
}

/// Exercise the installed quest-side trigger, including its SQL name resolution.
/// Complete dispatch cases separately prove positive reward/source participants.
pub(crate) fn qmech_siblings_incomplete_source_is_an_integrity_refusal() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    let started = std::time::Instant::now();
    runtime()?.block_on(async move {
        let h = Harness::create(admin,"m09_source_guard").await?;
        let mut tx = h.pool.begin().await?;
        let key="oteryn:quest-reward/m09-incomplete-source";
        let plan=serde_json::json!({"key":key,"quest":"oteryn:quest.m09_source_fixture",
            "take":[],"give":{"item":"fixture:b3.stone","count":1},"outfit":null});
        sqlx::query("INSERT INTO game_quest_reward_sources(character_id,cause_id,cause_ordinal,transition_key,plan,mint_transaction_id,grant_keys) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,100,$3,$4,encode($5,'hex')::uuid,'{}')")
            .bind(id(CHARACTER).as_slice()).bind(id(SESSION).as_slice()).bind(key)
            .bind(plan).bind(id(151).as_slice()).execute(&mut *tx).await?;
        let error=sqlx::query("SET CONSTRAINTS game_quest_reward_complete IMMEDIATE")
            .execute(&mut *tx).await.err().ok_or("incomplete source was accepted")?;
        let database=error.as_database_error().ok_or("expected integrity error")?;
        assert_eq!(database.code().as_deref(),Some("23514"),"{error}");
        assert!(database.message().contains("quest reward lacks exact independent outputs"),"{error}");
        tx.rollback().await?;
        h.cleanup().await
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED m09_sibling_source_guard duration={:?}",
        started.elapsed()
    );
    Ok(())
}
