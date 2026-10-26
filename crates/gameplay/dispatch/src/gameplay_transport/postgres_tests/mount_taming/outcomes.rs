//! Refusal, failure, replay and reload qualification.
use super::*;

pub(super) async fn run(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    actor: ExactActorRef,
    session: GameSessionId,
    index: &ItemDefinitionIndex,
) -> TestResult {
    use crate::domain::appearance::{
        AccountUnlocks, AppearanceContent, EarnedCosmetics, UnlockFact,
    };
    use oteryn_protocol_oteryn::world_object::{UseTarget, encode_item_use_intent};
    let key = "oteryn:item.tibia.i5907";
    let source = index
        .definition_ref("Item", key, index.revision(key).ok_or("sling")?)
        .ok_or("sling index")?;
    let use_on = |entity| UseIntent {
        target: UseTarget::ItemByDefinition(source),
        use_with: Some(entity),
        use_with_map_item: None,
        use_with_position: None,
    };
    let mut retained = admitted(owner, actor, session)?;
    let (wrong, entity) = spawn(owner, actor, "oteryn:creature.rat", 1).await?;
    let (results, next) = wire::exchange(
        owner,
        retained,
        &[encode_item_use_intent(&use_on(entity)).map_err(debug)?],
    )
    .await?;
    assert_eq!(results, [UseDisposition::NothingToUse]);
    retained = next;
    owner
        .runtime
        .lock()
        .await
        .remove_test_actor(wrong)
        .map_err(debug)?;
    let (far, entity) = spawn(owner, actor, "oteryn:creature.bear", 4).await?;
    let (results, next) = wire::exchange(
        owner,
        retained,
        &[encode_item_use_intent(&use_on(entity)).map_err(debug)?],
    )
    .await?;
    assert_eq!(results, [UseDisposition::TooFar]);
    retained = next;
    owner
        .runtime
        .lock()
        .await
        .remove_test_actor(far)
        .map_err(debug)?;
    assert_eq!(quantity(h).await?, 20);
    assert_eq!(h.count("game_mount_taming_attempts").await?, 0);
    let (target, entity) = spawn(owner, actor, "oteryn:creature.bear", 1).await?;
    let mut stale_entity = entity;
    stale_entity.generation += 1;
    let (results, next) = wire::exchange(
        owner,
        retained,
        &[encode_item_use_intent(&use_on(stale_entity)).map_err(debug)?],
    )
    .await?;
    assert_eq!(results, [UseDisposition::NoTarget]);
    retained = next;
    use crate::gameplay_transport::capabilities::{OfferedCapability, SelectedCapabilities};
    let offered = [
        OfferedCapability {
            id: 4,
            requires: &[],
        },
        OfferedCapability {
            id: 6,
            requires: &[],
        },
        OfferedCapability {
            id: 14,
            requires: &[4],
        },
        OfferedCapability {
            id: 15,
            requires: &[4],
        },
    ];
    for (selected, expected) in [
        (&[4, 6, 14][..], UseDisposition::Rejected),
        (&[4, 14, 15][..], UseDisposition::NoTarget),
    ] {
        let mut probe = retained;
        probe.continuity.selected_capabilities =
            SelectedCapabilities::select(&offered, selected).ok_or("caps")?;
        let (results, _) = wire::exchange(
            owner,
            probe,
            &[encode_item_use_intent(&use_on(entity)).map_err(debug)?],
        )
        .await?;
        assert_eq!(results, [expected]);
    }
    owner
        .runtime
        .lock()
        .await
        .remove_test_actor(target)
        .map_err(debug)?;
    assert_eq!(quantity(h).await?, 20);
    assert_eq!(h.count("game_mount_taming_attempts").await?, 0);
    // Canary Bear failure row 1 flees; row 2 breaks one slingshot; row 3 leaves
    // the live hostile bear with its normal AI/combat owner and no item consumption.
    let rule = crate::content::native_gameplay::mount_taming::rules()
        .iter()
        .find(|r| r.item_key == key)
        .ok_or("rule")?;
    owner.states.chat.admit_as(
        session,
        "Tamer",
        Some(*item_fence()?.character_id.as_bytes()),
        owner.owner_now(),
    );
    for (expected, bucket, remaining) in [(1_u8, 60_u128, 20), (2, 61, 19), (3, 62, 19)] {
        let draw = u64::try_from((bucket << 64).div_ceil(300))?;
        assert_eq!(rule.decide(draw), expected);
        let (target, entity) = spawn(owner, actor, "oteryn:creature.bear", 1).await?;
        crate::gameplay_transport::mount_taming::test_draw::set(draw);
        let payload = encode_item_use_intent(&use_on(entity)).map_err(debug)?;
        let command = retained.continuity.next_command_id;
        let (results, next) =
            wire::exchange(owner, retained, std::slice::from_ref(&payload)).await?;
        assert_eq!(results, [UseDisposition::Committed]);
        retained = next;
        assert_eq!(quantity(h).await?, remaining);
        let alive = owner
            .runtime
            .lock()
            .await
            .current_live_creature_identity(target)
            .is_ok();
        assert_eq!(alive, expected != 1);
        assert_eq!(
            owner.states.chat.drain(session),
            [oteryn_protocol_oteryn::chat::ChatLine::Private {
                speaker_name: "Game".into(),
                text: rule.failures[usize::from(expected - 1)].text.clone()
            }]
        );
        // Fresh transport with the same original CommandRef answers its immutable receipt.
        let mut replay = retained;
        replay.continuity.next_command_id = command;
        let (results, _) = wire::exchange(owner, replay, &[payload]).await?;
        assert_eq!(results, [UseDisposition::Committed]);
        assert_eq!(quantity(h).await?, remaining);
        if alive {
            owner
                .runtime
                .lock()
                .await
                .remove_test_actor(target)
                .map_err(debug)?;
        }
    }
    assert_eq!(h.count("game_account_appearance_unlocks").await?, 0);
    assert_eq!(h.count("game_mount_taming_attempts").await?, 3);
    let (target, entity) = spawn(owner, actor, "oteryn:creature.bear", 1).await?;
    crate::gameplay_transport::mount_taming::test_draw::set(0);
    let success = use_on(entity);
    let success_command = retained.continuity.next_command_id;
    let payload = encode_item_use_intent(&success).map_err(debug)?;
    let (results, next) = wire::exchange(owner, retained, std::slice::from_ref(&payload)).await?;
    assert_eq!(results, [UseDisposition::Committed]);
    retained = next;
    assert!(
        owner
            .runtime
            .lock()
            .await
            .current_live_creature_identity(target)
            .is_err()
    );
    assert_eq!(quantity(h).await?, 18);
    assert_eq!(h.count("game_account_appearance_unlocks").await?, 1);
    assert_eq!(h.count("game_account_achievements").await?, 2);
    assert_eq!(h.count("game_item_use_records").await?, 2);
    // The same existing account-cosmetics view used by mount selection sees the earned key.
    let cosmetics = owner
        .active_generation
        .and_then(crate::content::ActiveGeneration::native_gameplay)
        .and_then(|n| n.cosmetics())
        .ok_or("cosmetics")?;
    let current = owner
        .root
        .read_current_character_appearance_state(
            owner.character,
            owner.holder,
            postgres_fixture::fence(1)?,
            cosmetics.earned_grant_keys(),
        )
        .await
        .map_err(debug)?;
    let unlocks = EarnedCosmetics {
        content: cosmetics,
        facts: &current.earned,
    };
    assert_eq!(unlocks.mount(&rule.mount_key), UnlockFact::Compatible);
    let (target, entity) = spawn(owner, actor, "oteryn:creature.bear", 1).await?;
    let (results, next) = wire::exchange(
        owner,
        retained,
        &[encode_item_use_intent(&use_on(entity)).map_err(debug)?],
    )
    .await?;
    assert_eq!(results, [UseDisposition::RequirementNotMet]);
    retained = next;
    assert!(
        owner
            .runtime
            .lock()
            .await
            .current_live_creature_identity(target)
            .is_ok()
    );
    assert_eq!(quantity(h).await?, 18);
    assert_eq!(h.count("game_mount_taming_attempts").await?, 4);
    // A newly opened production durability root resolves the original receipt and ownership.
    let root = crate::durability::DurabilityRoot::connect_test_runtime(&h.database.url)?;
    assert!(root.maintain_ready_once().await?);
    runtime_role(&root).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = root.open_character_authority(&seal).await.map_err(debug)?;
    use sha2::Digest;
    let binding = sha2::Sha256::digest(&payload).into();
    let receipt = root
        .read_mount_taming_receipt(
            &authority,
            item_fence()?,
            crate::foundation::CommandRef::new(
                session,
                crate::foundation::CommandId::new(success_command).map_err(debug)?,
            ),
            binding,
        )
        .await
        .map_err(debug)?
        .ok_or("receipt")?;
    assert_eq!(receipt.outcome, 0);
    let current = root
        .read_current_character_appearance_state(
            &authority,
            &h.node,
            postgres_fixture::fence(1)?,
            cosmetics.earned_grant_keys(),
        )
        .await
        .map_err(debug)?;
    assert_eq!(
        EarnedCosmetics {
            content: cosmetics,
            facts: &current.earned
        }
        .mount(&rule.mount_key),
        UnlockFact::Compatible
    );
    // Stale connection-generation writes are refused atomically.
    let mut stale = retained;
    let mut fence = item_fence()?;
    fence.connection_generation = crate::foundation::ConnectionGeneration::new(2).map_err(debug)?;
    stale.item_fence = Some(fence);
    let (results, _) = wire::exchange(
        owner,
        stale,
        &[encode_item_use_intent(&use_on(entity)).map_err(debug)?],
    )
    .await?;
    assert_eq!(results, [UseDisposition::Rejected]);
    assert_eq!(quantity(h).await?, 18);
    rollback(owner, h, actor, retained, index).await?;
    Ok(())
}

