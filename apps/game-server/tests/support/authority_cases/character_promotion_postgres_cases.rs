//! SQL authority boundaries for PREM-2b+PREM-5; runtime purchase paths live in dispatch's suite.
use crate::bestiary_postgres_harness::{self as harness, Harness, TestResult, debug};
use crate::durability::npc_purchase::{NpcPurchaseGate, PurchaseOutcome};

struct RefuseAfterStaging;
impl NpcPurchaseGate for RefuseAfterStaging {
    async fn authorize(
        &self,
        tx: &mut crate::durability::item_mint_audit::Type2Transaction<'_>,
        _account: [u8; 16],
        _service: &crate::durability::npc_purchase::PurchaseService,
    ) -> Result<bool, crate::durability::DurabilityError> {
        sqlx::query("INSERT INTO game_character_blessings(character_id,blessing_key,provenance) VALUES(encode($1,'hex')::uuid,'oteryn:blessing.the_wisdom_of_solitude','negative-fixture')")
            .bind(harness::id(harness::CHARACTER).as_slice()).execute(&mut ***tx).await?;
        Err(crate::durability::DurabilityError::InvalidStoredState)
    }
}

#[test]
fn promotion_postgres_fences_callback_error_and_post_fee_failure_roll_back() -> TestResult {
    let Some(admin) = harness::configured_admin() else {
        return Ok(());
    };
    harness::runtime()?.block_on(async move {
        let h=Harness::create(admin,"promotion_atomic",true).await?;
        eprintln!("POSTGRES_STARTED PREM-2b+PREM-5 promotion_atomic version=17.6");
        let request=crate::character_promotion_fixture::prepare(&h).await?;
        let seal=h.recovery.seal_current().map_err(debug)?;
        let authority=h.root.open_character_authority(&seal).await.map_err(debug)?;
        let coordinator=crate::gameplay_transport::spell_premium_coordinator::SpellPremiumCoordinator::new(Default::default(),None,None,Some(h.root.clone()));
        let sequencer=crate::durability::character_revision_sequencer::CharacterRevisionSequencer::new();
        let fence=harness::fence(1)?;
        for case in ["account","backpack","generation"] {
            let mut invalid=request.clone();
            let mut current=fence;
            match case {
                "account"=>invalid.account=harness::id(99),
                "backpack"=>invalid.expected_backpack=None,
                _=>current.character_lease_generation+=1,
            }
            let result=sequencer.acquire(fence.character_id).await.commit_npc_purchase(&h.root,&authority,&h.node,current,invalid,&coordinator).await;
            assert!(matches!(result,Err(crate::durability::character_progression::CharacterProgressionError::AuthorityRejected)),"{case}: {result:?}");
        }
        let result=sequencer.acquire(fence.character_id).await.commit_npc_purchase(&h.root,&authority,&h.node,fence,request.clone(),&RefuseAfterStaging).await;
        assert!(matches!(result,Err(crate::durability::character_progression::CharacterProgressionError::Unavailable(crate::durability::DurabilityError::InvalidStoredState))));
        assert_eq!(h.count("game_character_blessings").await?,0);
        // Fail after the fee and Character receipt have staged, at the permanent effect.
        sqlx::query("CREATE TRIGGER test_refuse_promotion BEFORE INSERT ON game_character_promotions FOR EACH ROW EXECUTE FUNCTION game_character_immutable()").execute(&h.pool).await?;
        let result=sequencer.acquire(fence.character_id).await.commit_npc_purchase(&h.root,&authority,&h.node,fence,request.clone(),&coordinator).await;
        assert!(matches!(result,Err(crate::durability::character_progression::CharacterProgressionError::Unavailable(_))),"{result:?}");
        for relation in ["game_item_fee_burns","game_character_npc_purchase_receipts","game_character_promotions"] {assert_eq!(h.count(relation).await?,0,"{relation}");}
        assert_eq!(h.root_revision().await?,"1");
        let coins:i64=sqlx::query_scalar("SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(harness::id(212).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(coins,2);
        sqlx::query("DROP TRIGGER test_refuse_promotion ON game_character_promotions").execute(&h.pool).await?;
        let mut slot=sequencer.acquire(fence.character_id).await;
        assert!(matches!(slot.commit_npc_purchase(&h.root,&authority,&h.node,fence,request.clone(),&coordinator).await.map_err(debug)?,PurchaseOutcome::Committed(_)));
        assert!(matches!(slot.commit_npc_purchase(&h.root,&authority,&h.node,fence,request,&coordinator).await.map_err(debug)?,PurchaseOutcome::Committed(_)));
        assert_eq!(h.count("game_item_fee_burns").await?,1);
        assert_eq!(h.root_revision().await?,"2");
        drop(slot);
        drop(authority);
        let reopened=h.root.open_character_authority(&seal).await.map_err(debug)?;
        assert!(h.root.read_promotion_for_session(&reopened,&h.node,harness::item_fence()?).await.map_err(debug)?);
        let snapshot=h.root.read_character_charm_progression(
            &reopened,&h.node,harness::fence(2)?,
            crate::durability::charm_state::CharmProgressionReadRequest {
                catalogue_revision:"content-1".into(),
                catalogue:crate::charm_state_postgres_cases::promotion_fixture::catalogue(),
                races:vec![],
            },crate::durability::charm_state::BestiaryCharmFacts::new([]).map_err(debug)?,
        ).await.map_err(debug)?;
        assert_eq!(snapshot.balance.echoes_earned,crate::domain::charm::PROMOTION_ECHOES);
        drop(reopened);
        // A damaged restore cannot turn a paid permanent promotion into an absent flag.
        let mut tx=h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
        sqlx::query("DELETE FROM game_character_promotions").execute(&mut *tx).await?;
        tx.commit().await?;
        assert!(h.root.open_character_authority(&seal).await.is_err());
        drop(seal);
        h.cleanup().await
    })
}

#[test]
fn promotion_blessing_postgres_refuses_effect_without_paid_receipt() -> TestResult {
    let Some(admin) = harness::configured_admin() else {
        return Ok(());
    };
    harness::runtime()?.block_on(async move {
        let h=Harness::create(admin,"promotion_guards",true).await?;
        eprintln!("POSTGRES_STARTED PREM-2b+PREM-5 character_promotion version=17.6");
        for statement in [
            "INSERT INTO game_character_promotions(character_id,purchase_occurrence_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid)",
            "INSERT INTO game_character_blessings(character_id,blessing_key,provenance) VALUES(encode($1,'hex')::uuid,'oteryn:blessing.the_wisdom_of_solitude',(encode($2,'hex')::uuid)::text)",
        ] {
            let mut tx=h.pool.begin().await?;
            sqlx::query("SET LOCAL ROLE oteryn_game_runtime").execute(&mut *tx).await?;
            sqlx::query(statement).bind(harness::id(harness::CHARACTER).as_slice()).bind(harness::id(99).as_slice()).execute(&mut *tx).await?;
            assert!(tx.commit().await.is_err(),"unpaid effect must roll back");
        }
        assert_eq!(h.count("game_character_promotions").await?,0);
        assert_eq!(h.count("game_character_blessings").await?,0);
        let seal=h.recovery.seal_current().map_err(debug)?;
        let authority=h.root.open_character_authority(&seal).await.map_err(debug)?;
        assert!(!h.root.read_promotion_for_session(&authority,&h.node,harness::item_fence()?).await.map_err(debug)?);
        let mut stale=harness::item_fence()?;
        stale.character_lease_generation+=1;
        assert!(h.root.read_promotion_for_session(&authority,&h.node,stale).await.is_err());
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
