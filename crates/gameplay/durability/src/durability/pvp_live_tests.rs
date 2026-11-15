use super::*;
use crate::bestiary_postgres_harness as harness;
use crate::foundation::{CommandId, CommandRef};
use harness::{CHARACTER, Harness, TestResult, WORLD, debug, id};

fn run<F>(tag: &'static str, body: F) -> TestResult
where
    F: AsyncFnOnce(&Harness) -> TestResult + Send + 'static,
{
    let Some(admin) = harness::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> std::result::Result<(), String> {
            let result: TestResult = harness::runtime().map_err(debug)?.block_on(async move {
                let h = Harness::create(admin, tag, true).await?;
                let result = body(&h).await.map_err(|e| e.to_string());
                h.cleanup().await?;
                result.map_err(Into::into)
            });
            result.map_err(debug)
        })?
        .join()
        .map_err(|_| "PvP test thread failed")?
        .map_err(Into::into)
}

async fn target(tx: &mut Transaction<'_, Postgres>) -> TestResult {
    sqlx::query("INSERT INTO game_character_roots(character_id,account_id,world_id,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,name,sex) SELECT encode($1,'hex')::uuid,account_id,world_id,1,1,profile_revision,ruleset_revision,content_revision,starter_template_revision,'Target Hero',sex FROM game_character_roots WHERE character_id=encode($2,'hex')::uuid")
        .bind(id(61).as_slice()).bind(id(CHARACTER).as_slice()).execute(&mut **tx).await?;
    Ok(())
}

fn facts() -> LivePvpFacts {
    LivePvpFacts {
        world: WorldPvpType::Open,
        relation: PvpRelation {
            war_between: false,
            same_party_or_guild: Some(false),
            friendly_fire: false,
            target_aggressor_of_actor: false,
            target_aggressor_of_actor_group: false,
        },
        actor_black: false,
        target_black: false,
        marked: false,
        post_login_immune: false,
        same_party: false,
        attacker: id(CHARACTER),
        target: id(61),
        world_id: id(WORLD),
        actor_placement: id(71),
        target_placement: id(72),
    }
}

async fn authority(
    tx: &mut Transaction<'_, Postgres>,
    h: &Harness,
    n: u64,
) -> TestResult<(
    super::super::spell_owner_commit::SpellLanePermit,
    SpellItemAuthority,
)> {
    let permit =
        super::super::spell_owner_commit::SpellLanePermit::of_fresh_scope(harness::scope()?)
            .await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let recovery = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let command = CommandRef::new(
        harness::item_fence()?.game_session_id,
        CommandId::new(n).map_err(debug)?,
    );
    let authority =
        super::super::spell_item_transaction::assert_spell_item_authority_in_transaction(
            tx,
            &permit,
            &h.root,
            &recovery,
            &h.node,
            &harness::item_fence()?,
            command,
            [7; 32],
        )
        .await
        .map_err(debug)?;
    Ok((permit, authority))
}

#[test]
fn active_war_harm_does_not_assign_white_skull_or_unjustified_receipt() -> TestResult {
    run("pvp_war_harm", async |h| {
        let mut tx = h.pool.begin().await?;
        target(&mut tx).await?;
        let (_permit, authority) = authority(&mut tx, h, 900).await?;
        let mut facts = facts();
        facts.relation.war_between = true;
        record_live_pvp_harm_in_transaction(&mut tx, &authority, &facts, 0, [3; 32], 10)
            .await
            .map_err(debug)?;
        let unjustified: bool = sqlx::query_scalar(
            "SELECT unjustified FROM game_character_pvp_harm WHERE attacker=encode($1,'hex')::uuid",
        )
        .bind(facts.attacker.as_slice())
        .fetch_one(&mut *tx)
        .await?;
        assert!(
            !unjustified,
            "war opponents are justified even when unmarked"
        );
        let white: bool = sqlx::query_scalar("SELECT white_skull_until IS NOT NULL FROM game_character_pvp_state WHERE character_id=encode($1,'hex')::uuid").bind(facts.attacker.as_slice()).fetch_one(&mut *tx).await?;
        assert!(!white);
        let contributors = death_contributors_in_transaction(
            &mut tx,
            crate::domain::CharacterId::from_bytes(facts.target).map_err(debug)?,
        )
        .await
        .map_err(debug)?;
        assert_eq!(contributors.len(), 1);
        assert!(!contributors[0].unjustified);
        tx.rollback().await?;
        Ok(())
    })
}

#[test]
fn defensive_retaliation_preserves_original_aggressor() -> TestResult {
    run("pvp_retaliation", async |h| {
        let mut tx = h.pool.begin().await?;
        target(&mut tx).await?;
        let (_permit, authority) = authority(&mut tx, h, 901).await?;
        let mut facts = facts();
        facts.relation.target_aggressor_of_actor = true;
        facts.marked = true;
        sqlx::query("INSERT INTO game_character_pvp_aggression(attacker,target,until_at,scope_generation,attacker_placement,target_placement) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,clock_timestamp()+interval '70 seconds',1,$3,$4)")
            .bind(facts.target.as_slice()).bind(facts.attacker.as_slice()).bind(facts.target_placement.as_slice()).bind(facts.actor_placement.as_slice()).execute(&mut *tx).await?;
        for ordinal in [0, 1] {
            record_live_pvp_harm_in_transaction(&mut tx, &authority, &facts, ordinal, [4; 32], 10)
                .await
                .map_err(debug)?;
        }
        let reverse: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_pvp_aggression WHERE attacker=encode($1,'hex')::uuid AND target=encode($2,'hex')::uuid)")
            .bind(facts.attacker.as_slice()).bind(facts.target.as_slice()).fetch_one(&mut *tx).await?;
        assert!(
            !reverse,
            "defending must not make the original aggressor a retaliator"
        );
        let originals: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_character_pvp_aggression")
                .fetch_one(&mut *tx)
                .await?;
        assert_eq!(originals, 1);
        let unjustified: bool =
            sqlx::query_scalar("SELECT bool_or(unjustified) FROM game_character_pvp_harm")
                .fetch_one(&mut *tx)
                .await?;
        assert!(!unjustified);
        tx.rollback().await?;
        Ok(())
    })
}
