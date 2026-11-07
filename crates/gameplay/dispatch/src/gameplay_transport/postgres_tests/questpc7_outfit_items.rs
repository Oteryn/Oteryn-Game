//! Selected outfit routes use native NPC MINT, consumptive items and appearance facts.
use super::fixture::{Harness, debug, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;
use oteryn_protocol_oteryn::world_object::{
    UseDisposition, UseIntent, UseTarget, encode_item_use_intent,
};

fn run_case(name: &'static str) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let start = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run(name, || {
        super::quest_world_callbacks::scenario(
            name,
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    });
    eprintln!(
        "QUESTPC7_OUTFIT_ITEMS POSTGRES_CASE_COMPLETED case={name} duration={:?} success={}",
        start.elapsed(),
        result.is_ok()
    );
    result
}

#[test]
fn questpc7_fiend_personal_tokens_require_actual_native_addons() -> TestResult {
    run_case("questpc7-outfits-fiend")
}
#[test]
fn questpc7_inquisition_personal_tokens_require_actual_native_addons() -> TestResult {
    run_case("questpc7-outfits-inquisition")
}

pub(super) async fn talk(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    npc: oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
    sequence: &mut u64,
    words: &str,
) -> TestResult<NpcIntentDisposition> {
    let hi = *sequence;
    *sequence = sequence.checked_add(1).ok_or("NPC command overflow")?;
    super::classic_quests::talk(owner, admitted, npc, hi, "hi").await?;
    let command = *sequence;
    *sequence = sequence.checked_add(1).ok_or("NPC command overflow")?;
    super::qengine_rewards::talk(owner, admitted, npc, command, words).await
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
    let (next, disposition, _) = super::rust_lottery::exchange(
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
    let parent = &intro
        .requirements
        .as_ref()
        .ok_or("requirements")?
        .completed_quests[0];
    let [npc] = super::classic_quests::install(owner, admitted, [intro.npc.as_str()]).await?;
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
            &format!("{slug} commission")
        )
        .await?,
        NpcIntentDisposition::Rejected,
        "the prerequisite completion is authoritative"
    );
    eprintln!("QUESTPC7_OUTFIT_ITEMS parent refusal complete");
    // Only this prerequisite is a fenced fixture; the subject programme and all
    // inventory/appearance milestones start fresh and run through their real owners.
    super::questpc7_eligibility::complete_parent_key(owner, admitted, parent).await?;
    let parent_revision:String=sqlx::query_scalar("SELECT completed_character_revision::text FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2").bind(id(41).as_slice()).bind(parent).fetch_one(&h.pool).await?;
    assert_eq!(
        talk(owner, admitted, npc, &mut npc_sequence, "hi").await?,
        NpcIntentDisposition::Ok
    );
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
    let after:String=sqlx::query_scalar("SELECT completed_character_revision::text FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2").bind(id(41).as_slice()).bind(parent).fetch_one(&h.pool).await?;
    assert_eq!(after, parent_revision, "prerequisite remains read-only");
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
    let reply = super::quest_world_callbacks::wire(
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
