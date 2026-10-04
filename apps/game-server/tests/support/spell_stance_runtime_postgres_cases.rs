//! Actual Spell owner consumes a genuine PostgreSQL Stance receipt.
use crate::bestiary_postgres_harness::{
    Harness, SESSION, TestResult, configured_admin, debug, fence, id, runtime,
};
use crate::durability::character_stance::StanceChangeOutcome;
use crate::foundation::GameSessionId;

/// This exercises a genuine private writer receipt, never a test receipt
/// constructor. The test book opens only its Premium header gate; the native
/// parameters and sealed source profile are the real canonical Protector.
#[test]
fn stance_real_receipt_pays_actual_actor_once_and_preserves_intervening_hp() -> TestResult {
    run("stance_actor_receipt", async |h| {
        use crate::foundation::{ChannelContentPin, ChannelId, ChannelRuntimeV1, WorldId};
        use crate::spell::cast::{CharacterCastFacts, PlayerSpellState, prepare_native_owner_cast};
        use crate::spell::stance_execution::PreparedStance;
        use crate::spell::{OperationalCastFacts, SpellBook, Vocation};
        use oteryn_simulation_determinism::SemanticTimeMicros;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = WorldId::decode(&id(crate::bestiary_postgres_harness::WORLD)).map_err(debug)?;
        let channel =
            ChannelId::decode(&id(crate::bestiary_postgres_harness::CHANNEL)).map_err(debug)?;
        let mut owner = ChannelRuntimeV1::from_committed_assignment(
            world,
            channel,
            h.node.fact().node_id(),
            h.node.fact().registration_revision(),
            1,
            1,
            "runtime-scope-assignment:1",
            2,
            ChannelContentPin::test(world),
        )
        .map_err(debug)?;
        let session = GameSessionId::decode(&id(SESSION)).map_err(debug)?;
        let reservation = owner.reserve_fresh_session(session).map_err(debug)?;
        let actor = owner.commit_fresh_session(reservation).map_err(debug)?;
        let document: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tools/content-schema/spell-authoring/samples/native-spell-profiles.json"
        ))?;
        let profile = document["profiles"]
            .as_array()
            .ok_or("profiles")?
            .iter()
            .find(|r| r["name"] == "Protector")
            .ok_or("Protector")?;
        let mut spell = crate::spell::authoring::spell_from_bundle(
            &serde_json::json!({"spell":profile["spell"]}),
            &profile["dependencies"],
        )
        .map_err(debug)?;
        spell.premium = false; // Explicit test header gate, no production entitlement.
        let book = SpellBook::new(vec![spell.clone()]).map_err(debug)?;
        let now = SemanticTimeMicros::from_micros(1_000);
        let mut state = PlayerSpellState::new(
            CharacterCastFacts {
                vocation: Vocation::EliteKnight,
                level: 50,
                magic_level: 0,
                max_health: 500,
                max_mana: 600,
                max_soul: 100,
            },
            0,
            0,
        )
        .ok_or("state")?;
        state.make_playable(now).map_err(debug)?;
        let operational = OperationalCastFacts {
            caster_position: crate::spell::chain::TilePosition {
                x: 0,
                y: 0,
                floor: 7,
            },
            target_position: None,
            target: None,
            line_of_sight_clear: None,
            direction_available: false,
            wheel_unlocked: None,
            in_protection_zone: false,
            target_tile_solid: None,
            target_tile_creature: None,
        };
        let paid = prepare_native_owner_cast(
            &state,
            &spell,
            &operational,
            crate::spell::native::Facts::Stance {
                active: None,
                vocation: Vocation::EliteKnight,
            },
            now,
            &mut |_, _| 0,
        )
        .map_err(debug)?;
        let prepared = PreparedStance::new(
            &state,
            paid,
            &spell,
            actor,
            session,
            99,
            std::num::NonZeroU32::new(1).ok_or("index")?,
            fence(1)?,
            "content-1".into(),
            "policy-1".into(),
            now,
        )
        .map_err(debug)?;
        state.reserve_stance(prepared.clone()).map_err(debug)?;
        assert_eq!(state.vitals().mana, 600); // Preparing did not pay.
        let outcome = h
            .root
            .commit_character_stance(
                &authority,
                &h.node,
                prepared.fence,
                prepared.request.clone(),
            )
            .await
            .map_err(debug)?;
        let StanceChangeOutcome::Committed(receipt) = outcome else {
            return Err("new stance not committed".into());
        };
        let (wounded, damage) = state.after_creature_damage(25).ok_or("damage")?;
        assert_eq!(damage.applied, 25);
        state = wounded;
        state
            .commit_stance_receipt(
                &prepared,
                &receipt,
                &spell,
                SemanticTimeMicros::from_micros(2_000_000),
            )
            .map_err(debug)?;
        assert_eq!(state.vitals().health, 475);
        assert_eq!(state.vitals().mana, 580);
        assert_eq!(state.revision(), 3);
        assert_eq!(state.durable_stance_key(), Some("protector"));
        let committed = state.clone();
        assert!(
            state
                .commit_stance_receipt(
                    &prepared,
                    &receipt,
                    &spell,
                    SemanticTimeMicros::from_micros(2_000_001)
                )
                .is_err()
        );
        assert_eq!(state, committed);
        let durable = h
            .root
            .read_character_stance(&authority, prepared.fence.character_id)
            .await
            .map_err(debug)?;
        let mut reloaded = PlayerSpellState::new(state.character_facts(), 0, 0).ok_or("reload")?;
        reloaded.load_owned_stance(&durable, &book).map_err(debug)?;
        assert_eq!(
            reloaded.standard_stance(),
            Some(crate::spell::native_actor_states::StandardStance::Protector)
        );
        let mut out_of_book =
            PlayerSpellState::new(state.character_facts(), 0, 0).ok_or("reload")?;
        out_of_book
            .load_owned_stance(&durable, &SpellBook::default())
            .map_err(debug)?;
        assert_eq!(out_of_book.standard_stance(), None);
        assert_eq!(out_of_book.durable_stance_key(), Some("protector"));
        let mut wrong_class = PlayerSpellState::new(
            CharacterCastFacts {
                vocation: Vocation::Knight,
                ..state.character_facts()
            },
            0,
            0,
        )
        .ok_or("class")?;
        wrong_class
            .load_owned_stance(&durable, &book)
            .map_err(debug)?;
        assert_eq!(wrong_class.standard_stance(), None);
        assert_eq!(wrong_class.durable_stance_key(), Some("protector"));
        Ok(())
    })
}

fn run<F>(tag: &'static str, body: F) -> TestResult
where
    F: AsyncFnOnce(&Harness) -> TestResult,
{
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, tag, true).await?;
        let outcome = body(&harness).await;
        harness.cleanup().await?;
        outcome
    })
}
