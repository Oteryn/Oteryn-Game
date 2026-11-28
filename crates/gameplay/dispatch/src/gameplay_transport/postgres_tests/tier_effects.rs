//! The live periodic spell owner applies the Avatar and the existing logout owner saves it.

use super::srx3_native_source_cast_fixtures::NativeOwnerClock;
use super::{Dependencies, Harness, TestResult, debug, id, prepare_current_session, run};
use crate::domain::condition::{ConditionValues, ConflictKey};
use crate::durability::character_equipment::{EquipmentSnapshot, EquippedItem};
use crate::durability::item_mint::TypedDefinitionRef;
use crate::foundation::{CharacterLease, ExactActorRef, GameSessionId, WorldId};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::attack::tier_effects::periodic_occurrence;
use crate::gameplay_transport::equipment_move::equipment_effects_owner::equipped_offence;
use oteryn_simulation_determinism::{
    GameplayDecisionRoot, SemanticTimeMicros, deterministic_decision_u64,
};

#[test]
fn global_transcendence_periodic_avatar_and_condition_checkpoint_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            run(async move |_| scenario(admin).await).map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "Transcendence scenario thread panicked")??;
    Ok(())
}

async fn scenario(admin: String) -> TestResult {
    let h = Harness::create_with_admission_seed(admin, "tier_transcendence", true, false).await?;
    prepare_current_session(&h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world = WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, pin) =
        crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None);
    let generation = controller.active().ok_or("active generation")?;
    let native = generation.native_gameplay().ok_or("native gameplay")?;
    let book = native.spell_book();
    let offensive = book
        .spells
        .iter()
        .find(|spell| spell.aggressive)
        .ok_or("offensive spell")?;
    let outfit = native
        .spell_appearances()
        .ok_or("spell appearances")?
        .for_look_type(1823)
        .ok_or("qualified Avatar of Balance outfit")?;
    let room = pin.qualified_room().clone();
    let (channel_pin, _, _, _) = pin.into_channel_parts();
    let root = GameplayDecisionRoot::from_bytes(channel_pin.server_artifact_digest());
    let deps = Dependencies::with_content(channel_pin, room, book.clone())?;
    let mut owner = deps.owner(&h, &authority);
    owner.active_generation = Some(generation);
    let actor = deps.actor;
    let session = deps.admitted.game_session_id;
    let character = crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?;
    {
        let mut runtime = deps.runtime.lock().await;
        runtime
            .initialize_first_entry_position(actor)
            .map_err(debug)?;
        runtime
            .bind_attacker_lease(
                actor,
                session,
                CharacterLease::new(character, 2).map_err(debug)?,
            )
            .map_err(debug)?;
    }
    deps.initialize(&owner).await;
    {
        let runtime = deps.runtime.lock().await;
        let mut states = owner.states.spell_states.lock().await;
        let state = states
            .get_mut(&runtime, actor, session)
            .ok_or("current player")?;
        let equipment = EquipmentSnapshot {
            character: id(41),
            content_digest: native.source_digest(),
            character_revision: 1,
            revision: 1,
            combat_mode: None,
            items: vec![EquippedItem {
                slot: 3,
                tier: 10,
                item_instance_id: id(71),
                quantity: 1,
                state_revision: 1,
                definition: TypedDefinitionRef {
                    family: "Item".into(),
                    production_key: "oteryn:item.tibia.i3389".into(),
                    revision_ref: "definition-r1".into(),
                },
            }],
        };
        state.install_equipped_offence(
            equipped_offence(&equipment, native).ok_or("qualified legs")?,
        );
    }

    // Find a successful idle check and two consecutive successful rolls. The first tests
    // the action gate. The pair tests the active-Avatar exclusion independently of RNG.
    const PERIOD: u64 = 2_000_000;
    let first = owner.owner_now().get() / PERIOD + 1;
    let wins = |window| {
        deterministic_decision_u64(
            &root,
            periodic_occurrence(actor, session, window),
            "tier_transcendence",
            0,
        )
        .is_ok_and(|draw| (u128::from(draw) * 10_000) < (235_u128 << 64))
    };
    let idle = (first..first + 20_000)
        .find(|window| wins(*window))
        .ok_or("idle proc draw")?;
    let active = (idle + 1..idle + 40_000)
        .find(|window| wins(*window) && wins(*window + 1))
        .ok_or("consecutive proc draws")?;
    let clock = NativeOwnerClock::new(&owner);
    let idle = idle * PERIOD;
    let active = active * PERIOD;
    clock.set(idle);
    h.root.await_ready().await?;
    owner
        .tick_spell_periodic_at(actor, session, SemanticTimeMicros::from_micros(idle))
        .await;
    assert_eq!(avatar_outfit(&owner, (actor, session), idle).await?, None);

    for now in [active, active + PERIOD] {
        {
            let runtime = deps.runtime.lock().await;
            let mut states = owner.states.spell_states.lock().await;
            states
                .get_mut(&runtime, actor, session)
                .ok_or("current player")?
                .note_offensive_spell(offensive, SemanticTimeMicros::from_micros(now - 1));
        }
        clock.set(now);
        h.root.await_ready().await?;
        owner
            .tick_spell_periodic_at(actor, session, SemanticTimeMicros::from_micros(now))
            .await;
        assert_eq!(
            avatar_outfit(&owner, (actor, session), now).await?,
            Some((outfit.look_type(), active))
        );
        let runtime = deps.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        let state = states
            .get(&runtime, actor, session)
            .ok_or("current player")?;
        assert!(matches!(
            state.owned_conditions().attributes_at(now),
            Some(ConditionValues::Attributes {
                duration_ms: 7_000,
                critical_chance_percent: 100,
                critical_extra_percentage_points: 15,
                incoming_reduction_percent: 15,
                ..
            })
        ));
        assert!(state.cooldowns.spells.is_empty());
        assert!(state.cooldowns.groups.is_empty());
    }

    h.root.await_ready().await?;
    assert!(owner.save_conditions_at_actor_end(actor, session).await);
    h.root.await_ready().await?;
    let saved = h
        .root
        .read_character_conditions(&authority, crate::domain::CharacterId::from_bytes(id(41))?)
        .await?
        .ok_or("condition checkpoint")?;
    let restored = crate::gameplay_transport::condition_persistence::prepare(
        Some(&saved),
        saved.lease_generation + 1,
        saved.content_digest,
        0,
    )
    .flatten()
    .ok_or("restore Avatar checkpoint")?;
    assert_eq!(restored.outfit_at(4_999_999), Some(outfit.look_type()));
    assert!(
        restored
            .displayed_temporary_appearance_at(4_999_999)
            .is_some()
    );
    assert!(restored.attributes_at(4_999_999).is_some());
    assert_eq!(restored.outfit_at(5_000_000), None);
    assert!(restored.attributes_at(5_000_000).is_none());
    let expiry = active + 7_000_000;
    clock.set(expiry);
    h.root.await_ready().await?;
    owner
        .tick_spell_periodic_at(actor, session, SemanticTimeMicros::from_micros(expiry))
        .await;
    assert_eq!(avatar_outfit(&owner, (actor, session), expiry).await?, None);
    eprintln!("PG_MARKER global-transcendence-avatar-checkpoint");

    drop(clock);
    drop(owner);
    drop(authority);
    drop(seal);
    h.cleanup().await
}

async fn avatar_outfit(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    (actor, session): (ExactActorRef, GameSessionId),
    now: u64,
) -> TestResult<Option<(u32, u64)>> {
    let runtime = owner.runtime.lock().await;
    let states = owner.states.spell_states.lock().await;
    let state = states
        .get(&runtime, actor, session)
        .ok_or("current player")?;
    let conditions = state.owned_conditions();
    Ok(conditions.outfit_at(now).zip(
        conditions
            .active_at(ConflictKey::Outfit, now)
            .map(|instance| instance.started_at()),
    ))
}
