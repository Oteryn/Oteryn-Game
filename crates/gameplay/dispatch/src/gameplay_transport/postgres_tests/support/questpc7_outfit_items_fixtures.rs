use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use super::fixture::{Harness, debug, id};

use super::{AdmittedSession, ComposedFreshAdmission, TestResult};

use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;

use oteryn_protocol_oteryn::world_object::{
    UseDisposition, UseIntent, UseTarget, encode_item_use_intent,
};

pub(super) async fn talk(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    npc: oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
    sequence: &mut u64,
    words: &str,
) -> TestResult<NpcIntentDisposition> {
    let hi = *sequence;
    *sequence = sequence.checked_add(1).ok_or("NPC command overflow")?;
    super::classic_quests_fixtures::talk(owner, admitted, npc, hi, "hi").await?;
    let command = *sequence;
    *sequence = sequence.checked_add(1).ok_or("NPC command overflow")?;
    super::qengine_rewards_fixtures::talk(owner, admitted, npc, command, words).await
}

pub(super) async fn quantity(h: &Harness, item: u32) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key=$1")
        .bind(format!("oteryn:item.tibia.i{item}")).fetch_one(&h.pool).await?)
}

pub(super) async fn use_token(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    item: u32,
) -> TestResult {
    let reference = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .and_then(|n| n.item_index())
        .and_then(|index| {
            index.definition_ref(
                "Item",
                &format!("oteryn:item.tibia.i{item}"),
                "definition-r1",
            )
        })
        .ok_or("native token reference")?;
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(std::num::NonZeroU64::new(1).ok_or("handle")?),
        use_with: None,
        use_with_map_item: None,
        use_with_position: None,
    })
    .map_err(debug)?;
    let (next, disposition, _) = super::rust_lottery_fixtures::exchange(
        owner,
        *admitted,
        admitted.continuity.next_command_id,
        &payload,
        Some(reference),
        None,
    )
    .await?;
    *admitted = next;
    assert_eq!(
        disposition,
        UseDisposition::Committed,
        "native consumptive item {item}"
    );
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    name: &str,
) -> TestResult {
    let version: String = sqlx::query_scalar("SHOW server_version")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("QUESTPC7_OUTFIT_ITEMS POSTGRES_STARTED version={version} case={name}");
    let (slug, outfit, items, achievement) = if name.ends_with("fiend") {
        (
            "fiend",
            "fiend_slayer",
            [50060, 50061],
            "oteryn:achievement/fiend_slayer",
        )
    } else {
        (
            "inquisition",
            "hand_of_the_inquisition",
            [31738, 31737],
            "oteryn:achievement/inquisition_s_arm",
        )
    };
    let callbacks = oteryn_gameplay_quests::quest::reward_callbacks::load()?;
    let intro = callbacks
        .iter()
        .find(|c| c.key == format!("oteryn:quest-reward/questpc7-{slug}-commission"))
        .ok_or("intro")?;
    let intro_word = intro.keywords.first().ok_or("report keyword")?;
    let fiend = slug == "fiend";
    let parent = if fiend {
        "oteryn:quest.no_rest_for_the_wicked_quest"
    } else {
        "oteryn:quest.grave_danger_quest"
    };
    let parent_revision = completion_revision(h, parent).await?;
    let [npc] = if fiend {
        super::classic_quests_fixtures::install_above(owner, admitted, [intro.npc.as_str()]).await?
    } else {
        super::classic_quests_fixtures::install(owner, admitted, [intro.npc.as_str()]).await?
    };
    let mut npc_sequence = 2000_u64;
    let mut selected = admitted
        .continuity
        .selected_capabilities
        .as_slice()
        .to_vec();
    selected.extend([3, 4, 6, 11, 15, 18, 25]);
    selected.sort_unstable();
    selected.dedup();
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered, &selected)
            .ok_or("capabilities")?;
    assert_eq!(
        talk(owner, admitted, npc, &mut npc_sequence, "hi").await?,
        NpcIntentDisposition::Ok
    );
    assert_eq!(
        talk(
            owner,
            admitted,
            npc,
            &mut npc_sequence,
            intro_word
        )
        .await?,
        NpcIntentDisposition::Rejected,
        "commission requires its prerequisite"
    );
    eprintln!("QUESTPC7_OUTFIT_ITEMS prerequisite refusal complete");
    if fiend {
        super::quest_container_chests_fixtures::forgemaster::claim_for_fiend(h, owner, admitted)
            .await?;
    } else {
        super::qengine_outfit_stories_fixtures::seed_quest_prerequisites(
            h, owner, admitted,
            &[(oteryn_gameplay_quests::quest::grave_danger::QUESTLINE.into(), 4)], 233,
        ).await?;
        owner.refresh_quest_session(admitted.game_session_id, false).await;
    }
    assert_eq!(
        talk(owner, admitted, npc, &mut npc_sequence, "hi").await?,
        NpcIntentDisposition::Ok
    );
    if fiend {
    for words in [format!("{slug} commission"), format!("{slug} first token")] {
        eprintln!("QUESTPC7_OUTFIT_ITEMS requesting {words}");
        assert_eq!(
            talk(owner, admitted, npc, &mut npc_sequence, &words).await?,
            NpcIntentDisposition::Ok
        );
    }
    assert_eq!(quantity(h, items[0]).await?, 1);
    assert_eq!(
        talk(
            owner,
            admitted,
            npc,
            &mut npc_sequence,
            &format!("{slug} second token")
        )
        .await?,
        NpcIntentDisposition::Rejected,
        "owning a token or advancing acquisition is not an earned cosmetic"
    );
    assert_eq!(quantity(h, items[1]).await?, 0);
    use_token(owner, admitted, items[0]).await?;
    assert_eq!(quantity(h, items[0]).await?, 0);
    assert_eq!(
        talk(
            owner,
            admitted,
            npc,
            &mut npc_sequence,
            &format!("{slug} second token")
        )
        .await?,
        NpcIntentDisposition::Ok
    );
    } else {
        assert_eq!(talk(owner, admitted, npc, &mut npc_sequence, intro_word).await?, NpcIntentDisposition::Ok);
        for (index, item) in items.into_iter().enumerate() {
            // Inventory setup for this USE test. Native boss loot admission has its own loader proof.
            let fence = owner.current_quest_fence(admitted.game_session_id).await.map_err(|()| "fence")?.ok_or("fence")?;
            super::quest_world_callbacks_fixtures::input_item(owner, fence.item_fence(),
                &oteryn_gameplay_quests::quest::world_callbacks::Offering {
                    position: oteryn_gameplay_quests::quest::world_callbacks::Position { x: 33293, y: 31677, floor: -15 },
                    item: format!("oteryn:item.tibia.i{item}"),
                    count: 1,
                }, 970 + u64::try_from(index)?, true).await?;
        }
        assert_eq!(quantity(h, items[0]).await?, 1);
        use_token(owner, admitted, items[0]).await?;
        assert_eq!(quantity(h, items[0]).await?, 0);
    }
    assert_eq!(quantity(h, items[1]).await?, 1);
    assert_eq!(
        talk(
            owner,
            admitted,
            npc,
            &mut npc_sequence,
            &format!("{slug} complete")
        )
        .await?,
        NpcIntentDisposition::Rejected,
        "the final token must actually be consumed"
    );
    use_token(owner, admitted, items[1]).await?;
    assert_eq!(quantity(h, items[1]).await?, 0);
    assert_eq!(
        talk(
            owner,
            admitted,
            npc,
            &mut npc_sequence,
            &format!("{slug} complete")
        )
        .await?,
        NpcIntentDisposition::Ok
    );
    let completed:bool=sqlx::query_scalar("SELECT completed_character_revision IS NOT NULL FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2").bind(id(41).as_slice()).bind(&intro.quest).fetch_one(&h.pool).await?;
    assert!(completed);
    assert_eq!(
        completion_revision(h, parent).await?,
        parent_revision,
        "the outfit programme does not complete its prerequisite quest"
    );
    if fiend {
        assert_eq!(parent_revision, None, "the Fiend journey starts uncompleted");
    }
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let observed = owner
        .appearance_observation(actor, admitted.game_session_id, vec![])
        .await
        .ok_or("appearance")?;
    let outfit_key = format!("oteryn:outfit.{outfit}");
    assert_eq!(
        observed
            .catalogue
            .outfits
            .iter()
            .find(|o| o.key == outfit_key)
            .ok_or("earned outfit")?
            .permitted_addons,
        3
    );
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_account_achievements WHERE achievement_key=$1",
    )
    .bind(achievement)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(
        count, 1,
        "source consumptive owner grants the selected full-outfit achievement once"
    );
    let change = oteryn_protocol_oteryn::appearance::Change {
        context: observed.catalogue.context,
        expected_root_revision: observed.catalogue.root_revision,
        prior: observed.catalogue.selected,
        selection: oteryn_protocol_oteryn::appearance::Selection {
            outfit_key,
            colours: [0; 4],
            addons: 3,
            mount_key: None,
        },
        content_revision: owner
            .appearance_context()
            .ok_or("cosmetics")?
            .1
            .content_revision()
            .into(),
    };
    let next = admitted.continuity.next_command_id;
    let reply = super::quest_world_callbacks_fixtures::wire(
        owner,
        admitted,
        next,
        25,
        change.encode().map_err(debug)?,
    )
    .await?;
    assert!(matches!(
        oteryn_protocol_oteryn::appearance::ChangeResult::decode(&reply).map_err(debug)?,
        oteryn_protocol_oteryn::appearance::ChangeResult::Committed(_)
    ));
    Ok(())
}

async fn completion_revision(h: &Harness, quest: &str) -> TestResult<Option<String>> {
    Ok(sqlx::query_scalar(
        "SELECT completed_character_revision::text FROM game_character_quest_states
         WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2",
    )
    .bind(id(41).as_slice())
    .bind(quest)
    .fetch_optional(&h.pool)
    .await?
    .flatten())
}
