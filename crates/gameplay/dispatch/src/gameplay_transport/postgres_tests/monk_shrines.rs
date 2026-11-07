//! The production callback content, real USE protocol, and the durable quest owner.
use super::fixture::{Harness, debug};
use super::quest_world_callbacks::{local, wire};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::{
    monk_shrines,
    world_callbacks::{self, Position},
};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

#[path = "monk_shrines/npc.rs"]
mod npc;
#[path = "monk_shrines/rewards.rs"]
mod rewards;

#[test]
fn monk_shrine_selectors_match_the_real_world_source_tiles() -> TestResult {
    let root = std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../content/world/placements"
    ));
    let index: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("index.json"))?)?;
    let palette = index["palette"].as_array().ok_or("palette")?;
    for callback in world_callbacks::load()?
        .iter()
        .filter(|c| c.quest == monk_shrines::QUEST)
    {
        let p = callback.selector.position.ok_or("position")?;
        let name = format!(
            "region-z{:02}-x{:03}-y{:03}.b3",
            -p.floor,
            p.x / 256,
            p.y / 256
        );
        let mut budget = oteryn_world_bundle::sector::Budget {
            tiles: 65536,
            entries: 65536 * 4096,
        };
        let region = oteryn_world_bundle_compiler::b3::decode_region(
            &std::fs::read(root.join(name))?,
            oteryn_world_bundle::sector::TileLimits {
                max_entries: 4096,
                max_text_bytes: 65536,
            },
            &mut budget,
        )?;
        let tile = region
            .sectors
            .iter()
            .flat_map(|(_, _, tiles)| tiles)
            .find(|t| t.x == p.x && t.y == p.y)
            .ok_or("source tile")?;
        assert_eq!(tile.house, 0, "{}", callback.key);
        assert!(
            tile.items.iter().any(|item| item.depth == 0
                && palette
                    .get(item.palette as usize)
                    .and_then(|row| row["source_item_id"].as_u64())
                    == callback.selector.appearance.map(u64::from)),
            "{} must resolve the actual source shrine, not an empty/wall tile",
            callback.key
        );
    }
    Ok(())
}

#[test]
fn monk_shrines_real_use_persists_once_and_checks_each_level() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("monk-shrines", || {
        super::quest_world_callbacks::scenario(
            "monk-guidance",
            Position {
                x: 32454,
                y: 32292,
                floor: -7,
            },
            false,
        )
    })
}

#[test]
fn monk_shrine_full_backpack_keeps_entitlement_until_retry() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("monk-full-backpack", || {
        super::quest_world_callbacks::scenario(
            "monk-guidance-full-backpack",
            Position {
                x: 32454,
                y: 32292,
                floor: -7,
            },
            false,
        )
    })
}

