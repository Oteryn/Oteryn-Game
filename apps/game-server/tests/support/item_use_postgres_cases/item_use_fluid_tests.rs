use super::*;
use crate::durability::item_use::fluid::{
    ItemFluidChange, ItemFluidOutcome, ItemFluidRequest, ItemFluidSplash,
};

// I10a (migration 0560): a carried vial is filled and emptied through `commit_item_fluid` under
// the Character's item fence. Only setup bypasses guards.
const VIAL: u8 = 140;
const LOOSE_VIAL: u8 = 210;
const CUP: u8 = 220;
// Crystal 201189f8 `data/items/items.xml`: 2874 "vial", a fluid container.
const VIAL_KEY: &str = "oteryn:item.tibia.i2874";
// Crystal `fluids.lua`: sub-type 7 is mana fluid (`addMana(math.random(50, 150))`).
const MANA: u8 = 7;

fn change(item: u8, before: u8, after: u8) -> ItemFluidChange {
    ItemFluidChange {
        item_instance_id: id(item),
        definition: definition(VIAL_KEY),
        fluid_before: before,
        fluid_after: after,
    }
}

fn fluid(command: u64, tx: u8, item: u8, before: u8, after: u8) -> ItemFluidRequest {
    ItemFluidRequest {
        command_id: command,
        transaction_id: id(tx),
        change: change(item, before, after),
        other: None,
    }
}

impl Use {
    async fn splash_fluid(
        &self,
        request: &ItemFluidRequest,
        splash: ItemFluidSplash,
    ) -> Result<ItemFluidOutcome, ItemUseError> {
        let seal = self
            .harness
            .recovery
            .seal_current()
            .map_err(|_| ItemUseError::AuthorityRejected)?;
        let authority = self
            .harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(|_| ItemUseError::AuthorityRejected)?;
        self.harness
            .root
            .commit_item_fluid_at(
                &authority,
                &self.harness.node,
                item_fence().map_err(|_| ItemUseError::InvalidInput)?,
                request.clone(),
                Some(splash),
            )
            .await
    }
    async fn change_fluid_under(
        &self,
        fence: CurrentCharacterItemFence,
        request: &ItemFluidRequest,
    ) -> Result<ItemFluidOutcome, ItemUseError> {
        let seal = self
            .harness
            .recovery
            .seal_current()
            .map_err(|_| ItemUseError::AuthorityRejected)?;
        let authority = self
            .harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(|_| ItemUseError::AuthorityRejected)?;
        self.harness
            .root
            .commit_item_fluid(&authority, &self.harness.node, fence, request.clone())
            .await
    }

    async fn change_fluid(
        &self,
        request: &ItemFluidRequest,
    ) -> Result<ItemFluidOutcome, ItemUseError> {
        let fence = item_fence().map_err(|_| ItemUseError::InvalidInput)?;
        self.change_fluid_under(fence, request).await
    }

    /// The fluids the Character's backpack read shows (domain 9's sub-types).
    async fn shown_fluids(&self) -> TestResult<std::collections::BTreeMap<[u8; 16], u8>> {
        let seal = self.harness.recovery.seal_current().map_err(debug)?;
        let authority = self
            .harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let backpack = self
            .harness
            .root
            .read_character_backpack(&authority, item_fence().map_err(debug)?.character_id)
            .await
            .map_err(debug)?
            .ok_or("no backpack")?;
        Ok(backpack.fluids)
    }

    async fn fluid_of(&self, seed: u8) -> TestResult<Option<i16>> {
        Ok(sqlx::query_scalar(
            "SELECT fluid_subtype FROM game_item_fluid_states \
              WHERE item_instance_id = encode($1,'hex')::uuid",
        )
        .bind(id(seed).as_slice())
        .fetch_optional(&self.harness.pool)
        .await?)
    }
}

#[test]
fn a_map_source_fill_persists_the_donor_fluid_subtype() -> TestResult {
    run(async |admin| {
        let used = Use::create(admin, "fluid_map_fill").await?;
        println!("POSTGRES_STARTED item_use_fluid_map_fill");
        seed_vials(&used).await?;
        // Crystal 201189f8 data/scripts/actions/items/fluids.lua:35-40 and items.xml i137.
        let water = fluid(40, 170, VIAL, 0, 1);
        assert_eq!(
            used.change_fluid(&water).await.map_err(debug)?,
            ItemFluidOutcome::Written
        );
        assert_eq!(used.fluid_of(VIAL).await?, Some(1));
        used.cleanup().await
    })
}

