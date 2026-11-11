//! Shared outfit owner proofs through the production NPC dispatcher and PostgreSQL.
use super::TestResult;
use crate::gameplay_transport::connection::FreshAdmissionAuthority;

#[test]
fn engoutfit_battle_mage_atomic_outfit_and_addons() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-l2-battle-mage-story")
}

#[test]
fn engoutfit_beggar_atomic_outfit_and_addons() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-l6-beggar-female-story")
}

#[test]
fn engoutfit_assassin_atomic_outfit_and_addons() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-l7-assassin-story")
}

#[test]
fn engoutfit_deepling_atomic_addon() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-deepling")
}

#[test]
fn engoutfit_source_outfits_are_durable_and_replay_safe() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("engoutfit-source", || {
        super::quest_world_callbacks::scenario(
            "engoutfit-source",
            oteryn_gameplay_quests::quest::world_callbacks::Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    })?;
    assert!(started.elapsed().as_secs_f64() > 0.01);
    eprintln!(
        "POSTGRES_CASE_COMPLETED engoutfit-source duration={:?}",
        started.elapsed()
    );
    Ok(())
}

pub(super) async fn exercise_source(
    owner: &mut super::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut super::AdmittedSession,
    h: &super::fixture::Harness,
    authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    fault: bool,
) -> TestResult {
    use super::fixture::debug;
    use crate::domain::appearance::{AppearanceContent, Sex};
    use crate::durability::quest_state::{QuestTransitionOutcome, npc_program::NpcProgramRequest};
    use crate::foundation::{CommandId, CommandRef};
    use oteryn_gameplay_quests::quest::npc_rules::NpcRuleTrigger;
    const TRACK: &str = "oteryn:quest-progress/quest/u8_4/in_service_of_yalahar/questline";
    let [npc] = super::classic_quests::install(owner, admitted, ["oteryn:npc.palimuth"]).await?;
    // Establish the encounter prerequisite through the real fenced quest owner.
    let source =
        oteryn_gameplay_quests::quest::npc_rules::NpcRuleCatalog::runtime().map_err(debug)?;
    let program = source
        .program("oteryn:quest.in_service_of_yalahar_quest")
        .ok_or("Yalahar source")?;
    let canonical = owner.quest_catalogue.as_ref().ok_or("catalogue")?;
    let tracks = program
        .document
        .tracks
        .iter()
        .map(|track| {
            let mut native = canonical.track(&track.key).cloned().ok_or("source track")?;
            // This isolated fixture retains source absent values. The production
            // catalogue separately carries the published legacy-default metadata.
            native.initial = track.initial;
            native.min = native.min.min(track.initial);
            Ok(native)
        })
        .collect::<TestResult<Vec<_>>>()?;
    let pin = format!(
        "oteryn:quest-npc-pin/{}",
        program
            .digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    );
    let marker = canonical.transition(&pin).cloned().ok_or("source pin")?;
    let setup = "oteryn:quest-transition/engoutfit/precondition";
    let addon_setup = "oteryn:quest-transition/engoutfit/addon-precondition";
    owner.quest_catalogue = Some(std::sync::Arc::new(
        oteryn_gameplay_quests::quest::QuestStateCatalogue::new(
            canonical.content_revision(),
            tracks,
            vec![
                marker,
                oteryn_gameplay_quests::quest::QuestTransition {
                    key: setup.into(),
                    quest: "oteryn:quest.in_service_of_yalahar_quest".into(),
                    effects: vec![oteryn_gameplay_quests::quest::QuestEffect {
                        track: TRACK.into(),
                        from: oteryn_gameplay_quests::quest::QuestComparison::Any,
                        effect: oteryn_gameplay_quests::quest::QuestEffectKind::Set(52),
                    }],
                    completes: false,
                    experience: None,
                    world_change: None,
                },
            oteryn_gameplay_quests::quest::QuestTransition {
                key: addon_setup.into(), quest:"oteryn:quest.in_service_of_yalahar_quest".into(),
                effects: vec![
                    oteryn_gameplay_quests::quest::QuestEffect { track:TRACK.into(), from:oteryn_gameplay_quests::quest::QuestComparison::Any, effect:oteryn_gameplay_quests::quest::QuestEffectKind::Set(54) },
                    oteryn_gameplay_quests::quest::QuestEffect { track:"oteryn:quest-progress/quest/u8_4/in_service_of_yalahar/side_decision".into(), from:oteryn_gameplay_quests::quest::QuestComparison::Any, effect:oteryn_gameplay_quests::quest::QuestEffectKind::Set(1) },
                ], completes:false, experience:None, world_change:None,
            },
            ],
        )
        .map_err(debug)?,
    ));
    super::qengine_outfit_stories::commit_fixture(owner, admitted, setup, 2799).await?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    super::classic_quests::talk(owner, admitted, npc, 2800, "hi").await?;
    if fault {
        sqlx::raw_sql("CREATE SEQUENCE engoutfit_fault_hits; CREATE FUNCTION engoutfit_omit_receipt() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN PERFORM nextval('engoutfit_fault_hits'); RETURN NULL; END $$; CREATE TRIGGER engoutfit_omit_receipt BEFORE INSERT ON game_account_appearance_unlock_receipts FOR EACH ROW WHEN(NEW.unlock_key LIKE 'oteryn:outfit.yalaharian/quest-grant/%') EXECUTE FUNCTION engoutfit_omit_receipt();").execute(&h.pool).await?;
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(2))
            .ok_or("clock")?;
        let current = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?;
        let result = owner
            .npc_talk(
                admitted.runtime_actor.ok_or("actor")?,
                crate::gameplay_transport::connection::UseCommand {
                    game_session_id: admitted.game_session_id,
                    command_id: 2801,
                    item_fence: Some(current.item_fence()),
                },
                oteryn_protocol_oteryn::npc_service::NpcTalkIntent {
                    npc_actor: npc,
                    text: "mission please".into(),
                },
            )
            .await;
        assert!(
            result.unknown
                || result.disposition
                    == oteryn_protocol_oteryn::npc_service::NpcIntentDisposition::Rejected
        );
        let hits: i64 =
            sqlx::query_scalar("SELECT last_value FROM engoutfit_fault_hits WHERE is_called")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(
            hits, 1,
            "the production grant insert must have reached the fault"
        );
        let value: i64 = sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2").bind(fence.character_id.as_bytes().as_slice()).bind(TRACK).fetch_one(&h.pool).await?;
        assert_eq!(value, 52);
        let writes: (i64, i64, i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_account_appearance_unlocks WHERE unlock_key LIKE 'oteryn:outfit.yalaharian/quest-grant/%'),(SELECT count(*) FROM game_account_appearance_unlock_receipts WHERE unlock_key LIKE 'oteryn:outfit.yalaharian/quest-grant/%'),(SELECT count(*) FROM game_quest_npc_program_receipts)").fetch_one(&h.pool).await?;
        assert_eq!(writes, (0, 0, 0));
        let revision: String = sqlx::query_scalar("SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid").bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(
            revision,
            fence.expected_character_revision.get().to_string()
        );
        return Ok(());
    }
    super::classic_quests::talk(owner, admitted, npc, 2801, "mission please").await?;
    assert_eq!(
        super::classic_quests::progress(owner, admitted, authority, TRACK).await?,
        53
    );
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_account_appearance_unlocks WHERE unlock_key='oteryn:outfit.yalaharian/quest-grant/0'")
        .fetch_one(&h.pool).await?;
    assert_eq!(count, 1);
    let receipt = owner
        .root
        .reconcile_npc_program(
            authority,
            fence.character_id,
            NpcProgramRequest {
                command: CommandRef::new(
                    admitted.game_session_id,
                    CommandId::new(2801).map_err(debug)?,
                ),
                npc: "oteryn:npc.palimuth".into(),
                trigger: NpcRuleTrigger::Say,
                message: "mission please".into(),
                topic: 0,
                transition_key:
                    "oteryn:quest-npc/in_service_of_yalahar_quest/palimuth/conversation-244-27-1-1"
                        .into(),
            },
        )
        .await
        .map_err(debug)?
        .ok_or("source receipt")?;
    assert!(matches!(
        receipt.transition,
        QuestTransitionOutcome::AlreadyCommitted(_)
    ));
    assert!(receipt.magic_effects.is_empty());
    let current = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let content = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .and_then(|n| n.cosmetics())
        .ok_or("cosmetics")?;
    let state = owner
        .root
        .read_current_character_appearance_state(
            authority,
            owner.holder,
            current,
            content.earned_grant_keys(),
        )
        .await
        .map_err(debug)?;
    let snapshot =
        crate::durability::character_appearance::npc_outfits::snapshot(content, &state.earned);
    for sex in [Sex::Female, Sex::Male] {
        let look = content
            .look_type("oteryn:outfit.yalaharian", sex)
            .ok_or("Yalaharian look")?;
        assert!(snapshot.contains_key(&u16::try_from(look)?));
    }
    let after: i64 = sqlx::query_scalar("SELECT count(*) FROM game_account_appearance_unlocks WHERE unlock_key='oteryn:outfit.yalaharian/quest-grant/0'").fetch_one(&h.pool).await?;
    assert_eq!(count, after);
    super::qengine_outfit_stories::commit_fixture(owner, admitted, addon_setup, 2810).await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let mut bags = Vec::new();
    let mut occurrence = 28100;
    super::qengine_outfit_stories::supply(
        owner,
        admitted,
        h,
        &oteryn_gameplay_quests::quest::reward_callbacks::ItemAmount {
            item: "oteryn:item.tibia.i9041".into(),
            count: 1,
            fluid_subtype: None,
        },
        &mut bags,
        &mut occurrence,
    )
    .await?;
    super::classic_quests::talk(owner, admitted, npc, 2811, "job please").await?;
    super::classic_quests::talk(owner, admitted, npc, 2812, "yes please").await?;
    assert_eq!(
        super::classic_quests::progress(owner, admitted, authority, TRACK).await?,
        55
    );
    let current = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let state = owner
        .root
        .read_current_character_appearance_state(
            authority,
            owner.holder,
            current,
            content.earned_grant_keys(),
        )
        .await
        .map_err(debug)?;
    let snapshot =
        crate::durability::character_appearance::npc_outfits::snapshot(content, &state.earned);
    for sex in [Sex::Female, Sex::Male] {
        let look = u16::try_from(
            content
                .look_type("oteryn:outfit.yalaharian", sex)
                .ok_or("look")?,
        )?;
        assert_eq!(snapshot.get(&look).copied(), Some(1));
    }
    let charm: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i9041' AND lifecycle=1").fetch_one(&h.pool).await?;
    assert_eq!(
        charm, 1,
        "the source checks this charm without consuming it"
    );
    Ok(())
}

