use super::atelier_fixtures::fixture as atelier;
use super::fixture::{Harness, debug, fence, id};
use super::kill_reward_e2e_fixtures::scenario_thread::run_async;
use super::{TestResult, maintain_ready};
use crate::durability::character_revision_sequencer::CharacterRevisionSequencer;
use crate::durability::character_wheel::{
    WheelChangeFacts, WheelChangeRequest, WheelCommitOutcome, WheelDomain, WheelOccurrence,
    WheelRefusal, WheelSlots,
};
use crate::durability::npc_purchase::PurchaseOutcome;
use crate::gameplay_transport::spell_premium_coordinator::SpellPremiumCoordinator;
use oteryn_gameplay_progression::wheel_gem_data::WheelGemData;
use std::sync::Arc;

fn request(
    seed: u8,
    expected_wheel_revision: u64,
    points: [u16; 36],
) -> TestResult<WheelChangeRequest> {
    Ok(WheelChangeRequest {
        occurrence: WheelOccurrence::from_bytes(id(seed)).map_err(debug)?,
        expected_wheel_revision,
        slots: WheelSlots::new(points).map_err(debug)?,
        vessels: None,
    })
}

#[test]
fn wheel_save_mixed_refund_and_spend_respects_source_slot_retry_order() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_async(async move |_| {
        let h = Harness::create(admin, "wheel_source_order", true).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let gate =
            SpellPremiumCoordinator::new(Default::default(), None, None, Some(h.root.clone()));
        let sequencer = CharacterRevisionSequencer::new();
        atelier::eligible(&h.pool).await?;
        let promotion_fence = fence(3)?;
        let promoted = sequencer
            .acquire(promotion_fence.character_id)
            .await
            .commit_npc_purchase(
                &h.root,
                &authority,
                &h.node,
                promotion_fence,
                atelier::prepare(&h).await?,
                &gate,
            )
            .await
            .map_err(debug)?;
        assert!(matches!(promoted, PurchaseOutcome::Committed(_)));
        atelier::higher_level(&h).await?;

        let ruleset = Arc::new(WheelGemData::embedded().map_err(debug)?.wheel_ruleset()?);
        let current = fence(5)?;
        let save = async |request, at_temple| -> TestResult<WheelCommitOutcome> {
            maintain_ready(&h.root).await?;
            sequencer
                .acquire(current.character_id)
                .await
                .commit_wheel(
                    &h.root,
                    &authority,
                    &h.node,
                    current,
                    request,
                    Arc::clone(&ruleset),
                    WheelChangeFacts { at_temple },
                    &gate,
                )
                .await
                .map_err(debug)
        };

        // Fill the unaffected blue and purple paths except the purple tip. This leaves
        // 150 of the level-2000 Character's 1,950 points for the two paths under test.
        let mut initial = [0; 36];
        for (index, slot) in ruleset.slots().iter().enumerate() {
            if slot.domain == WheelDomain::Blue
                || (slot.domain == WheelDomain::Purple && index != 35)
            {
                initial[index] = slot.capacity;
            }
        }
        assert_eq!(
            initial.iter().map(|points| u32::from(*points)).sum::<u32>(),
            1800
        );
        initial[14] = 50; // Green centre.
        initial[15] = 50; // Red centre.
        initial[9] = 50; // Red upper spoke, opened by the red centre.
        let seed = request(171, 0, initial)?;
        assert!(matches!(
            save(seed, false).await?,
            WheelCommitOutcome::Committed(_)
        ));

        // Centre refunds run first. The red spoke then needs a full green spoke
        // before retaining any points, but its old 50 leaves just 50 for that
        // green spend. The final vector fits exactly, yet neither retry can proceed.
        let mut blocked = initial;
        blocked[15] = 0;
        blocked[8] = 75;
        blocked[9] = 25;
        assert_eq!(
            save(request(172, 1, blocked)?, true).await?,
            WheelCommitOutcome::Refused(WheelRefusal::NotAdjacent),
        );

        // A full red-spoke refund needs no open parent. It runs after the first
        // green spend attempt, so the green spoke succeeds on the retry.
        let mut after = blocked;
        after[9] = 0;
        let mixed = request(173, 1, after)?;
        assert_eq!(
            save(mixed, false).await?,
            WheelCommitOutcome::Refused(WheelRefusal::RemovalNotAtTemple)
        );
        let WheelCommitOutcome::Committed(receipt) = save(mixed, true).await? else {
            return Err("mixed Wheel save did not commit after the refund".into());
        };
        assert_eq!((receipt.before, receipt.after), (seed.slots, mixed.slots));
        assert_eq!(
            (receipt.before_wheel_revision, receipt.after_wheel_revision),
            (1, 2)
        );
        assert_eq!(
            save(mixed, false).await?,
            WheelCommitOutcome::AlreadyCommitted(receipt)
        );

        maintain_ready(&h.root).await?;
        let reloaded = h
            .root
            .read_character_wheel(&authority, current.character_id, Arc::clone(&ruleset))
            .await
            .map_err(debug)?;
        assert_eq!((reloaded.wheel_revision, reloaded.slots), (2, mixed.slots));
        let receipts: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_character_wheel_receipts WHERE character_id=encode($1,'hex')::uuid",
        )
        .bind(current.character_id.as_bytes().as_slice())
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(receipts, 2);

        let mut retry_before = initial;
        retry_before[30] -= 51; // A partial blue tip does not open another slot.
        retry_before[8] = 2;
        retry_before[9] = 75;
        retry_before[16] = 24;
        assert_eq!(retry_before.iter().copied().map(u32::from).sum::<u32>(), 1950);
        assert!(matches!(
            save(request(174, 2, retry_before)?, true).await?,
            WheelCommitOutcome::Committed(_)
        ));

        // The red centre refunds 50. The green spoke cannot afford 75, but
        // Crystal clears its old 2 before retrying. That lets the other red
        // spoke spend 51 and reopen the upper red spoke, whose refund then
        // funds the green spend. The final allocation still uses 1,950.
        let mut retry_after = retry_before;
        retry_after[15] = 0;
        retry_after[8] = 75;
        retry_after[9] = 1;
        retry_after[16] = 75;
        let retry = request(175, 3, retry_after)?;
        let WheelCommitOutcome::Committed(receipt) = save(retry, true).await? else {
            return Err("the Wheel save must commit after the temporary refund".into());
        };
        assert_eq!(receipt.after, retry.slots);
        drop(authority);
        drop(seal);
        drop(gate);
        h.cleanup().await
    })
}