#[test]
fn pouring_on_ground_persists_a_ten_second_donor_splash_and_empties_container() -> TestResult {
    run(async |admin| {
        let used = Use::create(admin, "fluid_ground_splash").await?;
        println!("POSTGRES_STARTED item_use_fluid_ground_splash");
        seed_vials(&used).await?;
        used.change_fluid(&fluid(41, 172, VIAL, 0, MANA))
            .await
            .map_err(debug)?;
        let empty = fluid(42, 174, VIAL, MANA, 0);
        let splash = ItemFluidSplash {
            placement: b"test-ground".to_vec(),
            fluid_subtype: MANA,
            splash_item_id: 2886,
            lifetime_millis: 10_000,
        };
        assert_eq!(
            used.splash_fluid(&empty, splash).await.map_err(debug)?,
            ItemFluidOutcome::Written
        );
        assert_eq!(used.fluid_of(VIAL).await?, None);
        let row: (i16, i32, bool) = sqlx::query_as(
            "SELECT fluid_subtype,splash_item_id, expires_at BETWEEN clock_timestamp()+interval '9 seconds' AND clock_timestamp()+interval '10 seconds' FROM game_item_fluid_splashes WHERE transaction_id=encode($1,'hex')::uuid")
            .bind(id(174).as_slice()).fetch_one(&used.harness.pool).await?;
        assert_eq!(row, (i16::from(MANA), 2886, true));
        let seal = used.harness.recovery.seal_current().map_err(debug)?;
        let authority = used
            .harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let fence = item_fence().map_err(debug)?;
        let RuntimeScopeRefV1::Channel {
            world_id,
            channel_id,
        } = fence.runtime_scope
        else {
            return Err("expected Channel scope".into());
        };
        let active = used
            .harness
            .root
            .refresh_item_fluid_splashes(&authority, *world_id.as_bytes(), *channel_id.as_bytes())
            .await
            .map_err(debug)?;
        assert!(active.iter().any(|row| row.placement == b"test-ground"));
        sqlx::query("UPDATE game_item_fluid_splashes SET expires_at=clock_timestamp()-interval '1 second' WHERE transaction_id=encode($1,'hex')::uuid")
            .bind(id(174).as_slice()).execute(&used.harness.pool).await?;
        assert!(
            used.harness
                .root
                .refresh_item_fluid_splashes(
                    &authority,
                    *world_id.as_bytes(),
                    *channel_id.as_bytes(),
                )
                .await
                .map_err(debug)?
                .is_empty()
        );
        used.cleanup().await
    })
}

