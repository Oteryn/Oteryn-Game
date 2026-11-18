//! Crystal 201189f8 data-global/npc/the_queen_of_the_banshees.lua:62-72,89-149.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, QuestTrack, npc_rules::NpcRuleCatalog};

pub(super) const NAME: &str = "banshee-queen-kiss";
pub(super) const LANDING: Position = Position {
    x: 32202,
    y: 31812,
    floor: -8,
};
const ENTRY: Position = Position {
    x: 33294,
    y: 31677,
    floor: -15,
};
const QUEST: &str = "oteryn:quest.the_queen_of_the_banshees_quest";
const QUEEN: &str = "oteryn:npc.the_queen_of_the_banshees";
const TRACK: &str = "oteryn:quest-progress/quest/u7_2/the_queen_of_the_banshees/";
const FLAGS: &[&str] = &["last_seal", "last_seal_door", "banshee_door"];
const SEALS: &[&str] = &[
    "first_seal",
    "second_seal",
    "third_seal",
    "fourth_seal",
    "fifth_seal",
    "sixth_seal",
];
const SOURCE: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/the_queen_of_the_banshees.lua:137-149";

#[test]
fn queen_kiss_refuses_pz_lock_then_commits_arrival_and_three_seal_flags() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(NAME, || {
        super::quest_world_callbacks::scenario(NAME, ENTRY, false)
    })
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
) -> TestResult {
    use serde_json::json;
    let tracks: Vec<_> = FLAGS
        .iter()
        .map(|name| {
            json!({
                "key":format!("{TRACK}{name}"),"quest":QUEST,"initial":-1,"min":-1,"max":1,
                "sources":[SOURCE],
            })
        })
        .collect();
    let mut effects: Vec<_> = FLAGS
        .iter()
        .map(|name| {
            json!({
                "kind":"set", "track":format!("{TRACK}{name}"),"value":1,
            })
        })
        .collect();
    effects.extend([
        json!({"kind":"teleport","position":LANDING,
            "pz_locked_reply":"You have spilled too much blood recently and the dead are hungry for your soul. Perhaps return when you regained you inner balance."}),
        json!({"kind":"magic_effect","effect":"CONST_ME_TELEPORT"}),
        json!({"kind":"set_topic","value":0}),
    ]);
    let program = json!({
        "schema":1,"quest":QUEST,"tracks":tracks,
        "rules":[{
            "key":"oteryn:quest-npc/banshees/queen/kiss","quest":QUEST,"npc":QUEEN,
            "trigger":"say","keywords":["yes"],"topics":[8],
            "guards":[{"kind":"storage","track":format!("{TRACK}last_seal"),"op":"lt","value":1}],
            "effects":effects,"reply":["So be it! Hmmmmmm..."],"sources":[SOURCE],
        }]
    })
    .to_string();
    let catalogue = QuestStateCatalogue::new(
        "content-1",
        SEALS
            .iter()
            .map(|name| QuestTrack {
                key: format!("{TRACK}{name}"),
                quest: QUEST.into(),
                initial: 1,
                min: -1,
                max: 1,
            })
            .collect(),
        vec![],
    )
    .map_err(debug)?;
    owner.quest_catalogue = Some(std::sync::Arc::new(
        NpcRuleCatalog::from_json_documents(&[&program])?.bind_fixture(catalogue)?,
    ));
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let session = admitted.game_session_id;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let origin = owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?
        .position();
    let [queen] = super::classic_quests::install(owner, admitted, [QUEEN]).await?;

    for (index, text) in [
        "hi",
        "seventh seal",
        "yes",
        "yes",
        "yes",
        "yes",
        "yes",
        "yes",
        "kiss",
    ]
    .into_iter()
    .enumerate()
    {
        super::classic_quests::talk(owner, admitted, queen, 6500 + u64::try_from(index)?, text)
            .await?;
    }
    {
        let mut states = owner.states.spell_states.lock().await;
        let deadline = oteryn_simulation_determinism::SemanticTimeMicros::from_micros(
            owner.owner_now().get() + 60_000_000,
        );
        crate::gameplay_transport::pvp_live::lock_mut(&mut states, actor, session).restore(
            None,
            Some(deadline),
            None,
        );
    }
    let refused = super::classic_quests::talk(owner, admitted, queen, 6510, "yes").await?;
    assert!(
        refused
            .iter()
            .any(|line| line.contains("spilled too much blood"))
    );
    assert_eq!(arrivals(h).await?, 0);
    assert!(flags(h).await?.is_empty());
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position(),
        origin
    );

    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(61))
        .ok_or("clock")?;
    for (index, text) in [
        "seventh seal",
        "yes",
        "yes",
        "yes",
        "yes",
        "yes",
        "yes",
        "kiss",
        "yes",
    ]
    .into_iter()
    .enumerate()
    {
        super::classic_quests::talk(owner, admitted, queen, 6520 + u64::try_from(index)?, text)
            .await?;
    }
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position(),
        super::quest_world_callbacks::local(LANDING)
    );
    assert_eq!(flags(h).await?, vec![1, 1, 1]);
    assert_eq!(arrivals(h).await?, 1);
    Ok(())
}

async fn flags(h: &Harness) -> TestResult<Vec<i64>> {
    let keys: Vec<_> = FLAGS.iter().map(|name| format!("{TRACK}{name}")).collect();
    Ok(
        sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=ANY($1)")
            .bind(keys)
            .fetch_all(&h.pool)
            .await?,
    )
}

async fn arrivals(h: &Harness) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT count(*) FROM game_quest_npc_teleport_receipts")
            .fetch_one(&h.pool)
            .await?,
    )
}
