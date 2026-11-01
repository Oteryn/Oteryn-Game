//! The ordinary USE route composes an instant reset with one durable activation.
use super::*;
use crate::durability::item_use::concoction::kooldown_aid::KOOLDOWN_AID_KEY;
use crate::spell::{Cooldowns, STAIRHOP_COOLDOWN_KEY};
use oteryn_protocol_oteryn::tutorial_notices::TutorialNotice;
use oteryn_protocol_oteryn::world_object::encode_use_item_intent;
use oteryn_simulation_determinism::SemanticTimeMicros;
use std::num::NonZeroU64;

const TEXT: &str = "Your spells are no longer on cooldown.";

pub(super) async fn run(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    harness: &Harness,
    actor: ExactActorRef,
    session: GameSessionId,
    index: &ItemDefinitionIndex,
) -> TestResult {
    let before = armed(owner.owner_now())?;
    install(owner, actor, session, before.clone()).await?;
    let open = encode_use_item_intent(NonZeroU64::new(2).ok_or("bag handle")?);
    let used = encode_use_item_intent(NonZeroU64::new(3).ok_or("concoction handle")?);
    let intent = UseIntent {
        target: UseTarget::Item(NonZeroU64::new(3).ok_or("source handle")?),
        use_with: None,
        use_with_map_item: None,
        use_with_position: None,
    };
    let command = |command_id| -> TestResult<UseCommand> {
        Ok(UseCommand {
            game_session_id: session,
            command_id,
            item_fence: Some(item_fence()?),
        })
    };
    let mut stale = command(900)?;
    stale
        .item_fence
        .as_mut()
        .ok_or("fence")?
        .connection_generation = crate::foundation::ConnectionGeneration::new(2).map_err(debug)?;
    let rejected = owner
        .use_drome_cube(
            actor,
            stale,
            intent.clone(),
            Some(ItemKey::Instance(id(110))),
        )
        .await
        .map_err(|_| "stale fence outcome")?
        .ok_or("stale fence result")?;
    assert_eq!(rejected.disposition, UseDisposition::Rejected);
    assert_eq!(source_quantity(harness).await?, 1);
    assert_eq!(harness.count("game_character_concoction_effects").await?, 0);
    assert_eq!(snapshot(owner, actor, session).await?, before);

    ComposedFreshAdmission::lose_next_kooldown_ack();
    let lost = wire::exchange(
        owner,
        cases::admitted(owner, actor, session)?,
        &[open, used.clone()],
    )
    .await?;
    assert_eq!(lost.results, vec![UseDisposition::Committed]);
    assert!(lost.notices.is_empty());
    assert_eq!(lost.retained.continuity.next_command_id, 2);
    assert_eq!(source_quantity(harness).await?, 0);
    assert_eq!(harness.count("game_item_use_records").await?, 1);
    assert_eq!(harness.count("game_character_concoction_effects").await?, 1);
    assert_eq!(
        snapshot(owner, actor, session).await?,
        before,
        "unknown storage acknowledgement must not apply the instant effect"
    );

    let replayed = wire::exchange(owner, lost.retained, std::slice::from_ref(&used)).await?;
    assert_eq!(replayed.results, vec![UseDisposition::Committed]);
    assert_eq!(
        replayed.notices,
        vec![TutorialNotice::StatusText(TEXT.into())]
    );
    let reset = snapshot(owner, actor, session).await?;
    let after = owner.owner_now();
    assert_eq!(
        reset.spells.len(),
        before.spells.len(),
        "expire, do not erase conditions"
    );
    assert!(reset.spells.values().all(|time| *time <= after));
    for key in ["attack", "healing"] {
        assert!(reset.groups.get(key).is_some_and(|time| *time <= after));
    }
    assert_eq!(
        reset.groups.get(STAIRHOP_COOLDOWN_KEY),
        before.groups.get(STAIRHOP_COOLDOWN_KEY)
    );
    assert_eq!(harness.count("game_character_concoction_effects").await?, 1);
    assert_eq!(harness.count("game_drome_cube_uses").await?, 0);

    // A settled historical result is not permission to reset a later spell.
    let later = armed(after)?;
    install(owner, actor, session, later.clone()).await?;
    let replay = owner
        .use_drome_cube(actor, command(2)?, intent.clone(), None)
        .await
        .map_err(|_| "historical outcome")?
        .ok_or("historical receipt")?;
    assert_eq!(replay.disposition, UseDisposition::Committed);
    assert_eq!(snapshot(owner, actor, session).await?, later);
    assert!(owner.take_tutorial_notice(session).is_none());
    let mut changed = intent.clone();
    changed.target = UseTarget::Item(NonZeroU64::new(4).ok_or("changed handle")?);
    let conflict = owner
        .use_drome_cube(actor, command(2)?, changed, None)
        .await
        .map_err(|_| "conflicting receipt outcome")?
        .ok_or("conflicting receipt")?;
    assert_eq!(conflict.disposition, UseDisposition::Rejected);
    assert_eq!(snapshot(owner, actor, session).await?, later);

    let proof: (bool, bool, i64) = sqlx::query_as(
        "SELECT e.created_xact_id=u.created_xact_id,e.expires_at=e.started_at, \
                octet_length(e.wire_binding)::bigint \
           FROM game_character_concoction_effects e JOIN game_item_use_records u USING(transaction_id) \
          WHERE e.concoction='kooldown_aid' AND u.cause_kind=527 AND u.shape=1",
    ).fetch_one(&harness.pool).await?;
    assert_eq!(proof, (true, true, 32));
    let audit: Vec<u8> = sqlx::query_scalar(
        "SELECT a.envelope FROM game_item_audit_outbox a \
          JOIN game_character_concoction_effects e USING(transaction_id)",
    )
    .fetch_one(&harness.pool)
    .await?;
    let (_, audit) =
        crate::durability::item_use_audit::decode_item_use_envelope(&audit).map_err(debug)?;
    assert_eq!(audit.cause.ok_or("audit cause")?.kind, 527);
    assert_eq!(audit.source_before.ok_or("before")?.quantity, 1);
    assert_eq!(audit.source_after.ok_or("after")?.quantity, 0);
    assert!(audit.flask.is_none());
    assert!(
        sqlx::query(
            "UPDATE game_character_concoction_effects SET started_at=started_at-interval '1 day'"
        )
        .execute(&harness.pool)
        .await
        .is_err()
    );
    assert!(
        sqlx::query("DELETE FROM game_character_concoction_effects")
            .execute(&harness.pool)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("TRUNCATE game_character_concoction_effects")
            .execute(&harness.pool)
            .await
            .is_err()
    );
    assert!(
        sqlx::query(
            "INSERT INTO game_character_concoction_effects \
        (transaction_id,character_id,concoction,started_at,expires_at,wire_binding) \
        VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'kooldown_aid',now(),now(),$3)"
        )
        .bind(id(200).as_slice())
        .bind(id(41).as_slice())
        .bind([0_u8; 32].as_slice())
        .execute(&harness.pool)
        .await
        .is_err()
    );

    let root = crate::durability::DurabilityRoot::connect_test_runtime(&harness.database.url)?;
    assert!(root.maintain_ready_once().await?);
    runtime_role(&root).await?;
    let seal = harness.recovery.seal_current().map_err(debug)?;
    let authority = root.open_character_authority(&seal).await.map_err(debug)?;
    let original = crate::foundation::CommandRef::new(
        session,
        crate::foundation::CommandId::new(2).map_err(debug)?,
    );
    assert!(
        root.read_kooldown_aid_receipt(
            &authority,
            item_fence()?,
            original,
            Sha256::digest(&used).into()
        )
        .await
        .map_err(debug)?
    );
    drop(authority);
    drop(seal);
    drop(root);
    assert_eq!(
        snapshot(owner, actor, session).await?,
        later,
        "restart read does not reset live spells"
    );

    seed_next(harness, index).await?;
    let definition = index
        .definition_ref(
            "Item",
            KOOLDOWN_AID_KEY,
            index
                .revision(KOOLDOWN_AID_KEY)
                .ok_or("kooldown revision")?,
        )
        .ok_or("kooldown index")?;
    let next = encode_item_use_intent(&UseIntent {
        target: UseTarget::ItemByDefinition(definition),
        use_with: None,
        use_with_map_item: None,
        use_with_position: None,
    })
    .map_err(debug)?;
    let cooling = wire::exchange(owner, replayed.retained, std::slice::from_ref(&next)).await?;
    assert_eq!(cooling.results, vec![UseDisposition::Exhausted]);
    assert!(cooling.notices.is_empty());
    assert_eq!(next_quantity(harness).await?, 1);
    assert_eq!(snapshot(owner, actor, session).await?, later);
    // Only the isolated fixture's effect clock is changed; session/lease clocks stay current.
    effect_clock(harness, 86_399).await?;
    let boundary = wire::exchange(owner, cooling.retained, std::slice::from_ref(&next)).await?;
    assert_eq!(boundary.results, vec![UseDisposition::Exhausted]);
    assert_eq!(next_quantity(harness).await?, 1);
    effect_clock(harness, 86_400).await?;
    let ready = wire::exchange(owner, boundary.retained, std::slice::from_ref(&next)).await?;
    assert_eq!(ready.results, vec![UseDisposition::Committed]);
    assert_eq!(ready.notices, vec![TutorialNotice::StatusText(TEXT.into())]);
    assert_eq!(next_quantity(harness).await?, 0);
    assert_eq!(harness.count("game_item_use_records").await?, 2);
    assert_eq!(harness.count("game_character_concoction_effects").await?, 2);
    let reset_again = snapshot(owner, actor, session).await?;
    assert!(
        reset_again
            .spells
            .values()
            .all(|time| *time <= owner.owner_now())
    );
    assert_eq!(
        reset_again.groups.get(STAIRHOP_COOLDOWN_KEY),
        later.groups.get(STAIRHOP_COOLDOWN_KEY)
    );
    eprintln!("KOOLDOWN_AID POSTGRES_CASE_COMPLETED");
    Ok(())
}

