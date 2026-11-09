//! Every retained machine component uses the production selector and network USE.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use oteryn_protocol_oteryn::world_object::{COMMAND_TYPE_USE_INTENT, UseDisposition};

pub(super) const NAME: &str = "qengine-rewards-m03-use-aliases";
const QUEST: &str = "oteryn:quest.25_years_of_tibia_quest";

#[test]
fn m03_every_machine_counterpart_native_use_guards_replay_and_free_return_postgres() -> TestResult {
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run(NAME, || {
        super::quest_world_callbacks::scenario(NAME, super::qengine_m03_use::ENTRY, false)
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED {NAME} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    let version: String = sqlx::query_scalar("SELECT version()")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("M03_ALL_MACHINE_COUNTERPARTS_POSTGRES {version}");
    let catalogue = owner
        .quest_catalogue
        .as_ref()
        .ok_or("production catalogue")?;
    let callbacks = oteryn_gameplay_quests::quest::world_callbacks::load_for(catalogue)?;
    let mut entries: Vec<_> = callbacks
        .iter()
        .filter(|c| c.quest == QUEST && c.admission.is_some())
        .cloned()
        .collect();
    let returns: Vec<_> = callbacks
        .into_iter()
        .filter(|c| c.quest == QUEST && c.admission.is_none())
        .collect();
    entries.sort_by_key(|c| c.selector.appearance);
    assert_eq!(entries.len(), 9);
    assert_eq!(returns.len(), 9);
    assert!(
        returns
            .iter()
            .any(|c| c.selector.native.as_ref().is_some_and(|n| n.ordinal == 2))
    );
    // Fixture geometry must retain each original stack, not manufacture its selector.
    for callback in entries.iter().chain(&returns) {
        let native = callback
            .selector
            .native
            .as_ref()
            .ok_or("native counterpart")?;
        let map = owner.bundle_map().ok_or("machine map")?;
        let p = native.position;
        let tile = map
            .overlay
            .base()
            .tile(p.x, p.y, p.floor)
            .ok_or_else(|| format!("missing retained machine component {}", callback.key))?;
        let id = tile
            .ids()
            .iter()
            .zip(tile.depths())
            .filter(|(_, d)| **d == 0)
            .nth(usize::from(native.ordinal))
            .map(|(id, _)| *id)
            .ok_or("native ordinal")?;
        let key = crate::map::view::placement_key(
            crate::map::overlay::TilePos {
                x: p.x,
                y: p.y,
                floor: p.floor,
            },
            native.ordinal,
        )
        .ok_or("placement key")?;
        assert!(
            map.facts
                .quest_callback_native_object(key, id, native.appearance)
        );
    }
    owner.admit_premium(super::fixture::id(40), admitted.game_session_id);
    assert!(
        owner
            .session_premium_gate(admitted.game_session_id)
            .await
            .premium_current()
    );
    super::qengine_m03_use::facts(h, true, 5).await?;
    // All nine bindings enforce the live level gate before any progress exists.
    for callback in &entries {
        place_fixture_actor(
            owner,
            admitted,
            callback.selector.native.as_ref().ok_or("source")?.position,
        )
        .await?;
        let result = click(owner, admitted, callback).await?;
        assert_eq!(
            result.0,
            UseDisposition::Rejected,
            "low level: {}",
            callback.key
        );
        assert_eq!(super::qengine_m03_use::footprint(h).await?, (0, 0));
    }
    super::qengine_outfit_stories::earn_level(owner, admitted, authority, 20).await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let mut receipts = 0;
    for entry in &entries {
        let source = entry.selector.native.as_ref().ok_or("source")?;
        let returning = returns
            .iter()
            .find(|c| c.selector.appearance == entry.selector.appearance)
            .ok_or("paired return")?;
        eprintln!(
            "M03_COUNTERPART_USE appearance={} entry_ordinal={} return_ordinal={}",
            source.appearance,
            source.ordinal,
            returning.selector.native.as_ref().ok_or("return")?.ordinal
        );
        place_fixture_actor(owner, admitted, source.position).await?;
        let before = super::qengine_m03_use::footprint(h).await?;
        super::qengine_m03_use::facts(h, false, 5).await?;
        assert_eq!(
            click(owner, admitted, entry).await?.0,
            UseDisposition::Rejected,
            "promotion: {}",
            entry.key
        );
        super::qengine_m03_use::facts(h, true, 4).await?;
        assert_eq!(
            click(owner, admitted, entry).await?.0,
            UseDisposition::Rejected,
            "blessings: {}",
            entry.key
        );
        super::qengine_m03_use::facts(h, true, 5).await?;
        let premium = owner.premium_coordinator.take();
        let denied = click(owner, admitted, entry).await;
        owner.premium_coordinator = premium;
        assert_eq!(
            denied?.0,
            UseDisposition::Rejected,
            "Premium: {}",
            entry.key
        );
        assert_eq!(
            super::qengine_m03_use::footprint(h).await?,
            before,
            "refusals write nothing"
        );
        let entered = click(owner, admitted, entry).await?;
        assert_eq!(entered.0, UseDisposition::Committed, "entry: {}", entry.key);
        receipts += 1;
        assert_eq!(super::qengine_m03_use::footprint(h).await?, (1, receipts));
        assert_eq!(
            super::qengine_m03_use::position(owner, admitted).await?,
            (33453, 31030, 8)
        );
        replay(owner, admitted, &entered).await?;
        assert_eq!(super::qengine_m03_use::footprint(h).await?, (1, receipts));
        // Position setup only; every return still crosses the unmodified USE dispatcher.
        place_fixture_actor(
            owner,
            admitted,
            returning.selector.native.as_ref().ok_or("return")?.position,
        )
        .await?;
        super::qengine_m03_use::facts(h, false, 0).await?;
        let premium = owner.premium_coordinator.take();
        let returned = click(owner, admitted, returning).await;
        owner.premium_coordinator = premium;
        let returned = returned?;
        assert_eq!(
            returned.0,
            UseDisposition::Committed,
            "free return: {}",
            returning.key
        );
        receipts += 1;
        assert_eq!(
            super::qengine_m03_use::position(owner, admitted).await?,
            (32430, 32167, 8)
        );
        replay(owner, admitted, &returned).await?;
        assert_eq!(super::qengine_m03_use::footprint(h).await?, (1, receipts));
    }
    assert_eq!(receipts, 18);
    let state:(i64,i64)=sqlx::query_as("SELECT (SELECT value FROM game_character_quest_tracks WHERE quest_key=$1 AND track_key='oteryn:quest-progress/chosen-source/25_years_of_tibia_quest/s1'),(SELECT count(*) FROM game_character_quest_receipts WHERE quest_key=$1 AND completes)").bind(QUEST).fetch_one(&h.pool).await?;
    assert_eq!(
        state,
        (1, 0),
        "all nine entries and exits preserve progress without completing the quest"
    );
    Ok(())
}

type Click = (UseDisposition, Vec<u8>, Vec<u8>, u64);
async fn click(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    callback: &Callback,
) -> TestResult<Click> {
    let native = callback
        .selector
        .native
        .as_ref()
        .ok_or("exact native selector")?;
    let command = admitted.continuity.next_command_id;
    let (outcome, reply, input) =
        super::qengine_m03_use::use_part(owner, admitted, command, native.position, native.ordinal)
            .await?;
    Ok((outcome, reply, input, command))
}
async fn replay(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    original: &Click,
) -> TestResult {
    let actor = admitted.runtime_actor.ok_or("replay actor")?;
    let before = owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?;
    let reply = super::quest_world_callbacks::wire(
        owner,
        admitted,
        original.3,
        COMMAND_TYPE_USE_INTENT,
        original.2.clone(),
    )
    .await?;
    assert_eq!(
        reply, original.1,
        "each exact request replays its original response"
    );
    let after = owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?;
    assert_eq!(
        after, before,
        "exact replay never advances the movement revision"
    );
    Ok(())
}
// Set up a source position using the sealed, scoped movement owner. This does
// not seed quest state or bypass USE admission, and retains the real B3 stack.
async fn place_fixture_actor(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    at: Position,
) -> TestResult {
    let map = owner.bundle_map().ok_or("map")?;
    let doors = crate::map::floor::BundleDoors(map.facts.doors());
    let mut runtime = owner.runtime.lock().await;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let expected = runtime.read_actor_position(actor).map_err(debug)?;
    let proof = crate::map::floor::prepare_scripted_quest_teleport_facing(
        &runtime,
        owner.movement_cells(),
        actor,
        admitted.game_session_id,
        expected,
        super::quest_world_callbacks::local(at),
        false,
        &doors,
        None,
        true,
    )
    .map_err(debug)?;
    runtime.commit_source_step(proof).map_err(debug)?;
    Ok(())
}
