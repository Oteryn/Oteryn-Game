//! Disposable node-boot build provisioning. The node retains its incarnation
//! proof; the root-only control socket supplies only an admitted session id.
use super::*;
use crate::content::native_gameplay::NativeGameplayState;
use crate::qualification_spell_character as qualification_build;

fn request(bytes: &[u8]) -> Option<([u8; 16], &str)> {
    let text = std::str::from_utf8(bytes).ok()?.trim_end_matches('\n');
    let (session, profile) = text.strip_prefix("qualify ")?.split_once(' ')?;
    let session = parse_operation_id(session.as_bytes())?;
    matches!(
        profile,
        "explorer" | "knight" | "paladin" | "sorcerer" | "druid" | "monk" | "c17a" | "c17c"
    )
    .then_some((session, profile))
}

pub(super) async fn prepare(
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    proof: &NodeIncarnationProof,
    config: &NodeConfig,
    native: Option<&NativeGameplayState>,
    bytes: &[u8],
) -> &'static str {
    if !config.qualification_fixture {
        return "rejected";
    }
    let Some(native) = native else {
        return "rejected";
    };
    let Some((session, profile)) = request(bytes) else {
        return "rejected";
    };
    match prepare_build(root, authority, proof, config, native, session, profile).await {
        Ok(()) => "qualified",
        Err(_) => "rejected",
    }
}

async fn prepare_build(
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    proof: &NodeIncarnationProof,
    config: &NodeConfig,
    native: &NativeGameplayState,
    session: [u8; 16],
    profile: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let session = crate::foundation::GameSessionId::decode(&session)?;
    // Independently resolve current admission, never synthesize a fence from
    // fixture SQL or expose the node proof to the client process.
    let current = crate::durability::fresh_admission::FreshAdmissionStore::from_root(root.clone())
        .current_session(session)
        .await
        .map_err(|error| format!("qualification admission: {error:?}"))?;
    let scope = current.current_runtime_scope();
    if Some(*scope.world_id().as_bytes()) != super::super::uuid_bytes(&config.scope.world_id)
        || scope.channel_id().map(|id| *id.as_bytes())
            != super::super::uuid_bytes(&config.scope.channel_id)
    {
        return Err("qualification scope mismatch".into());
    }
    let character = crate::domain::CharacterId::from_bytes(
        *current.current_character_lease().character_id().as_bytes(),
    )
    .map_err(|error| format!("qualification character: {error:?}"))?;
    let interpretation = root
        .read_character_root_interpretation(authority, character)
        .await
        .map_err(|error| format!("qualification interpretation: {error:?}"))?
        .ok_or("qualification interpretation unavailable")?;
    let [profile_revision, ruleset_revision, content_revision, _] = interpretation.revisions();
    let progression = native
        .progression()
        .ok_or("qualification progression unavailable")?;
    let policy = progression.policy_for(profile_revision, ruleset_revision, content_revision)?;
    let context = policy.context.clone();
    let (vocation, level) = match profile {
        "c17a" | "c17c" => {
            let words = if profile == "c17a" {
                "utani hur"
            } else {
                "utevo res ina"
            };
            let spell = native
                .spell_book()
                .spoken(words)
                .ok_or("qualification spell absent")?
                .spell;
            if spell.premium || !spell.vocations.contains(&crate::spell::Vocation::Sorcerer) {
                return Err("qualification spell requires a free Sorcerer".into());
            }
            ("sorcerer", spell.level)
        }
        "explorer" => ("knight", 100),
        _ => (profile, 100),
    };
    let spec = qualification_build::QualificationBuildSpec {
        vocation,
        level,
        context,
        xp_occurrence: uuid_v7()?,
        build_occurrence: uuid_v7()?,
    };
    if matches!(profile, "c17a" | "c17c") {
        prepare_admitted_spell_character(root, authority, proof, &current, spec, policy).await
    } else {
        qualification_build::prepare_character_with_policy(
            root, authority, proof, &current, spec, policy,
        )
        .await?;
        prepare_items(root, authority, proof, &current, native).await
    }
}

