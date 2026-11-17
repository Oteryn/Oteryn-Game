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
        "explorer" | "knight" | "paladin" | "sorcerer" | "druid" | "monk"
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
        Err(error) => {
            emit(PROCESS, &refusal_warning(&session, profile, error.as_ref()));
            "rejected"
        }
    }
}

/// This fixture emits only classified errors/static context, never request bytes or node proof.
fn refusal_warning(
    session: &[u8; 16],
    profile: &str,
    error: &(dyn std::error::Error + 'static),
) -> oteryn_error_codes::Line<'static> {
    use oteryn_error_codes::{FieldValue, Level, Line};
    let text = error.to_string();
    // Suppress common credential-bearing forms if a future adapter adds raw diagnostic text.
    let lower = text.to_ascii_lowercase();
    let sensitive = [
        "://",
        "password",
        "token:",
        "token=",
        "token(",
        "secret",
        "authorization:",
        "authorization=",
        "bearer ",
        "private key",
        "credential",
        "seed",
        "-----begin",
    ]
    .iter()
    .any(|marker| lower.contains(marker));
    let reason = if sensitive {
        "redacted_sensitive_error".to_owned()
    } else {
        text.chars().take(200).collect()
    };
    // Line's token renderer additionally bounds and sanitizes every extension field.
    Line::new(Level::Warn, "node", "qualification_fixture_refused")
        .field("reason", FieldValue::Owned(reason))
        .field("session", FieldValue::Owned(uuid_text(session)))
        .field("profile", FieldValue::Owned(profile.to_owned()))
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
    qualification_build::prepare_character_with_policy(
        root,
        authority,
        proof,
        &current,
        qualification_build::QualificationBuildSpec {
            vocation: if profile == "explorer" {
                "knight"
            } else {
                profile
            },
            level: 100,
            context,
            xp_occurrence: uuid_v7()?,
            build_occurrence: uuid_v7()?,
        },
        policy,
    )
    .await?;
    prepare_items(root, authority, proof, &current, native).await
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
    fn refusal_warning_retains_error_kind_and_validated_context() {
        let session = [0x12; 16];
        let error: Box<dyn std::error::Error> = "qualification admission: Unavailable".into();
        let line = refusal_warning(&session, "knight", error.as_ref());
        assert_eq!(line.level, oteryn_error_codes::Level::Warn);
        let rendered = line.render("test", 0);
        assert!(rendered.contains("event=qualification_fixture_refused"));
        assert!(rendered.contains("reason=qualification_admission:_Unavailable"));
        assert!(rendered.contains("session=12121212-1212-1212-1212-121212121212"));
        assert!(rendered.contains("profile=knight"));
        let error: Box<dyn std::error::Error> =
            "qualification admission: SessionUseAuthorization(AuthorityRejected)".into();
        let rendered = refusal_warning(&session, "knight", error.as_ref()).render("test", 0);
        assert!(rendered.contains("SessionUseAuthorization_AuthorityRejected_"));
        assert!(!rendered.contains("redacted_sensitive_error"));
    }

    #[test]
    fn refusal_warning_bounds_reason_and_cannot_inject_a_log_line() {
        let error: Box<dyn std::error::Error> = "invalid\n\r\x1b[31m 世界 ".repeat(100).into();
        let rendered = refusal_warning(&[0x12; 16], "druid", error.as_ref()).render("test", 0);
        let reason = rendered
            .split("reason=")
            .nth(1)
            .and_then(|s| s.split_whitespace().next());
        assert!(reason.is_some_and(|reason| reason.chars().count() <= 200 && reason.is_ascii()));
        assert_eq!(rendered.lines().count(), 1);
        assert!(!rendered.contains('\r') && !rendered.contains('\x1b'));
    }

    #[test]
    fn refusal_warning_redacts_sensitive_error_text() {
        for detail in [
            "postgresql://user:sensitive@host/database",
            "password=sensitive",
            "Token: sensitive",
            "secret=sensitive",
            "Authorization: Bearer sensitive",
            "private key sensitive",
            "credential=sensitive",
            "seed=sensitive",
            "-----BEGIN sensitive",
        ] {
            let error: Box<dyn std::error::Error> = detail.into();
            let rendered =
                refusal_warning(&[0x12; 16], "paladin", error.as_ref()).render("test", 0);
            assert!(rendered.contains("reason=redacted_sensitive_error"));
            assert!(!rendered.contains("sensitive@") && !rendered.contains("=sensitive"));
        }
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
        for profile in ["explorer", "knight", "paladin", "sorcerer", "druid", "monk"] {
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