async fn rollback(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    actor: ExactActorRef,
    retained: AdmittedSession,
    index: &ItemDefinitionIndex,
) -> TestResult {
    // A commit-time failure happens after the grant, achievements and burn were staged.
    // No subset may survive; the uncertain Channel owner must stop serving it.
    sqlx::raw_sql("CREATE FUNCTION taming_test_abort() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'MOUNTTAME injected commit rejection'; END $$; CREATE CONSTRAINT TRIGGER taming_test_abort AFTER INSERT ON game_mount_taming_attempts DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION taming_test_abort();")
        .execute(&h.pool).await?;
    let (_, entity) = spawn(owner, actor, "oteryn:creature.midnight_panther", 1).await?;
    let key = "oteryn:item.tibia.i12306";
    let source = index
        .definition_ref("Item", key, index.revision(key).ok_or("whip")?)
        .ok_or("whip ref")?;
    let payload = oteryn_protocol_oteryn::world_object::encode_item_use_intent(&UseIntent {
        target: oteryn_protocol_oteryn::world_object::UseTarget::ItemByDefinition(source),
        use_with: Some(entity),
        use_with_map_item: None,
        use_with_position: None,
    })
    .map_err(debug)?;
    crate::gameplay_transport::mount_taming::test_draw::set(0);
    let (results, _) = wire::exchange(owner, retained, &[payload]).await?;
    assert!(results.is_empty());
    let quantity: i64 = sqlx::query_scalar(
        "SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(id(110).as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(quantity, 20);
    assert_eq!(h.count("game_mount_taming_attempts").await?, 4);
    assert_eq!(h.count("game_account_appearance_unlocks").await?, 1);
    assert_eq!(h.count("game_account_achievements").await?, 2);
    assert_eq!(h.count("game_item_use_records").await?, 2);
    assert!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .is_err()
    );
    Ok(())
}