#[test]
fn engoutfit_source_outfits_rollback_when_insert_is_suppressed() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("engoutfit-source-fault", || {
        super::quest_world_callbacks::scenario(
            "engoutfit-source-fault",
            oteryn_gameplay_quests::quest::world_callbacks::Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    })?;
    assert!(started.elapsed().as_secs_f64() > 0.01);
    eprintln!(
        "POSTGRES_CASE_COMPLETED engoutfit-source-fault duration={:?}",
        started.elapsed()
    );
    Ok(())
}

#[test]
fn engoutfit_rock_steady_collection_commits_exact_handins_and_weapon() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("engoutfit-rock-steady", || {
        super::quest_world_callbacks::scenario(
            "engoutfit-rock-steady",
            oteryn_gameplay_quests::quest::world_callbacks::Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    })?;
    assert!(started.elapsed().as_secs_f64() > 0.01);
    eprintln!(
        "POSTGRES_CASE_COMPLETED engoutfit-rock-steady duration={:?}",
        started.elapsed()
    );
    Ok(())
}

pub(super) async fn exercise_rock(
    owner: &mut super::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut super::AdmittedSession,
    h: &super::fixture::Harness,
    authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    use super::fixture::debug;
    use crate::domain::appearance::{AppearanceContent, Sex};
    use crate::durability::quest_state::{QuestCause, QuestTransitionRequest};
    use crate::foundation::{CommandId, CommandRef};
    use oteryn_gameplay_quests::quest::reward_callbacks::ItemAmount;
    const TRACK: &str = "oteryn:quest-progress/rock_steady/collection";
    let [npc] = super::classic_quests::install(owner, admitted, ["oteryn:npc.rock_steady"]).await?;
    super::classic_quests::talk(owner, admitted, npc, 2900, "hi").await?;
    super::classic_quests::talk(owner, admitted, npc, 2901, "help").await?;
    assert_eq!(
        super::classic_quests::progress(owner, admitted, authority, TRACK).await?,
        1
    );
    super::classic_quests::talk(owner, admitted, npc, 2902, "collect").await?;
    super::classic_quests::talk(owner, admitted, npc, 2903, "yes").await?;
    assert_eq!(
        super::classic_quests::progress(owner, admitted, authority, TRACK).await?,
        2
    );
    let mut bags = Vec::new();
    let mut occurrence = 29000;
    for (index, item) in [14022, 14023, 14021].into_iter().enumerate() {
        let command = 2910 + u64::try_from(index)? * 10;
        let text = if index == 2 { "collect" } else { "yes" };
        if index != 2 {
            super::classic_quests::talk(owner, admitted, npc, command, "collect").await?;
        }
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(2))
            .ok_or("clock")?;
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?;
        let refused = owner
            .npc_talk(
                admitted.runtime_actor.ok_or("actor")?,
                crate::gameplay_transport::connection::UseCommand {
                    game_session_id: admitted.game_session_id,
                    command_id: command + 1,
                    item_fence: Some(fence.item_fence()),
                },
                oteryn_protocol_oteryn::npc_service::NpcTalkIntent {
                    npc_actor: npc,
                    text: text.into(),
                },
            )
            .await;
        assert_eq!(
            refused.disposition,
            oteryn_protocol_oteryn::npc_service::NpcIntentDisposition::Rejected
        );
        assert_eq!(
            super::classic_quests::progress(owner, admitted, authority, TRACK).await?,
            2 + i64::try_from(index)?
        );
        let grants: i64 = sqlx::query_scalar("SELECT count(*) FROM game_account_appearance_unlocks WHERE unlock_key LIKE 'oteryn:outfit.deepling/quest-grant/%'").fetch_one(&h.pool).await?;
        assert_eq!(grants, 0);
        super::qengine_outfit_stories::supply(
            owner,
            admitted,
            h,
            &ItemAmount {
                item: format!("oteryn:item.tibia.i{item}"),
                count: 1,
                fluid_subtype: None,
            },
            &mut bags,
            &mut occurrence,
        )
        .await?;
        if index != 2 {
            super::classic_quests::talk(owner, admitted, npc, command + 2, "collect").await?;
        }
        super::classic_quests::talk(owner, admitted, npc, command + 3, text).await?;
        assert_eq!(
            super::classic_quests::progress(owner, admitted, authority, TRACK).await?,
            3 + i64::try_from(index)?
        );
        let left: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_instances WHERE lifecycle=1 AND definition_production_key=$1").bind(format!("oteryn:item.tibia.i{item}")).fetch_one(&h.pool).await?;
        assert_eq!(left, 0);
    }
    let grants: (i64, i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_account_appearance_unlocks WHERE unlock_key IN ('oteryn:outfit.deepling/quest-grant/0','oteryn:outfit.deepling/quest-grant/1')),(SELECT count(*) FROM game_account_appearance_unlock_receipts WHERE unlock_key IN ('oteryn:outfit.deepling/quest-grant/0','oteryn:outfit.deepling/quest-grant/1'))").fetch_one(&h.pool).await?;
    assert_eq!(grants, (2, 2));
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    assert!(
        owner
            .root
            .reconcile_character_quest_transition(
                authority,
                fence.character_id,
                QuestTransitionRequest {
                    transition_key: "oteryn:quest-reward/m36-rock-steady-weapon".into(),
                    cause: QuestCause::Command(CommandRef::new(
                        admitted.game_session_id,
                        CommandId::new(2933).map_err(debug)?
                    )),
                }
            )
            .await
            .map_err(debug)?
            .is_some()
    );
    let content = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .and_then(|n| n.cosmetics())
        .ok_or("cosmetics")?;
    let state = owner
        .root
        .read_current_character_appearance_state(
            authority,
            owner.holder,
            fence,
            content.earned_grant_keys(),
        )
        .await
        .map_err(debug)?;
    let snapshot =
        crate::durability::character_appearance::npc_outfits::snapshot(content, &state.earned);
    for sex in [Sex::Female, Sex::Male] {
        let look = u16::try_from(
            content
                .look_type("oteryn:outfit.deepling", sex)
                .ok_or("Deepling look")?,
        )?;
        assert_eq!(snapshot.get(&look).copied(), Some(1));
    }
    Ok(())
}
