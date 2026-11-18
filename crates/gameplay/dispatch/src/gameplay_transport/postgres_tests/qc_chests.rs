//! Lane qc: personal quest chests through the production world-callback owner.
use super::fixture::{Harness, debug, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::foundation::{CharacterId, CharacterLease};
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

/// Scenario names `qc-<quest>-chest` select the callback with that key suffix.
pub(super) fn matches(name: &str) -> bool {
    name.starts_with("qc-") && name.ends_with("-chest")
}

#[test]
fn qc_rapier_chest_grants_one_rapier_completes_the_quest_and_refuses_a_second_claim() -> TestResult
{
    run("qc-rapier-chest")
}

/// Skull of Ratha: both bags in the tomb and the underground chest (TibiaWiki
/// Skull of Ratha Quest), at every source placement through the catalogue owner.
#[test]
fn qc_skull_of_ratha_three_source_chests_use_replay_and_refusal() -> TestResult {
    const CLAIMS: [&str; 3] = [
        "oteryn:reward-claim.kv.witch_house_quest",
        "oteryn:reward-claim.quest.u7_6.explorer_society.skull_of_ratha.bag1",
        "oteryn:reward-claim.quest.u7_6.explorer_society.skull_of_ratha.bag2",
    ];
    let selected: Vec<_> = super::quest_catalogue_chests::cases()?
        .into_iter()
        .filter(|case| CLAIMS.contains(&case.claim.as_str()))
        .collect();
    assert_eq!(
        selected
            .iter()
            .map(|case| case.claim.as_str())
            .collect::<std::collections::BTreeSet<_>>(),
        CLAIMS.into_iter().collect(),
        "every Skull of Ratha reward must execute"
    );
    let started = std::time::Instant::now();
    super::quest_catalogue_chests::execute(selected)?;
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_some() {
        println!(
            "POSTGRES_CASE_COMPLETED qc_skull_of_ratha_three_source_chests_use_replay_and_refusal duration={:?}",
            started.elapsed()
        );
    }
    Ok(())
}

#[test]
fn qc_sanctuary_pinned_legion_helmet_chest_uses_live_uid() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let cases: Vec<_> = super::quest_catalogue_chests::cases()?
        .into_iter()
        .filter(|case| case.claim == "oteryn:reward-claim.kv.legiondawnport")
        .collect();
    assert_eq!(cases.len(), 1);
    assert_eq!(cases[0].unique, 6006);
    super::quest_catalogue_chests::execute(cases)
}

fn run(name: &'static str) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let callback = oteryn_gameplay_quests::quest::world_callbacks::load()?
        .into_iter()
        .find(|c| c.key == format!("oteryn:quest-callback/{name}"))
        .ok_or("qc chest callback")?;
    let trigger = callback.selector.position.ok_or("qc chest position")?;
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run(name, move || {
        super::quest_world_callbacks::scenario(name, trigger, false)
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    callback: &Callback,
    trigger: Position,
) -> TestResult {
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    owner
        .runtime
        .lock()
        .await
        .bind_attacker_lease(
            admitted.runtime_actor.ok_or("actor")?,
            admitted.game_session_id,
            CharacterLease::new(
                CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?,
                fence.character_lease_generation,
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;
    let key = oteryn_world_bundle::bundle::placement_key(trigger.floor, trigger.x, trigger.y, 1)
        .ok_or("chest key")?;
    let mut placement = owner
        .bundle_map()
        .ok_or("map")?
        .overlay
        .base()
        .digest()
        .to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let payload = oteryn_protocol_oteryn::world_object::encode_use_intent(&WorldObjectTarget {
        placement,
        expected_revision: 0,
    })
    .map_err(debug)?;
    let reward = callback.effects.item_reward.as_ref().ok_or("reward")?;
    let held = async || -> TestResult<i64> {
        Ok(sqlx::query_scalar(
            "SELECT COALESCE(sum(quantity),0)::bigint FROM game_item_instances \
             WHERE lifecycle=1 AND definition_production_key=$1",
        )
        .bind(&reward.item)
        .fetch_one(&h.pool)
        .await?)
    };
    let completed = async || -> TestResult<bool> {
        Ok(sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM game_character_quest_states WHERE \
             character_id=encode($1,'hex')::uuid AND quest_key=$2 \
             AND completed_character_revision IS NOT NULL)",
        )
        .bind(id(41).as_slice())
        .bind(&callback.quest)
        .fetch_one(&h.pool)
        .await?)
    };
    assert_eq!(held().await?, 0);
    assert!(!completed().await?);
    let command = admitted.continuity.next_command_id;
    let first =
        super::quest_world_callbacks::wire(owner, admitted, command, 2, payload.clone()).await?;
    assert_eq!(
        oteryn_protocol_oteryn::world_object::decode_use_result(&first).map_err(debug)?,
        UseDisposition::Committed
    );
    assert_eq!(held().await?, i64::from(reward.count));
    assert_eq!(completed().await?, callback.completes);
    let audits = async || -> TestResult<i64> {
        Ok(
            sqlx::query_scalar("SELECT count(*) FROM game_item_audit_outbox")
                .fetch_one(&h.pool)
                .await?,
        )
    };
    let before = audits().await?;
    // The same command id replays the committed result without a second mint.
    let replay =
        super::quest_world_callbacks::wire(owner, admitted, command, 2, payload.clone()).await?;
    assert_eq!(
        oteryn_protocol_oteryn::world_object::decode_use_result(&replay).map_err(debug)?,
        UseDisposition::Committed
    );
    // The chest is personal: the shared object is untouched and a new use is refused.
    assert_eq!(
        owner.bundle_map().ok_or("map")?.facts.object_revision(key),
        0
    );
    let next = admitted.continuity.next_command_id;
    let second = super::quest_world_callbacks::wire(owner, admitted, next, 2, payload).await?;
    assert_eq!(
        oteryn_protocol_oteryn::world_object::decode_use_result(&second).map_err(debug)?,
        UseDisposition::Rejected
    );
    assert_eq!(audits().await?, before);
    assert_eq!(held().await?, i64::from(reward.count));
    Ok(())
}
