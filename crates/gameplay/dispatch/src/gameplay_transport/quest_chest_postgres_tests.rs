//! Literal chest USE on the real connection: durable MINT, quest refresh and XP settlement.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::combat_pickup::resolve_item_definition_facts;
use crate::durability::item_loop_move::ItemLoopContent;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_transfer::{ItemDefinitionFacts, ItemWeightLookup, ItemWeightRefused};
use crate::gameplay_transport::connection::FirstEntryOutcome;
use crate::interaction_chest_use::entry_chest;

// Only the fixture-injected entry definitions need this adapter. Facts still come from
// the existing Content resolver; grant_starter_items remains the real durable bag owner.
struct EntryContent<'a>(&'a crate::content::CanonicalReferencePlayableContent);
impl ItemWeightLookup for EntryContent<'_> {
    fn generation_digest(&self) -> [u8; 32] {
        [8; 32]
    }
    fn weight_of(&self, definition: &TypedDefinitionRef) -> Result<u32, ItemWeightRefused> {
        resolve_item_definition_facts(self.0, definition)
            .ok()
            .and_then(|facts| facts.weight)
            .ok_or(ItemWeightRefused::Unknown)
    }
}
impl ItemLoopContent for EntryContent<'_> {
    fn loop_content_digest(&self) -> [u8; 32] {
        self.generation_digest()
    }
    fn loop_facts(&self, definition: &TypedDefinitionRef) -> Option<ItemDefinitionFacts> {
        resolve_item_definition_facts(self.0, definition).ok()
    }
}

#[test]
fn quest_chest_use_dispatch_mints_once_refreshes_and_pays_xp_once() -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "chest dispatch e2e panicked")?
        .map_err(Into::into)
}

fn scenario() -> TestResult {
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "quest_chest_dispatch", false, false)
                .await?;
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let dependencies = Dependencies::new()?;
        let (generation, _) = super::death_ml_loss_composed::production_generation_with_pin(
            dependencies.admitted.world_id,
            "content-1",
        )?;
        let mut content = crate::interaction_chest_use::with_entry_chest(dependencies.room.door())
            .map_err(debug)?;
        for definition in &mut content.definitions {
            if let crate::content::ReferenceDefinitionKind::RewardClaim(claim) =
                &mut definition.kind
            {
                for placement in &mut claim.placements {
                    placement.quest_transition =
                        Some(super::quest_runtime_fixture::TRANSITION.into());
                }
            }
        }
        let mut owner = dependencies.owner(&harness, &authority);
        owner.active_generation = generation.active();
        owner.chest = &content;
        let catalogue = super::quest_runtime_fixture::catalogue()?;
        owner.quest_catalogue = Some(catalogue);
        let mut admitted = dependencies.admitted;
        owner.admit_progression_session(&mut admitted).await;
        owner.admit_quest_session(&admitted).await;
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|_| "fence")?
            .ok_or("current fence")?;
        let item_fence = crate::durability::item_transfer::CurrentCharacterItemFence {
            character_id: fence.character_id,
            game_session_id: fence.game_session_id,
            connection_generation: fence.connection_generation,
            character_lease_generation: fence.character_lease_generation,
            runtime_scope: fence.runtime_scope,
            scope_ownership_generation: fence.scope_ownership_generation,
        };
        let weights = EntryContent(&content);
        sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&harness.pool).await?;
        assert!(
            harness
                .root
                .record_content_activation(
                    &crate::durability::content_activation::ContentActivationRequest {
                        world_id: admitted.world_id,
                        channel_id: admitted.channel_id,
                        activation_sequence: 1,
                        previous_sequence: None,
                        server_artifact_digest: weights.generation_digest(),
                        client_artifact_digest: [7; 32],
                        frame_binding_digest: [6; 32],
                    }
                )
                .await
                .map_err(debug)?
        );
        let request = super::fresh_fixture::fresh::Source::new(0)?.request()?;
        harness
            .root
            .initialize_admission_equipment(
                &authority,
                &harness.node,
                &item_fence,
                weights.generation_digest(),
                request.operation(),
            )
            .await
            .map_err(debug)?;
        let template = crate::durability::starter_grant::StarterTemplate {
            definition: TypedDefinitionRef {
                family: "Item".into(),
                production_key: entry_chest::BACKPACK_ITEM.into(),
                revision_ref: entry_chest::DEFINITION_REVISION.into(),
            },
            ..crate::durability::starter_grant::StarterTemplate::main()
        };
        assert!(matches!(
            harness
                .root
                .grant_starter_items(&authority, &harness.node, &item_fence, &weights, &template)
                .await
                .map_err(debug)?,
            crate::durability::starter_grant::StarterGrantOutcome::Minted(_)
        ));
        dependencies.initialize(&owner).await;
        owner
            .runtime
            .lock()
            .await
            .initialize_first_entry_position(dependencies.actor)
            .map_err(debug)?;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = Some(item_fence);
        let target = oteryn_protocol_oteryn::world_object::WorldObjectTarget {
            placement: entry_chest::PLACEMENT.as_bytes().to_vec(),
            expected_revision: 0,
        };
        let payload =
            oteryn_protocol_oteryn::world_object::encode_use_intent(&target).map_err(debug)?;
        let mut input = Vec::new();
        for command_id in [1, 2] {
            let frame = oteryn_protocol_oteryn::encode_client_command(
                1,
                &oteryn_protocol_oteryn::ClientCommandValue {
                    command_id,
                    command_type: 2,
                    payload: &payload,
                },
            )
            .map_err(debug)?;
            input.extend(u32::try_from(frame.len())?.to_be_bytes());
            input.extend(frame);
        }
        let (end, output) =
            super::weather_wire::fixture_access::serve_input(&owner, admitted, &input).await?;
        assert!(!output.is_empty(), "{end}");
        let paid: (i64, i64, i64, i64) = sqlx::query_as("SELECT total_experience, (SELECT count(*) FROM game_character_xp_receipts), (SELECT count(*) FROM game_character_quest_xp_obligations), (SELECT count(*) FROM game_character_quest_receipts) FROM game_character_progression_state").fetch_one(&harness.pool).await?;
        assert_eq!(paid, (50, 1, 0, 1), "{end}");
        let bag = harness
            .root
            .read_character_backpack(&authority, fence.character_id)
            .await
            .map_err(debug)?
            .ok_or("bag")?;
        assert_eq!(bag.entries.len(), 1);
        assert_eq!(bag.entries[0].item.quantity, entry_chest::REWARD_COUNT);
        // A replay on a new connection reaches the durable claim receipt again.
        let _ = super::weather_wire::fixture_access::serve_input(&owner, admitted, &input).await?;
        assert_eq!(
            harness
                .root
                .read_character_backpack(&authority, fence.character_id)
                .await
                .map_err(debug)?
                .ok_or("bag")?
                .entries
                .len(),
            1
        );
        {
            let entry = owner.quest_sessions.lock().map_err(debug)?;
            assert!(
                entry
                    .get(&admitted.game_session_id)
                    .and_then(|entry| entry.copy.as_ref())
                    .ok_or("refreshed copy")?
                    .xp_obligations()
                    .is_empty()
            );
        }
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}
