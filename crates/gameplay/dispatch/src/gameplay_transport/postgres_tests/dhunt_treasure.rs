//! Exercise a counted renewable reward through native physical USE and PostgreSQL MINT.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue,
    authored_bindings::Bindings,
    world_callbacks::{Callback, Position},
};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
use serde_json::json;
use std::sync::Arc;

pub(super) const SCENARIO: &str = "dhunt-hoard-rewards";
const QUEST: &str = "oteryn:quest.authored.draccoon_herald_outfits_quest";
const KILLS: &str = "oteryn:quest-progress/dhunt-fixture/kills";
const CLAIMS: &str = "oteryn:quest-progress/dhunt-fixture/claims";
const GATE: &str = "oteryn:quest-progress/dhunt-fixture/gate";
const ACHIEVEMENT: &str = "oteryn:achievement/hoard_of_the_dragon";
const BUNDLES: &[&[(u16, i64)]] = &[
    &[(3035, 5)],
    &[(3029, 1), (3030, 1), (3032, 1)],
    &[(3089, 1)],
    &[(3039, 1)],
    &[(237, 10)],
    &[(236, 8)],
];

#[test]
fn repeatable_use_mints_one_bundle_and_grants_the_tenth_claim_achievement_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty(), "PostgreSQL admin URL is empty");
    let source = Position {
        x: 33137,
        y: 32351,
        floor: -6,
    };
    super::kill_reward_e2e::scenario_thread::run(SCENARIO, move || {
        super::quest_world_callbacks::scenario(SCENARIO, source, false)
    })
}

