use crate::bestiary_postgres_harness as f;
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::durability::item_mint::{
    GroundPlacement, ItemMintCause, ItemMintOutcome, ItemMintRequest,
};
use crate::durability::item_transfer::{
    ItemTransferDestination, ItemTransferOutcome, ItemTransferRequest,
};
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::durability::reward_claim_mint::QuestRewardAdmission;
use crate::foundation::{CommandId, CommandRef, GameSessionId};
use oteryn_gameplay_items::item_ref_admission::generation_item_facts;
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTransition, loader,
    reward_callbacks::{self, StateCondition},
};
use std::sync::Arc;

use super::engitem_reward_text::{Weights, definition, equip_backpack};

const LUBO: &str = "oteryn:quest-reward/qb-citizen-backpack";
const AMBER: &str = "oteryn:quest-reward/l6-citizen-rook-backpack";
const AGE_TIMER: &str = "oteryn:quest-transition/citizen-backpack-fixture-age-timer";

#[test]
fn citizen_backpack_first_payment_reserves_the_account_until_grant() -> f::TestResult {
    let Some(admin) = f::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            f::runtime()
                .map_err(f::debug)?
                .block_on(async {
                    let h = f::Harness::create(admin, "citizen_backpack", true).await?;
                    let result = exercise(&h).await;
                    h.cleanup().await?;
                    result
                })
                .map_err(f::debug)
        })?
        .join()
        .map_err(|_| "citizen backpack test thread panicked")?
        .map_err(Into::into)
}

