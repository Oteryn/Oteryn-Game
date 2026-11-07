//! Crystal9f Rabaz mission/book/ordered samples; source witnesses live in authoring.
use super::*;

#[test]
fn questl2_botany_full_player_journey() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::super::fixture::run(async |admin| {
        let started = std::time::Instant::now();
        let h = Harness::create_quest_reward(admin, "l2_botany", true).await?;
        let version: String = sqlx::query_scalar("SELECT current_setting('server_version')")
            .fetch_one(&h.pool)
            .await?;
        eprintln!("POSTGRES_CASE_STARTED l2-botany-binding server_version={version}");
        let callbacks = oteryn_gameplay_quests::quest::world_callbacks::load()?;
        let present = ["l2-botany-book", "l2-botany-sample-1", "l2-botany-sample-2"]
            .iter()
            .all(|key| callbacks.iter().any(|c| c.key.ends_with(key)));
        h.cleanup().await?;
        eprintln!(
            "POSTGRES_BINDING_PROBE l2-botany duration={:?}",
            started.elapsed()
        );
        assert!(
            present,
            "Rabaz book and ordered plant sample bindings are absent"
        );
        Ok(())
    })?;
    scenario("qengine-rewards-l2-botany")
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    use super::questl2_journeys::{quantity, use_at, walk};
    use crate::map::view::MapFacts;
    use oteryn_protocol_oteryn::world_object::UseDisposition;
    use oteryn_protocol_oteryn::world_spatial::StepDirection::{East, North, South, West};
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_character_quest_tracks")
            .fetch_one(&h.pool)
            .await?,
        0
    );
    let mut selected = admitted
        .continuity
        .selected_capabilities
        .as_slice()
        .to_vec();
    selected.extend([3, 4, 6, 11, 15, 18]);
    selected.sort_unstable();
    selected.dedup();
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered, &selected)
            .ok_or("capabilities")?;
    let door = Position {
        x: 33007,
        y: 31536,
        floor: -10,
    };
    let book = Position {
        x: 33004,
        y: 31530,
        floor: -10,
    };
    let first = Position {
        x: 33003,
        y: 31529,
        floor: -10,
    };
    let second = Position {
        x: 33002,
        y: 31529,
        floor: -10,
    };
    use_at(owner, admitted, door, UseDisposition::Locked).await?;
    let [rabaz] =
        super::super::classic_quests::install_nearby(owner, admitted, ["oteryn:npc.rabaz"]).await?;
    let callbacks = oteryn_gameplay_quests::quest::reward_callbacks::load()?;
    let start = callbacks
        .iter()
        .find(|c| c.key.ends_with("/l2-botany-start"))
        .ok_or("Rabaz mission")?;
    let finish = callbacks
        .iter()
        .find(|c| c.key.ends_with("/l2-botany-return"))
        .ok_or("Rabaz hand-in")?;
    assert_eq!(
        wire_grant(owner, admitted, rabaz, start).await?.0,
        NpcIntentDisposition::Ok
    );
    assert_eq!(
        quantity(h, 11699).await?,
        0,
        "book is collected from the source chest"
    );
    use_at(owner, admitted, door, UseDisposition::Committed).await?;
    walk(owner, admitted, East).await?;
    for _ in 0..7 {
        walk(owner, admitted, North).await?;
    }
    for _ in 0..3 {
        walk(owner, admitted, West).await?;
    }
    use_at(owner, admitted, first, UseDisposition::Rejected).await?;
    use_at(owner, admitted, book, UseDisposition::Committed).await?;
    assert_eq!(quantity(h, 11699).await?, 1);
    use_at(owner, admitted, book, UseDisposition::Rejected).await?;
    use_at(owner, admitted, first, UseDisposition::Rejected).await?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "fence")?
        .ok_or("fence")?;
    super::super::quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &oteryn_gameplay_quests::quest::world_callbacks::Offering {
            position: book,
            item: "oteryn:item.tibia.i5908".into(),
            count: 1,
        },
        5300,
        true,
    )
    .await?;
    walk(owner, admitted, North).await?;
    walk(owner, admitted, West).await?;
    use_at(owner, admitted, second, UseDisposition::Rejected).await?;
    use_at(owner, admitted, first, UseDisposition::Committed).await?;
    use_at(owner, admitted, first, UseDisposition::Rejected).await?;
    use_at(owner, admitted, second, UseDisposition::Committed).await?;
    assert_eq!(quantity(h, 11699).await?, 1);
    assert_eq!(quantity(h, 5908).await?, 1);
    for _ in 0..4 {
        walk(owner, admitted, East).await?;
    }
    for _ in 0..7 {
        walk(owner, admitted, South).await?;
    }
    let map = owner.bundle_map().ok_or("map")?;
    let key =
        oteryn_world_bundle::bundle::placement_key(door.floor, door.x, door.y, 1).ok_or("door")?;
    let (_, facts) = map
        .facts
        .quest_callback_entry(map.overlay.base(), key)
        .ok_or("door facts")?;
    assert_eq!(
        facts.appearance_id, 7721,
        "native quest door closes on departure"
    );
    use_at(owner, admitted, door, UseDisposition::Committed).await?;
    walk(owner, admitted, South).await?;
    walk(owner, admitted, West).await?;
    let before: i64 =
        sqlx::query_scalar("SELECT total_experience FROM game_character_progression_state")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(
        wire_grant(owner, admitted, rabaz, finish).await?.0,
        NpcIntentDisposition::Ok
    );
    assert_eq!(quantity(h, 11699).await?, 0);
    assert_eq!(quantity(h, 5908).await?, 1);
    assert_eq!(quantity(h, 11700).await?, 1);
    assert_eq!(quantity(h, 3035).await?, 10);
    let after: i64 =
        sqlx::query_scalar("SELECT total_experience FROM game_character_progression_state")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(after - before, 3000);
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT value FROM game_character_quest_tracks WHERE track_key='oteryn:quest-progress/quest/u8_6/an_interest_in_botany/questline'").fetch_one(&h.pool).await?,4);
    // Both direct outputs belong to the single complete reservation/MINT transaction.
    let outputs:Vec<(String,String)>=sqlx::query_as("SELECT i.definition_production_key,a.created_xact_id::text FROM game_item_instances i JOIN game_item_audit_outbox a ON a.item_instance_id=i.item_instance_id AND a.transaction_id=i.minted_transaction_id WHERE i.lifecycle=1 AND i.definition_production_key IN ('oteryn:item.tibia.i11700','oteryn:item.tibia.i3035') ORDER BY i.definition_production_key").fetch_all(&h.pool).await?;
    assert_eq!(outputs.len(), 2);
    assert_eq!(
        outputs[0].1, outputs[1].1,
        "hat and 1000 gold commit in one PostgreSQL transaction"
    );
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    for (word, expected) in [
        ("hi", NpcIntentDisposition::Ok),
        // The finished mission has no further confirmation conversation.
        ("yes", NpcIntentDisposition::NoConversation),
    ] {
        let payload = oteryn_protocol_oteryn::npc_service::encode_npc_talk_intent(&NpcTalkIntent {
            npc_actor: rabaz,
            text: word.into(),
        })
        .map_err(debug)?;
        let command = admitted.continuity.next_command_id;
        let result =
            super::super::quest_world_callbacks::wire(owner, admitted, command, 7, payload).await?;
        assert_eq!(
            oteryn_protocol_oteryn::npc_service::decode_npc_intent_result(&result)
                .map_err(debug)?,
            expected
        );
    }
    assert_eq!(quantity(h, 11700).await?, 1);
    assert_eq!(quantity(h, 3035).await?, 10);
    assert!(
        owner
            .bundle_map()
            .ok_or("map")?
            .facts
            .movement_items(crate::map::overlay::TilePos {
                x: first.x,
                y: first.y,
                floor: first.floor
            })
            .is_empty(),
        "sampling retains the source plant"
    );
    Ok(())
}
