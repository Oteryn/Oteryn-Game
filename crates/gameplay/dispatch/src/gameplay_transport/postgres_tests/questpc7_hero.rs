//! Selected Rathleton investigation through native commands and death settlement.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use oteryn_gameplay_quests::quest::world_callbacks::Position;
#[test]
fn questpc7_hero_full_native_investigation() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("questpc7-hero", || {
        super::quest_world_callbacks::scenario(
            "questpc7-hero",
            Position {
                x: 33564,
                y: 31951,
                floor: -14,
            },
            false,
        )
    });
    eprintln!(
        "QUESTPC7_HERO POSTGRES_CASE_COMPLETED duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
const TRACK: &str = "oteryn:quest-progress/questpc7/hero/programme";
const KILLS: &str = "oteryn:quest-progress/questpc7/hero/constructs";
const BOSS: &str = "oteryn:quest-progress/questpc7/hero/maxxen";
async fn value(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    a: &AdmittedSession,
    key: &str,
) -> TestResult<i64> {
    let f = owner
        .current_quest_fence(a.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let s = owner
        .root
        .read_character_quest_state(owner.character, f.character_id)
        .await
        .map_err(debug)?;
    Ok(s.tracks().get(key).copied().unwrap_or(-1))
}
async fn walk(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    a: &mut AdmittedSession,
    x: i32,
    y: i32,
) -> TestResult {
    use oteryn_protocol_oteryn::world_spatial::{
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, StepDisposition, decode_step_result,
        encode_step_intent,
    };
    for _ in 0..32 {
        let p = owner
            .runtime
            .lock()
            .await
            .read_actor_position(a.runtime_actor.ok_or("actor")?)
            .map_err(debug)?
            .position();
        if (p.x, p.y) == (x, y) {
            return Ok(());
        }
        let direction = if p.x < x {
            StepDirection::East
        } else if p.x > x {
            StepDirection::West
        } else if p.y < y {
            StepDirection::South
        } else {
            StepDirection::North
        };
        tokio::time::sleep(std::time::Duration::from_millis(210)).await;
        let n = a.continuity.next_command_id;
        let reply = super::quest_world_callbacks::wire(
            owner,
            a,
            n,
            COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
            encode_step_intent(direction),
        )
        .await?;
        assert_eq!(
            decode_step_result(&reply).map_err(debug)?,
            StepDisposition::Moved
        );
    }
    Err("walk bound".into())
}
async fn use_at(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    a: &mut AdmittedSession,
    x: u16,
    y: u16,
    appearance: u16,
) -> TestResult<oteryn_protocol_oteryn::world_object::UseDisposition> {
    tokio::time::sleep(std::time::Duration::from_millis(210)).await;
    let target =
        super::questpc7_hoist_story::target(owner, Position { x, y, floor: -14 }, appearance)?;
    let n = a.continuity.next_command_id;
    let reply = super::quest_world_callbacks::wire(owner, a, n, 2, target).await?;
    oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)
}
async fn guardian(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    x: i32,
    y: i32,
) -> TestResult<Option<crate::foundation::ExactActorRef>> {
    let r = owner.runtime.lock().await;
    let found: Vec<_> = r
        .positioned_melee_census()
        .map_err(debug)?
        .into_iter()
        .filter(|(_, p, s)| {
            s.is_none()
                && p.position() == crate::foundation::MovementLocalPosition { x, y, floor: 14 }
        })
        .map(|(a, _, _)| a)
        .collect();
    assert!(found.len() <= 1);
    Ok(found.first().copied())
}
pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    a: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    use crate::gameplay_transport::connection::FreshAdmissionAuthority;
    use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
    use oteryn_protocol_oteryn::world_object::UseDisposition;
    let version: String = sqlx::query_scalar("SHOW server_version")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("QUESTPC7_HERO POSTGRES_STARTED version={version}");
    let callbacks = oteryn_gameplay_quests::quest::world_callbacks::load().map_err(debug)?;
    assert!(
        callbacks
            .iter()
            .any(|c| c.key == "oteryn:quest-callback/questpc7-hero-entry"),
        "selected Hero native entry is absent"
    );
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    native
        .install_companion_policies(&mut *owner.runtime.lock().await)
        .map_err(debug)?;
    let actor = a.runtime_actor.ok_or("actor")?;
    let f = owner
        .current_quest_fence(a.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    owner
        .runtime
        .lock()
        .await
        .bind_attacker_lease(
            actor,
            a.game_session_id,
            crate::foundation::CharacterLease::new(
                crate::foundation::CharacterId::decode(f.character_id.as_bytes()).map_err(debug)?,
                f.character_lease_generation,
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;
    let mut selected = a.continuity.selected_capabilities.as_slice().to_vec();
    selected.extend([3, 4, 6, 11, 15, 18, 25]);
    selected.sort_unstable();
    selected.dedup();
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    a.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered, &selected)
            .ok_or("capabilities")?;
    assert_eq!(value(owner, a, TRACK).await?, -1);
    assert_eq!(
        value(owner, a, "oteryn:quest-progress/hero_rathleton/access_door").await?,
        -1
    );
    walk(owner, a, 33564, 31951).await?;
    assert_eq!(value(owner, a, TRACK).await?, 1);
    walk(owner, a, 33568, 31951).await?;
    assert_eq!(
        value(owner, a, TRACK).await?,
        1,
        "investigation evidence is required before dungeon access"
    );
    walk(owner, a, 33566, 31951).await?;
    let premium = owner.premium_coordinator;
    owner.premium_coordinator = None;
    assert_eq!(
        use_at(owner, a, 33567, 31950, 20476).await?,
        UseDisposition::Rejected
    );
    owner.premium_coordinator = premium;
    assert_eq!(value(owner, a, TRACK).await?, 1);
    assert_eq!(
        use_at(owner, a, 33567, 31950, 20476).await?,
        UseDisposition::Committed
    );
    assert_eq!(value(owner, a, TRACK).await?, 2);
    walk(owner, a, 33568, 31951).await?;
    assert_eq!(value(owner, a, TRACK).await?, 3);
    owner.refresh_quest_session(a.game_session_id, false).await;
    assert_eq!(
        guardian(owner, 33570, 31951).await?,
        None,
        "boss cannot start before the ten constructs"
    );
    walk(owner, a, 33570, 31952).await?;
    assert_eq!(
        use_at(owner, a, 33571, 31952, 1855).await?,
        UseDisposition::Rejected,
        "no premature loot"
    );
    for count in 1..=10 {
        let target = guardian(owner, 33566, 31950)
            .await?
            .ok_or("native construct")?;
        super::questpc7_native_deaths::lethal_creature(
            owner,
            a,
            target,
            12000 + count,
            "oteryn:creature.glooth_golem",
        )
        .await?;
        assert_eq!(value(owner, a, KILLS).await?, i64::try_from(count)?);
        owner.refresh_quest_session(a.game_session_id, false).await;
        assert_eq!(guardian(owner, 33566, 31950).await?, None);
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(61))
            .ok_or("clock")?;
        owner.refresh_quest_session(a.game_session_id, false).await;
    }
    assert_eq!(
        guardian(owner, 33566, 31950).await?,
        None,
        "quota is bounded"
    );
    assert_eq!(
        use_at(owner, a, 33571, 31952, 1855).await?,
        UseDisposition::Rejected,
        "constructs alone are insufficient"
    );
    let boss = guardian(owner, 33570, 31951)
        .await?
        .ok_or("native Professor Maxxen")?;
    super::questpc7_native_deaths::lethal_creature(
        owner,
        a,
        boss,
        13000,
        "oteryn:creature.professor_maxxen",
    )
    .await?;
    assert_eq!(value(owner, a, BOSS).await?, 1);
    let premium = owner.premium_coordinator;
    owner.premium_coordinator = None;
    assert_eq!(
        use_at(owner, a, 33571, 31952, 1855).await?,
        UseDisposition::Rejected,
        "fresh Premium is required at reward delivery"
    );
    owner.premium_coordinator = premium;
    for item in [21897, 21203, 3035] {
        assert_eq!(
            super::questpc7_outfit_items::quantity(h, item).await?,
            0,
            "refusal must not mint any reward"
        );
    }
    assert_eq!(
        use_at(owner, a, 33571, 31952, 1855).await?,
        UseDisposition::Committed
    );
    assert_eq!(value(owner, a, TRACK).await?, 4);
    assert_eq!(
        value(owner, a, "oteryn:quest-progress/hero_rathleton/access_door").await?,
        1,
        "native dungeon-door access is earned with completion"
    );
    for (item, amount) in [(21897, 1), (21203, 4), (3035, 5)] {
        assert_eq!(
            super::questpc7_outfit_items::quantity(h, item).await?,
            amount
        );
    }
    let completed:bool=sqlx::query_scalar("SELECT completed_character_revision IS NOT NULL FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key='oteryn:quest.hero_of_rathleton_quest'").bind(super::fixture::id(41).as_slice()).fetch_one(&h.pool).await?;
    assert!(completed);
    assert_eq!(
        use_at(owner, a, 33571, 31952, 1855).await?,
        UseDisposition::Rejected
    );
    for (item, amount) in [(21897, 1), (21203, 4), (3035, 5)] {
        assert_eq!(
            super::questpc7_outfit_items::quantity(h, item).await?,
            amount,
            "repeat cannot duplicate rewards"
        );
    }
    Ok(())
}