// The spell rows use a fully admitted Character: admission has already bound
// level-1 progression. The older bootstrap-only fixture rejects that row. Keep
// this entry point strict about an untouched free build and use the same fenced
// initialization, XP and vocation owners; never reset an existing Character.
async fn prepare_admitted_spell_character<const N: usize>(
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    proof: &NodeIncarnationProof,
    current: &crate::foundation::GameSessionAuthoritySnapshot<
        crate::foundation::AuthenticatedTransportRefV1,
    >,
    spec: qualification_build::QualificationBuildSpec<'_>,
    policy: crate::domain::progression::FiniteProgressionPolicy<String, N>,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::durability::character_build::{
        BuildCause, BuildChangeRequest, BuildOccurrence, NO_VOCATION, convert_vocation,
    };
    use crate::durability::character_progression::{
        CurrentCharacterGameplayFence, ExperienceAwardRequest, ExperienceRewardOccurrence,
        ProgressionInitializationRequest,
    };
    use crate::durability::character_revision_sequencer::CharacterRevisionSequencer;
    if spec.vocation != "sorcerer"
        || !(1..=1000).contains(&spec.level)
        || policy.context != spec.context
    {
        return Err("C17 requires its source free Sorcerer build and policy".into());
    }
    let character = crate::domain::CharacterId::from_bytes(
        *current.current_character_lease().character_id().as_bytes(),
    )
    .map_err(fixture_error)?;
    let sequencer = CharacterRevisionSequencer::new();
    let mut slot = sequencer.acquire(character).await;
    let before = root
        .read_character_build_state(authority, character)
        .await
        .map_err(fixture_error)?;
    let progression = root
        .read_character_progression(authority, character)
        .await
        .map_err(fixture_error)?;
    if before.vocation() != NO_VOCATION
        || progression
            .as_ref()
            .is_some_and(|state| state.level != 1 || state.total_experience.get() != 0)
    {
        return Err("C17 spell fixture requires an untouched disposable Character".into());
    }
    let experience = policy
        .thresholds
        .iter()
        .find(|threshold| threshold.level == spec.level)
        .ok_or("C17 spell level absent from served policy")?
        .minimum_experience;
    let mut fence = CurrentCharacterGameplayFence {
        character_id: character,
        game_session_id: current.current_game_session_id(),
        connection_generation: current.current_connection_generation(),
        character_lease_generation: current.current_character_lease().generation(),
        runtime_scope: current.current_runtime_scope(),
        scope_ownership_generation: current.current_scope_generation(),
        expected_character_revision: root
            .read_current_character(authority, character)
            .await
            .map_err(fixture_error)?
            .revision,
    };
    // The initializer reconciles an existing admission row only when every
    // context/policy revision matches. A mismatched policy cannot reach XP.
    root.initialize_character_progression(
        authority,
        proof,
        fence,
        ProgressionInitializationRequest {
            context: spec.context.clone(),
            policy_revision: policy.policy_revision.clone(),
            reward_revision: policy.reward_revision.clone(),
            policy: policy.clone(),
        },
    )
    .await
    .map_err(fixture_error)?;
    if spec.level > 1 {
        slot.commit_experience(
            root,
            authority,
            proof,
            fence,
            ExperienceAwardRequest {
                occurrence: ExperienceRewardOccurrence::from_bytes(spec.xp_occurrence)
                    .map_err(fixture_error)?,
                amount: experience,
                context: spec.context.clone(),
                policy_revision: policy.policy_revision.clone(),
                reward_revision: policy.reward_revision.clone(),
                policy,
            },
            None,
        )
        .await
        .map_err(fixture_error)?;
    }
    fence.expected_character_revision = root
        .read_current_character(authority, character)
        .await
        .map_err(fixture_error)?
        .revision;
    let formula = crate::spell::mana_training::CompiledTrainingFormula::from_profile(
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tools/content-schema/native-gameplay/build-training.json"
        )),
        &spec.context.content,
    )
    .map_err(fixture_error)?;
    let after = convert_vocation(&formula, &before, spec.vocation).map_err(fixture_error)?;
    slot.commit_build(
        root,
        authority,
        proof,
        fence,
        BuildChangeRequest {
            occurrence: BuildOccurrence::from_bytes(spec.build_occurrence)
                .map_err(fixture_error)?,
            cause: BuildCause::VocationChoice,
            before,
            after,
            pruned_stance: None,
        },
        &formula,
        None,
    )
    .await
    .map_err(fixture_error)?;
    if !matches!(crate::gameplay_transport::load_character_cast_facts(root, authority, character).await,
        crate::gameplay_transport::CastFactsLoad::Ready { facts, .. }
        if facts.level == spec.level && facts.vocation == crate::spell::Vocation::Sorcerer)
    {
        return Err("C17 fenced fixture did not produce the source build".into());
    }
    Ok(())
}