#[test]
fn monk_shrine_capacity_keeps_entitlement_until_retry() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("monk-capacity", || {
        super::quest_world_callbacks::scenario(
            "monk-guidance-capacity",
            Position {
                x: 32454,
                y: 32292,
                floor: -7,
            },
            false,
        )
    })
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    world: &crate::map::boot::BundleWorld,
    full_backpack: bool,
    capacity: bool,
) -> TestResult {
    use crate::gameplay_transport::connection::FreshAdmissionAuthority;
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[4, 6, 11, 15, 18],
        )
        .ok_or("USE capability selection")?;
    assert!(admitted.continuity.selected_capabilities.contains(11));
    use crate::durability::character_build::{
        BuildCause, BuildChangeRequest, BuildOccurrence, DurableBuildState, convert_vocation,
    };
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let formula = native.training_formula().ok_or("training formula")?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    owner
        .revision_sequencer
        .acquire(fence.character_id)
        .await
        .commit_build(
            owner.root,
            authority,
            owner.holder,
            fence,
            BuildChangeRequest {
                occurrence: BuildOccurrence::from_bytes(super::fixture::id(92)).map_err(debug)?,
                cause: BuildCause::VocationChoice,
                before: DurableBuildState::default(),
                after: convert_vocation(formula, &DurableBuildState::default(), "monk")
                    .map_err(debug)?,
                pruned_stance: None,
            },
            formula,
            None,
        )
        .await
        .map_err(debug)?;
    let (manop, pema, return_position) = npc::install(owner, admitted).await?;
    npc::talk(owner, admitted, manop, 1000, "hi").await?;
    npc::talk(owner, admitted, manop, 1001, "pilgrimage").await?;
    npc::talk(owner, admitted, pema, 1002, "hi").await?;
    let start = npc::talk(owner, admitted, pema, 1003, "pilgrimage").await?;
    assert!(start.contains("begins"));
    let begun = owner
        .root
        .read_character_quest_state(authority, fence.character_id)
        .await
        .map_err(debug)?;
    npc::journal(&begun, 0, false)?;
    if full_backpack {
        rewards::fill(owner, admitted, authority).await?;
    }
    let callbacks = world_callbacks::load()?;
    for (index, name) in monk_shrines::NAMES.iter().enumerate() {
        let callback = callbacks
            .iter()
            .find(|c| c.key == format!("oteryn:quest-callback/monk-{name}"))
            .ok_or("shrine")?;
        let p = callback.selector.position.ok_or("position")?;
        let key =
            oteryn_world_bundle::bundle::placement_key(p.floor, p.x, p.y, 1).ok_or("placement")?;
        let mut placement = world.base().digest().to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        {
            // Travel between distant shrine fixtures through the movement owner. Each
            // completion below still requires an actual client USE at the source position.
            let mut runtime = owner.runtime.lock().await;
            let actor = admitted.runtime_actor.ok_or("actor")?;
            let expected = runtime.read_actor_position(actor).map_err(debug)?;
            let doors = crate::map::floor::BundleDoors(world.facts().doors());
            let proof = crate::map::floor::prepare_quest_callback_teleport(
                &runtime,
                owner.movement_cells,
                actor,
                admitted.game_session_id,
                expected,
                local(Position { x: p.x - 1, ..p }),
                false,
                &doors,
            )
            .map_err(debug)?;
            runtime.commit_source_step(proof).map_err(debug)?;
        }
        let minimum = i64::from(callback.minimum_level.ok_or("level")?);
        if index == 0 {
            use crate::durability::quest_state::{QuestCause, QuestTransitionRequest};
            let fence = owner
                .current_quest_fence(admitted.game_session_id)
                .await
                .map_err(|()| "fence")?
                .ok_or("fence")?;
            let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
            let bypass = slot
                .commit_quest_transition(
                    owner.root,
                    owner.character,
                    owner.holder,
                    fence,
                    QuestTransitionRequest {
                        transition_key: callback.key.clone(),
                        cause: QuestCause::Use(crate::foundation::CommandRef::new(
                            admitted.game_session_id,
                            crate::foundation::CommandId::new(999).map_err(debug)?,
                        )),
                    },
                    owner.quest_catalogue.clone().ok_or("catalogue")?,
                )
                .await;
            assert!(matches!(bypass, Err(crate::durability::character_progression::CharacterProgressionError::InvalidInput)),
                "the generic transition writer cannot bypass the callback's level gate");
        }
        for (case, (level, expected)) in [
            (minimum - 1, UseDisposition::Rejected),
            (minimum, UseDisposition::Committed),
            (minimum, UseDisposition::Rejected),
        ]
        .into_iter()
        .enumerate()
        {
            use crate::durability::character_progression::{
                ExperienceAwardRequest, ExperienceCommitOutcome, ExperienceRewardOccurrence,
            };
            let binding = owner
                .progression_binding(admitted.game_session_id)
                .ok_or("progression")?;
            let experience = binding
                .policy
                .thresholds
                .iter()
                .find(|threshold| i64::from(threshold.level) == level)
                .ok_or("level threshold")?
                .minimum_experience
                .get();
            let fence = owner
                .current_quest_fence(admitted.game_session_id)
                .await
                .map_err(|()| "fence")?
                .ok_or("fence")?;
            let current = owner
                .root
                .read_character_progression(authority, fence.character_id)
                .await
                .map_err(debug)?
                .ok_or("progression state")?;
            let amount = experience
                .checked_sub(current.total_experience.get())
                .ok_or("experience")?;
            if amount > 0 {
                let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
                let outcome = slot
                    .commit_experience(
                        owner.root,
                        authority,
                        owner.holder,
                        fence,
                        ExperienceAwardRequest {
                            occurrence: ExperienceRewardOccurrence::from_bytes(super::fixture::id(
                                u8::try_from(index * 3 + case + 100)?,
                            ))
                            .map_err(debug)?,
                            amount: oteryn_simulation_determinism::ExactI64::new(amount),
                            context: binding.context.clone(),
                            policy_revision: binding.policy_revision.clone(),
                            reward_revision: binding.reward_revision.clone(),
                            policy: binding.policy.clone(),
                        },
                        None,
                    )
                    .await
                    .map_err(debug)?;
                assert!(
                    matches!(outcome, ExperienceCommitOutcome::Committed(_)),
                    "level {level}: {outcome:?}"
                );
            }
            // Advance the fixture's owner clock past the shared USE cadence, as the
            // other callback scenarios do; do not disable the production action gate.
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(std::time::Duration::from_secs(2))
                .ok_or("clock")?;
            if capacity && case == 1 {
                rewards::fill_capacity(owner, admitted, authority).await?;
            }
            let before_xp = owner
                .root
                .read_character_progression(authority, fence.character_id)
                .await
                .map_err(debug)?
                .ok_or("progression")?
                .total_experience
                .get();
            let payload =
                oteryn_protocol_oteryn::world_object::encode_use_intent(&WorldObjectTarget {
                    placement: placement.clone(),
                    expected_revision: world.facts().object_revision(key),
                })
                .map_err(debug)?;
            let result = wire(
                owner,
                admitted,
                u64::try_from(index * 3 + case + 1)?,
                2,
                oteryn_protocol_oteryn::world_object::encode_use_intent(&WorldObjectTarget {
                    placement: placement.clone(),
                    expected_revision: world.facts().object_revision(key),
                })
                .map_err(debug)?,
            )
            .await
            .map_err(|error| format!("{name} level {level}: {error}"))?;
            assert_eq!(
                oteryn_protocol_oteryn::world_object::decode_use_result(&result).map_err(debug)?,
                if (full_backpack || capacity) && case == 2 {
                    UseDisposition::NoRoom
                } else {
                    expected
                },
                "{name} level {level}"
            );
            // The ordinary quest refresh drains the durable XP entitlement through
            // the progression owner; a repeated command cannot pay it twice.
            owner
                .refresh_quest_session(admitted.game_session_id, false)
                .await;
            let after_xp = owner
                .root
                .read_character_progression(authority, fence.character_id)
                .await
                .map_err(debug)?
                .ok_or("progression")?
                .total_experience
                .get();
            assert_eq!(
                after_xp - before_xp,
                if case == 1 {
                    callback.experience.ok_or("shrine XP")?
                } else {
                    0
                }
            );
            if case == 1 {
                let replay = wire(
                    owner,
                    admitted,
                    u64::try_from(index * 3 + case + 1)?,
                    2,
                    payload,
                )
                .await?;
                assert_eq!(replay, result, "exact USE replay returns the same response");
            }
        }
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?;
        // Re-open the owner read, rather than inspecting the dispatch session cache.
        let copy = h
            .root
            .read_character_quest_state(authority, fence.character_id)
            .await
            .map_err(debug)?;
        assert_eq!(
            copy.tracks()
                .get(&format!("{}{name}", monk_shrines::FLAG_PREFIX)),
            Some(&1)
        );
        assert_eq!(
            copy.tracks().get(monk_shrines::COUNT_TRACK),
            Some(&(i64::try_from(index)? + 1))
        );
        npc::journal(&copy, index + 1, false)?;
        let offensive = (index + 1).saturating_sub(2).min(5) as u8;
        assert_eq!(monk_shrines::offensive_count(copy.tracks()), offensive);
        let tuning = oteryn_gameplay_combat_rules::monk_shrines::MonkShrineBonuses::checked_in()
            .ok_or("Monk tuning")?;
        assert_eq!(
            tuning.attack(100, offensive, true, false),
            Some(100 + 10 * i64::from(offensive))
        );
        assert_eq!(
            tuning.attack(100, offensive, true, true),
            Some(100 + 20 * i64::from(offensive))
        );
        assert_eq!(
            tuning.taken(100, (index + 1) as u8, true),
            Some(100 - 2 * (index as i64 + 1))
        );
        // A committed shrine changes the live bonus projection before the refresh cadence.
        {
            let sessions = owner.states.quest_sessions.lock().map_err(debug)?;
            let live = sessions
                .get(&admitted.game_session_id)
                .and_then(|entry| entry.copy.as_ref())
                .ok_or("live quest copy")?;
            assert_eq!(live.tracks(), copy.tracks());
        }
        if full_backpack || capacity {
            rewards::retry(
                owner, admitted, h, authority, world, callback, &placement, key,
            )
            .await?;
            return Ok(());
        }
        let receipts: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(
            receipts,
            i64::try_from(index)? + 3,
            "repeat cannot produce another completion receipt"
        );
    }
    let delivered: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_monk_reward_entitlements WHERE state='DELIVERED'",
    )
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(
        delivered, 7,
        "all seven original equipment rewards are delivered"
    );
    let claims: i64 = sqlx::query_scalar("SELECT count(*) FROM game_reward_claims WHERE claim_production_key LIKE 'oteryn:reward-claim.monk-%'").fetch_one(&h.pool).await?;
    assert_eq!(claims, 7, "replay and repeat never duplicate equipment");
    for spell in ["mystic_repulse", "forceful_uppercut", "focus_harmony"] {
        let learned: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_character_known_spells WHERE spell_key=$1",
        )
        .bind(format!("candidate:spell/{spell}"))
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(learned, 1, "{spell}");
    }
    npc::finish(owner, admitted, h, authority, world, pema, return_position).await?;
    Ok(())
}