async fn exercise(h: &f::Harness) -> f::TestResult {
    let seal = h.recovery.seal_current().map_err(f::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(f::debug)?;
    let loaded = loader::load_embedded_quest_state("content-1").map_err(f::debug)?;
    let canonical = loaded.catalogue();
    let callbacks = reward_callbacks::load_for(canonical)?;
    let completed = callbacks
        .iter()
        .find(|callback| callback.key == LUBO)
        .ok_or("backpack")?;
    let timer_read = completed.state_ops.first().ok_or("timer")?;
    let StateCondition::ElapsedAtLeast { seconds } = timer_read.from else {
        return Err("backpack has no source wait".into());
    };
    let now: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM now()))::bigint")
        .fetch_one(&h.pool)
        .await?;
    let mut transitions = Vec::new();
    for route in [LUBO, AMBER] {
        for key in [
            route.to_owned(),
            format!("{route}-start"),
            format!("{route}-materials"),
        ] {
            let callback = callbacks
                .iter()
                .find(|callback| callback.key == key)
                .ok_or("callback")?;
            // Amber's start and completion also write a track in the mainland quest.
            let keys = std::iter::once(key.clone()).chain(
                (0..callback.participants.len())
                    .map(|i| reward_callbacks::participant_key(&key, i)),
            );
            for key in keys {
                transitions.push(canonical.transition(&key).cloned().ok_or(key)?);
            }
        }
    }
    transitions.push(QuestTransition {
        key: AGE_TIMER.into(),
        quest: completed.quest.clone(),
        completes: false,
        experience: None,
        world_change: None,
        effects: vec![QuestEffect {
            track: timer_read.track.clone(),
            from: QuestComparison::Any,
            effect: QuestEffectKind::Set(now - i64::from(seconds) - 1),
        }],
    });
    let catalogue = Arc::new(
        QuestStateCatalogue::new(
            canonical.content_revision(),
            canonical
                .declared_tracks()
                .filter(|track| {
                    transitions
                        .iter()
                        .any(|transition| transition.quest == track.quest)
                })
                .cloned()
                .collect(),
            transitions,
        )
        .map_err(f::debug)?,
    );
    let (controller, _) =
        oteryn_gameplay_content::content::native_gameplay::test_support::activated_with_loot_tables(
            f::scope()?.world_id(),
            None,
        );
    let native = controller
        .active()
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let weights = Weights(native);
    let apply = |fence: CurrentCharacterGameplayFence, key: String, ordinal| -> f::TestResult<_> {
        let callback = callbacks
            .iter()
            .find(|callback| callback.key == key)
            .ok_or("callback")?;
        Ok(h.root.commit_fixture_quest_reward(
            &authority,
            &h.node,
            fence,
            QuestTransitionRequest {
                transition_key: key,
                cause: QuestCause::Command(CommandRef::new(
                    fence.game_session_id,
                    CommandId::new(ordinal).map_err(f::debug)?,
                )),
            },
            catalogue.clone(),
            QuestRewardAdmission {
                native,
                weights: &weights,
                fee_payment: None,
                actor_conditions: None,
                carried_use: None,
                creature_use: None,
                peer_use: None,
            },
            callback.clone(),
        ))
    };
    let value = |key: &str| -> f::TestResult<_> {
        Ok(callbacks
            .iter()
            .find(|callback| callback.key == key)
            .ok_or("callback")?
            .state_ops
            .first()
            .ok_or("timer")?
            .track
            .clone())
    };
    let primary = f::fence(1)?;
    equip_backpack(h, &authority, native, &weights).await?;
    let first_leather = stock(h, &authority, primary, &weights, (5878, 100), 9401).await?;
    for (fence, key, ordinal) in [
        (primary, format!("{LUBO}-start"), 9500),
        (f::fence(2)?, format!("{LUBO}-materials"), 9501),
    ] {
        let outcome = apply(fence, key.clone(), ordinal)?
            .await
            .map_err(|error| format!("{key}: {error:?}"))?;
        assert!(
            matches!(outcome, QuestTransitionOutcome::Committed(_)),
            "first character: {outcome:?}"
        );
    }
    let (timer, observed_now): (i64, i64) = sqlx::query_as(
        "SELECT value, floor(extract(epoch FROM now()))::bigint
         FROM game_character_quest_tracks WHERE track_key=$1",
    )
    .bind(&timer_read.track)
    .fetch_one(&h.pool)
    .await?;
    assert!((now..=observed_now).contains(&timer));
    assert!(observed_now - timer < i64::from(seconds));
    let remaining: i64 = sqlx::query_scalar(
        "SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE item_instance_id=$1 AND lifecycle=1",
    ).bind(first_leather).fetch_one(&h.pool).await?;
    assert_eq!(remaining, 0);

    let other = alternate(&h.pool).await?;
    stock(h, &authority, other, &weights, (2854, 1), 9402).await?;
    let leather = stock(h, &authority, other, &weights, (5878, 100), 9403).await?;
    let outcome = apply(other, format!("{AMBER}-start"), 9502)?
        .await
        .map_err(|error| format!("Amber start: {error:?}"))?;
    assert!(
        matches!(outcome, QuestTransitionOutcome::Committed(_)),
        "second character starts: {outcome:?}"
    );
    let other = CurrentCharacterGameplayFence {
        expected_character_revision: f::fence(2)?.expected_character_revision,
        ..other
    };
    let outcome = apply(other, format!("{AMBER}-materials"), 9503)?
        .await
        .map_err(|error| format!("Amber materials: {error:?}"))?;
    assert!(
        matches!(outcome, QuestTransitionOutcome::Refused(_)),
        "second character paid: {outcome:?}"
    );

    let remaining: i64 =
        sqlx::query_scalar("SELECT quantity FROM game_item_instances WHERE item_instance_id=$1")
            .bind(leather)
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(remaining, 100);
    let effects: (i64, i64, i64, i64) = sqlx::query_as(
        "SELECT
         (SELECT count(*) FROM game_quest_callback_consumptions WHERE character_id=$1),
         (SELECT count(*) FROM game_quest_account_reservations WHERE character_id=$1),
         (SELECT count(*) FROM game_quest_account_reservations WHERE character_id=$2),
         (SELECT count(*) FROM game_character_quest_tracks WHERE character_id=$1 AND track_key=$3)",
    )
    .bind(sqlx::types::Uuid::from_bytes(
        *other.character_id.as_bytes(),
    ))
    .bind(sqlx::types::Uuid::from_bytes(
        *primary.character_id.as_bytes(),
    ))
    .bind(value(&format!("{AMBER}-materials"))?)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(effects, (0, 0, 1, 0));

    own_session(&h.pool, f::CHARACTER, 52, 2).await?;
    let resumed = CurrentCharacterGameplayFence {
        game_session_id: GameSessionId::decode(&f::id(52)).map_err(f::debug)?,
        character_lease_generation: 2,
        ..f::fence(3)?
    };
    // Use the fenced quest writer to arrange elapsed wall time. Both the
    // SET_NOW receipt and the eventual grant still prove PostgreSQL time.
    let aged = h
        .root
        .commit_character_quest_transition(
            &authority,
            &h.node,
            resumed,
            QuestTransitionRequest {
                transition_key: AGE_TIMER.into(),
                cause: QuestCause::Command(CommandRef::new(
                    resumed.game_session_id,
                    CommandId::new(9504).map_err(f::debug)?,
                )),
            },
            catalogue.clone(),
        )
        .await
        .map_err(|error| format!("age timer: {error:?}"))?;
    assert!(
        matches!(aged, QuestTransitionOutcome::Committed(_)),
        "{aged:?}"
    );
    let resumed = CurrentCharacterGameplayFence {
        expected_character_revision: f::fence(4)?.expected_character_revision,
        ..resumed
    };
    let outcome = apply(resumed, LUBO.into(), 9505)?
        .await
        .map_err(|error| format!("Lubo grant: {error:?}"))?;
    assert!(
        matches!(outcome, QuestTransitionOutcome::Committed(_)),
        "original payer collects: {outcome:?}"
    );
    let callback = callbacks
        .iter()
        .find(|callback| callback.key == LUBO)
        .ok_or("backpack")?;
    let key = callback.outfit.as_ref().ok_or("outfit")?.grant_key(1);
    let granted: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_account_appearance_unlock_receipts
         WHERE account_id=$1 AND granting_character_id=$2 AND unlock_key=$3",
    )
    .bind(sqlx::types::Uuid::from_bytes(f::id(40)))
    .bind(sqlx::types::Uuid::from_bytes(
        *primary.character_id.as_bytes(),
    ))
    .bind(key)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(granted, 1);
    assert_eq!(h.count("game_quest_account_reservations").await?, 0);
    Ok(())
}

