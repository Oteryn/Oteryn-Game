//! Whole selected Grave Danger journey through native world and death owners.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use oteryn_gameplay_quests::quest::world_callbacks::Position;
#[test]
fn questpc7_grave_whole_native_journey_and_reward() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("questpc7-grave", || {
        super::quest_world_callbacks::scenario(
            "questpc7-grave",
            Position {
                x: 33221,
                y: 31670,
                floor: -7,
            },
            false,
        )
    });
    eprintln!(
        "QUESTPC7_GRAVE POSTGRES_CASE_COMPLETED duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
use super::questpc7_ferumbras::{guardian, step, use_at, value, walk};
const QUEST: &str = "oteryn:quest.grave_danger_quest";
const TRACK: &str = "oteryn:quest-progress/questpc7/grave/programme";
const VISITS: &str = "oteryn:quest-progress/questpc7/grave/visits";
const PASSAGES: &str = "oteryn:quest-progress/questpc7/grave/passages";
fn callbacks() -> TestResult<Vec<oteryn_gameplay_quests::quest::world_callbacks::Callback>> {
    Ok(oteryn_gameplay_quests::quest::world_callbacks::load()
        .map_err(debug)?
        .into_iter()
        .filter(|c| c.key.starts_with("oteryn:quest-callback/questpc7-grave-"))
        .collect())
}
fn callback<'a>(
    rows: &'a [oteryn_gameplay_quests::quest::world_callbacks::Callback],
    name: &str,
) -> TestResult<&'a oteryn_gameplay_quests::quest::world_callbacks::Callback> {
    rows.iter()
        .find(|c| c.key == format!("oteryn:quest-callback/questpc7-grave-{name}"))
        .ok_or_else(|| format!("missing {name}").into())
}
pub(super) fn objects() -> TestResult<Vec<(Position, u16)>> {
    let mut cells = std::collections::BTreeMap::new();
    for c in callbacks()? {
        for p in c.selector.position.into_iter().chain(c.teleport) {
            for x in p.x - 2..=p.x + 2 {
                for y in p.y - 2..=p.y + 2 {
                    cells.entry((x, y, p.floor)).or_insert(100);
                }
            }
        }
    }
    for c in callbacks()? {
        if let Some(p) = c.selector.position {
            cells.insert(
                (p.x, p.y, p.floor),
                c.selector.appearance.ok_or("appearance")?,
            );
        }
    }
    for o in oteryn_gameplay_quests::quest::death_objectives::load()
        .map_err(debug)?
        .iter()
        .filter(|o| o.quest == QUEST)
    {
        let p = o.guardian.as_ref().ok_or("guardian")?.position;
        cells.entry((p.x, p.y, p.floor)).or_insert(100);
    }
    Ok(cells
        .into_iter()
        .map(|((x, y, floor), appearance)| (Position { x, y, floor }, appearance))
        .collect())
}
async fn assert_at(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    a: &AdmittedSession,
    p: Position,
) -> TestResult {
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(a.runtime_actor.ok_or("actor")?)
            .map_err(debug)?
            .position(),
        super::quest_world_callbacks::local(p),
        "native earned destination"
    );
    Ok(())
}
async fn no_rewards(h: &Harness) -> TestResult {
    let counts:(i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM game_account_appearance_unlocks WHERE unlock_key LIKE 'oteryn:outfit.hand_of_the_inquisition/quest-grant/%'),(SELECT count(*) FROM game_account_achievements WHERE achievement_key='oteryn:achievement/inquisition_s_hand')").fetch_one(&h.pool).await?;
    assert_eq!(counts, (0, 0));
    Ok(())
}
pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    a: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    use crate::gameplay_transport::connection::FreshAdmissionAuthority;
    use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
    use oteryn_protocol_oteryn::world_object::{UseDisposition, decode_use_result};
    use oteryn_protocol_oteryn::world_spatial::StepDirection;
    let version: String = sqlx::query_scalar("SHOW server_version")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("QUESTPC7_GRAVE POSTGRES_STARTED version={version}");
    let rows = callbacks()?;
    assert_eq!(rows.len(), 22);
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
    assert_eq!(value(owner, a, TRACK).await?, -1, "no quest progress seeds");
    step(owner, a, StepDirection::East).await?;
    assert_eq!(value(owner, a, TRACK).await?, 1);
    assert_eq!(value(owner, a, VISITS).await?, 0);
    for i in 0..12 {
        let c = callback(&rows, &format!("survey-{i}"))?;
        let target = c.selector.position.ok_or("grave")?;
        let appearance = c.selector.appearance.ok_or("grave appearance")?;
        if i == 0 {
            let premium = owner.premium_coordinator;
            owner.premium_coordinator = None;
            assert_eq!(
                use_at(owner, a, target, appearance).await?,
                UseDisposition::Rejected
            );
            owner.premium_coordinator = premium;
            assert_eq!(
                value(owner, a, VISITS).await?,
                0,
                "current Premium at each grave"
            );
        }
        assert_eq!(
            use_at(owner, a, target, appearance).await?,
            UseDisposition::Committed
        );
        assert_eq!(value(owner, a, VISITS).await?, i64::from(i + 1));
        assert_at(owner, a, c.teleport.ok_or("earned journey")?).await?;
    }
    no_rewards(h).await?;
    let objectives = oteryn_gameplay_quests::quest::death_objectives::load().map_err(debug)?;
    let names = [
        "count_vlarkorth",
        "duke_krule",
        "earl_osam",
        "lord_azaram",
        "sir_baeloc",
        "sir_nictros",
        "king_zelos",
        "scarlett_etzel",
    ];
    for (i, name) in names.iter().enumerate() {
        if i == 7 {
            let c = callback(&rows, "cobra")?;
            walk(owner, a, c.selector.position.ok_or("cobra entrance")?).await?;
            assert_eq!(
                value(
                    owner,
                    a,
                    "oteryn:quest-progress/quest/u12_20/grave_danger/cobra_bastion/access"
                )
                .await?,
                1,
                "retained source step grants actual Cobra access"
            );
            assert_eq!(
                use_at(
                    owner,
                    a,
                    c.selector.position.ok_or("Cobra")?,
                    c.selector.appearance.ok_or("Cobra appearance")?
                )
                .await?,
                UseDisposition::Committed
            );
            assert_eq!(value(owner, a, TRACK).await?, 3);
            assert_at(owner, a, c.teleport.ok_or("Cobra destination")?).await?;
        }
        owner.refresh_quest_session(a.game_session_id, false).await;
        let objective = objectives
            .iter()
            .find(|o| o.key == format!("oteryn:quest-death/questpc7-grave-{name}"))
            .ok_or("native objective")?;
        let at = objective.guardian.as_ref().ok_or("guardian")?.position;
        let target = guardian(owner, at).await?.ok_or("earned native boss")?;
        super::questpc7_native_deaths::lethal_creature(
            owner,
            a,
            target,
            20000 + u64::try_from(i)?,
            &format!("oteryn:creature.{name}"),
        )
        .await?;
        assert_eq!(
            value(
                owner,
                a,
                &format!("oteryn:quest-progress/questpc7/grave/boss_{i}")
            )
            .await?,
            1
        );
        let name = match i {
            0 => "knight-passage-0",
            1 => "knight-passage-1",
            2 => "knight-passage-2",
            3 => "knight-passage-3",
            4 | 5 => "knight-passage-4",
            6 => "zelos-passage",
            _ => "scarlett-passage",
        };
        let c = callback(&rows, name)?;
        let exit = c.selector.position.ok_or("passage")?;
        if i == 4 {
            walk(owner, a, exit).await?;
            assert_eq!(
                value(owner, a, PASSAGES).await?,
                4,
                "one Sir cannot substitute for the other"
            );
            step(owner, a, StepDirection::East).await?;
            continue;
        }
        walk(owner, a, exit).await?;
        let expected = if i < 4 {
            i64::try_from(i + 1)?
        } else if i == 5 {
            5
        } else if i == 6 {
            6
        } else {
            7
        };
        assert_eq!(value(owner, a, PASSAGES).await?, expected);
        assert_at(owner, a, c.teleport.ok_or("earned passage")?).await?;
    }
    assert_eq!(
        value(
            owner,
            a,
            "oteryn:quest-progress/crystalserver/quest/u12_20/grave_danger/bosses/king_zelos_door"
        )
        .await?,
        1
    );
    no_rewards(h).await?;
    let reward = callback(&rows, "reward")?;
    let p = reward.selector.position.ok_or("reward")?;
    let appearance = reward.selector.appearance.ok_or("armour")?;
    let premium = owner.premium_coordinator;
    owner.premium_coordinator = None;
    assert_eq!(
        use_at(owner, a, p, appearance).await?,
        UseDisposition::Rejected
    );
    owner.premium_coordinator = premium;
    no_rewards(h).await?;
    let command = a.continuity.next_command_id;
    let payload = super::questpc7_hoist_story::target(owner, p, appearance)?;
    assert_eq!(
        use_at(owner, a, p, appearance).await?,
        UseDisposition::Committed
    );
    let replay = super::quest_world_callbacks::wire(owner, a, command, 2, payload).await?;
    assert_eq!(
        decode_use_result(&replay).map_err(debug)?,
        UseDisposition::Committed
    );
    assert_eq!(value(owner, a, TRACK).await?, 4);
    let counts:(i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM game_account_appearance_unlocks WHERE unlock_key='oteryn:outfit.hand_of_the_inquisition/quest-grant/0'),(SELECT count(*) FROM game_account_achievements WHERE achievement_key='oteryn:achievement/inquisition_s_hand')").fetch_one(&h.pool).await?;
    assert_eq!(counts, (1, 1));
    let completed:bool=sqlx::query_scalar("SELECT completed_character_revision IS NOT NULL FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2").bind(super::fixture::id(41).as_slice()).bind(QUEST).fetch_one(&h.pool).await?;
    assert!(completed);
    let before:String=sqlx::query_scalar("SELECT completed_character_revision::text FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2").bind(super::fixture::id(41).as_slice()).bind(QUEST).fetch_one(&h.pool).await?;
    assert_eq!(
        use_at(owner, a, p, appearance).await?,
        UseDisposition::NothingToUse,
        "completed phase removes the selected overlay and preserves native armour behaviour"
    );
    let after:String=sqlx::query_scalar("SELECT completed_character_revision::text FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2").bind(super::fixture::id(41).as_slice()).bind(QUEST).fetch_one(&h.pool).await?;
    assert_eq!(after, before, "inactive repeat cannot advance completion");
    let repeated:(i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM game_account_appearance_unlocks WHERE unlock_key LIKE 'oteryn:outfit.hand_of_the_inquisition/quest-grant/%'),(SELECT count(*) FROM game_account_achievements WHERE achievement_key='oteryn:achievement/inquisition_s_hand')").fetch_one(&h.pool).await?;
    assert_eq!(
        repeated,
        (1, 1),
        "inactive repeat creates no additional grants or addons"
    );
    let observed = owner
        .appearance_observation(actor, a.game_session_id, vec![])
        .await
        .ok_or("appearance")?;
    assert!(
        observed
            .catalogue
            .outfits
            .iter()
            .any(|o| o.key == "oteryn:outfit.hand_of_the_inquisition")
    );
    let change = oteryn_protocol_oteryn::appearance::Change {
        context: observed.catalogue.context,
        expected_root_revision: observed.catalogue.root_revision,
        prior: observed.catalogue.selected,
        selection: oteryn_protocol_oteryn::appearance::Selection {
            outfit_key: "oteryn:outfit.hand_of_the_inquisition".into(),
            colours: [0; 4],
            addons: 0,
            mount_key: None,
        },
        content_revision: owner
            .appearance_context()
            .ok_or("cosmetics")?
            .1
            .content_revision()
            .into(),
    };
    let command = a.continuity.next_command_id;
    let reply =
        super::quest_world_callbacks::wire(owner, a, command, 25, change.encode().map_err(debug)?)
            .await?;
    assert!(matches!(
        oteryn_protocol_oteryn::appearance::ChangeResult::decode(&reply).map_err(debug)?,
        oteryn_protocol_oteryn::appearance::ChangeResult::Committed(_)
    ));
    Ok(())
}
