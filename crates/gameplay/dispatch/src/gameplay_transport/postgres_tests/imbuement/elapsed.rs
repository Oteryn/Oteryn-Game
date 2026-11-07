use super::*;
use crate::foundation::ExactActorRef;
use crate::gameplay_transport::connection::UseCommand;
use crate::gameplay_transport::item_move::{EquipmentMove, EquipmentMoveResult};
use crate::gameplay_transport::item_view::ItemKey;
use std::time::Duration;

async fn budget(h: &Harness, item: u8) -> TestResult<Option<i64>> {
    Ok(sqlx::query_scalar("SELECT remaining_ms FROM game_item_imbuements WHERE item_instance_id=encode($1,'hex')::uuid AND slot=0")
        .bind(id(item).as_slice()).fetch_optional(&h.pool).await?)
}
async fn equip(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    actor: ExactActorRef,
    command_id: u64,
    equip: bool,
) -> TestResult {
    let request = if equip {
        EquipmentMove::Equip {
            item: ItemKey::Instance(id(71)),
            to: EquipmentSlot::RightHand,
        }
    } else {
        EquipmentMove::Unequip {
            item: ItemKey::Instance(id(71)),
            from: EquipmentSlot::RightHand,
        }
    };
    let result = owner
        .move_equipment(
            actor,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id,
                item_fence: admitted.item_fence,
            },
            request,
        )
        .await;
    assert!(
        matches!(result, EquipmentMoveResult::Committed),
        "{result:?}"
    );
    Ok(())
}

