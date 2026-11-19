//! Lane qc: Obsidian Knife and Sinatuki through native NPC wire commands.
use super::fixture::{Harness, debug, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_gameplay_quests::quest::reward_callbacks::RewardCallback;
use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};
use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;

pub(super) const START: Position = Position { x: 33294, y: 31677, floor: -15 };
const CYCLOPS: &str = "oteryn:npc.a_sweaty_cyclops";

pub(super) fn cyclops_placement() -> TestResult<Position> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../content/world/npc-placements");
    let index: serde_json::Value = serde_json::from_slice(&std::fs::read(root.join("index.json"))?)?;
    for shard in index["shards"].as_array().ok_or("placement shards")? {
        let locator = shard.as_str().ok_or("placement shard")?;
        let filename = std::path::Path::new(locator).file_name().ok_or("shard name")?;
        let document: serde_json::Value = serde_json::from_slice(&std::fs::read(root.join(filename))?)?;
        for row in document["records"].as_array().ok_or("placement records")? {
            let row = &row["declaration"];
            if row["npc"] == CYCLOPS {
                let cell = &row["cell"];
                return Ok(Position {
                    x: u16::try_from(cell["x"].as_i64().ok_or("placement x")?)?,
                    y: u16::try_from(cell["y"].as_i64().ok_or("placement y")?)?,
                    floor: -i8::try_from(cell["floor"].as_i64().ok_or("placement floor")?)?,
                });
            }
        }
    }
    Err("missing shipped NPC placement for the Sweaty Cyclops".into())
}

pub(super) fn cyclops_world(native: &crate::content::native_gameplay::NativeGameplayState)
    -> TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups_fixtures::{NativeCrop, native_crop};
    let at = cyclops_placement()?;
    let start = (at.x-1, at.y);
    native_crop(native, NativeCrop {
        bytes: include_bytes!("../../../../../../../content/world/placements/region-z03-x127-y123.b3"),
        region: (u8::try_from(-at.floor)?,at.x/256,at.y/256),
        bounds: (at.x-3, at.y-2, at.x+2, at.y+2),
        start,
        expected: Vec::new(),
    })
}

async fn count(h: &Harness, item: u32) -> TestResult<i64> {
    super::questpc7_outfit_items_fixtures::quantity(h, item).await
}

async fn mint(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    item: u32,
    amount: u16,
    nonce: u64,
) -> TestResult {
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let at = owner.runtime.lock().await.read_actor_position(actor).map_err(debug)?.position();
    super::quest_world_callbacks_fixtures::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: Position {
                x: u16::try_from(at.x)?,
                y: u16::try_from(at.y)?,
                floor: -i8::try_from(at.floor)?,
            },
            item: format!("oteryn:item.tibia.i{item}"),
            count: amount,
        },
        nonce,
        true,
    )
    .await
}

async fn completed(h: &Harness, quest: &str) -> TestResult<bool> {
    Ok(sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM game_character_quest_states WHERE \
         character_id=encode($1,'hex')::uuid AND quest_key=$2 \
         AND completed_character_revision IS NOT NULL)",
    )
    .bind(id(41).as_slice())
    .bind(quest)
    .fetch_one(&h.pool)
    .await?)
}

fn callback<'a>(callbacks: &'a [RewardCallback], name: &str) -> TestResult<&'a RewardCallback> {
    callbacks
        .iter()
        .find(|c| c.key == format!("oteryn:quest-reward/{name}"))
        .ok_or_else(|| format!("missing callback {name}").into())
}

fn select_npc_capabilities(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    let mut selected = admitted
        .continuity
        .selected_capabilities
        .as_slice()
        .to_vec();
    selected.extend([3, 6, 25]);
    selected.sort_unstable();
    selected.dedup();
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered, &selected)
            .ok_or("capabilities")?;
    Ok(())
}

pub(super) fn matches(name: &str) -> bool {
    matches!(name, "qc-obsidian-knife" | "qc-sinatuki")
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    name: &str,
) -> TestResult {
    select_npc_capabilities(owner, admitted)?;
    let callbacks = oteryn_gameplay_quests::quest::reward_callbacks::load().map_err(debug)?;
    if name == "qc-obsidian-knife" {
        obsidian_knife(owner, admitted, h, &callbacks).await
    } else {
        sinatuki(owner, admitted, h, &callbacks).await
    }
}