async fn seed_vials(used: &Use) -> TestResult {
    let mut script = item(VIAL, VIAL_KEY, 1);
    script.push_str(&entry(VIAL, BACKPACK, 6));
    // A vial of the World that no backpack holds.
    script.push_str(&item(LOOSE_VIAL, VIAL_KEY, 1));
    script.push_str(&item(CUP, VIAL_KEY, 1));
    script.push_str(&entry(CUP, BACKPACK, 7));
    let mut tx = used.harness.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await?;
    sqlx::raw_sql(sqlx::AssertSqlSafe(script))
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

#[test]
fn a_carried_vial_is_filled_and_emptied_once_per_command() -> TestResult {
    run(async |admin| {
        let used = Use::create(admin, "fluid").await?;
        println!("POSTGRES_STARTED item_use_fluid");
        seed_vials(&used).await?;
        let fill = fluid(30, 150, VIAL, 0, MANA);
        assert_eq!(
            used.change_fluid(&fill).await.map_err(debug)?,
            ItemFluidOutcome::Written
        );
        assert_eq!(used.fluid_of(VIAL).await?, Some(i16::from(MANA)));
        assert_eq!(used.shown_fluids().await?, [(id(VIAL), MANA)].into());
        // The same command replays without writing; a changed request for it conflicts.
        assert_eq!(
            used.change_fluid(&fill).await.map_err(debug)?,
            ItemFluidOutcome::AlreadyCommitted
        );
        assert!(matches!(
            used.change_fluid(&fluid(30, 150, VIAL, 0, 1)).await,
            Err(ItemUseError::ConflictingCommand)
        ));
        // A plan from a sub-type the vial no longer holds writes nothing.
        assert!(matches!(
            used.change_fluid(&fluid(31, 152, VIAL, 0, 1)).await,
            Err(ItemUseError::StaleState)
        ));
        // A vial no backpack of the holder carries is not the holder's to change.
        assert!(matches!(
            used.change_fluid(&fluid(32, 154, LOOSE_VIAL, 0, MANA))
                .await,
            Err(ItemUseError::StaleState)
        ));
        let mut stale = item_fence().map_err(debug)?;
        stale.character_lease_generation = 2;
        assert!(matches!(
            used.change_fluid_under(stale, &fluid(33, 156, VIAL, MANA, 0))
                .await,
            Err(ItemUseError::AuthorityRejected)
        ));
        assert_eq!(used.fluid_of(VIAL).await?, Some(i16::from(MANA)));
        // Pouring the vial into the empty cup moves the whole fluid in one command
        // (`fluids.lua:39-48`); a pour that is not a whole move is refused.
        let pour = |command, tx| ItemFluidRequest {
            other: Some(change(CUP, 0, MANA)),
            ..fluid(command, tx, VIAL, MANA, 0)
        };
        assert!(matches!(
            used.change_fluid(&ItemFluidRequest {
                other: Some(change(CUP, 0, 1)),
                ..fluid(36, 162, VIAL, MANA, 0)
            })
            .await,
            Err(ItemUseError::InvalidInput)
        ));
        assert_eq!(
            used.change_fluid(&pour(37, 164)).await.map_err(debug)?,
            ItemFluidOutcome::Written
        );
        assert_eq!(used.fluid_of(VIAL).await?, None);
        assert_eq!(used.fluid_of(CUP).await?, Some(i16::from(MANA)));
        assert_eq!(
            used.change_fluid(&pour(37, 164)).await.map_err(debug)?,
            ItemFluidOutcome::AlreadyCommitted
        );
        // Pouring back fills the vial again.
        assert_eq!(
            used.change_fluid(&ItemFluidRequest {
                other: Some(change(VIAL, 0, MANA)),
                ..fluid(38, 166, CUP, MANA, 0)
            })
            .await
            .map_err(debug)?,
            ItemFluidOutcome::Written
        );
        // Drinking empties it.
        assert_eq!(
            used.change_fluid(&fluid(34, 158, VIAL, MANA, 0))
                .await
                .map_err(debug)?,
            ItemFluidOutcome::Written
        );
        assert_eq!(used.fluid_of(VIAL).await?, None);
        assert_eq!(used.harness.count("game_item_fluid_transitions").await?, 6);
        assert!(matches!(
            used.change_fluid(&fluid(35, 160, VIAL, 0, 0)).await,
            Err(ItemUseError::InvalidInput)
        ));
        used.cleanup().await
    })
}

#[test]
fn the_runtime_role_cannot_change_a_fluid_without_its_transition() -> TestResult {
    run(async |admin| {
        let used = Use::create(admin, "fluid_raw").await?;
        seed_vials(&used).await?;
        assert_eq!(
            used.runtime_sql(format!(
                "INSERT INTO game_item_fluid_states(item_instance_id, world_id, fluid_subtype, \
                   last_transaction_id) VALUES ({}, {}, 7, {});",
                uuid(VIAL),
                uuid(WORLD),
                uuid(150),
            ))
            .await?,
            Some("23514".into())
        );
        // The state row keeps its item and World whatever transition the runtime records.
        assert_eq!(
            used.runtime_sql(format!(
                "INSERT INTO game_item_fluid_transitions(transaction_id, game_session_id, \
                   command_id, character_id, item_instance_id, fluid_before, fluid_after, \
                   created_xact_id) VALUES ({}, {}, 1, {}, {}, 0, 7, '1'::xid8); \
                 INSERT INTO game_item_fluid_states(item_instance_id, world_id, fluid_subtype, \
                   last_transaction_id) VALUES ({}, {}, 7, {}); \
                 UPDATE game_item_fluid_states SET world_id = {};",
                uuid(150),
                uuid(SESSION),
                uuid(CHARACTER),
                uuid(VIAL),
                uuid(VIAL),
                uuid(WORLD),
                uuid(150),
                uuid(LOOSE_VIAL),
            ))
            .await?,
            Some("42501".into())
        );
        assert_eq!(used.fluid_of(VIAL).await?, None);
        used.cleanup().await
    })
}
