//! Whole selected personal Grimvale programme; subject state is earned natively.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn questpc7_grimvale_whole_native_investigation_and_rewards() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("questpc7-grimvale-story", || {
        super::quest_world_callbacks::scenario(
            "qengine-rewards-grimvale-story",
            Position {
                x: 33328,
                y: 31690,
                floor: -7,
            },
            false,
        )
    });
    eprintln!(
        "QUESTPC7_GRIMVALE_STORY POSTGRES_CASE_COMPLETED duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

use super::questpc7_ferumbras::{guardian, step, use_at, value, walk};
use oteryn_gameplay_quests::quest::world_callbacks::Callback;
use oteryn_protocol_oteryn::world_object::{UseDisposition, decode_use_result};
use oteryn_protocol_oteryn::world_spatial::StepDirection;
const QUEST: &str = "oteryn:quest.grimvale_quest";
const TRACK: &str = "oteryn:quest-progress/questpc7/grimvale/programme";
fn track(name: &str) -> String {
    format!("oteryn:quest-progress/questpc7/grimvale/{name}")
}
fn callbacks() -> TestResult<Vec<Callback>> {
    Ok(oteryn_gameplay_quests::quest::world_callbacks::load()
        .map_err(debug)?
        .into_iter()
        .filter(|c| {
            c.quest == QUEST
                && c.key
                    .starts_with("oteryn:quest-callback/questpc7-grimvale-")
        })
        .collect())
}
fn callback<'a>(rows: &'a [Callback], name: &str) -> TestResult<&'a Callback> {
    rows.iter()
        .find(|c| c.key == format!("oteryn:quest-callback/questpc7-grimvale-{name}"))
        .ok_or_else(|| format!("missing native Grimvale interaction {name}").into())
}
pub(super) fn objects() -> TestResult<Vec<(Position, u16)>> {
    let rows = callbacks()?;
    let mut cells = std::collections::BTreeMap::new();
    for c in &rows {
        for p in c.selector.position.into_iter().chain(c.teleport) {
            for x in p.x - 2..=p.x + 2 {
                for y in p.y - 2..=p.y + 2 {
                    cells.entry((x, y, p.floor)).or_insert(100);
                }
            }
        }
    }
    for c in &rows {
        let p = c.selector.position.ok_or("position")?;
        cells.insert(
            (p.x, p.y, p.floor),
            c.selector.appearance.ok_or("appearance")?,
        );
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
async fn carried(h: &Harness, item: u32) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce(sum(i.quantity),0)::bigint FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) WHERE e.character_id=encode($1,'hex')::uuid AND i.lifecycle=1 AND i.definition_production_key=$2").bind(super::fixture::id(41).as_slice()).bind(format!("oteryn:item.tibia.i{item}")).fetch_one(&h.pool).await?)
}
async fn facts(h: &Harness) -> TestResult<(i64, i64, Option<String>)> {
    let achievements:i64=sqlx::query_scalar("SELECT count(*) FROM game_account_achievements WHERE achievement_key IN ('oteryn:achievement/over_the_moon','oteryn:achievement/drama_in_darama')").fetch_one(&h.pool).await?;
    let completed:Option<String>=sqlx::query_scalar::<_,Option<String>>("SELECT completed_character_revision::text FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2").bind(super::fixture::id(41).as_slice()).bind(QUEST).fetch_optional(&h.pool).await?.flatten();
    Ok((carried(h, 27565).await?, achievements, completed))
}
async fn at(
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
        "exact native destination"
    );
    Ok(())
}
async fn click(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    a: &mut AdmittedSession,
    c: &Callback,
) -> TestResult<UseDisposition> {
    use_at(
        owner,
        a,
        c.selector.position.ok_or("position")?,
        c.selector.appearance.ok_or("appearance")?,
    )
    .await
}
async fn passage(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    a: &mut AdmittedSession,
    c: &Callback,
) -> TestResult {
    walk(owner, a, c.selector.position.ok_or("position")?).await?;
    at(owner, a, c.teleport.ok_or("teleport")?).await
}
async fn kill(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    a: &AdmittedSession,
    name: &str,
    nonce: u64,
) -> TestResult {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(61))
        .ok_or("clock")?;
    owner.refresh_quest_session(a.game_session_id, false).await;
    let objective = oteryn_gameplay_quests::quest::death_objectives::load()
        .map_err(debug)?
        .iter()
        .find(|o| o.key == format!("oteryn:quest-death/questpc7-grimvale-{name}"))
        .ok_or("objective")?;
    let p = objective.guardian.as_ref().ok_or("guardian")?.position;
    let observed_guardian = guardian(owner, p).await?;
    if observed_guardian.is_none() {
        let runtime = owner.runtime.lock().await;
        eprintln!(
            "GRIMVALE_GUARDIAN_DIAGNOSTIC name={name} policy={:?} player={:?} center={p:?}",
            runtime
                .companion_policy(&objective.creature)
                .map(|policy| policy.maximum_health),
            runtime.read_actor_position(a.runtime_actor.ok_or("actor")?)
        );
    }
    let target = observed_guardian.ok_or_else(|| format!("missing active native {name}"))?;
    super::questpc7_native_deaths::lethal_creature(
        owner,
        a,
        target,
        nonce,
        &format!("oteryn:creature.{name}"),
    )
    .await?;
    // Retire the settled death before starting the next respawn interval.
    owner.refresh_quest_session(a.game_session_id, false).await;
    Ok(())
}
pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    a: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let version: String = sqlx::query_scalar("SHOW server_version")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("QUESTPC7_GRIMVALE_STORY POSTGRES_STARTED version={version}");
    super::questpc7_native_deaths::prepare_native_hunter(owner, a).await?;
    let rows = callbacks()?;
    let entry = callback(&rows, "entry")?;
    let mut selected = a.continuity.selected_capabilities.as_slice().to_vec();
    selected.extend([3, 6]);
    selected.sort_unstable();
    selected.dedup();
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    a.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered, &selected)
            .ok_or("NPC capabilities")?;
    assert_eq!(value(owner, a, TRACK).await?, -1);
    assert_eq!(facts(h).await?, (0, 0, None));
    let premium = owner.premium_coordinator;
    owner.premium_coordinator = None;
    walk(owner, a, entry.selector.position.ok_or("entry")?).await?;
    assert_eq!(
        value(owner, a, TRACK).await?,
        -1,
        "fresh Current Premium authority required at entry"
    );
    owner.premium_coordinator = premium;
    step(owner, a, StepDirection::West).await?;
    walk(owner, a, entry.selector.position.ok_or("entry")?).await?;
    assert_eq!(value(owner, a, TRACK).await?, 0);
    for i in 0..5 {
        let c = callback(&rows, &format!("nightshade-{i}"))?;
        if i == 0 {
            let premium = owner.premium_coordinator;
            owner.premium_coordinator = None;
            assert_eq!(click(owner, a, c).await?, UseDisposition::Rejected);
            assert_eq!(carried(h, 22082).await?, 0);
            owner.premium_coordinator = premium;
        }
        assert_eq!(click(owner, a, c).await?, UseDisposition::Committed);
        assert_eq!(carried(h, 22082).await?, i + 1);
        assert_eq!(value(owner, a, &track("flowers")).await?, i + 1);
        at(owner, a, c.teleport.ok_or("plot destination")?).await?;
    }
    assert_eq!(value(owner, a, TRACK).await?, 1);
    let prepare = callback(&rows, "prepare-trap")?;
    for i in 0..10 {
        if i == 9 {
            assert_eq!(value(owner, a, &track("afflicted")).await?, 9);
            assert_eq!(click(owner, a, prepare).await?, UseDisposition::Rejected);
            assert_eq!(carried(h, 22074).await?, 0);
            assert_eq!(value(owner, a, TRACK).await?, 1);
        }
        kill(
            owner,
            a,
            ["werewolf", "werebear", "wereboar"][i % 3],
            26000 + u64::try_from(i)?,
        )
        .await?;
        assert_eq!(
            value(owner, a, &track("afflicted")).await?,
            i64::try_from(i + 1)?
        );
    }
    assert_eq!(click(owner, a, prepare).await?, UseDisposition::Committed);
    assert_eq!(carried(h, 22074).await?, 1);
    assert_eq!(carried(h, 22082).await?, 5);
    assert_eq!(value(owner, a, TRACK).await?, 2);
    at(owner, a, prepare.teleport.ok_or("Feroxa approach")?).await?;
    let deploy = callback(&rows, "deploy-trap")?;
    let premium = owner.premium_coordinator;
    owner.premium_coordinator = None;
    assert_eq!(click(owner, a, deploy).await?, UseDisposition::Rejected);
    assert_eq!(carried(h, 22074).await?, 1);
    assert_eq!(value(owner, a, TRACK).await?, 2);
    owner.premium_coordinator = premium;
    let command = a.continuity.next_command_id;
    let payload = super::questpc7_hoist_story::target(
        owner,
        deploy.selector.position.ok_or("trap")?,
        deploy.selector.appearance.ok_or("trap")?,
    )?;
    assert_eq!(click(owner, a, deploy).await?, UseDisposition::Committed);
    assert_eq!(carried(h, 22074).await?, 0);
    let replay = super::quest_world_callbacks::wire(owner, a, command, 2, payload).await?;
    assert_eq!(
        decode_use_result(&replay).map_err(debug)?,
        UseDisposition::Committed
    );
    assert_eq!(carried(h, 22074).await?, 0);
    assert_eq!(value(owner, a, TRACK).await?, 3);
    kill(owner, a, "feroxa", 26100).await?;
    assert_eq!(value(owner, a, &track("feroxa")).await?, 1);
    passage(owner, a, callback(&rows, "feroxa-passage")?).await?;
    assert_eq!(value(owner, a, TRACK).await?, 4);
    for (i, name) in ["edron", "cormaya"].iter().enumerate() {
        passage(owner, a, callback(&rows, &format!("survey-{name}"))?).await?;
        assert_eq!(
            value(owner, a, &track("sites")).await?,
            i64::try_from(i + 1)?
        );
    }
    assert_eq!(value(owner, a, TRACK).await?, 5);
    for (i, name) in ["bloodback", "darkfang", "sharpclaw", "black_vixen"]
        .iter()
        .enumerate()
    {
        kill(owner, a, name, 26200 + u64::try_from(i)?).await?;
        assert_eq!(value(owner, a, &track(name)).await?, 1);
        passage(owner, a, callback(&rows, &format!("leader-passage-{i}"))?).await?;
        assert_eq!(
            value(owner, a, &track("passages")).await?,
            i64::try_from(i + 1)?
        );
    }
    assert_eq!(value(owner, a, TRACK).await?, 6);
    let [rhargu, domizian] =
        super::classic_quests::install(owner, a, ["oteryn:npc.rhargu", "oteryn:npc.domizian"])
            .await?;
    let mut sequence = a.continuity.next_command_id;
    super::questpc7_outfit_items::talk(owner, a, rhargu, &mut sequence, "feud").await?;
    assert_eq!(value(owner, a, TRACK).await?, 7);
    passage(owner, a, callback(&rows, "rhargu-passage")?).await?;
    let site = callback(&rows, "hyaena-evil-site")?;
    assert_eq!(click(owner, a, site).await?, UseDisposition::Committed);
    at(owner, a, site.teleport.ok_or("lairs")?).await?;
    assert_eq!(value(owner, a, TRACK).await?, 8);
    for i in 0..20 {
        if i == 19 {
            let exit = callback(&rows, "hyaena-passage")?;
            walk(owner, a, exit.selector.position.ok_or("exit")?).await?;
            assert_eq!(
                value(owner, a, TRACK).await?,
                8,
                "nineteen kills cannot request pardon"
            );
            step(owner, a, StepDirection::East).await?;
        }
        kill(owner, a, "werehyaena", 26300 + u64::try_from(i)?).await?;
        assert_eq!(value(owner, a, &track("werehyaena")).await?, i + 1);
    }
    passage(owner, a, callback(&rows, "hyaena-passage")?).await?;
    assert_eq!(value(owner, a, TRACK).await?, 9);

    let mut sequence = a.continuity.next_command_id;
    super::questpc7_outfit_items::talk(owner, a, domizian, &mut sequence, "pardon").await?;
    assert_eq!(value(owner, a, TRACK).await?, 10);
    passage(owner, a, callback(&rows, "domizian-passage")?).await?;
    for (i, name) in ["oskayaat_mosaic", "oskayaat_moon"].iter().enumerate() {
        passage(owner, a, callback(&rows, name)?).await?;
        assert_eq!(
            value(owner, a, &track("oskayaat")).await?,
            i64::try_from(i + 1)?
        );
    }
    assert_eq!(value(owner, a, TRACK).await?, 11);
    assert_eq!(facts(h).await?, (0, 0, None));
    let reward = callback(&rows, "ritual")?;
    let premium = owner.premium_coordinator;
    owner.premium_coordinator = None;
    assert_eq!(click(owner, a, reward).await?, UseDisposition::Rejected);
    assert_eq!(facts(h).await?, (0, 0, None));
    assert_eq!(value(owner, a, TRACK).await?, 11);
    owner.premium_coordinator = premium;
    let command = a.continuity.next_command_id;
    let payload = super::questpc7_hoist_story::target(
        owner,
        reward.selector.position.ok_or("ritual")?,
        reward.selector.appearance.ok_or("ritual")?,
    )?;
    assert_eq!(click(owner, a, reward).await?, UseDisposition::Committed);
    assert_eq!(value(owner, a, TRACK).await?, 12);
    assert_eq!(value(owner, a, &track("investigation_access")).await?, 1);
    let earned = facts(h).await?;
    assert_eq!((earned.0, earned.1), (1, 2));
    assert!(earned.2.is_some(), "durable completion with all rewards");
    let replay = super::quest_world_callbacks::wire(owner, a, command, 2, payload).await?;
    assert_eq!(
        decode_use_result(&replay).map_err(debug)?,
        UseDisposition::Committed
    );
    assert_eq!(facts(h).await?, earned);
    let exit = callback(&rows, "earned-access-oskayaat_ritual")?;
    assert_eq!(click(owner, a, exit).await?, UseDisposition::Committed);
    at(owner, a, exit.teleport.ok_or("earned exit")?).await?;
    assert_eq!(facts(h).await?, earned);
    let gate = callback(&rows, "earned-access-oskayaat_mosaic")?;
    walk(owner, a, gate.selector.position.ok_or("gate")?).await?;
    let premium = owner.premium_coordinator;
    owner.premium_coordinator = None;
    assert_eq!(click(owner, a, gate).await?, UseDisposition::Rejected);
    assert_eq!(facts(h).await?, earned);
    owner.premium_coordinator = premium;
    assert_eq!(click(owner, a, gate).await?, UseDisposition::Committed);
    at(owner, a, gate.teleport.ok_or("earned reentry")?).await?;
    assert_eq!(facts(h).await?, earned);
    eprintln!(
        "QUESTPC7_GRIMVALE_STORY all13 selected stages,35 native kills,5 blossoms,exact trap,amulet,two achievements,completion,replay and earned island travel PASS"
    );
    Ok(())
}
