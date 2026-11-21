use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use super::fixture::{Harness, id};

use super::{AdmittedSession, ComposedFreshAdmission, TestResult};

use crate::durability::character_authority::ReconciledCharacterAuthority;

use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};

use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;

use sha2::{Digest, Sha384};

const TRACK: &str = "oteryn:quest-progress/questpc7/friends/program";

pub(super) const LEGACY_CASE: &str = "questpc7-friends-legacy";

pub(super) async fn exercise_legacy(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    trigger: Position,
) -> TestResult {
    const QUEST: &str = "oteryn:quest.friends_and_traders";
    const QUENTIN: &str = "oteryn:npc.quentin";
    super::qc_pinned_world::seed_saved_progress(
        h,
        owner,
        admitted,
        QUEST,
        "0a665893aa6b602883c71bfee786a71e86b2b9d3fdbffedf0ce619f17156d223",
        TRACK,
        14,
    )
    .await?;
    let (bytes, pin) = super::qc_pinned_world::artifact()?;
    let bundle = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    assert_eq!(
        Some(oteryn_world_bundle::npc::hex(&bundle.digest).as_str()),
        pin["digest"].as_str()
    );
    let placement = bundle
        .npcs
        .npcs
        .iter()
        .find(|npc| npc.key == QUENTIN)
        .and_then(|npc| {
            npc.placements
                .iter()
                .find(|p| (p.x, p.y, p.floor) == (trigger.x, trigger.y, trigger.floor))
        })
        .ok_or("pinned Quentin placement")?;
    let quentin =
        super::classic_quests_fixtures::place_pinned_contact(owner, admitted, QUENTIN, *placement).await?;
    super::classic_quests_fixtures::talk(owner, admitted, quentin, 5_000, "hi").await?;
    let disposition =
        super::qengine_rewards_fixtures::talk(owner, admitted, quentin, 5_001, "complete friendships")
            .await?;
    assert_eq!(disposition, NpcIntentDisposition::Ok);
    let state: (i64, bool) = sqlx::query_as(
        "SELECT t.value, s.completed_character_revision IS NOT NULL FROM game_character_quest_tracks t JOIN game_character_quest_states s USING(character_id,quest_key) WHERE t.track_key=$1",
    ).bind(TRACK).fetch_one(&h.pool).await?;
    assert_eq!(state, (15, true));
    Ok(())
}

async fn count(h: &Harness, item: u32) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce(sum(i.quantity),0)::bigint FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) WHERE e.character_id=encode($1,'hex')::uuid AND i.lifecycle=1 AND i.definition_production_key=$2")
        .bind(id(41).as_slice()).bind(format!("oteryn:item.tibia.i{item}")).fetch_one(&h.pool).await?)
}

