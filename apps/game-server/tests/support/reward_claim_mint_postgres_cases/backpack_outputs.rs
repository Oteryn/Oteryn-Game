//! Reuses PC8 eb196696c7d2606656f07cc72bf850ecdec81e10's complete output owner.
//! Every output is a peer in the real equipped backpack, with its own audit.
use super::*;
use crate::durability::reward_claim_mint::RewardClaimMintContent;

async fn claim(
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    request: RewardClaimMintRequest,
) -> Result<RewardClaimMintOutcome, RewardClaimMintError> {
    let fence = fence().map_err(|_| RewardClaimMintError::InvalidInput)?;
    let mut candidate = h
        .root
        .freeze_reward_claim_mint_weighted(authority, &h.node, fence, request, &TEST_WEIGHTS)
        .await?;
    h.root
        .commit_reward_claim_mint_weighted_noticed(
            authority,
            &h.node,
            fence,
            &mut candidate,
            None,
            &TEST_WEIGHTS,
        )
        .await
        .map(|(outcome, _)| outcome)
}

#[test]
fn questl2_peer_outputs_are_atomic_audited_and_replay_safe() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    let started = std::time::Instant::now();
    std::thread::Builder::new().stack_size(64 << 20).spawn(move || -> Result<(), String> {
        runtime().map_err(debug)?.block_on(async move {
            let h = Harness::create(admin, "l2peer").await?;
            let version: String = sqlx::query_scalar("SELECT current_setting('server_version')").fetch_one(&h.pool).await?;
            eprintln!("POSTGRES_CASE_STARTED questl2-peer-outputs server_version={version}");
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
            let backpack = h.equip_backpack(&authority, 1).await?;
            let mut request = coins(command(2)?, "fixture:l2.peer", 100);
            request.backpack_outputs = true;
            request.content_revision = format!("sha256:{}", "d1".repeat(32));
            request.contents = vec![RewardClaimMintContent { fluid_subtype: None,
                item: facts(COIN, STACKABLE), quantity: 100,
                key_number: None, text: None,
            }, RewardClaimMintContent { fluid_subtype: None,
                item: facts(STONE, ItemStackClass::NonStackable), quantity: 1,
                key_number: None, text: None,
            }];
            let first = match claim(&h, &authority, request.clone()).await.map_err(debug)? {
                RewardClaimMintOutcome::Committed(c) => c,
                other => return Err(format!("expected all peer outputs: {other:?}").into()),
            };
            assert_eq!(first.quantity, 100);
            assert_eq!(h.count("game_item_nested_entries").await?, 0);
            let rows: Vec<(String, i64)> = sqlx::query_as("SELECT i.definition_production_key,i.quantity::bigint FROM game_item_container_entries e JOIN game_item_instances i USING(item_instance_id) WHERE e.parent_item_instance_id=encode($1,'hex')::uuid ORDER BY e.placement_ordinal")
                .bind(backpack.as_slice()).fetch_all(&h.pool).await?;
            assert_eq!(rows, vec![(COIN.into(),100),(COIN.into(),100),(STONE.into(),1)]);
            let audited: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_container_entries e JOIN game_item_audit_outbox a ON a.item_instance_id=e.item_instance_id AND a.transaction_id=e.placed_transaction_id WHERE e.parent_item_instance_id=encode($1,'hex')::uuid")
                .bind(backpack.as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(audited, 3);
            let before = h.footprint().await?;
            assert!(matches!(claim(&h, &authority, request.clone()).await.map_err(debug)?, RewardClaimMintOutcome::AlreadyCommitted(_)));
            assert_eq!(h.footprint().await?, before);
            // A late third-output failure rolls back the root, first peer and audits.
            h.tamper("CREATE FUNCTION l2_peer_fault() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.definition_production_key='fixture:b3.stone' THEN RAISE EXCEPTION 'injected final peer failure'; END IF; RETURN NEW; END $$").await?;
            h.tamper("CREATE TRIGGER aa_l2_peer_fault BEFORE INSERT ON game_item_instances FOR EACH ROW EXECUTE FUNCTION l2_peer_fault()").await?;
            let mut failed = request.clone();
            failed.command = command(3)?;
            failed.claim.production_key = "fixture:l2.peer.failure".into();
            assert!(claim(&h, &authority, failed).await.is_err());
            let after = h.footprint().await?;
            assert_eq!(&after[1..], &before[1..]);
            h.tamper("DROP TRIGGER aa_l2_peer_fault ON game_item_instances").await?;
            let mut full = request.clone();
            full.command = command(4)?;
            full.claim.production_key = "fixture:l2.peer.full".into();
            full.contents = vec![request.contents[0].clone(); 19];
            refused(claim(&h, &authority, full).await, RewardClaimRefusal::MainBackpackFull)?;
            let after = h.footprint().await?;
            assert_eq!(&after[1..], &before[1..]);
            let before = h.footprint().await?;
            request.backpack_outputs = false;
            assert!(claim(&h, &authority, request).await.is_err());
            assert_eq!(h.footprint().await?, before);
            drop(authority); drop(seal);
            h.cleanup().await
        }).map_err(|error| error.to_string())
    })?.join().map_err(|_| "peer output test thread failed")?.map_err(|error| -> Box<dyn std::error::Error> { error.into() })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED questl2-peer-outputs duration={:?}",
        started.elapsed()
    );
    Ok(())
}