pub(super) async fn exercise(
    h: &Harness,
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    sword_ref: NonZeroU32,
    admitted: AdmittedSession,
    actor: ExactActorRef,
) -> TestResult {
    // Protection lifetimes are fixture provisioning only; equipment moves, reads and expiry
    // below run through the same production owners as the wire command.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
    sqlx::query("UPDATE game_item_imbuements SET category=3,imbuement_key='oteryn:imbuement.snake_skin',tier=3 WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(id(71).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_imbuements(item_instance_id,slot,category,imbuement_key,imbuement_revision,tier,remaining_ms,applying_occurrence_id,applying_transaction_id) VALUES(encode($1,'hex')::uuid,1,2,'oteryn:imbuement.lich_shroud','r1',3,1000,game_character_uuid_v7(),game_character_uuid_v7())")
        .bind(id(71).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    // The real imbuement dispatch reads the fixture lifetime before it enters equipment.
    let (admitted, _, reply) = exchange(
        owner,
        sword_ref,
        admitted,
        1,
        Some(ImbuementAction::Query),
        true,
    )
    .await?;
    assert_eq!(
        reply.and_then(|r| r.window).ok_or("window")?.applied[0].remaining_ms,
        1000
    );
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    // Featherweight lives on the main backpack, which is owned separately from hand slots.
    sqlx::query("INSERT INTO game_item_imbuements(item_instance_id,slot,category,imbuement_key,imbuement_revision,tier,remaining_ms,applying_occurrence_id,applying_transaction_id) VALUES(encode($1,'hex')::uuid,0,8,'oteryn:imbuement.featherweight','r1',1,72000000,game_character_uuid_v7(),game_character_uuid_v7())")
        .bind(id(70).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    let f = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "current fence")?
        .ok_or("session")?;
    assert!(
        !owner
            .root
            .read_worn_imbuements(owner.character, owner.holder, f)
            .await
            .map_err(debug)?
            .is_empty(),
        "worn reader"
    );
    assert!(
        owner
            .checkpoint_timed_equipment(actor, admitted.game_session_id, true)
            .await,
        "initial clock"
    );
    owner.tick_timed_items().await;
    owner.clock_origin -= Duration::from_secs(2);
    owner.tick_timed_items().await;
    assert_eq!(
        budget(h, 71).await?,
        Some(1000),
        "inventory pauses combat budget"
    );
    assert!(
        budget(h, 70).await?.is_some_and(|n| n < 72_000_000),
        "worn backpack consumes without combat"
    );
    equip(owner, admitted, actor, 100, true).await?;
    owner.clock_origin -= Duration::from_secs(2);
    owner.tick_timed_items().await;
    assert_eq!(
        budget(h, 71).await?,
        Some(1000),
        "equipped but out of fight pauses"
    );
    {
        use crate::content::ReferenceResistanceKind as Kind;
        let runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        let state = states.get(&runtime, actor, admitted.game_session_id).ok_or("protection state")?;
        assert_eq!(state.equipped_element_protection(Kind::Earth).map(|p| p.numerator), Some(15));
        assert_eq!(state.equipped_element_protection(Kind::Death).map(|p| p.numerator), Some(10));
    }
    // Inject a real transaction failure. The unchanged request is retried, with no
    // time lost and no debit of the recovery interval.
    sqlx::raw_sql("CREATE FUNCTION imbuetime_fail() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'checkpoint failure'; END $$; CREATE CONSTRAINT TRIGGER imbuetime_fail AFTER INSERT ON game_item_imbuement_checkpoints DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION imbuetime_fail();").execute(&h.pool).await?;
    owner.clock_origin -= Duration::from_secs(2);
    let before = budget(h, 70).await?;
    assert!(
        !owner
            .checkpoint_timed_equipment(actor, admitted.game_session_id, true)
            .await
    );
    assert_eq!(budget(h, 70).await?, before, "failed commit rolls back");
    {
        let effects = owner.read_admission_effects(admitted.game_session_id).await.map_err(|_| "suspended effects")?.ok_or("effects")?;
        assert!(!effects.0.protections.contains_key(&crate::content::ReferenceResistanceKind::Earth));
        let runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        assert_eq!(states.get(&runtime, actor, admitted.game_session_id).ok_or("suspended state")?.equipped_element_protection(crate::content::ReferenceResistanceKind::Earth), None);
    }
    sqlx::raw_sql("DROP TRIGGER imbuetime_fail ON game_item_imbuement_checkpoints; DROP FUNCTION imbuetime_fail();").execute(&h.pool).await?;
    // Commit the retained request without letting the owner observe its acknowledgement.
    let retained = owner
        .states
        .timed_items
        .imbuements
        .lock()
        .await
        .get(&admitted.game_session_id)
        .and_then(|clock| clock.pending.clone())
        .ok_or("retained checkpoint")?;
    let current = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "fence")?
        .ok_or("current")?;
    owner
        .root
        .checkpoint_imbuements(owner.character, owner.holder, current, retained)
        .await
        .map_err(debug)?;
    let committed = budget(h, 70).await?;
    owner.clock_origin -= Duration::from_secs(10);
    assert!(
        owner
            .checkpoint_timed_equipment(actor, admitted.game_session_id, true)
            .await
    );
    assert_eq!(
        budget(h, 70).await?,
        committed,
        "lost acknowledgement replays without a second debit"
    );
    let recovered = budget(h, 70).await?.ok_or("recovered")?;
    assert!(
        (1_900..3_000).contains(&(before.ok_or("before")? - recovered)),
        "failure interval is paused"
    );
    // Enter combat using the same attack-state owner as a received hit.
    {
        let runtime = owner.runtime.lock().await;
        owner.states.attack.lock().await.record_hit_taken(
            &runtime,
            actor,
            admitted.game_session_id,
            owner.owner_now(),
        );
    }
    owner.clock_origin -= Duration::from_secs(2);
    owner.tick_timed_items().await;
    assert_eq!(budget(h, 71).await?, None, "expiry frees the occupied slot");
    let count:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_imbuement_checkpoints WHERE changes @> '[{\"after_ms\":0}]'::jsonb").fetch_one(&h.pool).await?;
    assert_eq!(count, 1, "one immutable expiry audit");
    {
        use crate::content::ReferenceResistanceKind as Kind;
        let runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        let state = states.get(&runtime, actor, admitted.game_session_id).ok_or("expired state")?;
        assert_eq!(state.equipped_element_protection(Kind::Earth), None);
        assert_eq!(state.equipped_element_protection(Kind::Death), None);
    }
    // A receipt replay after expiry is identical; a changed request under its key fails closed.
    let (transaction,changes):(String,serde_json::Value)=sqlx::query_as("SELECT transaction_id::text,changes FROM game_item_imbuement_checkpoints WHERE changes @> '[{\"after_ms\":0}]'::jsonb").fetch_one(&h.pool).await?;
    let hex = transaction.replace('-', "");
    let mut transaction_id = [0; 16];
    for (i, b) in transaction_id.iter_mut().enumerate() {
        *b = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16)?;
    }
    let mut request = crate::durability::imbuement::ImbuementCheckpoint {
        transaction_id,
        changes: serde_json::from_value(changes)?,
    };
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "fence")?
        .ok_or("current")?;
    owner
        .root
        .checkpoint_imbuements(owner.character, owner.holder, fence, request.clone())
        .await
        .map_err(debug)?;
    let mut stale_lifetime = request.clone();
    stale_lifetime.transaction_id = id(210);
    stale_lifetime.changes.retain(|d| d.after_ms == 0);
    assert!(
        owner
            .root
            .checkpoint_imbuements(owner.character, owner.holder, fence, stale_lifetime)
            .await
            .is_err(),
        "an old lifetime cannot be debited again under a fresh receipt"
    );
    let mut oversized = request.clone();
    oversized.transaction_id = id(211);
    oversized.changes = vec![oversized.changes[0].clone(); 31];
    assert!(
        owner
            .root
            .checkpoint_imbuements(owner.character, owner.holder, fence, oversized)
            .await
            .is_err(),
        "the 31st row is outside the checkpoint bound"
    );
    request.changes[0].after_ms += 1;
    assert!(
        owner
            .root
            .checkpoint_imbuements(owner.character, owner.holder, fence, request)
            .await
            .is_err()
    );
    // Unequip through the production handler, then inspect the now-empty slot over the wire.
    equip(owner, admitted, actor, 101, false).await?;
    let (_, _, reply) = exchange(
        owner,
        sword_ref,
        admitted,
        2,
        Some(ImbuementAction::Query),
        true,
    )
    .await?;
    assert!(
        reply
            .and_then(|r| r.window)
            .ok_or("window")?
            .applied
            .is_empty()
    );
    assert!(
        owner
            .stop_timed_actor(actor, admitted.game_session_id)
            .await
    );
    let saved = budget(h, 70).await?;
    owner.clock_origin -= Duration::from_secs(3600);
    // Reconstruct the volatile clock as after logout/restart. The first sample is a baseline,
    // never elapsed offline wall time; the durable remainder is the only source of truth.
    owner.states.timed_items.imbuements.lock().await.clear();
    assert!(
        owner
            .checkpoint_timed_equipment(actor, admitted.game_session_id, true)
            .await
    );
    assert_eq!(
        budget(h, 70).await?,
        saved,
        "offline/restart time is paused"
    );
    let seal = h.recovery.seal_current().map_err(debug)?;
    let reopened = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let reloaded = h
        .root
        .read_worn_imbuements(&reopened, owner.holder, fence)
        .await
        .map_err(debug)?;
    assert_eq!(
        reloaded
            .iter()
            .find(|b| b.item == id(70))
            .map(|b| i64::from(b.remaining_ms)),
        saved
    );
    drop(reopened);
    drop(seal);
    let mut stale = fence;
    stale.character_lease_generation += 1;
    assert!(
        owner
            .root
            .read_worn_imbuements(owner.character, owner.holder, stale)
            .await
            .is_err()
    );
    Ok(())
}
