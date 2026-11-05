//! P13 client command -> current account projection -> native skills reply.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::foundation::ChannelContentPin;
use crate::gameplay_transport::capabilities::{OfferedCapability, SelectedCapabilities};
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::skill_progress::{
    CAPABILITY_SKILL_PROGRESS_V1, COMMAND_TYPE_SKILL_PROGRESS_QUERY, SkillProgressSnapshot,
};

#[test]
fn skill_progress_postgres_real_dispatch_projection_replay_and_fences() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .name("skill-progress".into())
        .stack_size(128 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "skill progress scenario panicked")??;
    Ok(())
}

fn scenario() -> TestResult {
    use super::cyclopedia::support::exchange;
    run(async |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "skill_progress", true, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, pin) =
            crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None);
        let active = controller.active().ok_or("active generation")?;
        let channel_pin = ChannelContentPin::from_activation(
            world,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            pin.frame_binding().digest(),
            pin.qualified_room().map_revision_digest(),
            (2, 0, 7),
        );
        let deps = Dependencies::with_content(
            channel_pin,
            crate::content::qualify_native_entry_room(world).map_err(debug)?,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        deps.runtime
            .lock()
            .await
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        assert!(owner.skill_progress_available());
        deps.initialize(&owner).await;
        let fence = owner
            .current_quest_fence(deps.admitted.game_session_id)
            .await
            .map_err(|_| "current fence")?
            .ok_or("session")?;

        // The fixture supplies a committed Character build. The display does not
        // require an equipment owner and cannot modify trained progress.
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO game_character_build_state(character_id,vocation,magic_level,mana_spent,fist_level,fist_tries,club_level,club_tries,sword_level,sword_tries,axe_level,axe_tries,distance_level,distance_tries,shielding_level,shielding_tries,fishing_level,fishing_tries,committed_character_revision,last_build_occurrence_id) VALUES(encode($1,'hex')::uuid,'paladin',30,0,100,0,100,0,100,0,100,0,100,0,100,0,100,0,2,encode($2,'hex')::uuid) ON CONFLICT(character_id) DO UPDATE SET vocation='paladin',magic_level=30,mana_spent=0,fist_level=100,fist_tries=0,club_level=100,club_tries=0,sword_level=100,sword_tries=0,axe_level=100,axe_tries=0,distance_level=100,distance_tries=0,shielding_level=100,shielding_tries=0,fishing_level=100,fishing_tries=0")
            .bind(id(41).as_slice()).bind(id(30).as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
        let equipment_rows: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_equipment_state WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(
            equipment_rows, 0,
            "skills display must not require initialized equipment"
        );
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice())
            .bind(active.identity().server_artifact_digest().as_slice())
            .bind(active.identity().client_artifact_digest().as_slice())
            .bind(pin.frame_binding().digest().as_slice()).execute(&h.pool).await?;
        let before: serde_json::Value = sqlx::query_scalar(
            "SELECT to_jsonb(b) FROM game_character_build_state b WHERE character_id=encode($1,'hex')::uuid",
        ).bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        let before_revision: String = sqlx::query_scalar(
            "SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
        ).bind(id(41).as_slice()).fetch_one(&h.pool).await?;

        let mut admitted = deps.admitted;
        admitted.item_fence = Some(fence.item_fence());
        admitted.first_entry = FirstEntryOutcome::Positioned;
        let kind = COMMAND_TYPE_SKILL_PROGRESS_QUERY;
        assert_eq!(
            exchange(&owner, &mut admitted, 1, kind, &[]).await?.0,
            CommandStatus::Rejected
        );
        let offers = [OfferedCapability {
            id: CAPABILITY_SKILL_PROGRESS_V1,
            requires: &[],
        }];
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(&offers, &[CAPABILITY_SKILL_PROGRESS_V1])
                .ok_or("skill progress capability")?;
        assert_eq!(
            exchange(&owner, &mut admitted, 2, kind, &[0]).await?.0,
            CommandStatus::Rejected
        );
        let (status, bytes) = exchange(&owner, &mut admitted, 3, kind, &[]).await?;
        assert_eq!(status, CommandStatus::Accepted);
        let zero = SkillProgressSnapshot::decode(&bytes).ok_or("progress projection")?;
        assert_eq!(
            zero.skills.map(|skill| skill.level),
            [30, 100, 100, 100, 100, 100, 100, 100]
        );
        assert!(
            zero.skills
                .iter()
                .all(|skill| skill.progress == 0 && skill.percent == 0 && skill.required > 0)
        );
        assert_eq!(
            exchange(&owner, &mut admitted, 3, kind, &[]).await?,
            (CommandStatus::DuplicateReplay, bytes)
        );
        let after_zero: serde_json::Value = sqlx::query_scalar("SELECT to_jsonb(b) FROM game_character_build_state b WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(
            after_zero, before,
            "projection and replay preserve the complete build"
        );
        let formula = active
            .native_gameplay()
            .ok_or("native")?
            .training_formula()
            .ok_or("formula")?;
        use oteryn_gameplay_domain::domain::character_build::BuildFormula;
        for (family, skill) in zero.skills.into_iter().enumerate() {
            assert_eq!(
                Some(skill.required),
                formula.required("paladin", family, skill.level + 1)
            );
        }
        let magic_required = formula.required("paladin", 0, 31).ok_or("magic required")?;
        let sword_required = formula
            .required("paladin", 3, 101)
            .ok_or("sword required")?;
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE game_character_build_state SET mana_spent=$1,sword_tries=$2 WHERE character_id=encode($3,'hex')::uuid")
            .bind(i64::try_from(magic_required / 2)?).bind(i64::try_from(sword_required / 4)?).bind(id(41).as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
        let (status, bytes) = exchange(&owner, &mut admitted, 4, kind, &[]).await?;
        assert_eq!(status, CommandStatus::Accepted);
        let trained = SkillProgressSnapshot::decode(&bytes).ok_or("trained progress")?;
        assert_eq!(trained.skills[0].progress, magic_required / 2);
        assert_eq!(trained.skills[0].required, magic_required);
        assert_eq!(trained.skills[0].percent, 5_000);
        assert_eq!(trained.skills[3].progress, sword_required / 4);
        assert_eq!(trained.skills[3].required, sword_required);
        assert_eq!(trained.skills[3].percent, 2_500);
        let mut stale = admitted;
        stale.continuity.connection_generation += 1;
        assert!(owner.skill_progress_page(stale, 5).await.is_none());
        let mut stale = admitted;
        let mut stale_fence = stale.item_fence.ok_or("item fence")?;
        stale_fence.character_lease_generation += 1;
        stale.item_fence = Some(stale_fence);
        assert!(owner.skill_progress_page(stale, 5).await.is_none());
        owner.active_generation = None;
        assert_eq!(
            exchange(&owner, &mut admitted, 5, kind, &[]).await?.0,
            CommandStatus::Rejected
        );
        let revision: String = sqlx::query_scalar("SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(
            revision, before_revision,
            "read never advances CharacterRevision"
        );
        eprintln!("SKILL_PROGRESS POSTGRES_CASE_COMPLETED");
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
