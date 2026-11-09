//! M26: production room reads retain real activity membership and current scope custody.
use super::TestResult;
use super::fixture::debug;
use crate::foundation::GameSessionId;
use crate::gameplay_transport::{ComposedFreshAdmission, connection::AdmittedSession};

pub(super) async fn check(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    origin_session: GameSessionId,
) -> TestResult {
    use crate::durability::quest_state::rotten_blood::room_taints::read_in_transaction;
    use crate::durability::spell_item_transaction::{
        assert_spell_item_scope_in_transaction, begin_spell_owner_transaction,
    };
    let permit = owner.spell_lane_permit().await.ok_or("room taint lane")?;
    let runtime = owner.runtime.lock().await;
    let character = admitted
        .item_fence
        .ok_or("activity character")?
        .character_id;
    let digest = runtime.content_pin().server_artifact_digest();
    let pass = owner.root.try_issue_semantic_pass().map_err(debug)?;
    let mut context = (owner, &runtime, &permit);
    pass.run_with_context(&mut context, |holder, deadline, context| Box::pin(async move {
        let (owner, runtime, permit) = context;
        let mut tx = begin_spell_owner_transaction(holder, deadline).await?;
        let binding = runtime.binding();
        let authority = assert_spell_item_scope_in_transaction(&mut tx, permit, owner.root,
            owner.character, owner.holder, binding.runtime_scope(), binding.scope_generation().get())
            .await.map_err(|_| crate::durability::DurabilityError::Unavailable)?;
        let committed_default = read_in_transaction(&mut tx, &authority, owner.holder, &[admitted.game_session_id], digest).await;
        assert_eq!(committed_default.map_err(|_| crate::durability::DurabilityError::Unavailable)?, vec![(admitted.game_session_id, 0)], "committed source default");
        // A transaction-local fixture exercises nonzero reads. All receipt
        // constraints remain active; this unproven fixture is rolled back.
        sqlx::query("INSERT INTO game_character_rotten_blood(character_id,regular_taints,child_flags,final_taint,completed,last_cleanse_at) VALUES(encode($1,'hex')::uuid,3,7,false,false,NULL) ON CONFLICT(character_id) DO UPDATE SET regular_taints=3,child_flags=7")
            .bind(character.as_bytes().as_slice()).execute(&mut *tx).await?;
        let actual = read_in_transaction(&mut tx, &authority, owner.holder, &[admitted.game_session_id], digest).await;
        assert!(actual.is_ok(), "actual activity participant taints: {actual:?}");
        assert_eq!(actual.map_err(|_| crate::durability::DurabilityError::Unavailable)?, vec![(admitted.game_session_id,3)]);
        let foreign = read_in_transaction(&mut tx, &authority, owner.holder, &[origin_session], digest).await;
        assert!(matches!(foreign, Err(crate::durability::spell_item_transaction::SpellItemError::Rejected("room taint participant"))), "origin command/session cannot borrow the Instance: {foreign:?}");
        let wrong_pin = read_in_transaction(&mut tx, &authority, owner.holder, &[admitted.game_session_id], [0x91;32]).await;
        assert!(matches!(wrong_pin, Err(crate::durability::spell_item_transaction::SpellItemError::Rejected("room taint participant pin"))), "destination content is exact: {wrong_pin:?}");
        tx.rollback().await?;
        Ok(())
    })).await.map_err(debug)?;
    eprintln!(
        "M26_ROTTEN_ROOM_TAINTS production_reader=true committed_default=true transaction_fixture_rolled_back=true foreign_session_rejected=true wrong_pin_rejected=true"
    );
    Ok(())
}
