//! P13 client command -> current account projection -> native skills reply.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::foundation::ChannelContentPin;
use crate::gameplay_transport::capabilities::{OfferedCapability, SelectedCapabilities};
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::loyalty_skills::{
    CAPABILITY_LOYALTY_SKILLS_V1, COMMAND_TYPE_LOYALTY_SKILLS_QUERY, LoyaltySkills,
};

#[test]
fn loyalty_skills_postgres_real_dispatch_projection_replay_and_fences() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .name("loyalty-skills".into())
        .stack_size(128 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "loyalty skills scenario panicked")??;
    Ok(())
}

fn scenario() -> TestResult {
    use super::cyclopedia::support::exchange;
    run(async |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "loyalty_skills", true, false).await?;
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
        assert!(owner.loyalty_skills_available());
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
        let kind = COMMAND_TYPE_LOYALTY_SKILLS_QUERY;
        assert_eq!(
            exchange(&owner, &mut admitted, 1, kind, &[]).await?.0,
            CommandStatus::Rejected
        );
        let offers = [OfferedCapability {
            id: CAPABILITY_LOYALTY_SKILLS_V1,
            requires: &[],
        }];
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(&offers, &[CAPABILITY_LOYALTY_SKILLS_V1])
                .ok_or("loyalty skills capability")?;
        assert_eq!(
            exchange(&owner, &mut admitted, 2, kind, &[0]).await?.0,
            CommandStatus::Rejected
        );
        let (status, bytes) = exchange(&owner, &mut admitted, 3, kind, &[]).await?;
        assert_eq!(status, CommandStatus::Accepted);
        let zero = LoyaltySkills::decode(&bytes).ok_or("zero projection reply")?;
        assert_eq!((zero.points, zero.bonus_percent, zero.title), (0, 0, None));
        assert_eq!(
            zero.skills.map(|skill| skill.base),
            [30, 100, 100, 100, 100, 100, 100, 100]
        );
        assert!(
            zero.skills
                .iter()
                .all(|skill| skill.base == skill.with_loyalty)
        );

        // Another account's points never affect the requesting Character.
        sqlx::query(
            "INSERT INTO game_character_account_guards(account_id) VALUES(encode($1,'hex')::uuid)",
        )
        .bind(id(90).as_slice())
        .execute(&h.pool)
        .await?;
        sqlx::query("INSERT INTO game_account_loyalty_projections(account_id,loyalty_points,platform_revision) VALUES(encode($1,'hex')::uuid,7000,1)")
            .bind(id(90).as_slice()).execute(&h.pool).await?;
        assert_eq!(
            exchange(&owner, &mut admitted, 4, kind, &[]).await?,
            (CommandStatus::Accepted, bytes)
        );
        let account: Vec<u8> = sqlx::query_scalar(
            "SELECT uuid_send(account_id) FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
        ).bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        sqlx::query("INSERT INTO game_account_loyalty_projections(account_id,loyalty_points,platform_revision) VALUES(encode($1,'hex')::uuid,3600,1)")
            .bind(account.as_slice()).execute(&h.pool).await?;
        let (status, original) = exchange(&owner, &mut admitted, 5, kind, &[]).await?;
        assert_eq!(status, CommandStatus::Accepted);
        let loyal = LoyaltySkills::decode(&original).ok_or("loyal projection reply")?;
        assert_eq!(
            (loyal.points, loyal.bonus_percent, loyal.title.as_deref()),
            (3600, 50, Some("Keeper of Tibia"))
        );
        assert_eq!(
            loyal.skills.map(|skill| skill.base),
            [30, 100, 100, 100, 100, 100, 100, 100]
        );
        // Native paladin formula: magic x1.4, melee x1.2, distance/shield/fishing x1.1.
        // Adding 50% to cumulative progress gives these absolute levels, not +50% levels.
        assert_eq!(
            loyal.skills.map(|skill| skill.with_loyalty),
            [31, 102, 102, 102, 102, 104, 104, 104]
        );

        sqlx::query("UPDATE game_account_loyalty_projections SET loyalty_points=7000,platform_revision=2 WHERE account_id=encode($1,'hex')::uuid")
            .bind(account.as_slice()).execute(&h.pool).await?;
        assert_eq!(
            exchange(&owner, &mut admitted, 5, kind, &[]).await?,
            (CommandStatus::DuplicateReplay, original),
            "same-command replay returns the retained response",
        );
        let (status, current) = exchange(&owner, &mut admitted, 6, kind, &[]).await?;
        assert_eq!(status, CommandStatus::Accepted);
        let current = LoyaltySkills::decode(&current).ok_or("current projection reply")?;
        assert_eq!(
            (
                current.points,
                current.bonus_percent,
                current.title.as_deref()
            ),
            (7000, 50, Some("Enlightened of Tibia"))
        );
        assert_eq!(
            current.skills, loyal.skills,
            "the skill bonus is capped at fifty percent"
        );

        owner.active_generation = None;
        assert!(!owner.loyalty_skills_available());
        assert_eq!(
            exchange(&owner, &mut admitted, 7, kind, &[]).await?.0,
            CommandStatus::Rejected
        );
        owner.active_generation = Some(active);
        let mut stale = admitted;
        stale.continuity.connection_generation += 1;
        assert!(owner.loyalty_skills_page(stale, 8).await.is_none());
        let mut stale = admitted;
        let mut stale_fence = stale.item_fence.ok_or("item fence")?;
        stale_fence.character_lease_generation += 1;
        stale.item_fence = Some(stale_fence);
        assert!(
            owner.loyalty_skills_page(stale, 8).await.is_none(),
            "the SQL read checks the current lease"
        );
        let after: serde_json::Value = sqlx::query_scalar(
            "SELECT to_jsonb(b) FROM game_character_build_state b WHERE character_id=encode($1,'hex')::uuid",
        ).bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        let after_revision: String = sqlx::query_scalar(
            "SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
        ).bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(
            after, before,
            "display and replay never mutate trained progress"
        );
        assert_eq!(
            after_revision, before_revision,
            "the read never advances CharacterRevision"
        );
        let stored: (i64, i64) = sqlx::query_as(
            "SELECT loyalty_points,platform_revision FROM game_account_loyalty_projections WHERE account_id=encode($1,'hex')::uuid",
        ).bind(account.as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(
            stored,
            (7000, 2),
            "Game preserves Platform's projected points and revision"
        );
        eprintln!("LOYALTY POSTGRES_CASE_COMPLETED");
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

#[test]
fn loyalty_skills_postgres_current_instance_reads_its_projection_and_rejects_origin_authority()
-> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .name("loyalty-skills-instance".into())
        .stack_size(256 << 20)
        .spawn(|| instance_scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "loyalty skills Instance scenario panicked")??;
    Ok(())
}

fn instance_scenario() -> TestResult {
    use crate::durability::character_progression::CharacterProgressionError;
    use crate::foundation::{CommandId, CommandRef, RuntimeScopeRefV1};

    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "loyalty_skills_instance", true, false)
            .await?;
        // This fixture uses the existing control allocator, authenticated evidence,
        // ActivityHandoffOwner and current-session reader. Retaining it keeps the
        // evidence source alive; no session/assignment/handoff rows are fabricated.
        let entered = super::activity_runtime::enter_fixture(&h).await?;
        let fence = entered.fence;
        assert!(matches!(
            fence.runtime_scope,
            RuntimeScopeRefV1::Instance { .. }
        ));
        assert_eq!(fence.runtime_scope, entered.receipt.destination_scope);
        let source = entered.request.participants[0].source;
        let content_digest = entered.request.destination_content.server_artifact_digest;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let command = CommandRef::new(fence.game_session_id, CommandId::new(2).map_err(debug)?);

        // A nonzero stored training remainder proves this read neither relevels nor
        // rewrites durable progress when returning the Platform-owned projection.
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO game_character_build_state(character_id,vocation,magic_level,mana_spent,fist_level,fist_tries,club_level,club_tries,sword_level,sword_tries,axe_level,axe_tries,distance_level,distance_tries,shielding_level,shielding_tries,fishing_level,fishing_tries,committed_character_revision,last_build_occurrence_id) VALUES(encode($1,'hex')::uuid,'paladin',30,123,100,321,100,321,100,321,100,321,100,321,100,321,100,321,2,encode($2,'hex')::uuid) ON CONFLICT(character_id) DO UPDATE SET vocation='paladin',magic_level=30,mana_spent=123,fist_level=100,fist_tries=321,club_level=100,club_tries=321,sword_level=100,sword_tries=321,axe_level=100,axe_tries=321,distance_level=100,distance_tries=321,shielding_level=100,shielding_tries=321,fishing_level=100,fishing_tries=321")
            .bind(fence.character_id.as_bytes().as_slice()).bind(id(30).as_slice())
            .execute(&mut *tx).await?;
        tx.commit().await?;
        let account: Vec<u8> = sqlx::query_scalar(
            "SELECT uuid_send(account_id) FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
        ).bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
        sqlx::query("INSERT INTO game_account_loyalty_projections(account_id,loyalty_points,platform_revision) VALUES(encode($1,'hex')::uuid,3600,1)")
            .bind(account.as_slice()).execute(&h.pool).await?;
        let before: (serde_json::Value, String) = sqlx::query_as(
            "SELECT to_jsonb(b),r.character_revision::text FROM game_character_build_state b JOIN game_character_roots r USING(character_id) WHERE character_id=encode($1,'hex')::uuid",
        ).bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;

        let (build, points) = h
            .root
            .read_loyalty_build_state(&authority, &h.node, &fence, command, content_digest)
            .await
            .map_err(debug)?;
        assert_eq!(points, 3_600);
        assert_eq!(build.vocation(), "paladin");
        assert_eq!(build.magic(), (30, 123));
        assert_eq!(build.skills(), [(100, 321); 7]);

        let source_command =
            CommandRef::new(source.game_session_id, CommandId::new(2).map_err(debug)?);
        assert!(
            matches!(
                h.root
                    .read_loyalty_build_state(
                        &authority,
                        &h.node,
                        &source,
                        source_command,
                        entered.request.participants[0]
                            .source_position
                            .server_artifact_digest,
                    )
                    .await,
                Err(CharacterProgressionError::AuthorityRejected)
            ),
            "the terminated origin Channel session cannot read after handoff"
        );
        let mut origin_scope = fence;
        origin_scope.runtime_scope = source.runtime_scope;
        assert!(
            matches!(
                h.root
                    .read_loyalty_build_state(
                        &authority,
                        &h.node,
                        &origin_scope,
                        command,
                        content_digest,
                    )
                    .await,
                Err(CharacterProgressionError::AuthorityRejected)
            ),
            "the current Instance session cannot borrow its origin Channel scope"
        );
        let mut wrong_content = content_digest;
        wrong_content[0] ^= 1;
        assert!(
            matches!(
                h.root
                    .read_loyalty_build_state(&authority, &h.node, &fence, command, wrong_content,)
                    .await,
                Err(CharacterProgressionError::AuthorityRejected)
            ),
            "Instance reads require the retained destination content binding"
        );

        // Refusals leave the live Instance fence usable and preserve both owners'
        // durable values. The read does not advance CharacterRevision.
        assert_eq!(
            h.root
                .read_loyalty_build_state(&authority, &h.node, &fence, command, content_digest,)
                .await
                .map_err(debug)?,
            (build, points)
        );
        let after: (serde_json::Value, String) = sqlx::query_as(
            "SELECT to_jsonb(b),r.character_revision::text FROM game_character_build_state b JOIN game_character_roots r USING(character_id) WHERE character_id=encode($1,'hex')::uuid",
        ).bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(after, before);
        let projection: (i64, i64) = sqlx::query_as(
            "SELECT loyalty_points,platform_revision FROM game_account_loyalty_projections WHERE account_id=encode($1,'hex')::uuid",
        ).bind(account.as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(projection, (3_600, 1));
        eprintln!("LOYALTY_INSTANCE POSTGRES_CASE_COMPLETED");
        drop(authority);
        drop(seal);
        drop(entered);
        h.cleanup().await
    })
}