fn armed(now: SemanticTimeMicros) -> TestResult<Cooldowns> {
    let future = now.checked_add(3_600_000_000).map_err(debug)?;
    Ok(Cooldowns {
        spells: [
            ("oteryn:spell.light_healing".into(), future),
            ("other_unresolved_spell".into(), future),
        ]
        .into(),
        groups: [
            ("attack".into(), future),
            ("healing".into(), future),
            (STAIRHOP_COOLDOWN_KEY.into(), future),
        ]
        .into(),
    })
}

async fn install(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
    cooldowns: Cooldowns,
) -> TestResult {
    let _runtime = owner.runtime.lock().await;
    let mut states = owner.states.spell_states.lock().await;
    let (_, _, state) = states
        .actors
        .iter_mut()
        .find(|(a, s, _)| *a == actor && *s == session)
        .ok_or("spell state")?;
    state.cooldowns = cooldowns;
    Ok(())
}

async fn snapshot(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
) -> TestResult<Cooldowns> {
    let _runtime = owner.runtime.lock().await;
    let states = owner.states.spell_states.lock().await;
    Ok(states
        .actors
        .iter()
        .find(|(a, s, _)| *a == actor && *s == session)
        .ok_or("spell state")?
        .2
        .cooldowns
        .clone())
}

