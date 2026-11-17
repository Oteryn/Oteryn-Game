//! Whole fixed Forgotten Knowledge expedition and actual native powerful learning.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
#[test]
fn questpc7_forgotten_whole_native_paths_and_learning() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("questpc7-forgotten-story", || {
        super::quest_world_callbacks::scenario(
            "qengine-rewards-forgotten-story",
            Position {
                x: 32327,
                y: 32093,
                floor: -7,
            },
            false,
        )
    });
    eprintln!(
        "QUESTPC7_FORGOTTEN_STORY POSTGRES_CASE_COMPLETED duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
use super::questpc7_ferumbras::{guardian, step, use_at, value, walk};
use oteryn_gameplay_quests::quest::world_callbacks::Callback;
use oteryn_protocol_oteryn::world_object::UseDisposition;
use oteryn_protocol_oteryn::world_spatial::StepDirection;
const QUEST: &str = "oteryn:quest.forgotten_knowledge_quest";
const TRACK: &str = "oteryn:quest-progress/questpc7/forgotten/programme";
fn callbacks() -> TestResult<Vec<Callback>> {
    Ok(oteryn_gameplay_quests::quest::world_callbacks::load()
        .map_err(debug)?
        .into_iter()
        .filter(|c| {
            c.quest == QUEST
                && c.key
                    .starts_with("oteryn:quest-callback/questpc7-forgotten-")
        })
        .collect())
}
fn callback<'a>(rows: &'a [Callback], name: &str) -> TestResult<&'a Callback> {
    rows.iter()
        .find(|c| c.key == format!("oteryn:quest-callback/questpc7-forgotten-{name}"))
        .ok_or_else(|| format!("missing native Forgotten interaction {name}").into())
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
    for x in 32324..=32336 {
        for y in 32088..=32096 {
            cells.entry((x, y, -7)).or_insert(100);
        }
    }
    cells.entry((33419, 31125, -9)).or_insert(100);
    Ok(cells
        .into_iter()
        .map(|((x, y, floor), appearance)| (Position { x, y, floor }, appearance))
        .collect())
}
async fn carried(h: &Harness, item: u32) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce(sum(i.quantity),0)::bigint FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) WHERE e.character_id=encode($1,'hex')::uuid AND i.lifecycle=1 AND i.definition_production_key=$2").bind(super::fixture::id(41).as_slice()).bind(format!("oteryn:item.tibia.i{item}")).fetch_one(&h.pool).await?)
}
async fn completed(h: &Harness) -> TestResult<Option<String>> {
    Ok(sqlx::query_scalar::<_,Option<String>>("SELECT completed_character_revision::text FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2").bind(super::fixture::id(41).as_slice()).bind(QUEST).fetch_optional(&h.pool).await?.flatten())
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
// Independent ordinary service fixture; no production shrine or quest seed.
pub(super) fn native_items() -> [(Position, u32); 1] {
    [(
        Position {
            x: 32328,
            y: 32092,
            floor: -7,
        },
        25060,
    )]
}
async fn stock(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    a: &AdmittedSession,
    item: &str,
    count: u16,
    nonce: u64,
) -> TestResult {
    let f = owner
        .current_quest_fence(a.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    super::quest_world_callbacks::input_item(
        owner,
        f.item_fence(),
        &oteryn_gameplay_quests::quest::world_callbacks::Offering {
            position: Position {
                x: 32328,
                y: 32093,
                floor: -7,
            },
            item: item.into(),
            count,
        },
        nonce,
        true,
    )
    .await
}
async fn money(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce(sum(i.quantity*CASE i.definition_production_key WHEN 'oteryn:item.tibia.i3031' THEN 1 WHEN 'oteryn:item.tibia.i3035' THEN 100 WHEN 'oteryn:item.tibia.i3043' THEN 10000 ELSE 0 END),0)::bigint FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) WHERE e.character_id=encode($1,'hex')::uuid AND i.lifecycle=1").bind(super::fixture::id(41).as_slice()).fetch_one(&h.pool).await?)
}
async fn imbue(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    a: &mut AdmittedSession,
) -> TestResult<oteryn_protocol_oteryn::imbuement::ImbuementResult> {
    use oteryn_protocol_oteryn::imbuement::{ImbuementAction, ImbuementResult};
    let reference = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .and_then(|n| n.item_index())
        .and_then(|i| i.definition_ref("Item", "oteryn:item.tibia.i3264", "definition-r1"))
        .ok_or("native sword reference")?;
    let f = owner
        .current_quest_fence(a.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    a.item_fence = Some(f.item_fence());
    let (next, status, reply) = super::imbuement::exchange(
        owner,
        reference,
        *a,
        a.continuity.next_command_id,
        Some(ImbuementAction::Apply {
            slot: 0,
            imbuement: 2,
        }),
        true,
    )
    .await?;
    *a = next;
    assert_ne!(
        status,
        oteryn_protocol_oteryn::CommandStatus::DuplicateReplay
    );
    let result = reply.ok_or("native imbuement reply")?.result;
    assert!(
        matches!(
            result,
            ImbuementResult::RequirementNotMet | ImbuementResult::Ok
        ),
        "unexpected service result {result:?}"
    );
    Ok(result)
}
pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    a: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    use oteryn_protocol_oteryn::imbuement::{CAPABILITY_IMBUE_V1, ImbuementResult};
    let version: String = sqlx::query_scalar("SHOW server_version")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("QUESTPC7_FORGOTTEN_STORY POSTGRES_STARTED version={version}");
    super::questpc7_native_deaths::prepare_native_hunter(owner, a).await?;
    let rows = callbacks()?;
    assert_eq!(rows.len(), 11);
    walk(
        owner,
        a,
        callback(&rows, "energy-entry")?
            .selector
            .position
            .ok_or("temple approach")?,
    )
    .await?;
    let mut selected = a.continuity.selected_capabilities.as_slice().to_vec();
    selected.extend([3, 4, 6, 11, 12, 14, 15, 18, CAPABILITY_IMBUE_V1]);
    selected.sort_unstable();
    selected.dedup();
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    a.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered, &selected)
            .ok_or("native capabilities")?;
    assert_eq!(value(owner, a, TRACK).await?, -1);
    assert!(completed(h).await?.is_none());
    // Normal service inputs use fenced native MINT and custody; no Forgotten tool/reward/progress is seeded.
    stock(owner, a, "oteryn:item.tibia.i3264", 1, 28100).await?;
    stock(owner, a, "oteryn:item.tibia.i3043", 25, 28101).await?;
    let recipe =
        oteryn_gameplay_items::imbuement::imbuement_recipe(1, 3).ok_or("Powerful Strike")?;
    assert_eq!(recipe.apply_gold, 250000);
    for (i, (key, count)) in recipe.materials.iter().enumerate() {
        stock(
            owner,
            a,
            key,
            u16::try_from(*count)?,
            28102 + u64::try_from(i)?,
        )
        .await?;
    }
    let before_money = money(h).await?;
    assert_eq!(
        imbue(owner, a).await?,
        ImbuementResult::RequirementNotMet,
        "a source shrine, valid sword, all materials and fee cannot substitute for earned knowledge"
    );
    assert_eq!(money(h).await?, before_money);
    let initial: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_imbuements")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(initial, 0);
    for (key, count) in recipe.materials {
        let id: u32 = key.rsplit('i').next().ok_or("id")?.parse()?;
        assert_eq!(carried(h, id).await?, i64::from(*count));
    }
    let [ivalisse, silus] = super::classic_quests::install_positions(
        owner,
        a,
        ["oteryn:npc.ivalisse", "oteryn:npc.a_strange_chalice"],
        [
            Position {
                x: 32326,
                y: 32093,
                floor: -7,
            },
            Position {
                x: 33419,
                y: 31125,
                floor: -9,
            },
        ],
    )
    .await?;
    let premium = owner.premium_coordinator;
    owner.premium_coordinator = None;
    let mut seq = a.continuity.next_command_id;
    super::questpc7_outfit_items::talk(owner, a, ivalisse, &mut seq, "mission").await?;
    assert_eq!(value(owner, a, TRACK).await?, -1);
    owner.premium_coordinator = premium;
    let mut seq = a.continuity.next_command_id;
    super::questpc7_outfit_items::talk(owner, a, ivalisse, &mut seq, "mission").await?;
    assert_eq!(value(owner, a, TRACK).await?, 0);
    // No tome: even the correct words cannot open the Energy Portal.
    let mut seq = a.continuity.next_command_id;
    super::questpc7_outfit_items::talk(owner, a, ivalisse, &mut seq, "tome").await?;
    assert_eq!(value(owner, a, TRACK).await?, 0);
    let tome = callback(&rows, "tome")?;
    walk(owner, a, tome.selector.position.ok_or("tome")?).await?;
    assert_eq!(click(owner, a, tome).await?, UseDisposition::Committed);
    assert_eq!(value(owner, a, TRACK).await?, 1);
    assert_eq!(carried(h, 23986).await?, 1);
    at(owner, a, tome.teleport.ok_or("tome return")?).await?;
    let mut seq = a.continuity.next_command_id;
    super::questpc7_outfit_items::talk(owner, a, ivalisse, &mut seq, "tome").await?;
    assert_eq!(value(owner, a, TRACK).await?, 2);
    assert_eq!(carried(h, 23986).await?, 0);
    let mut seq = a.continuity.next_command_id;
    super::questpc7_outfit_items::talk(owner, a, ivalisse, &mut seq, "tome").await?;
    assert_eq!(value(owner, a, TRACK).await?, 2);
    assert_eq!(carried(h, 23986).await?, 0);
    step(owner, a, StepDirection::North).await?;
    passage(owner, a, callback(&rows, "energy-entry")?).await?;
    for (i, (name, phase, next)) in [
        ("lloyd", 2, 3),
        ("lady_tenebris", 3, 4),
        ("the_enraged_thorn_knight", 5, 6),
        ("soul_of_dragonking_zyrtarch", 7, 8),
        ("the_time_guardian", 8, 9),
        ("the_last_lore_keeper", 9, 10),
    ]
    .into_iter()
    .enumerate()
    {
        assert_eq!(value(owner, a, TRACK).await?, phase);
        owner.refresh_quest_session(a.game_session_id, false).await;
        let objective = oteryn_gameplay_quests::quest::death_objectives::load()
            .map_err(debug)?
            .iter()
            .find(|o| o.key == format!("oteryn:quest-death/questpc7-forgotten-{name}"))
            .ok_or("objective")?;
        let target = guardian(
            owner,
            objective.guardian.as_ref().ok_or("guardian")?.position,
        )
        .await?
        .ok_or("native boss")?;
        super::questpc7_native_deaths::lethal_creature(
            owner,
            a,
            target,
            31000 + u64::try_from(i)?,
            &format!("oteryn:creature.{name}"),
        )
        .await?;
        assert_eq!(
            value(
                owner,
                a,
                &format!("oteryn:quest-progress/questpc7/forgotten/boss_{i}")
            )
            .await?,
            1
        );
        passage(owner, a, callback(&rows, &format!("passage-{i}"))?).await?;
        assert_eq!(value(owner, a, TRACK).await?, next);
        assert!(completed(h).await?.is_none());
        if i == 1 {
            let egg = callback(&rows, "egg")?;
            assert_eq!(click(owner, a, egg).await?, UseDisposition::Committed);
            assert_eq!(value(owner, a, TRACK).await?, 5);
            at(owner, a, egg.teleport.ok_or("Thorn Knight approach")?).await?;
        }
        if i == 2 {
            let mut seq = a.continuity.next_command_id;
            super::questpc7_outfit_items::talk(owner, a, ivalisse, &mut seq, "father").await?;
            assert_eq!(value(owner, a, TRACK).await?, 60);
            passage(owner, a, callback(&rows, "fire-entry")?).await?;
            // The source father, a disguised chalice, must actually supply his aid.
            let mut seq = a.continuity.next_command_id;
            super::questpc7_outfit_items::talk(owner, a, silus, &mut seq, "learned").await?;
            assert_eq!(
                value(owner, a, TRACK).await?,
                60,
                "the last word cannot bypass the father's story"
            );
            for (j, word) in [
                "chalice",
                "daughter",
                "story",
                "disguised",
                "thirsty",
                "learned",
            ]
            .into_iter()
            .enumerate()
            {
                let mut seq = a.continuity.next_command_id;
                super::questpc7_outfit_items::talk(owner, a, silus, &mut seq, word).await?;
                assert_eq!(
                    value(owner, a, "oteryn:quest-progress/questpc7/forgotten/father").await?,
                    i64::try_from(j + 1)?
                );
            }
            assert_eq!(value(owner, a, TRACK).await?, 7);
            passage(owner, a, callback(&rows, "father-passage")?).await?;
        }
    }
    let premium = owner.premium_coordinator;
    owner.premium_coordinator = None;
    let mut seq = a.continuity.next_command_id;
    super::questpc7_outfit_items::talk(owner, a, ivalisse, &mut seq, "knowledge").await?;
    assert_eq!(value(owner, a, TRACK).await?, 10);
    assert!(completed(h).await?.is_none());
    owner.premium_coordinator = premium;
    let mut seq = a.continuity.next_command_id;
    super::questpc7_outfit_items::talk(owner, a, ivalisse, &mut seq, "knowledge").await?;
    assert_eq!(value(owner, a, TRACK).await?, 11);
    let earned = completed(h).await?;
    assert!(earned.is_some());
    let heart: i64=sqlx::query_scalar("SELECT count(*) FROM game_character_quest_states WHERE quest_key='oteryn:quest.heart_of_destruction_quest' AND completed_character_revision IS NOT NULL").fetch_one(&h.pool).await?;
    assert_eq!(
        heart, 0,
        "Forgotten completion independently earns the alternative learning"
    );
    let mut seq = a.continuity.next_command_id;
    super::questpc7_outfit_items::talk(owner, a, ivalisse, &mut seq, "knowledge").await?;
    assert_eq!(completed(h).await?, earned);
    assert_eq!(imbue(owner, a).await?, ImbuementResult::Ok);
    assert_eq!(money(h).await?, before_money - 250000);
    for (key, _) in recipe.materials {
        let id: u32 = key.rsplit('i').next().ok_or("item id")?.parse()?;
        assert_eq!(carried(h, id).await?, 0);
    }
    let applied: (i16, i16, i64) =
        sqlx::query_as("SELECT slot,category,remaining_ms FROM game_item_imbuements")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(applied, (0, 0, 72_000_000));
    assert_eq!(completed(h).await?, earned);
    eprintln!(
        "QUESTPC7_FORGOTTEN_STORY exact tome, six native bosses, physical egg, Ivalisse, completion/repeat and real Powerful Strike PASS"
    );
    Ok(())
}