async fn alternate(pool: &sqlx::PgPool) -> f::TestResult<CurrentCharacterGameplayFence> {
    let character = sqlx::types::Uuid::from_bytes(f::id(44));
    let account = sqlx::types::Uuid::from_bytes(f::id(40));
    let world = sqlx::types::Uuid::from_bytes(f::id(f::WORLD));
    sqlx::query(
        "INSERT INTO game_character_roots(character_id,account_id,world_id,lifecycle,character_revision,
         profile_revision,ruleset_revision,content_revision,starter_template_revision,name,sex)
         VALUES($1,$2,$3,1,1,'profile-1','ruleset-1','content-1','starter-1','Fixture Alternate',2)",
    ).bind(character).bind(account).bind(world).execute(pool).await?;
    sqlx::query(
        "INSERT INTO game_character_progression_state VALUES
         ($1,1,50,1000,'profile-1','ruleset-1','content-1','simulation-1','evidence-1','declaration-1','policy-1','reward-1')",
    ).bind(character).execute(pool).await?;
    own_session(pool, 44, 51, 1).await?;
    Ok(CurrentCharacterGameplayFence {
        character_id: crate::domain::CharacterId::from_bytes(f::id(44)).map_err(f::debug)?,
        game_session_id: GameSessionId::decode(&f::id(51)).map_err(f::debug)?,
        ..f::fence(1)?
    })
}

// Arrange one session at a time on the shared account, like the existing
// same-account claim fixtures. Quest and inventory effects still use their owners.
async fn own_session(pool: &sqlx::PgPool, character: u8, session: u8, lease: i64) -> f::TestResult {
    let account = sqlx::types::Uuid::from_bytes(f::id(40));
    let character = sqlx::types::Uuid::from_bytes(f::id(character));
    let session = sqlx::types::Uuid::from_bytes(f::id(session));
    let world = sqlx::types::Uuid::from_bytes(f::id(f::WORLD));
    let channel = sqlx::types::Uuid::from_bytes(f::id(f::CHANNEL));
    let mut tx = pool.begin().await?;
    sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state=3 WHERE account_id=$1 AND session_state=1")
        .bind(account).execute(&mut *tx).await?;
    sqlx::query(
        "INSERT INTO game_durability_reconnect_sessions(game_session_id,account_id,character_id,world_id,runtime_scope_kind,
         runtime_scope_world_id,runtime_scope_channel_id,control_loss_epoch,original_grace_deadline,
         predecessor_generation,character_lease_generation,scope_ownership_generation,current_generation,current_transport_ref,session_state)
         VALUES($1,$2,$3,$4,1,$4,$5,1,999999,1,$6,1,1,$7,1)",
    ).bind(session).bind(account).bind(character).bind(world).bind(channel).bind(lease).bind([8_u8; 16].as_slice())
        .execute(&mut *tx).await?;
    sqlx::query(
        "INSERT INTO game_durability_admission_character_guards VALUES
         ($1,$2,$3,true,$4,$5,1,'test',1,'character-current',1,0,'{}')
         ON CONFLICT(character_id) DO UPDATE SET lease_generation=EXCLUDED.lease_generation,
         holder_game_session_id=EXCLUDED.holder_game_session_id",
    )
    .bind(character)
    .bind(account)
    .bind(world)
    .bind(lease)
    .bind(session)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "UPDATE game_durability_admission_account_guards SET presence_character_id=$1,holder_game_session_id=$2
         WHERE account_id=$3",
    ).bind(character).bind(session).bind(account).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

