//! C17c: parameter commands install the donor outfit on the real actor owner.
use super::death_ml_loss_composed::current_fence;
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::native_gameplay::test_support::cosmetics_fixture::cosmetics_test_packet;
use crate::content::{
    ContentActivationController, NativeEntryActivationIssuance, NodeBootQuiescence,
    activate_native_entry_room_with_gameplay,
};
use crate::durability::character_build::{
    BuildCause, BuildChangeRequest, BuildOccurrence, DurableBuildState, convert_vocation,
};
use crate::foundation::{CharacterId, CharacterLease, WorldId};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_protocol_oteryn::actor_spell::{SpellCastDisposition, SpellCastIntent, SpellTarget};
use oteryn_protocol_oteryn::actor_spell_v2::{ParameterSpellCastIntent, PrivateSpellEffect};
use oteryn_protocol_oteryn::appearance::{EntityRef, Look};

#[test]
fn creature_illusion_parameter_dispatch_outfit_duration_and_refusals_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new().stack_size(128 << 20).spawn(|| {
        run(async |admin| {
            let h = Harness::create_appearance(admin, "creature_illusion").await?;
            prepare_current_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
            let world = WorldId::decode(&id(42)).map_err(debug)?;
            let mut cosmetics = cosmetics_test_packet();
            cosmetics["authoring"]["document"]["content_revision"] = "content-1".into();
            use crate::content::native_gameplay::*;
            use sha2::Digest;
            let mut supplied = NativeGameplayInput::from_manifest(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../content/spells.manifest.json")).map_err(debug)?;
            let bytes = serde_json::to_vec(&cosmetics)?;
            supplied.cosmetics = PinnedGameplayBytes { sha256: sha2::Sha256::digest(&bytes).iter().map(|b| format!("{b:02x}")).collect(), bytes };
            let bytes = include_bytes!("../../../../../../tools/content-schema/native-gameplay/canary-thalom-world.json").to_vec();
            supplied.source_world = Some(PinnedGameplayBytes { sha256: sha2::Sha256::digest(&bytes).iter().map(|b| format!("{b:02x}")).collect(), bytes });
            supplied.build_training.as_mut().ok_or("build training")?.content_revision = "content-1".into();
            let room = crate::content::qualify_selected_native_gameplay_room(world, &supplied).map_err(debug)?;
            let issuance = NativeEntryActivationIssuance { world_id: world, activation_sequence: 1,
                server_artifact_digest: room.compiled().server_digest(), client_artifact_digest: room.compiled().client_digest(), frame_binding_digest: room.frame_binding().digest() };
            let mut controller = ContentActivationController::new();
            let pin = activate_native_entry_room_with_gameplay(&mut controller, &NodeBootQuiescence::before_channel_runtime(), world, &issuance, &supplied).map_err(debug)?;
            let active = controller.active().ok_or("active")?;
            let native = active.native_gameplay().ok_or("native")?;
            let room = pin.qualified_room().clone();
            let (cp, _, _, _) = pin.into_channel_parts();
            let mut deps = Dependencies::with_content(cp, room.clone(), native.spell_book().clone())?;
            deps.combat_room = Some(room);
            let mut owner = deps.owner(&h, &authority);
            owner.active_generation = Some(active);
            let actor = deps.actor;
            let session = deps.admitted.game_session_id;
            let formula = native.training_formula().ok_or("training")?;
            let druid = convert_vocation(formula, &DurableBuildState::default(), "druid").map_err(debug)?;
            h.root.commit_character_build(&authority, &h.node, current_fence(&owner, session).await?,
                BuildChangeRequest { occurrence: BuildOccurrence::from_bytes(id(90)).map_err(debug)?,
                    cause: BuildCause::VocationChoice, before: DurableBuildState::default(),
                    after: druid.clone(), pruned_stance: None }, formula).await.map_err(debug)?;
            {
                let mut runtime = deps.runtime.lock().await;
                runtime.initialize_first_entry_position(actor).map_err(debug)?;
                runtime.bind_attacker_lease(actor, session, CharacterLease::new(CharacterId::decode(&id(41)).map_err(debug)?, 2).map_err(debug)?).map_err(debug)?;
                let mut states = owner.states.spell_states.lock().await;
                assert!(states.initialize(&runtime, actor, session, crate::spell::cast::CharacterCastFacts {
                    level: 50, max_mana: 500, ..crate::gameplay_transport::actor_spell::tests::FACTS
                }, (0, 0), owner.owner_now()).is_some());
                assert!(states.load_owned_build(&runtime, actor, session, &druid));
                states.get_mut(&runtime, actor, session).ok_or("caster")?.enable_owned_training(&druid, formula).map_err(debug)?;
            }
            sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)")
                .bind(id(41).as_slice()).execute(&h.pool).await?;
            sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
            sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)").bind(id(42).as_slice()).bind(id(43).as_slice()).bind(active.identity().server_artifact_digest().as_slice()).bind(active.identity().client_artifact_digest().as_slice()).bind(deps.room.frame_binding().digest().as_slice()).execute(&h.pool).await?;
            h.root.initialize_familiar_group(&authority, &h.node, current_fence(&owner, session).await?).await.map_err(debug)?;
            let index = deps.spells.spells.iter().position(|s| s.name == "Creature Illusion").ok_or("spell")?;
            let spell = std::num::NonZeroU32::new(u32::try_from(index + 1)?).ok_or("index")?;
            let entity = EntityRef { identity: actor.placement_identity(), generation: actor.actor_local_generation() };
            let initial = owner.appearance_observation(actor, session, vec![entity]).await.ok_or("initial look")?.bindings[0].look.clone();
            let allowed = native.policies().policies().iter().find(|p| p.flags.illusionable && native.spell_appearances().and_then(|a| a.for_creature(&p.definition_key, &p.definition_revision)).is_some()).ok_or("illusionable monster")?;
            let denied = native.policies().policies().iter().find(|p| !p.flags.illusionable).ok_or("forbidden monster")?;
            for (command, name, disposition, text) in [
                (1, "unknown C17c monster", SpellCastDisposition::Rejected, "A creature with this name does not exist."),
                (2, denied.display_name.as_str(), SpellCastDisposition::Rejected, "Sorry, not possible."),
                (3, allowed.display_name.as_str(), SpellCastDisposition::Cast, ""),
            ] {
                let intent = ParameterSpellCastIntent { intent: SpellCastIntent { spell, target: SpellTarget::None, aim_at_target: false }, parameter: Some(name.into()) };
                let access = owner.refresh_spell_access(actor, session).await;
                let dispatch = owner.cast_parameters(actor, session, command, &intent, &access).await;
                let result = dispatch.result.ok_or_else(|| format!("command {command}: {:?}", dispatch.cast))?;
                assert_eq!(result.disposition, disposition, "{name}");
                if disposition == SpellCastDisposition::Rejected {
                    let feedback = result.feedback.ok_or("source refusal")?;
                    assert_eq!(feedback.text, text);
                    assert_eq!(feedback.effect, PrivateSpellEffect::Poff);
                    assert_eq!(owner.appearance_observation(actor, session, vec![entity]).await.ok_or("refused look")?.bindings[0].look, initial);
                }
            }
            let member = native.spell_appearances().and_then(|a| a.for_creature(&allowed.definition_key, &allowed.definition_revision)).ok_or("outfit")?;
            let transformed = owner.appearance_observation(actor, session, vec![entity]).await.ok_or("transformed look")?;
            assert_ne!(transformed.bindings[0].look, initial);
            assert_eq!(transformed.bindings[0].look, Look::Outfit { look_type: member.look_type(), colours: member.selection().colours, addons: member.selection().addons });
            {
                let runtime = deps.runtime.lock().await;
                let states = owner.states.spell_states.lock().await;
                let conditions = states.get(&runtime, actor, session).ok_or("caster")?.owned_conditions();
                let started = conditions.active_at(crate::domain::condition::ConflictKey::Outfit, owner.owner_now().get()).ok_or("outfit condition")?.started_at();
                assert!(conditions.displayed_temporary_appearance_at(started + 179_999_999).is_some());
                assert!(conditions.displayed_temporary_appearance_at(started + 180_000_000).is_none());
            }
            owner.clock_origin = owner.clock_origin.checked_sub(std::time::Duration::from_secs(181)).ok_or("clock")?;
            assert_eq!(owner.appearance_observation(actor, session, vec![entity]).await.ok_or("expired look")?.bindings[0].look, initial);
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        }).map_err(|error| error.to_string())
    })?.join().map_err(|_| "Creature Illusion thread panicked")?.map_err(Into::into)
}
