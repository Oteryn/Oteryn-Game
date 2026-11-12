//! The source timer policy must be admitted before the ruby can transform a helmet.
use super::{ComposedFreshAdmission, TestResult, fixture::Harness};

#[test]
fn l6_ruby_profile_admits_exact_helmet_absolute_policy() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    eprintln!("POSTGRES_CASE_STARTED l6-ruby-profile");
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            super::quest_world_callbacks::scenario(
                "l6-ruby-profile",
                oteryn_gameplay_quests::quest::world_callbacks::Position {
                    x: 7,
                    y: 1,
                    floor: -7,
                },
                false,
            )
            .map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "ruby profile scenario panicked")??;
    eprintln!(
        "POSTGRES_CASE_COMPLETED l6-ruby-profile duration={:?}",
        started.elapsed()
    );
    Ok(())
}

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    harness: &Harness,
) -> TestResult {
    eprintln!("POSTGRES_DATABASE_READY l6-ruby-profile");
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let policy = native
        .item_policy_for_source_id(3230)
        .ok_or("full helmet3230 native policy missing")?;
    assert_eq!(
        policy.record().semantics.charges,
        crate::content::ReferenceItemField::NotApplicable
    );
    let qualified =
        crate::durability::spell_items_abi::QualifiedItemDefinition::from_native_policy(policy)?;
    assert!(
        qualified.materializable
            && qualified.movable
            && qualified.inventory_destination
            && qualified.ground_destination
    );
    assert_eq!(
        qualified.definition.production_key,
        "oteryn:item.tibia.i3230"
    );
    assert_eq!(qualified.stack_maximum, 1);
    assert!(qualified.container_capacity.is_none());
    let decay = qualified.decay.ok_or("exact native helmet deadline")?;
    assert_eq!(decay.duration_millis, 1_800_000);
    let target = decay.target.ok_or("helmet decay target")?;
    assert_eq!(target.production_key, "oteryn:item.tibia.i3229");
    assert_eq!(target.revision_ref, "definition-r1");
    let active: Vec<u8> = sqlx::query_scalar("SELECT server_artifact_digest FROM game_content_activations WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid ORDER BY activation_sequence DESC LIMIT 1")
        .bind(owner.world_id.as_bytes().as_slice()).bind(owner.channel_id.as_bytes().as_slice()).fetch_one(&harness.pool).await?;
    assert_eq!(active, native.source_digest());
    Ok(())
}