async fn receipts(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?)
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
    super::quest_world_callbacks_fixtures::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: Position {
                x: 33293,
                y: 31677,
                floor: -15,
            },
            item: format!("oteryn:item.tibia.i{item}"),
            count: amount,
        },
        nonce,
        true,
    )
    .await
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    let version: String = sqlx::query_scalar("SHOW server_version")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("QUESTPC7_FRIENDS POSTGRES_STARTED version={version}");
    let applied: Vec<u8> =
        sqlx::query_scalar("SELECT checksum FROM _sqlx_migrations WHERE version=1286 AND success")
            .fetch_one(&h.pool)
            .await?;
    let source = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../apps/game-server/migrations/1286_quest_repeatable_exchanges.sql"
    ))?;
    let expected = Sha384::digest(source);
    assert_eq!(
        applied.as_slice(),
        &expected[..],
        "PG must execute the final migration bytes, including the exact counter-source proof"
    );
    eprintln!("QUESTPC7_FRIENDS MIGRATION_1286_SOURCE_MATCHED");

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
    assert!(admitted.continuity.selected_capabilities.contains(3));
    assert!(admitted.continuity.selected_capabilities.contains(6));
    assert!(admitted.continuity.selected_capabilities.contains(25));
    let callbacks = oteryn_gameplay_quests::quest::reward_callbacks::load()?;
    let actors = super::classic_quests_fixtures::install(
        owner,
        admitted,
        [
            "oteryn:npc.a_sweaty_cyclops",
            "oteryn:npc.marina",
            "oteryn:npc.gamon",
            "oteryn:npc.quentin",
            "oteryn:npc.tibra",
            "oteryn:npc.maealil",
            "oteryn:npc.yberius",
            "oteryn:npc.isimov",
            "oteryn:npc.amanda",
            "oteryn:npc.kasmir",
            "oteryn:npc.rahkem",
            "oteryn:npc.brewster",
            "oteryn:npc.tyrias",
        ],
    )
    .await?;
    let cyclops = actors[0];
    let marina = actors[1];
    let gamon = actors[2];
    let quentin = actors[3];
    let tibra = actors[4];
    super::classic_quests_fixtures::talk(owner, admitted, cyclops, 1000, "hi").await?;
    super::classic_quests_fixtures::talk(owner, admitted, cyclops, 1001, "forging friendship").await?;
    assert_eq!(
        super::classic_quests_fixtures::progress(owner, admitted, authority, TRACK).await?,
        0
    );
    assert_eq!(
        super::qengine_rewards_fixtures::talk(owner, admitted, cyclops, 1002, "deliver skirts").await?,
        NpcIntentDisposition::Rejected
    );
    for i in 0..3 {
        mint(owner, admitted, 3560, 1, 2000 + i).await?;
    }
    super::classic_quests_fixtures::talk(owner, admitted, cyclops, 1003, "deliver skirts").await?;
    assert_eq!(count(h, 3560).await?, 0);
    assert_eq!(
        super::classic_quests_fixtures::progress(owner, admitted, authority, TRACK).await?,
        1
    );
    let steel = callbacks
        .iter()
        .find(|c| c.key.ends_with("questpc7-friends-royal-steel"))
        .ok_or("steel callback")?;
    for used in 1..=2 {
        mint(owner, admitted, 3381, 1, 2010 + u64::try_from(used)?).await?;
        let (result, _, _) =
            super::qengine_rewards_fixtures::wire_grant(owner, admitted, cyclops, steel).await?;
        assert_eq!(
            result,
            NpcIntentDisposition::Ok,
            "an unlocked service must commit each exact exchange"
        );
        assert_eq!(count(h, 5887).await?, used);
        assert_eq!(
            super::classic_quests_fixtures::progress(
                owner,
                admitted,
                authority,
                "oteryn:quest-progress/questpc7/friends/steel-uses"
            )
            .await?,
            used
        );
    }
    assert_eq!(count(h, 3381).await?, 0);
    super::classic_quests_fixtures::talk(owner, admitted, marina, 1010, "hi").await?;
    mint(owner, admitted, 5902, 50, 2020).await?;
    super::classic_quests_fixtures::talk(owner, admitted, marina, 1011, "deliver honey").await?;
    assert_eq!(count(h, 5902).await?, 0);
    mint(owner, admitted, 5879, 20, 2021).await?;
    let yarn = callbacks
        .iter()
        .find(|c| c.key.ends_with("questpc7-friends-yarn"))
        .ok_or("yarn callback")?;
    for used in 1..=2 {
        let (result, _, _) =
            super::qengine_rewards_fixtures::wire_grant(owner, admitted, marina, yarn).await?;
        assert_eq!(result, NpcIntentDisposition::Ok);
        assert_eq!(count(h, 5886).await?, used);
        assert_eq!(
            super::classic_quests_fixtures::progress(
                owner,
                admitted,
                authority,
                "oteryn:quest-progress/questpc7/friends/yarn-uses"
            )
            .await?,
            used
        );
    }
    assert_eq!(count(h, 5879).await?, 0);
    mint(owner, admitted, 3043, 1, 2030).await?;
    let stake = callbacks
        .iter()
        .find(|c| c.key.ends_with("questpc7-friends-stake"))
        .ok_or("stake callback")?;
    let (result, _, _) = super::qengine_rewards_fixtures::wire_grant(owner, admitted, gamon, stake).await?;
    assert_eq!(result, NpcIntentDisposition::Ok);
    assert_eq!(count(h, 5941).await?, 1);
    let fees:i64=sqlx::query_scalar("SELECT coalesce(sum(fee_gold_units),0)::bigint FROM game_quest_reward_fees WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(fees, 5000, "Gamon's source price is paid exactly once");
    super::classic_quests_fixtures::talk(owner, admitted, tibra, 1020, "hi").await?;
    let before_unavailable_prayer = receipts(h).await?;
    assert_eq!(
        super::qengine_rewards_fixtures::talk(owner, admitted, tibra, 1021, "stake prayer").await?,
        NpcIntentDisposition::Ok,
        "ordinary NPC chat acknowledges an unavailable prayer without a quest action"
    );
    assert_eq!(
        super::classic_quests_fixtures::progress(owner, admitted, authority, TRACK).await?,
        3,
        "the selected priest order cannot be skipped"
    );
    assert_eq!(count(h, 5941).await?, 1);
    assert_eq!(receipts(h).await?, before_unavailable_prayer);
    for (index, _npc) in [
        "quentin", "tibra", "maealil", "yberius", "isimov", "amanda", "kasmir", "rahkem",
        "brewster", "tyrias",
    ]
    .into_iter()
    .enumerate()
    {
        let actor = actors[index + 3];
        let command = 1100 + u64::try_from(index)? * 2;
        super::classic_quests_fixtures::talk(owner, admitted, actor, command, "hi").await?;
        super::classic_quests_fixtures::talk(owner, admitted, actor, command + 1, "stake prayer").await?;
        assert_eq!(
            super::classic_quests_fixtures::progress(owner, admitted, authority, TRACK).await?,
            4 + i64::try_from(index)?
        );
        assert_eq!(
            count(h, 5941).await?,
            1,
            "each prayer retains the same carried stake"
        );
    }
    super::classic_quests_fixtures::talk(owner, admitted, quentin, 1200, "hi").await?;
    super::classic_quests_fixtures::talk(owner, admitted, quentin, 1201, "finish prayer").await?;
    assert_eq!(count(h, 5941).await?, 0);
    assert_eq!(count(h, 5942).await?, 1);
    // TibiaWiki "Blessed!": finishing The Blessed Stake unlocks the achievement.
    let blessed: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_account_achievements WHERE achievement_key='oteryn:achievement/blessed'",
    )
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(blessed, 1);
    let before_repeat_blessing = receipts(h).await?;
    assert_eq!(
        super::qengine_rewards_fixtures::talk(owner, admitted, quentin, 1202, "finish prayer").await?,
        NpcIntentDisposition::Ok
    );
    assert_eq!(
        super::classic_quests_fixtures::progress(owner, admitted, authority, TRACK).await?,
        14
    );
    assert_eq!(count(h, 5941).await?, 0);
    assert_eq!(receipts(h).await?, before_repeat_blessing);
    assert_eq!(count(h, 5942).await?, 1);
    super::classic_quests_fixtures::talk(owner, admitted, quentin, 1203, "complete friendships").await?;
    assert_eq!(
        super::classic_quests_fixtures::progress(owner, admitted, authority, TRACK).await?,
        15
    );
    let completed:bool=sqlx::query_scalar("SELECT completed_character_revision IS NOT NULL FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key='oteryn:quest.friends_and_traders'").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert!(completed);
    mint(owner, admitted, 3381, 1, 2040).await?;
    let (result, _, _) =
        super::qengine_rewards_fixtures::wire_grant(owner, admitted, cyclops, steel).await?;
    assert_eq!(
        result,
        NpcIntentDisposition::Ok,
        "the earned crafting service remains usable after quest completion"
    );
    assert_eq!(count(h, 5887).await?, 3);
    Ok(())
}
