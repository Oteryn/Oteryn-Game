//! Disposable node-boot build provisioning. The node retains its incarnation
//! proof; the root-only control socket supplies only an admitted session id.
use super::*;
use crate::content::character_progression_content::CharacterProgressionContent;
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
    progression: Option<&CharacterProgressionContent>,
    bytes: &[u8],
) -> &'static str {
    if !config.qualification_fixture {
        return "rejected";
    }
    let Some(progression) = progression else {
        return "rejected";
    };
    let Some((session, profile)) = request(bytes) else {
        return "rejected";
    };
    match prepare_build(
        root,
        authority,
        proof,
        config,
        progression,
        session,
        profile,
    )
    .await
    {
        Ok(()) => "qualified",
        Err(_) => "rejected",
    }
}

async fn prepare_build(
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    proof: &NodeIncarnationProof,
    config: &NodeConfig,
    progression: &CharacterProgressionContent,
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
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

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