async fn seed_next(harness: &Harness, index: &ItemDefinitionIndex) -> TestResult {
    let mut tx = harness.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family, \
        definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) \
        VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,$4,1,1,encode($5,'hex')::uuid)")
        .bind(id(114).as_slice()).bind(id(42).as_slice()).bind(KOOLDOWN_AID_KEY)
        .bind(index.revision(KOOLDOWN_AID_KEY).ok_or("kooldown revision")?)
        .bind(id(115).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id, \
        placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid, \
        encode($3,'hex')::uuid,2,encode($4,'hex')::uuid)")
        .bind(id(114).as_slice()).bind(id(42).as_slice()).bind(id(80).as_slice())
        .bind(id(116).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

async fn next_quantity(harness: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(id(114).as_slice())
    .fetch_one(&harness.pool)
    .await?)
}

async fn effect_clock(harness: &Harness, seconds: i64) -> TestResult {
    // format(%L) quotes a database timestamp, never interpolated user text.
    let ddl: String = sqlx::query_scalar(
        "SELECT format('CREATE OR REPLACE FUNCTION public.game_daily_effect_now() \
         RETURNS timestamptz LANGUAGE sql VOLATILE AS %L', \
         format('SELECT %L::timestamptz', min(started_at)+make_interval(secs => $1::bigint::double precision))) \
         FROM game_character_concoction_effects WHERE concoction='kooldown_aid'",
    ).bind(seconds).fetch_one(&harness.pool).await?;
    // The only interpolated value is a fixture timestamp quoted by PostgreSQL's %L.
    sqlx::query(sqlx::AssertSqlSafe(ddl))
        .execute(&harness.pool)
        .await?;
    Ok(())
}
