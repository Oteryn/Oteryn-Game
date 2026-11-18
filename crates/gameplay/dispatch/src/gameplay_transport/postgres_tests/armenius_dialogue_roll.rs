//! Crystal 201189f8 data-global/npc/armenius.lua:71-105.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use oteryn_gameplay_quests::quest::{
    npc_rules,
    world_callbacks::{Callback, Position},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

pub(super) const NAME: &str = "armenius-dialogue-roll";
const NPC: &str = "oteryn:npc.armenius";
const QUEST: &str = "oteryn:quest.blood_brothers_quest";
const MISSION: &str = "oteryn:quest-progress/quest/u8_4/blood_brothers/mission03";
const ARMENIUS: &str = "oteryn:creature.armenius";
const SOURCE: &str =
    "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/armenius.lua:71-105";
const TRIGGER: Position = Position {
    x: 32856,
    y: 31324,
    floor: -7,
};
const SEWER: Position = Position {
    x: 32759,
    y: 31241,
    floor: -9,
};
const BASEMENT: Position = Position {
    floor: -8,
    ..TRIGGER
};
const MONSTER: Position = Position {
    x: 32857,
    ..BASEMENT
};
const REPLIES: [&str; 3] = [
    "Oh, the nerve. Go to the rats which raised you.",
    "You know what, I changed my mind, BRING IT!",
    "Oh, the nerve. Sod off.",
];

#[test]
fn armenius_dialogue_roll_persists_each_route_and_earns_the_basement_achievement() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(NAME, || {
        for name in [
            "armenius-dialogue-roll-1",
            "armenius-dialogue-roll-2",
            "armenius-dialogue-roll-3",
        ] {
            super::quest_world_callbacks::scenario(name, TRIGGER, false)?;
        }
        Ok(())
    })
}

pub(super) fn world(row: &Callback) -> TestResult<crate::map::boot::BundleWorld> {
    super::quest_world_callbacks::world_with_cells(row, TRIGGER, &[SEWER, BASEMENT, MONSTER])
}

fn rules() -> Vec<Value> {
    let mut result = Vec::new();
    for (before, after, line) in [
        (0, 2, "Stop mumbling and don't bug me."),
        (
            2,
            3,
            "There's something about these words which makes me feel awkward. Or maybe it's you who causes that feeling. You better get lost.",
        ),
        (3, 4, "Whatever that's supposed to mean."),
        (
            4,
            5,
            "HAHAHAHAHA! What the... HAHAHAHA! Come on, say it again, just because it's so funny - and then I'll get rid of you, little mouse!",
        ),
    ] {
        result.push(rule(
            format!("topic-{before}"),
            before,
            vec![line],
            vec![],
            vec![json!({"kind":"set_topic","value":after})],
        ));
    }
    for (index, &line) in REPLIES.iter().enumerate() {
        let value = index + 1;
        let mut effects = vec![json!({"kind":"set","track":MISSION,"value":2})];
        if let Some(position) = [Some(SEWER), Some(BASEMENT), None][index] {
            effects.extend([
                json!({"kind":"teleport","position":position}),
                json!({"kind":"magic_effect","effect":"CONST_ME_TELEPORT"}),
            ]);
        }
        if value == 2 {
            effects.extend([
                json!({"kind":"teleport_encounter","creature":ARMENIUS,"position":MONSTER}),
                json!({"kind":"achievement","name":"His True Face"}),
            ]);
        }
        result.push(rule(
            format!("roll-{value}"),
            5,
            vec![line],
            vec![json!({"kind":"roll","sides":3,"value":value})],
            effects,
        ));
    }
    result
}

fn rule(
    key: String,
    topic: i32,
    reply: Vec<&str>,
    mut guards: Vec<Value>,
    effects: Vec<Value>,
) -> Value {
    guards.insert(
        0,
        json!({"kind":"storage","track":MISSION,"op":"eq","value":1}),
    );
    json!({
        "key":format!("oteryn:quest-npc/fixture-armenius/{key}"),"quest":QUEST,"npc":NPC,"trigger":"say",
        "keywords":["alori mort"],"keyword_mode":"exact","topics":[topic],"guards":guards,
        "reply":reply,"effects":effects,"sources":[SOURCE]
    })
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    name: &str,
) -> TestResult {
    let result: usize = name.rsplit_once('-').ok_or("roll case")?.1.parse()?;
    let base = owner.quest_catalogue.as_deref().ok_or("quest catalogue")?;
    let track = base.track(MISSION).ok_or("Armenius mission track")?;
    let document = json!({"schema":1,"quest":QUEST,
        "tracks":[{"key":MISSION,"quest":QUEST,"initial":track.initial,"min":track.min,"max":track.max,"sources":[SOURCE]}],
        "rules":rules()
    }).to_string();
    owner.quest_catalogue = Some(std::sync::Arc::new(npc_rules::with_fixture_programs(
        base.clone(),
        &[&document],
    )?));
    super::qengine_outfit_stories::seed_quest_prerequisites(
        h,
        owner,
        admitted,
        &[(MISSION.into(), 1)],
        125,
    )
    .await?;
    let [npc] = super::classic_quests::install(owner, admitted, [NPC]).await?;
    super::classic_quests::talk(owner, admitted, npc, 8190, "hi").await?;
    for command in 8191..8195 {
        assert!(
            !super::classic_quests::talk(owner, admitted, npc, command, "alori mort")
                .await?
                .is_empty()
        );
    }
    let actor = admitted.runtime_actor.ok_or("player")?;
    let before = owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?
        .position();
    // Choose an ordinary command occurrence which gives this case's roll.
    // The dispatcher and the locked writer both evaluate the real seed.
    let command = (8195_u64..8295)
        .find(|command| {
            let seed = Sha256::new()
                .chain_update(b"oteryn:quest-npc-dialogue-roll:v1\0")
                .chain_update(admitted.game_session_id.as_bytes())
                .chain_update(command.to_be_bytes())
                .chain_update(NPC.as_bytes())
                .finalize();
            let draw = u64::from_be_bytes(seed[..8].try_into().expect("SHA-256 prefix"));
            draw < u64::MAX - u64::MAX % 3 && draw % 3 + 1 == result as u64
        })
        .ok_or("command for the requested roll")?;
    super::classic_quests::talk(owner, admitted, npc, command, "alori mort").await?;
    assert_eq!(super::m39_bone_journey::stage(h, MISSION).await?, Some(2));
    let grant: i64 = sqlx::query_scalar("SELECT count(*) FROM game_account_achievements WHERE achievement_key='oteryn:achievement/his_true_face'")
        .fetch_one(&h.pool).await?;
    assert_eq!(grant, i64::from(result == 2));
    let runtime = owner.runtime.lock().await;
    let position = runtime
        .read_actor_position(actor)
        .map_err(debug)?
        .position();
    let expected = match result {
        1 => super::quest_world_callbacks::local(SEWER),
        2 => super::quest_world_callbacks::local(BASEMENT),
        3 => before,
        _ => return Err("roll out of range".into()),
    };
    assert_eq!(position, expected);
    let monsters = runtime
        .visible_entities()
        .creatures
        .iter()
        .filter(|row| runtime.matches_live_creature_identity(row.actor, ARMENIUS.as_bytes()))
        .count();
    assert_eq!(monsters, usize::from(result == 2));
    Ok(())
}