fn fixture_error(error: impl std::fmt::Debug) -> String {
    format!("{error:?}")
}

// Fixture claims use the ordinary fenced/audited reward owner, never SQL item
// inserts. They are setup evidence only, not proof of player chest interaction.
async fn prepare_items(
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    proof: &NodeIncarnationProof,
    current: &crate::foundation::GameSessionAuthoritySnapshot<
        crate::foundation::AuthenticatedTransportRefV1,
    >,
    native: &NativeGameplayState,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::durability::item_mint::TypedDefinitionRef;
    use crate::durability::item_transfer::CurrentCharacterItemFence;
    use crate::durability::reward_claim_mint::{RewardClaimMintOutcome, RewardClaimMintRequest};
    use oteryn_gameplay_items::item_ref_admission::generation_item_facts;

    let fence = CurrentCharacterItemFence {
        character_id: crate::domain::CharacterId::from_bytes(
            *current.current_character_lease().character_id().as_bytes(),
        )
        .map_err(|error| format!("qualification character: {error:?}"))?,
        game_session_id: current.current_game_session_id(),
        connection_generation: current.current_connection_generation(),
        character_lease_generation: current.current_character_lease().generation(),
        runtime_scope: current.current_runtime_scope(),
        scope_ownership_generation: current.current_scope_generation(),
    };
    let inventory = root
        .read_item_use_inventory(authority, proof, fence)
        .await
        .map_err(|error| format!("qualification inventory: {error:?}"))?;
    let bag = inventory
        .backpack
        .ok_or("qualification requires the real starter backpack")?;
    let backpack = generation_item_facts(native, &bag.backpack.definition)
        .map_err(|error| format!("qualification backpack: {error:?}"))?;
    let index = native
        .item_index()
        .ok_or("qualification Item index missing")?;
    // Axe, rope, shovel. Every profile needs the same map/melee prerequisites;
    // vocation and level still come from the served progression policy.
    for (ordinal, id) in [3276, 3003, 3457].into_iter().enumerate() {
        let key = format!("oteryn:item.tibia.i{id}");
        let definition = TypedDefinitionRef {
            family: "Item".into(),
            production_key: key.clone(),
            revision_ref: index
                .revision(&key)
                .ok_or("qualification Item absent")?
                .into(),
        };
        let item = generation_item_facts(native, &definition)
            .map_err(|error| format!("qualification Item {key}: {error:?}"))?;
        let request = RewardClaimMintRequest {
            backpack_outputs: false,
            additional_items: Vec::new(),
            // Reserved fixture ids cannot collide with the client's low command ids.
            command: crate::foundation::CommandRef::new(
                current.current_game_session_id(),
                crate::foundation::CommandId::new(u64::MAX - ordinal as u64)?,
            ),
            claim: TypedDefinitionRef {
                family: "RewardClaim".into(),
                production_key: format!("oteryn:qualification.item.i{id}"),
                revision_ref: "qualification-v1".into(),
            },
            source_placement: "oteryn:qualification.node-boot".into(),
            item,
            quantity: 1,
            contents: Vec::new(),
            root_text: None,
            root_fluid_subtype: None,
            root_charges_remaining: None,
            backpack: backpack.clone(),
            content_revision: inventory.content_revision.clone(),
            ruleset_revision: inventory.ruleset_revision.clone(),
            sim_revision: "qualification-v1".into(),
            achievement: None,
            quest_transition: None,
            key_number: None,
        };
        let mut candidate = root
            .freeze_reward_claim_mint(authority, proof, fence, request)
            .await
            .map_err(|error| format!("qualification Item reserve: {error:?}"))?;
        let outcome = root
            .commit_reward_claim_mint(authority, proof, fence, &mut candidate)
            .await
            .map_err(|error| format!("qualification Item commit: {error:?}"))?;
        if !matches!(outcome, RewardClaimMintOutcome::Committed(_)) {
            return Err(format!("qualification Item not committed: {outcome:?}").into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qualification_c17_profiles_accept_admission_initialized_progression()
    -> crate::bestiary_postgres_harness::TestResult {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(|| c17_admitted_profiles().map_err(|error| error.to_string()))?
            .join()
            .map_err(|_| "C17 provisioning test panicked")?
            .map_err(Into::into)
    }

    fn c17_admitted_profiles() -> crate::bestiary_postgres_harness::TestResult {
        use crate::bestiary_postgres_harness::{self as fixture, Harness, debug};
        let Some(admin) = fixture::configured_admin() else {
            return Ok(());
        };
        fixture::runtime()?.block_on(async {
            let world = crate::foundation::WorldId::decode(&fixture::id(fixture::WORLD))?;
            let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../content/spells.manifest.json");
            let room = crate::content::qualify_native_entry_room_from_gameplay_manifest(world, &manifest)?;
            let native = crate::content::qualified_native_gameplay_test_state(room.compiled())?;
            for (profile, level) in [("c17a", 14), ("c17c", 23)] {
                let h = Harness::create_with_admission_seed(admin.clone(), profile, false, false).await?;
                admit_fixture(&h).await?;
                let seal = h.recovery.seal_current().map_err(debug)?;
                let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
                let mut config = NodeConfig::parse(super::super::super::config::tests::NODE.as_bytes())?;
                config.qualification_fixture = true;
                config.scope.world_id = uuid_text(&fixture::id(fixture::WORLD));
                config.scope.channel_id = uuid_text(&fixture::id(fixture::CHANNEL));
                let policy = native.progression().ok_or("C17 progression absent")?
                    .policy_for("profile-1", "ruleset-1", "content-1")?;
                // Normal admission binds this exact level-1, zero-XP row before
                // the control-socket fixture can resolve the admitted session.
                h.root.initialize_character_progression(&authority, &h.node, {
                    let current = crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                        .current_session(crate::foundation::GameSessionId::decode(&fixture::id(fixture::SESSION))?).await?;
                    let mut fence = fixture::fence(1)?;
                    fence.character_lease_generation = current.current_character_lease().generation();
                    fence
                },
                    crate::durability::character_progression::ProgressionInitializationRequest {
                        context: policy.context.clone(), policy_revision: policy.policy_revision.clone(),
                        reward_revision: policy.reward_revision.clone(), policy,
                    }).await.map_err(debug)?;
                let request = format!("qualify {} {profile}\n", uuid_text(&fixture::id(fixture::SESSION)));
                assert_eq!(prepare(&h.root, &authority, &h.node, &config, Some(&native), request.as_bytes()).await,
                    "qualified", "profile={profile}");
                let character = crate::domain::CharacterId::from_bytes(fixture::id(fixture::CHARACTER))?;
                assert!(matches!(crate::gameplay_transport::load_character_cast_facts(&h.root, &authority, character).await,
                    crate::gameplay_transport::CastFactsLoad::Ready { facts, .. }
                    if facts.level == level && facts.vocation == crate::spell::Vocation::Sorcerer && facts.max_mana >= 100));
                let revision = h.root_revision().await?;
                assert_eq!(prepare(&h.root, &authority, &h.node, &config, Some(&native), request.as_bytes()).await, "rejected");
                assert_eq!(h.root_revision().await?, revision, "one-shot rejection changed the Character");
                drop(authority);
                drop(seal);
                h.cleanup().await?;
            }
            Ok(())
        })
    }

    async fn admit_fixture(
        h: &crate::bestiary_postgres_harness::Harness,
    ) -> crate::bestiary_postgres_harness::TestResult {
        use crate::bestiary_postgres_harness::debug;
        use crate::foundation::admission_authority_publication::*;
        use oteryn_gameplay_dispatch::gameplay_transport::composed_test_owner::fresh_fixture;
        let now: i64 =
            sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
                .fetch_one(&h.pool)
                .await?;
        let mut runtime = fresh_fixture::fresh::Source::new(now)?;
        runtime
            .rows
            .retain(|row| matches!(row.key, AdmissionAuthorityGuardKeyV1::Runtime(_)));
        let runtime = AdmissionAuthorityPublicationV1::prepare(&runtime, now).map_err(debug)?;
        h.root
            .publish_runtime_readiness(&h.node, &runtime)
            .await
            .map_err(debug)?;
        let now: i64 =
            sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
                .fetch_one(&h.pool)
                .await?;
        let mut source = fresh_fixture::fresh::Source::new(now)?;
        let runtime_row = source
            .rows
            .iter_mut()
            .find(|row| matches!(row.key, AdmissionAuthorityGuardKeyV1::Runtime(_)))
            .ok_or("runtime observation absent")?;
        *runtime_row = runtime
            .changes()
            .first()
            .ok_or("runtime publication absent")?
            .clone();
        let mut admission = fresh_fixture::fresh::Source::new(now)?;
        admission
            .rows
            .retain(|row| !matches!(row.key, AdmissionAuthorityGuardKeyV1::Runtime(_)));
        let publication =
            AdmissionAuthorityPublicationV1::prepare(&admission, now).map_err(debug)?;
        crate::durability::admission_authority_guards::AdmissionGuardStore::from_root(
            h.root.clone(),
        )
        .publish(&publication)
        .await
        .map_err(debug)?;
        let result =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .commit(&source.request()?)
                .await
                .map_err(debug)?;
        assert!(matches!(result, crate::foundation::fresh_admission_durability::FreshAdmissionDurableOutcomeV1::Committed(_)), "{result:?}");
        Ok(())
    }

    #[test]
    fn qualification_items_resolve_from_the_real_served_catalogue()
    -> Result<(), Box<dyn std::error::Error>> {
        let world = crate::foundation::WorldId::decode(
            &super::super::super::uuid_bytes("01890f4c-3b2a-7c01-8d11-9a321b7c0004")
                .ok_or("fixture world")?,
        )?;
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../content/spells.manifest.json");
        let room =
            crate::content::qualify_native_entry_room_from_gameplay_manifest(world, &manifest)?;
        let native = crate::content::qualified_native_gameplay_test_state(room.compiled())?;
        for (words, level) in [("utani hur", 14), ("utevo res ina", 23)] {
            let spell = native
                .spell_book()
                .spoken(words)
                .ok_or("C17 spell absent")?
                .spell;
            assert_eq!(spell.level, level);
            assert!(!spell.premium);
            assert!(spell.vocations.contains(&crate::spell::Vocation::Sorcerer));
            let crate::spell::ManaCost::Fixed(cost) = spell.mana else {
                return Err("C17 spell must have fixed mana".into());
            };
            let (_, mana, _) = oteryn_gameplay_characters::vitals::maxima(
                "sorcerer",
                level,
                oteryn_protocol_oteryn::actor_spell::MAX_VITAL_POOL,
                oteryn_protocol_oteryn::actor_spell::MAX_SOUL,
            )
            .ok_or("C17 source vitals absent")?;
            assert!(mana >= cost);
        }
        let index = native.item_index().ok_or("Item index")?;
        for id in [3276, 3003, 3457] {
            let key = format!("oteryn:item.tibia.i{id}");
            let definition = crate::durability::item_mint::TypedDefinitionRef {
                family: "Item".into(),
                production_key: key.clone(),
                revision_ref: index.revision(&key).ok_or("Item missing")?.into(),
            };
            let facts = oteryn_gameplay_items::item_ref_admission::generation_item_facts(
                &native,
                &definition,
            )
            .map_err(|error| format!("Item {id}: {error:?}"))?;
            assert_ne!(
                facts.stack,
                crate::durability::item_transfer::ItemStackClass::Unknown
            );
        }
        Ok(())
    }

    #[test]
    fn qualification_request_accepts_only_named_profiles_and_v7_sessions() {
        let id = "01890f4c-3b2a-7c01-8d11-9a321b7c0004";
        for profile in [
            "explorer", "knight", "paladin", "sorcerer", "druid", "monk", "c17a", "c17c",
        ] {
            assert!(request(format!("qualify {id} {profile}\n").as_bytes()).is_some());
        }
        for text in [
            format!("qualify {id} master_sorcerer"),
            format!("qualify {id} knight extra"),
            format!("qualify {id} knight\nqualify {id} druid"),
            "qualify 01890f4c-3b2a-4c01-8d11-9a321b7c0004 knight".into(),
        ] {
            assert!(request(text.as_bytes()).is_none());
        }
    }
}