async fn stock(
    h: &f::Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    fence: CurrentCharacterGameplayFence,
    weights: &Weights<'_>,
    (item, quantity): (u32, u32),
    ordinal: u64,
) -> f::TestResult<sqlx::types::Uuid> {
    let native = weights.0;
    let typed = definition(item);
    let mut mint = h
        .root
        .freeze_item_mint(
            authority,
            &h.node,
            ItemMintRequest {
                cause: ItemMintCause::for_test(
                    fence.runtime_scope.world_id(),
                    crate::foundation::ChannelId::decode(&f::id(f::CHANNEL)).map_err(f::debug)?,
                    fence.scope_ownership_generation,
                    1,
                    ordinal,
                    crate::durability::item_mint::TypedDefinitionRef {
                        family: "LootTable".into(),
                        production_key: "fixture:citizen-leather".into(),
                        revision_ref: "definition-r1".into(),
                    },
                    "fixture:citizen-leather".into(),
                    0,
                ),
                item: typed.clone(),
                quantity,
                ground: GroundPlacement {
                    spatial_position: vec![0, 0, 0, 1, 0, 0, 0, 2, 0, 3],
                    corpse_ref: f::id(3).to_vec(),
                    map_revision: "map-1".into(),
                    content_revision: "content-1".into(),
                    native_room_placement_context: f::id(6).to_vec(),
                },
                content_revision: "content-1".into(),
                ruleset_revision: "ruleset-1".into(),
                sim_revision: "sim-1".into(),
            },
        )
        .await
        .map_err(f::debug)?;
    let death = mint.fresh_death_lane_permit().await;
    let ItemMintOutcome::Committed(receipt) = h
        .root
        .commit_item_mint(&death, authority, &h.node, &mut mint)
        .await
        .map_err(f::debug)?
    else {
        return Err("stock mint refused".into());
    };
    drop(death);
    let backpack = item != 2854;
    let pack = backpack
        .then(|| generation_item_facts(native, &definition(2854)))
        .transpose()
        .map_err(f::debug)?;
    let mut transfer = h
        .root
        .freeze_item_transfer(
            authority,
            &h.node,
            fence.item_fence(),
            ItemTransferRequest {
                weight_generation_digest: native.source_digest(),
                command: CommandRef::new(
                    fence.game_session_id,
                    CommandId::new(ordinal).map_err(f::debug)?,
                ),
                source_item_instance_id: receipt.item_instance_id,
                destination: if backpack {
                    ItemTransferDestination::MainBackpack
                } else {
                    ItemTransferDestination::ContainerSlot
                },
                item: generation_item_facts(native, &typed).map_err(f::debug)?,
                backpack: pack,
                content_revision: "content-1".into(),
                ruleset_revision: "ruleset-1".into(),
                sim_revision: "sim-1".into(),
            },
            weights,
        )
        .await
        .map_err(f::debug)?;
    let lane =
        crate::durability::spell_owner_commit::SpellLanePermit::of_fresh_scope(fence.runtime_scope)
            .await
            .map_err(f::debug)?;
    let outcome = h
        .root
        .commit_item_transfer(
            &lane,
            authority,
            &h.node,
            fence.item_fence(),
            &mut transfer,
            weights,
        )
        .await
        .map_err(f::debug)?;
    assert!(
        matches!(outcome, ItemTransferOutcome::Committed(_)),
        "stock transfer: {outcome:?}"
    );
    Ok(sqlx::types::Uuid::from_bytes(receipt.item_instance_id))
}
