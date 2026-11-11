use super::*;
use crate::durability::reward_claim_mint::RewardClaimMintContent;

async fn claim_with_contents(
    harness: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    mut request: RewardClaimMintRequest,
) -> Result<RewardClaimMintOutcome, RewardClaimMintError> {
    if request.item.container_capacity.is_none() {
        return harness
            .claim(
                authority,
                fence().map_err(|_| RewardClaimMintError::InvalidInput)?,
                request,
            )
            .await;
    }
    request.content_revision = format!("sha256:{}", "d1".repeat(32));
    let fence = fence().map_err(|_| RewardClaimMintError::InvalidInput)?;
    let mut candidate = harness
        .root
        .freeze_reward_claim_mint_weighted(authority, &harness.node, fence, request, &TEST_WEIGHTS)
        .await?;
    harness
        .root
        .commit_reward_claim_mint_weighted_noticed(
            authority,
            &harness.node,
            fence,
            &mut candidate,
            None,
            &TEST_WEIGHTS,
        )
        .await
        .map(|(outcome, _)| outcome)
}

#[test]
fn filled_fluid_claims_commit_root_and_children_and_replay_without_refilling() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    let started = std::time::Instant::now();
    std::thread::Builder::new().stack_size(64 << 20).spawn(move || -> Result<(), String> {
        runtime().map_err(debug)?.block_on(async move {
            let harness = Harness::create(admin, "filledfluid").await?;
            let seal = harness.recovery.seal_current().map_err(debug)?;
            let authority = harness.root.open_character_authority(&seal).await.map_err(debug)?;
            harness.equip_backpack(&authority, 1).await?;
            let mut root = chest(command(2)?, "fixture:fluid.root", facts(STONE, ItemStackClass::NonStackable), 1);
            // Goblin milk is subtype9; Dreamer's wine is2. These exercise
            // the same durable owner used by the source-resolved recipes.
            root.root_fluid_subtype = Some(9);
            let first = harness.claimed(&authority, root.clone()).await?;
            assert_eq!(sqlx::query_scalar::<_, i16>("SELECT fluid_subtype FROM game_item_fluid_states WHERE item_instance_id=encode($1,'hex')::uuid")
                .bind(first.item_instance_id.as_slice()).fetch_one(&harness.pool).await?, 9);
            assert!(matches!(harness.claim(&authority, fence()?, root.clone()).await.map_err(debug)?, RewardClaimMintOutcome::AlreadyCommitted(_)));
            harness.root.commit_item_fluid(&authority, &harness.node, fence()?,
                crate::durability::item_use::fluid::ItemFluidRequest {
                    command_id: 10, transaction_id: id(79), other: None,
                    change: crate::durability::item_use::fluid::ItemFluidChange {
                        item_instance_id: first.item_instance_id,
                        definition: root.item.definition.clone(), fluid_before: 9, fluid_after: 0,
                    },
                }).await.map_err(debug)?;
            assert!(matches!(harness.claim(&authority, fence()?, root.clone()).await.map_err(debug)?, RewardClaimMintOutcome::AlreadyCommitted(_)));
            assert_eq!(harness.count("game_item_fluid_states").await?, 0);
            root.root_fluid_subtype = Some(1);
            assert!(matches!(harness.claim(&authority, fence()?, root).await, Err(RewardClaimMintError::ConflictingCause)));
            let mut bag = chest(command(3)?, "fixture:fluid.bag", backpack_facts(), 1);
            bag.contents = vec![RewardClaimMintContent {
                item: facts(STONE, ItemStackClass::NonStackable), quantity: 1,
                key_number: None, text: None, fluid_subtype: Some(2), charges_remaining: None,
            }];
            let bundle = match claim_with_contents(&harness, &authority, bag.clone()).await.map_err(debug)? {
                RewardClaimMintOutcome::Committed(result) => result,
                other => return Err(format!("expected filled bag commit: {other:?}").into()),
            };
            assert_eq!(sqlx::query_scalar::<_, i16>("SELECT f.fluid_subtype FROM game_item_fluid_states f JOIN game_item_nested_entries e USING(item_instance_id) WHERE e.parent_item_instance_id=encode($1,'hex')::uuid")
                .bind(bundle.item_instance_id.as_slice()).fetch_one(&harness.pool).await?, 2);
            assert!(matches!(claim_with_contents(&harness, &authority, bag).await.map_err(debug)?, RewardClaimMintOutcome::AlreadyCommitted(_)));
            harness.claimed(&authority, chest(command(4)?, "fixture:fluid.empty", facts(STONE, ItemStackClass::NonStackable), 1)).await?;
            assert_eq!(harness.count("game_item_fluid_states").await?, 1);
            assert_eq!(harness.count("game_item_fluid_transitions").await?, 1);
            // Present's jug and cup both retain the donor constructor's raw1
            // (water); sibling ordinary objects must not acquire fluid rows.
            let mut present = chest(command(5)?, "fixture:fluid.present", backpack_facts(), 1);
            present.contents = [Some(1), None, Some(1), None].into_iter().map(|fluid_subtype| RewardClaimMintContent {
                item: facts(STONE, ItemStackClass::NonStackable), quantity: 1,
                key_number: None, text: None, fluid_subtype, charges_remaining: None,
}).collect();
            let present_result = match claim_with_contents(&harness, &authority, present.clone()).await.map_err(debug)? {
                RewardClaimMintOutcome::Committed(result) => result,
                other => return Err(format!("expected Present fluid commit: {other:?}").into()),
            };
            assert_eq!(sqlx::query_as::<_, (i64, Option<i16>)>("SELECT e.placement_ordinal::bigint,f.fluid_subtype FROM game_item_nested_entries e LEFT JOIN game_item_fluid_states f USING(item_instance_id) WHERE e.parent_item_instance_id=encode($1,'hex')::uuid ORDER BY e.placement_ordinal")
                .bind(present_result.item_instance_id.as_slice()).fetch_all(&harness.pool).await?,
                vec![(1, Some(1)), (2, None), (3, Some(1)), (4, None)]);
            assert!(matches!(claim_with_contents(&harness, &authority, present).await.map_err(debug)?, RewardClaimMintOutcome::AlreadyCommitted(_)));
            assert_eq!(harness.count("game_item_fluid_states").await?, 3);
            // A writer omission and a substituted subtype must each roll back the
            // claim, root item, custody and receipt, preserving its reservation.
            for (offset, body) in [(6_u64, "RETURN NULL;"), (7, "NEW.fluid_subtype := 7; RETURN NEW;")] {
                harness.tamper(&format!("CREATE OR REPLACE FUNCTION quest_fluid_fault() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN {body} END $$")).await?;
                harness.tamper("CREATE TRIGGER aa_quest_fluid_fault BEFORE INSERT ON game_item_fluid_states FOR EACH ROW EXECUTE FUNCTION quest_fluid_fault()").await?;
                let before = harness.footprint().await?;
                let nested_before = harness.count("game_item_nested_entries").await?;
                let mut bad = chest(command(offset)?, &format!("fixture:fluid.bad.{offset}"), facts(STONE, ItemStackClass::NonStackable), 1);
                bad.root_fluid_subtype = Some(6);
                if offset == 6 {
                    bad.root_fluid_subtype = None;
                    bad.item = backpack_facts();
                    bad.contents = vec![RewardClaimMintContent {
                        item: facts(STONE, ItemStackClass::NonStackable), quantity: 1,
                        key_number: None, text: None, fluid_subtype: Some(6), charges_remaining: None,
                    }];
                }
                assert!(claim_with_contents(&harness, &authority, bad).await.is_err());
                let after = harness.footprint().await?;
                // The separate freeze may reserve an intent; no value commits.
                assert_eq!(&after[1..], &before[1..]);
                assert_eq!(harness.count("game_item_nested_entries").await?, nested_before);
                assert_eq!(harness.count("game_item_fluid_states").await?, 3);
                harness.tamper("DROP TRIGGER aa_quest_fluid_fault ON game_item_fluid_states").await?;
            }
            drop(authority); drop(seal);
            harness.cleanup().await
        }).map_err(|error| error.to_string())
    })?.join().map_err(|_| "fluid test thread failed")?.map_err(|error| -> Box<dyn std::error::Error> { error.into() })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED filled_fluid_claims_commit_root_and_children_and_replay_without_refilling {:?}",
        started.elapsed()
    );
    Ok(())
}