/// TibiaWiki Obsidian Knife Quest: the Sweaty Cyclops turns a Dragon Shield into
/// a Piece of Draconian Steel once Friends and Traders made him a friend. Uzgod's
/// forge (steel plus Obsidian Lance, 20 h cooldown) is the native NPC exchange
/// that `pick_exchange` already proves.
async fn obsidian_knife(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    callbacks: &[RewardCallback],
) -> TestResult {
    let [cyclops] = super::classic_quests_fixtures::install_positions(
        owner, admitted, [CYCLOPS], [cyclops_placement()?],
    ).await?;
    let steel = callback(callbacks, "qc-obsidian-draconian-steel")?;
    let words = vec!["hi", "uth'lokr", "yes"];
    mint(owner, admitted, 3416, 1, 7000).await?;
    // Confirmation alone, without the exchange topic, does not spend the shield.
    let (result, _, _) = super::qengine_rewards_fixtures::wire_conversation(
        owner, admitted, cyclops, steel, false, vec!["hi", "yes"],
    ).await?;
    assert_ne!(result, NpcIntentDisposition::Ok);
    assert_eq!(count(h, 3416).await?, 1);
    assert_eq!(count(h, 5889).await?, 0);
    let intro = callback(callbacks, "questpc7-friends-intro")?;
    let (result, _, _) = super::qengine_rewards_fixtures::wire_conversation(
        owner, admitted, cyclops, intro, false, vec!["hi", "forging friendship"],
    ).await?;
    assert_eq!(result, NpcIntentDisposition::Ok);
    let skirts = callback(callbacks, "questpc7-friends-skirts")?;
    for nonce in 7090..7093 {
        mint(owner, admitted, 3560, 1, nonce).await?;
    }
    let (result, _, _) = super::qengine_rewards_fixtures::wire_conversation(
        owner, admitted, cyclops, skirts, false, vec!["hi", "deliver skirts"],
    ).await?;
    assert_eq!(result, NpcIntentDisposition::Ok);
    assert_eq!(count(h, 3560).await?, 0);
    let (result, _, _) = super::qengine_rewards_fixtures::wire_conversation(
        owner, admitted, cyclops, steel, false, vec!["hi", "uth'lokr"],
    ).await?;
    assert_eq!(result, NpcIntentDisposition::Ok);
    assert_eq!(count(h, 3416).await?, 1, "the offer does not spend the shield");
    assert_eq!(count(h, 5889).await?, 0);
    for used in 1..=2 {
        if used > 1 {
            mint(owner, admitted, 3416, 1, 7000 + u64::try_from(used)?).await?;
        }
        let (result, _, _) = super::qengine_rewards_fixtures::wire_conversation(
            owner,
            admitted,
            cyclops,
            steel,
            false,
            words.clone(),
        )
        .await?;
        assert_eq!(result, NpcIntentDisposition::Ok);
        assert_eq!(count(h, 3416).await?, 0);
        assert_eq!(
            count(h, 5889).await?,
            used,
            "the steel service stays repeatable"
        );
    }
    Ok(())
}

/// TibiaWiki Sinatuki Quest: after the Barbarian Test and the Ice Islands
/// missions, 99 rainbow trout or green perch and then one more fish earn five
/// Shards and the Jamjam achievement.
async fn sinatuki(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    callbacks: &[RewardCallback],
) -> TestResult {
    let [sinatuki] =
        super::classic_quests_fixtures::install(owner, admitted, ["oteryn:npc.sinatuki"]).await?;
    let first = callback(callbacks, "qc-sinatuki-trout-99")?;
    let last = callback(callbacks, "qc-sinatuki-perch-last")?;
    let words = vec!["hi", "chuqua", "nupi"];
    mint(owner, admitted, 7158, 99, 8000).await?;
    super::qengine_rewards_fixtures::wire_conversation(
        owner,
        admitted,
        sinatuki,
        first,
        false,
        words.clone(),
    )
    .await?;
    assert_eq!(count(h, 7158).await?, 99, "the questlines gate the hand-in");
    super::qengine_outfit_stories_fixtures::seed_quest_prerequisites(
        h,
        owner,
        admitted,
        &[
            (
                "oteryn:quest-progress/quest/u8_0/barbarian_test/questline".into(),
                3,
            ),
            (
                "oteryn:quest-progress/quest/u8_0/the_ice_islands/questline".into(),
                5,
            ),
        ],
        91,
    )
    .await?;
    owner.refresh_quest_session(admitted.game_session_id, false).await;
    let (result, _, _) = super::qengine_rewards_fixtures::wire_conversation(
        owner,
        admitted,
        sinatuki,
        first,
        false,
        words.clone(),
    )
    .await?;
    assert_eq!(result, NpcIntentDisposition::Ok);
    assert_eq!(count(h, 7158).await?, 0);
    assert_eq!(count(h, 7290).await?, 0);
    assert!(!completed(h, &last.quest).await?);
    mint(owner, admitted, 7159, 1, 8001).await?;
    let (result, _, _) = super::qengine_rewards_fixtures::wire_conversation(
        owner,
        admitted,
        sinatuki,
        last,
        false,
        words.clone(),
    )
    .await?;
    assert_eq!(result, NpcIntentDisposition::Ok);
    assert_eq!(count(h, 7159).await?, 0);
    assert_eq!(count(h, 7290).await?, 5);
    assert!(completed(h, &last.quest).await?);
    let jamjam: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_account_achievements WHERE achievement_key='oteryn:achievement/jamjam'",
    )
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(jamjam, 1);
    // The reward is once only: a further fish is kept.
    mint(owner, admitted, 7158, 1, 8002).await?;
    super::qengine_rewards_fixtures::wire_conversation(owner, admitted, sinatuki, last, false, words)
        .await?;
    assert_eq!(count(h, 7158).await?, 1);
    assert_eq!(count(h, 7290).await?, 5);
    Ok(())
}