fn document(template: &Callback) -> TestResult<Vec<u8>> {
    let rewards: Vec<_> = BUNDLES
        .iter()
        .map(|bundle| {
            let (id, count) = bundle[0];
            let additional: Vec<_> = bundle[1..]
                .iter()
                .map(|(id, count)| json!({"item":format!("oteryn:item.tibia.i{id}"),"count":count}))
                .collect();
            json!({"item":format!("oteryn:item.tibia.i{id}"),"count":count,"additional":additional})
        })
        .collect();
    let mut callback = serde_json::to_value(template)?;
    callback["quest"] = QUEST.into();
    callback["key"] = "oteryn:quest-callback/dhunt-fixture/treasure".into();
    callback["storage"] = json!([
        {"track":KILLS,"minimum":1,"maximum":1,"set":0},
        {"track":CLAIMS,"minimum":0,"maximum":99,"set":1,"increment":true}
    ]);
    callback["state_ops"] = json!([]);
    callback["admission"] = serde_json::Value::Null;
    callback["success_message"] = serde_json::Value::Null;
    callback["effects"] = json!({
        "item_reward":rewards[0],
        "item_reward_per_use":true,
        "item_reward_alternatives":rewards[1..],
        "claim_achievement":{"track":CLAIMS,"value":10,"key":ACHIEVEMENT}
    });
    Ok(serde_json::to_vec(&json!({"schema":1,"quests":[{
        "quest":QUEST,
        "recipe_sha256":"c6bb54b0c423a165891b0ca32fb305a0963d1a06417249b0b317ec34cf65e708",
        "sources":["physical USE fixture"],
        "tracks":[
            {"key":KILLS,"initial":1,"minimum":0,"maximum":1},
            {"key":CLAIMS,"initial":8,"minimum":0,"maximum":100},
            {"key":GATE,"initial":1,"minimum":1,"maximum":1}
        ],
        "stages":[
            {"key":"claim","completes":false,"trigger":{"kind":"map_use","callback":callback}},
            {"key":"kill","completes":false,"trigger":{"kind":"kill","counter":{
                "key":"oteryn:quest-transition/native-kills/dhunt-fixture-rearm",
                "creature":"oteryn:creature.rat","track":KILLS,"gate":GATE,"gate_value":1,"maximum":1
            }}}
        ]
    }]}))?)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    template: &Callback,
) -> TestResult {
    let revision = owner
        .quest_catalogue
        .as_ref()
        .ok_or("baseline")?
        .content_revision();
    let base = QuestStateCatalogue::empty(revision).ok_or("catalogue")?;
    let bindings = Bindings::parse(&document(template)?)?;
    let callback = bindings.world_callbacks().next().ok_or("world callback")?;
    owner
        .states
        .quest_callbacks
        .fixture
        .set(vec![callback.clone()])
        .map_err(|_| "fixture already installed")?;
    owner.quest_catalogue = Some(Arc::new(bindings.attach(&base)?));
    owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .ok_or("native gameplay")?
        .install_companion_policies(&mut *owner.runtime.lock().await)
        .map_err(debug)?;
    owner
        .refresh_quest_session(admitted.game_session_id, true)
        .await;

    let native = template.selector.native.as_ref().ok_or("native source")?;
    let position = native.position;
    let key = oteryn_world_bundle::bundle::placement_key(
        position.floor,
        position.x,
        position.y,
        native.ordinal,
    )
    .ok_or("placement")?;
    let mut placement = owner
        .bundle_map()
        .ok_or("map")?
        .overlay
        .base()
        .digest()
        .to_vec();
    placement.extend_from_slice(&key.to_be_bytes());

    let mut before = quantities(h).await?;
    for (claim, command) in [(9, 1), (10, 3), (11, 5)] {
        let payload = oteryn_protocol_oteryn::world_object::encode_use_intent(&WorldObjectTarget {
            placement: placement.clone(),
            expected_revision: claim - 9,
        })
        .map_err(debug)?;
        for _ in 0..2 {
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(std::time::Duration::from_secs(1))
                .ok_or("clock")?;
            let reply = Box::pin(super::quest_world_callbacks::wire(
                owner,
                admitted,
                command,
                2,
                payload.clone(),
            ))
            .await?;
            assert_eq!(
                oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)?,
                UseDisposition::Committed
            );
        }
        let after = quantities(h).await?;
        let delta: Vec<_> = after
            .iter()
            .zip(&before)
            .filter_map(|(&(id, after), &(_, before))| {
                (after != before).then_some((id, after - before))
            })
            .collect();
        assert!(
            BUNDLES.contains(&delta.as_slice()),
            "one full bundle per fresh claim, got {delta:?}"
        );
        before = after;
        let tracks: Vec<i64> = sqlx::query_scalar(
            "SELECT value FROM game_character_quest_tracks WHERE quest_key = $1 AND track_key IN ($2, $3) ORDER BY track_key",
        ).bind(QUEST).bind(CLAIMS).bind(KILLS).fetch_all(&h.pool).await?;
        assert_eq!(tracks, [i64::try_from(claim)?, 0]);
        let awards: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_account_achievement_grant_requests WHERE achievement_key = $1",
        ).bind(ACHIEVEMENT).fetch_one(&h.pool).await?;
        assert_eq!(awards, i64::from(claim >= 10));
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(1))
            .ok_or("clock")?;
        let empty_stock =
            oteryn_protocol_oteryn::world_object::encode_use_intent(&WorldObjectTarget {
                placement: placement.clone(),
                expected_revision: claim - 8,
            })
            .map_err(debug)?;
        let denied = Box::pin(super::quest_world_callbacks::wire(
            owner,
            admitted,
            command + 1,
            2,
            empty_stock,
        ))
        .await?;
        assert_eq!(
            oteryn_protocol_oteryn::world_object::decode_use_result(&denied).map_err(debug)?,
            UseDisposition::Rejected,
            "the successful claim must consume the accumulated kills"
        );
        assert_eq!(quantities(h).await?, before);
        if claim < 11 {
            super::qengine_kill_stories::death(owner, admitted, "oteryn:creature.rat", command + 100)
                .await?;
        }
    }
    Ok(())
}

async fn quantities(h: &Harness) -> TestResult<Vec<(u16, i64)>> {
    let mut quantities = Vec::new();
    for id in [236, 237, 3029, 3030, 3032, 3035, 3039, 3089] {
        quantities.push((id, super::qengine_voucher_stories::quantity(h, id).await?));
    }
    Ok(quantities)
}
