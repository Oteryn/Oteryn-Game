//! Durable knowledge survives a fresh runtime admission, using the real PostgreSQL owner.
use super::fixture::{Harness, debug, run};
use super::tutorial_vitals_fixture::admit;
use super::{Dependencies, TestResult, prepare_current_session};
use std::collections::BTreeSet;

#[test]
fn spell_catalog_granted_spell_is_learned_after_rejoin_postgres() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("spell-catalog-rejoin", || {
        run(async |admin| {
            let h =
                Harness::create_with_admission_seed(admin, "spell_catalog", false, false).await?;
            let request = prepare_current_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let deps = Dependencies::new()?;
            let owner = deps.owner(&h, &authority);
            admit(&owner, &deps, &request).await?;
            let session = deps.admitted.game_session_id;
            let key = deps.spells.spells.first().ok_or("spell")?.key.clone();
            let fence = owner
                .current_quest_fence(session)
                .await
                .map_err(|()| "fence")?
                .ok_or("session")?;
            h.root
                .acquire_automatic_spells(
                    &authority,
                    &h.node,
                    fence,
                    &BTreeSet::from([key.clone()]),
                )
                .await
                .map_err(debug)?;
            drop(owner);
            // A fresh runtime has no cached spell knowledge. Re-use the admitted durable session
            // through its real first-entry reconciliation and load from PostgreSQL again.
            let joined = Dependencies::new()?;
            let owner = joined.owner(&h, &authority);
            admit(&owner, &joined, &request).await?;
            let runtime = joined.runtime.lock().await;
            let states = owner.states.spell_states.lock().await;
            let state = states
                .get(&runtime, joined.actor, session)
                .ok_or("rejoined actor")?;
            assert!(state.learned_spells().contains(&key));
            drop(states);
            drop(runtime);
            let learned = h
                .root
                .read_character_known_spells(&authority, fence.character_id)
                .await
                .map_err(debug)?;
            assert_eq!(learned, BTreeSet::from([key]));
            eprintln!("S-SPELLBOOK PostgreSQL granted knowledge restored on re-join");
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    })
}

#[test]
fn spell_catalog_projection_preserves_native_runes_costs_and_indices() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("spell-catalog-projection", || {
        let deps = Dependencies::new()?;
        let (controller, _) = super::death_ml_loss_composed::production_generation_with_pin(
            deps.admitted.world_id,
            "content-1",
        )?;
        let book = controller
            .active()
            .and_then(|active| active.native_gameplay())
            .ok_or("native generation")?
            .spell_book();
        let mut facts = crate::gameplay_transport::actor_spell::tests::FACTS;
        facts.level = 1000;
        facts.magic_level = 100;
        let mut state = crate::spell::cast::PlayerSpellState::new(facts, 0, 0).ok_or("state")?;
        let initial =
            crate::gameplay_transport::spell_catalog::catalog(book, &state, 0).ok_or("catalog")?;
        assert!(initial.iter().any(|entry| entry.rune_item.is_some()));
        for entry in &initial {
            let index = std::num::NonZeroU32::new(entry.cast_index).ok_or("index")?;
            let spell = book.indexed(index).ok_or("cast lookup")?;
            assert_eq!(entry.name, spell.name);
            assert_eq!(entry.soul, spell.soul);
            assert!(!entry.learned);
            match &spell.carrier {
                crate::spell::Carrier::Instant { words, .. } => {
                    assert_eq!(&entry.words, words);
                    assert_eq!(entry.rune_item, None);
                }
                crate::spell::Carrier::Rune { item, .. } => {
                    assert!(entry.words.is_empty());
                    assert_eq!(entry.rune_item, Some(*item));
                }
            }
            match spell.mana {
                crate::spell::ManaCost::Fixed(value) => {
                    assert_eq!((entry.mana, entry.mana_percent), (value, false))
                }
                crate::spell::ManaCost::PercentOfMaximum(value) => {
                    assert_eq!((entry.mana, entry.mana_percent), (value, true))
                }
            }
        }
        let codec =
            oteryn_protocol_oteryn::actor_spell_catalog::encode_catalog(&initial).map_err(debug)?;
        assert_eq!(
            oteryn_protocol_oteryn::actor_spell_catalog::decode_catalog(&codec).map_err(debug)?,
            initial
        );
        let gated = initial
            .iter()
            .find(|entry| entry.available && entry.level > 1)
            .ok_or("level gated")?;
        state.facts.level = 1;
        let lower = crate::gameplay_transport::spell_catalog::catalog(book, &state, 0)
            .ok_or("lower level")?;
        assert!(
            !lower
                .iter()
                .find(|entry| entry.cast_index == gated.cast_index)
                .ok_or("level entry")?
                .available
        );
        state.facts.level = 1000;
        let learned = book
            .indexed(std::num::NonZeroU32::new(initial[0].cast_index).ok_or("index")?)
            .ok_or("spell")?;
        state.install_learned_spells(BTreeSet::from([learned.key.clone()]));
        let updated =
            crate::gameplay_transport::spell_catalog::catalog(book, &state, 0).ok_or("updated")?;
        assert!(
            updated
                .iter()
                .find(|entry| entry.cast_index == initial[0].cast_index)
                .ok_or("learned")?
                .learned
        );
        state.facts.vocation = crate::spell::Vocation::None;
        let changed =
            crate::gameplay_transport::spell_catalog::catalog(book, &state, 0).ok_or("vocation")?;
        assert!(
            changed
                .iter()
                .any(|entry| entry.cast_index == initial[0].cast_index)
        );
        Ok(())
    })
}
