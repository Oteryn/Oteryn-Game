//! Native spell effects against a PostgreSQL-assigned instance.
use super::fixture::{Harness, debug, run};
use super::{TestResult, reset};
use crate::ability::{AbilityOccurrence, AiAbilityAdapter, RevisionSet};
use crate::ai_think::profile_schedule::{ProfileAbilityProposal, ScheduleList};
use crate::content::{ProjectV2DefinitionRef, ProjectV2Family};
use crate::foundation::{
    ChannelRuntimeV1, CompiledCreaturePolicies, ExactActorRef, MovementLocalPosition,
};

#[test]
fn q25years_retro_propagates_native_spell_damage_in_instance() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(async move |_| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "q25years_retro", true, false).await?;
        let world = super::fixture::fence(1)?.runtime_scope.world_id();
        let (controller, activated) =
            super::death_ml_loss_composed::production_generation_with_pin(world, "content-1")?;
        let active = controller.active().ok_or("active content")?;
        let native = active.native_gameplay().ok_or("native content")?;
        let prototype = native
            .policies()
            .policies()
            .iter()
            .find(|policy| policy.definition_key == "oteryn:creature.rat")
            .ok_or("rat policy")?;
        let start = activated.qualified_room().entry_start();
        let pin = activated.into_channel_pin();
        super::boss_activity::issue_content_and_reset(&h, &pin).await?;
        let scope = super::activity_runtime::allocate_ready_instance(&h, 88).await?;
        let assignment = h
            .root
            .read_runtime_scope_assignment(scope)
            .await
            .map_err(debug)?
            .ok_or("instance assignment")?;

        let mut runtime = ChannelRuntimeV1::from_committed_scope_assignment(
            scope,
            h.node.fact().node_id(),
            h.node.fact().registration_revision(),
            assignment.ownership_generation,
            assignment.source_revision,
            &assignment.decision_identity,
            8,
            pin.clone(),
        )
        .map_err(debug)?;
        let creatures = [
            "time_distortion",
            "hard_times",
            "blissful_times",
            "parcel_castle",
        ];
        let policies = creatures
            .iter()
            .map(|name| {
                let mut policy = prototype.as_ref().clone();
                policy.definition_key = format!("oteryn:creature.{name}");
                policy.display_name = name.to_string();
                policy.maximum_health = 1_000;
                policy
            })
            .collect();
        runtime
            .install_companion_policies(
                CompiledCreaturePolicies::from_active_artifact(
                    pin.server_artifact_digest(),
                    policies,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        let mut spawn = |name: &str, dx| -> TestResult<ExactActorRef> {
            let key = format!("oteryn:creature.{name}");
            let at = MovementLocalPosition {
                x: start.x + dx,
                y: start.y,
                floor: start.floor,
            };
            let actor = runtime
                .admit_encounter_actor(&key, 1_000, at)
                .map_err(|error| format!("admit {key} at {at:?}: {error:?}"))?;
            runtime
                .install_creature_policy(actor, &key)
                .map_err(|error| format!("install {key}: {error:?}"))?;
            Ok(actor)
        };
        let distortion = spawn("time_distortion", 0)?;
        let first = spawn("hard_times", 2)?;
        let second = spawn("hard_times", 4)?;
        let castle = spawn("parcel_castle", 6)?;
        let lethal_blast = spawn("blissful_times", 8)?;
        let distant_castle = spawn("parcel_castle", 11)?;

        cast(&mut runtime, distortion, "retro_time_distortion")?;
        assert!(runtime.matches_live_creature_identity(first, b"oteryn:creature.blissful_times"));
        assert!(runtime.matches_live_creature_identity(second, b"oteryn:creature.hard_times"));
        cast(&mut runtime, first, "retro_blissful_blast")?;
        assert!(!runtime.contains_live_creature(first));
        assert!(runtime.matches_live_creature_identity(second, b"oteryn:creature.blissful_times"));
        assert_eq!(runtime.live_creature_health(castle), Some(1_000));
        cast(&mut runtime, second, "retro_blissful_blast")?;
        assert!(!runtime.contains_live_creature(second));
        assert_eq!(runtime.live_creature_health(castle), Some(500));
        cast(&mut runtime, second, "retro_blissful_blast")?;
        assert_eq!(runtime.live_creature_health(castle), Some(500));

        cast(&mut runtime, lethal_blast, "retro_blissful_blast")?;
        assert!(!runtime.contains_live_creature(castle));
        assert!(!runtime.contains_live_creature(lethal_blast));
        assert_eq!(runtime.live_creature_health(distant_castle), Some(1_000));
        Ok(())
    })
}

fn cast(runtime: &mut ChannelRuntimeV1, issuer: ExactActorRef, spell: &str) -> TestResult {
    let hex: String = issuer
        .placement_identity()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let actor = format!("actor:{hex}");
    let proposal = ProfileAbilityProposal {
        issuer,
        target: issuer,
        ability: ProjectV2DefinitionRef {
            family: ProjectV2Family::Ability,
            key: format!("oteryn:ability.spell.{spell}"),
            revision: "definition-r1".into(),
        },
        list: ScheduleList::Defence,
        entry_index: 0,
        magnitude: None,
        range_tiles: 2,
        occurrence: AbilityOccurrence::new(
            &format!("ai-profile:{hex}:1:defence:0"),
            RevisionSet::new(
                "ruleset-1",
                "content-1",
                "world-1",
                "definition-r1",
                "sim-1",
            )
            .map_err(debug)?,
        )
        .map_err(debug)?,
        intent: AiAbilityAdapter::normalize(&actor, &[&actor]).map_err(debug)?,
    };
    assert!(
        crate::gameplay_transport::retro_combat::execute(runtime, &proposal, 1_000_000)
            .map_err(|error| format!("dispatch {spell} from {issuer:?}: {error:?}"))?
    );
    Ok(())
}
