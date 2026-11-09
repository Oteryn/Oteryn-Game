//! Whole fixed Feaster of Souls expedition and actual native powerful learning.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
#[test]
fn questpc7_feaster_native_sampling_prefix() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("questpc7-feaster-prefix", || {
        super::quest_world_callbacks::scenario(
            "qengine-rewards-feaster-prefix",
            Position {
                x: 32627,
                y: 32076,
                floor: -7,
            },
            false,
        )
    });
    eprintln!(
        "QUESTPC7_FEASTER_PREFIX POSTGRES_CASE_COMPLETED duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
use super::questpc7_ferumbras::{step, use_at, value, walk};
use oteryn_gameplay_quests::quest::world_callbacks::Callback;
use oteryn_protocol_oteryn::world_object::UseDisposition;
const QUEST: &str = "oteryn:quest.feaster_of_souls_quest";
const TRACK: &str = "oteryn:quest-progress/questpc7/feaster/programme";
fn callbacks() -> TestResult<Vec<Callback>> {
    Ok(oteryn_gameplay_quests::quest::world_callbacks::load()
        .map_err(debug)?
        .into_iter()
        .filter(|c| {
            c.quest == QUEST && c.key.starts_with("oteryn:quest-callback/questpc7-feaster-")
        })
        .collect())
}
fn callback<'a>(rows: &'a [Callback], name: &str) -> TestResult<&'a Callback> {
    rows.iter()
        .find(|c| c.key == format!("oteryn:quest-callback/questpc7-feaster-{name}"))
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
    let fixed: std::collections::BTreeSet<_> = native_items()?
        .into_iter()
        .map(|(p, _)| (p.x, p.y, p.floor))
        .collect();
    for c in &rows {
        let p = c.selector.position.ok_or("position")?;
        if fixed.contains(&(p.x, p.y, p.floor)) {
            continue;
        }
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
    cells.entry((32628, 32076, -7)).or_insert(100);
    Ok(cells
        .into_iter()
        .map(|((x, y, floor), appearance)| (Position { x, y, floor }, appearance))
        .collect())
}
async fn carried(h: &Harness, item: u32) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce(sum(i.quantity),0)::bigint FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid) c JOIN game_item_instances i ON i.item_instance_id=c.item WHERE i.lifecycle=1 AND i.definition_production_key=$3").bind(super::fixture::id(41).as_slice()).bind(super::fixture::id(42).as_slice()).bind(format!("oteryn:item.tibia.i{item}")).fetch_one(&h.pool).await?)
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
fn track(name: &str) -> String {
    format!("oteryn:quest-progress/questpc7/feaster/{name}")
}
pub(super) fn native_items() -> TestResult<Vec<(Position, u32)>> {
    let doc: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../../content/world/placements/authored-questpc7.json"
    ))?;
    doc["props"]
        .as_array()
        .ok_or_else(|| "props".into())
        .and_then(|rows| {
            rows.iter()
                .map(|r| {
                    let p = &r["position"];
                    Ok((
                        Position {
                            x: u16::try_from(p["x"].as_u64().ok_or("x")?)?,
                            y: u16::try_from(p["y"].as_u64().ok_or("y")?)?,
                            floor: i8::try_from(p["floor"].as_i64().ok_or("floor")?)?,
                        },
                        r["item"]
                            .as_str()
                            .ok_or("item")?
                            .rsplit('i')
                            .next()
                            .ok_or("id")?
                            .parse()?,
                    ))
                })
                .collect()
        })
}
async fn receipts(h: &Harness) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
            .fetch_one(&h.pool)
            .await?,
    )
}
async fn supply(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    a: &AdmittedSession,
    item: &str,
    position: Position,
    nonce: u64,
    pickup: bool,
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
            position,
            item: item.into(),
            count: 1,
        },
        nonce,
        pickup,
    )
    .await
}
async fn word(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    a: &mut AdmittedSession,
    npc: oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
    text: &str,
    accepted: bool,
) -> TestResult {
    // Snapshot admission clears NPC engagement. Keep the greeting and keyword
    // on one connection and retain the shared ingress/MINT command sequence.
    let greeting = a.continuity.next_command_id;
    let keyword = greeting.checked_add(1).ok_or("command exhaustion")?;
    let commands = [(greeting, "hi"), (keyword, text)];
    if accepted {
        super::grizzly_npc::wire::talk(owner, a, npc, &commands).await
    } else {
        super::grizzly_npc::wire::talk_refused(owner, a, npc, &commands).await
    }
}
pub(super) async fn begin(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    a: &mut AdmittedSession,
    h: &Harness,
) -> TestResult<(Vec<Callback>,oteryn_protocol_oteryn::world_spatial_entities::EntityRef)> {
    let version: String = sqlx::query_scalar("SHOW server_version")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("QUESTPC7_FEASTER_PREFIX POSTGRES_STARTED version={version}");
    let rows = callbacks()?;
    assert_eq!(rows.len(), 13);
    let mut selected = a.continuity.selected_capabilities.as_slice().to_vec();
    selected.extend([3, 4, 6, 11, 12, 14, 15, 18]);
    selected.sort_unstable();
    selected.dedup();
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    a.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered, &selected)
            .ok_or("capabilities")?;
    let [npc] = super::classic_quests::install_positions(
        owner,
        a,
        ["oteryn:npc.para_norbert"],
        [Position {
            x: 32628,
            y: 32076,
            floor: -7,
        }],
    )
    .await?;
    assert_eq!(value(owner, a, TRACK).await?, -1);
    assert_eq!(carried(h, 32582).await?, 0);
    let premium = owner.premium_coordinator;
    owner.premium_coordinator = None;
    word(owner, a, npc, "mission", false).await?;
    assert_eq!(value(owner, a, TRACK).await?, -1);
    assert_eq!(carried(h, 32582).await?, 0);
    owner.premium_coordinator = premium;
    word(owner, a, npc, "mission", true).await?;
    assert_eq!(value(owner, a, TRACK).await?, 0);
    assert_eq!(carried(h, 32582).await?, 1);
    Ok((rows,npc))
}
pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    a: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let (rows,npc)=begin(owner,a,h).await?;
    // Norbert's approach is also the entrance selector. A STEP callback needs
    // an actual crossing after accepting the mission, rather than a no-op walk.
    step(
        owner,
        a,
        oteryn_protocol_oteryn::world_spatial::StepDirection::South,
    )
    .await?;
    passage(owner, a, callback(&rows, "entry")?).await?;
    for i in 0..3 {
        let c = callback(&rows, &format!("bell-{i}"))?;
        assert_eq!(click(owner, a, c).await?, UseDisposition::Committed);
        at(owner, a, c.teleport.ok_or("bell destination")?).await?;
        assert_eq!(value(owner, a, &track("bells")).await?, i + 1);
    }
    assert_eq!(value(owner, a, TRACK).await?, 1);
    assert_eq!(carried(h, 32582).await?, 1);
    let grinder = callback(&rows, "grinder")?;
    assert_ne!(click(owner, a, grinder).await?, UseDisposition::Committed);
    assert_eq!(value(owner, a, TRACK).await?, 1);
    assert_eq!(carried(h, 32602).await?, 0);
    supply(
        owner,
        a,
        "oteryn:item.tibia.i3114",
        Position {
            x: 32629,
            y: 32075,
            floor: -7,
        },
        32000,
        true,
    )
    .await?;
    assert_eq!(click(owner, a, grinder).await?, UseDisposition::Committed);
    assert_eq!(carried(h, 3114).await?, 0);
    assert_eq!(carried(h, 32602).await?, 1);
    assert_eq!(value(owner, a, TRACK).await?, 2);
    let apparatus = callback(&rows, "apparatus")?;
    walk(
        owner,
        a,
        Position {
            x: 32628,
            y: 32077,
            floor: -7,
        },
    )
    .await?;
    assert_ne!(click(owner, a, apparatus).await?, UseDisposition::Committed);
    assert_eq!(carried(h, 32602).await?, 1);
    assert_eq!(carried(h, 32584).await?, 0);
    assert_eq!(value(owner, a, TRACK).await?, 2);
    supply(
        owner,
        a,
        "oteryn:item.tibia.i3661",
        apparatus.offerings[0].position,
        32001,
        false,
    )
    .await?;
    assert_eq!(click(owner, a, apparatus).await?, UseDisposition::Committed);
    assert_eq!(carried(h, 32602).await?, 0);
    assert_eq!(carried(h, 32584).await?, 1);
    assert_eq!(value(owner, a, TRACK).await?, 3);
    walk(
        owner,
        a,
        Position {
            x: 32627,
            y: 32076,
            floor: -7,
        },
    )
    .await?;
    word(owner, a, npc, "scalpel", true).await?;
    assert_eq!(carried(h, 32225).await?, 1);
    word(owner, a, npc, "scalpel", false).await?;
    assert_eq!(carried(h, 32225).await?, 1);
    let potion = callback(&rows, "potion")?;
    assert_eq!(click(owner, a, potion).await?, UseDisposition::Committed);
    assert_eq!(carried(h, 32584).await?, 0);
    at(owner, a, potion.teleport.ok_or("Brain Grounds")?).await?;
    assert_eq!(value(owner, a, TRACK).await?, 4);
    let sample = callback(&rows, "sample")?;
    // MINT preserves one fresh instance per occurrence. Carry the twenty probes
    // in an ordinary nested backpack so the scalpel and other expedition items
    // do not exhaust the twenty-entry main backpack during sampling.
    supply(
        owner,
        a,
        "oteryn:item.tibia.i2854",
        sample.selector.position.ok_or("sample position")?,
        32002,
        true,
    )
    .await?;
    let probe_bag: sqlx::types::Uuid = sqlx::query_scalar(
        "SELECT i.item_instance_id FROM game_item_container_entries e \
         JOIN game_item_instances i USING(item_instance_id) \
         WHERE i.definition_production_key='oteryn:item.tibia.i2854' AND i.lifecycle=1 \
         ORDER BY i.item_instance_id LIMIT 1",
    )
    .fetch_one(&h.pool)
    .await?;
    for i in 0..20 {
        if i == 19 {
            let premium = owner.premium_coordinator;
            owner.premium_coordinator = None;
            let before = receipts(h).await?;
            assert_ne!(click(owner, a, sample).await?, UseDisposition::Committed);
            assert_eq!(value(owner, a, &track("samples")).await?, 19);
            assert_eq!(carried(h, 32197).await?, 19);
            assert_eq!(receipts(h).await?, before);
            owner.premium_coordinator = premium;
        }
        let replay = if i == 0 {
            Some((
                a.continuity.next_command_id,
                super::questpc7_hoist_story::target(
                    owner,
                    sample.selector.position.ok_or("sample position")?,
                    sample.selector.appearance.ok_or("sample appearance")?,
                )?,
            ))
        } else {
            None
        };
        assert_eq!(click(owner, a, sample).await?, UseDisposition::Committed);
        if let Some((command, payload)) = replay {
            let before = receipts(h).await?;
            let reply = super::quest_world_callbacks::wire(owner, a, command, 2, payload).await?;
            assert_eq!(
                oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)?,
                UseDisposition::Committed
            );
            assert_eq!(
                receipts(h).await?,
                before,
                "sampling replay adds no occurrence"
            );
        }
        let probe: sqlx::types::Uuid = sqlx::query_scalar(
            "SELECT i.item_instance_id FROM game_item_container_entries e \
             JOIN game_item_instances i USING(item_instance_id) \
             WHERE i.definition_production_key='oteryn:item.tibia.i32197' AND i.lifecycle=1 \
             ORDER BY i.item_instance_id LIMIT 1",
        )
        .fetch_one(&h.pool)
        .await?;
        super::qengine_rewards::into_bag(
            owner,
            a,
            *probe.as_bytes(),
            *probe_bag.as_bytes(),
            400_000 + u64::try_from(i)?,
        )
        .await?;
        assert_eq!(value(owner, a, &track("samples")).await?, i + 1);
        assert_eq!(carried(h, 32197).await?, i + 1);
    }
    let before = receipts(h).await?;
    let f = owner
        .current_quest_fence(a.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    assert_ne!(click(owner, a, sample).await?, UseDisposition::Committed);
    assert_eq!(receipts(h).await?, before);
    assert_eq!(value(owner, a, &track("samples")).await?, 20);
    assert_eq!(carried(h, 32197).await?, 20);
    assert_eq!(
        owner
            .current_quest_fence(a.game_session_id)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?
            .expected_character_revision,
        f.expected_character_revision
    );
    step(
        owner,
        a,
        oteryn_protocol_oteryn::world_spatial::StepDirection::South,
    )
    .await?;
    passage(owner, a, callback(&rows, "samples-return")?).await?;
    word(owner, a, npc, "probes", true).await?;
    assert_eq!(carried(h, 32197).await?, 0);
    assert_eq!(carried(h, 32226).await?, 1);
    assert_eq!(value(owner, a, TRACK).await?, 5);
    word(owner, a, npc, "probes", false).await?;
    assert_eq!(carried(h, 32226).await?, 1);
    step(
        owner,
        a,
        oteryn_protocol_oteryn::world_spatial::StepDirection::North,
    )
    .await?;
    passage(owner, a, callback(&rows, "purification-entry")?).await?;
    let gather = callback(&rows, "gather-soul")?;
    for i in 0..20 {
        assert_eq!(click(owner, a, gather).await?, UseDisposition::Committed);
        assert_eq!(carried(h, 32227).await?, i + 1);
    }
    let before = receipts(h).await?;
    assert_ne!(click(owner, a, gather).await?, UseDisposition::Committed);
    assert_eq!(receipts(h).await?, before);
    assert_eq!(carried(h, 32227).await?, 20);
    let purify = callback(&rows, "purify")?;
    walk(
        owner,
        a,
        Position {
            x: 31919,
            y: 32354,
            floor: -8,
        },
    )
    .await?;
    for i in 0..20 {
        assert_eq!(click(owner, a, purify).await?, UseDisposition::Committed);
        assert_eq!(carried(h, 32227).await?, 19 - i);
        assert_eq!(carried(h, 32228).await?, i + 1);
        assert_eq!(value(owner, a, &track("purified")).await?, i + 1);
    }
    let before = receipts(h).await?;
    assert_ne!(click(owner, a, purify).await?, UseDisposition::Committed);
    assert_eq!(receipts(h).await?, before);
    passage(owner, a, callback(&rows, "souls-return")?).await?;
    word(owner, a, npc, "souls", true).await?;
    assert_eq!(carried(h, 32228).await?, 0);
    assert_eq!(value(owner, a, TRACK).await?, 6);
    assert_eq!(carried(h, 32582).await?, 1);
    assert_eq!(carried(h, 32225).await?, 1);
    assert_eq!(carried(h, 32226).await?, 1);
    let complete:i64=sqlx::query_scalar("SELECT count(*) FROM game_character_quest_states WHERE quest_key=$1 AND completed_character_revision IS NOT NULL").bind(QUEST).fetch_one(&h.pool).await?;
    assert_eq!(complete, 0, "prefix never claims whole Feaster completion");
    eprintln!(
        "QUESTPC7_FEASTER_PREFIX first6 source milestones, exact ingredients and20/20/20 durable bounded uses PASS"
    );
    Ok(())
}
