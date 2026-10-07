// MAP-ITEM-REF-1 Part B (ARCH-MAP-TRACK-PACKETS-V1 §2.7, CP D607): the durable half of a
// capability-4 corpse take. A committed corpse MINT is bound to the Channel's projected corpse
// (the KILL-REWARD part B interface), its live entries read back in loot order, one entry moves
// into the backpack with one TRANSFER, and the replay of that command answers the original
// commit (MOVED after a reconnect). Live end to end through the kill-reward settlement is
// KILL-REWARD part B. Reuses the D3-4 corpse MINT and loot-entry forgery.
use crate::corpse_transfer_postgres_cases::{
    equip_backpack, in_corpse, locations, mint_corpse, put_loot, take,
};
use crate::durability::item_transfer::{ItemTransferOutcome, TransferShape};
use crate::foundation::{ChannelId, CombatDeathFixture, ScopeOwnershipGeneration, WorldId};
use crate::item_transfer_postgres_cases::{
    CHANNEL, CHARACTER, Harness, SESSION, TestResult, WORLD, configured_admin, debug, fence, id,
    runtime,
};

#[test]
fn a_bound_corpse_mint_reads_its_entries_and_moves_one_once() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "itemrefcorpse").await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        equip_backpack(&harness, &authority, fence()?, SESSION).await?;
        let corpse = mint_corpse(&harness, &authority, 3000, id(CHARACTER)).await?;
        let first = put_loot(&harness, corpse, 3000, 1, 130).await?;
        let second = put_loot(&harness, corpse, 3000, 2, 134).await?;

        // The Channel's projected corpse is bound to the committed corpse MINT; an unbound
        // projection names no Item.
        let mut fixture = CombatDeathFixture::new(
            WorldId::decode(&id(WORLD)).map_err(debug)?,
            ChannelId::decode(&id(CHANNEL)).map_err(debug)?,
            ScopeOwnershipGeneration::new(1).map_err(debug)?,
        )
        .map_err(debug)?;
        // Fail closed: nothing binds before the death is projected (no corpse to name).
        assert!(
            fixture
                .bind_corpse_item(corpse, "ItemType", "fixture:corpse.rat", "corpse-r1")
                .is_err()
        );
        assert_eq!(fixture.bound_corpse_item_instance_id(), None);
        fixture
            .strike("item-ref", CombatDeathFixture::HEALTH)
            .map_err(debug)?;
        fixture.project_death().map_err(debug)?;
        assert_eq!(fixture.bound_corpse_item_instance_id(), None);
        fixture
            .bind_corpse_item(corpse, "ItemType", "fixture:corpse.rat", "corpse-r1")
            .map_err(debug)?;
        assert_eq!(fixture.bound_corpse_item_instance_id(), Some(corpse));

        // The bound corpse's live entries, in loot order; anything else is not a corpse.
        let contents = harness
            .root
            .read_corpse_contents(&authority, corpse)
            .await
            .map_err(debug)?
            .ok_or("a live corpse")?;
        let ids: Vec<[u8; 16]> = contents
            .iter()
            .map(|entry| entry.item.item_instance_id)
            .collect();
        assert_eq!(ids, vec![first, second]);
        assert_eq!(
            harness
                .root
                .read_corpse_contents(&authority, first)
                .await
                .map_err(debug)?,
            None
        );
        let before = harness.footprint().await?;
        harness
            .root
            .read_corpse_contents(&authority, corpse)
            .await
            .map_err(debug)?;
        assert_eq!(harness.footprint().await?, before);

        // One TRANSFER into the backpack; the replay of the same command answers the commit.
        let moved = match harness
            .transfer(&authority, fence()?, take(SESSION, 2, first)?)
            .await
            .map_err(debug)?
        {
            ItemTransferOutcome::Committed(result) => result,
            other => return Err(format!("expected a commit, got {other:?}").into()),
        };
        assert_eq!(moved.shape, TransferShape::NewEntry);
        let after = harness.footprint().await?;
        match harness
            .transfer(&authority, fence()?, take(SESSION, 2, first)?)
            .await
            .map_err(debug)?
        {
            ItemTransferOutcome::AlreadyCommitted(result) => assert_eq!(result, moved),
            other => return Err(format!("expected a replay, got {other:?}").into()),
        }
        assert_eq!(harness.footprint().await?, after);
        assert_eq!(locations(&harness, first).await?, 1);
        assert!(!in_corpse(&harness, first).await?);
        assert!(in_corpse(&harness, second).await?);
        let remaining = harness
            .root
            .read_corpse_contents(&authority, corpse)
            .await
            .map_err(debug)?
            .ok_or("a live corpse")?;
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].item.item_instance_id, second);

        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}
