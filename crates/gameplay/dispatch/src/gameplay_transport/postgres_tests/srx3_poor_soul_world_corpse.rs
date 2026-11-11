//! A real World XML death uses its published bundle frame, not the entry frame.
use super::fixture::{Harness, debug, run};
use super::{Dependencies, TestResult};
use crate::foundation::{ChannelContentPin, CreatureDeathOccurrenceKey, MovementLocalPosition};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;

const POOR_SOUL: &str = "oteryn:creature.poor_soul";

#[test]
fn srx3_poor_soul_actual_world_uncredited_corpse_accepts_current_bundle_frame_postgres()
-> TestResult {
    world_corpse(false)
}

#[test]
fn srx3_poor_soul_actual_world_uncredited_corpse_rejects_mismatched_bundle_reset_map_postgres()
-> TestResult {
    world_corpse(true)
}

fn world_corpse(mismatched_reset_map: bool) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("srx3-poor-soul-world-corpse", move || {
        run(async move |admin| {
            super::reset();
            // Crystal201189f world-monster.xml:36787. Select the original
            // source from the actual published bundle; do not synthesize a
            // spawn table, source callback, corpse or reward principal.
            let (scene, world, entry_pin) =
                super::srx3_soul_war_world_fixture::source_world_with_bundle(
                    MovementLocalPosition {
                        x: 33851,
                        y: 31870,
                        floor: 7,
                    },
                )?;
            assert_eq!(
                entry_pin.server_artifact_digest(),
                scene.pin.server_artifact_digest()
            );
            assert_eq!(
                entry_pin.client_artifact_digest(),
                scene.pin.client_artifact_digest()
            );
            assert_ne!(
                entry_pin.frame_binding_digest(),
                scene.pin.frame_binding_digest(),
                "entry activation and published World retain different real frames"
            );
            let active = scene.controller.active().ok_or("actual World activation")?;
            let native = active
                .native_gameplay()
                .ok_or("actual World native policies")?;
            let mut deps = Dependencies::with_content_and_capacity(
                scene.pin.clone(),
                scene.room.clone(),
                crate::spell::cast::v1_spell_book().map_err(debug)?,
                16,
            )?;
            let poor_soul = {
                let runtime = deps.runtime.get_mut();
                native.install_companion_policies(runtime).map_err(debug)?;
                let mut sources = world
                    .activation_spawns(|key| {
                        let policy = runtime.companion_policy(key).ok()?;
                        Some((
                            u64::try_from(policy.maximum_health).ok()?,
                            policy.blocked_by_nearby_players,
                        ))
                    })
                    .ok_or("actual published World spawn policies")?;
                sources.retain(|(points, _)| {
                    points
                        .iter()
                        .any(|(at, _, key, _, _)| *at == (33851, 31870, 7) && key == POOR_SOUL)
                });
                assert_eq!(sources.len(), 1, "one original XML source");
                assert_eq!(sources[0].0.len(), 1, "preserve the complete source cohort");
                assert_eq!(sources[0].0[0].1, 10_000, "actual XML respawn delay");
                assert_eq!(
                    runtime
                        .begin_activation_spawn_sources(sources)
                        .map_err(debug)?,
                    1
                );
                let realized = runtime.realize_activation_spawn_window(0).map_err(debug)?;
                assert_eq!(realized.realized, 1);
                assert_eq!(realized.remaining, 0);
                let actors = runtime.current_live_creature_census().map_err(debug)?;
                assert_eq!(actors.len(), 1);
                let actor = actors[0];
                assert!(runtime.matches_live_creature_identity(actor, POOR_SOUL.as_bytes()));
                assert_eq!(
                    runtime
                        .read_actor_position(actor)
                        .map_err(debug)?
                        .position(),
                    MovementLocalPosition {
                        x: 33851,
                        y: 31870,
                        floor: 7
                    }
                );
                actor
            };
            let h = Harness::create_with_admission_seed(
                admin,
                "srx3_poor_soul_world_corpse",
                false,
                false,
            )
            .await?;
            let evidence = super::house_transfer::RecoverySource::start(&h).await?;
            super::house_transfer::prepare_owned_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let mut owner = deps.owner(&h, &authority);
            owner.evidence = &evidence.evidence;
            owner.active_generation = Some(active);
            owner.rooms = crate::gameplay_transport::SeamRooms::new(None, Some(&scene.room));
            issue_world_pins(&h, &entry_pin, &scene.pin, &world, mismatched_reset_map).await?;
            let mut admitted = deps.admitted;
            super::srx3_activity_source_fixture::prepare_player(&h, &owner, &mut admitted, 8)
                .await?;
            let now = owner.owner_now();
            let begin = {
                let mut runtime = owner.runtime.lock().await;
                assert!(
                    runtime
                        .source_player_damage_shares(poor_soul)
                        .map_err(debug)?
                        .is_empty()
                );
                let health = runtime.companion_snapshot(poor_soul).map_err(debug)?.health;
                let hit = runtime
                    .borrow_exact_actor_commit_at_us(now.get())
                    .commit_damage(
                        poor_soul,
                        crate::foundation::OwnerDamageCommand {
                            target: POOR_SOUL.as_bytes(),
                            occurrence: b"srx3:poor-soul:world-source-death",
                            binding: b"srx3:poor-soul:world-source-death\0srx3:poor-soul:actual-xml-owner",
                            damage: health,
                        },
                    )
                    .map_err(debug)?;
                assert!(hit.applied && hit.health_before > 0 && hit.health_after == 0);
                let lethal = runtime
                    .borrow_combat_death()
                    .committed_lethal_receipt(poor_soul)
                    .map_err(debug)?;
                runtime
                    .borrow_combat_death()
                    .project_committed_lethal(lethal)
                    .map_err(debug)?;
                let notice = runtime
                    .next_source_creature_death()
                    .map_err(debug)?
                    .ok_or("actual XML actor must enqueue its qualified source death")?;
                assert_eq!(notice.actor(), poor_soul);
                assert_eq!(notice.enqueued_at_us(), Some(now.get()));
                let begin = runtime
                    .begin_source_creature_death(&notice)
                    .map_err(debug)?;
                assert!(!begin.has_reward_damage_credit());
                assert!(
                    begin.events().is_empty(),
                    "the copied Poor Soul vector is empty"
                );
                assert_eq!(
                    begin.uncredited_corpse_item_key(),
                    Some("oteryn:item.tibia.i33891")
                );
                assert_eq!(begin.frame_digest(), scene.pin.frame_binding_digest());
                begin
            };
            if mismatched_reset_map {
                assert_fresh_mint_rejected(&h, &owner, native, &begin).await?;
                drop(owner);
                drop(authority);
                drop(seal);
                return h.cleanup().await;
            }
            super::srx3_native_source_cast::drain_source_death(&owner, poor_soul, now).await?;
            {
                let runtime = owner.runtime.lock().await;
                assert!(!runtime.corpse_projection_pinned(poor_soul));
                assert!(
                    runtime
                        .source_registered_creature_census()
                        .map_err(debug)?
                        .iter()
                        .all(|(actor, _)| *actor != poor_soul)
                );
            }
            let rows = receipts(&h, begin.death_key()).await?;
            assert_eq!(rows.len(), 1, "one canonical body and no invented loot");
            let body = &rows[0];
            assert_eq!(body.0, "CORPSE_MATERIALIZATION");
            assert_eq!(body.5, "oteryn:item.tibia.i33891");
            assert_eq!(body.6, 1);
            assert!(body.7.is_empty(), "no Character supplies uncredited damage");
            let (proof, same_xid, exact_ground): (String, bool, bool) = sqlx::query_as(
                "SELECT convert_from(w.proof,'UTF8'), \
                 w.binding=sha256(w.proof) AND w.created_xact_id=r.created_xact_id \
                   AND w.created_xact_id=a.created_xact_id, \
                 g.scope_kind=1 AND g.instance_id IS NULL AND g.channel_id=r.death_channel_id \
                   AND g.world_id=r.death_world_id AND g.runtime_scope_ownership_generation=r.death_scope_ownership_generation \
                   AND g.native_room_placement_context=$2 AND g.map_revision=$3 AND g.content_revision=$4 \
                   AND g.corpse_ref=$5 \
                 FROM game_source_corpse_mints w JOIN game_item_mint_receipts r USING(transaction_id) \
                 JOIN game_item_audit_outbox a USING(transaction_id) \
                 JOIN game_item_ground_locations g ON g.item_instance_id=w.item_instance_id \
                 WHERE w.transaction_id=$1::uuid",
            )
            .bind(&body.2)
            .bind(scene.pin.frame_binding_digest().as_slice())
            .bind(format!("sha256:{}", hex(scene.pin.map_revision_digest())))
            .bind(format!("sha256:{}", hex(scene.pin.server_artifact_digest())))
            .bind(crate::durability::party_kill_xp::encode_death_key(begin.death_key()))
            .fetch_one(&h.pool).await?;
            assert!(
                same_xid && exact_ground,
                "the real completion and physical writer share exact scope and transaction"
            );
            let proof: serde_json::Value = serde_json::from_str(&proof)?;
            assert_eq!(proof["creature_key"], POOR_SOUL);
            assert_eq!(proof["events"], serde_json::json!([]));
            assert_eq!(proof["has_reward_damage_credit"], false);
            assert_eq!(proof["frame"], hex(scene.pin.frame_binding_digest()));
            assert_eq!(proof["map"], hex(scene.pin.map_revision_digest()));
            assert_eq!(proof["client"], hex(scene.pin.client_artifact_digest()));
            assert!(proof["bakragore_loot"].is_null());
            let invented_reward: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM game_party_kill_xp_receipts WHERE death_key=$1",
            )
            .bind(crate::durability::party_kill_xp::encode_death_key(
                begin.death_key(),
            ))
            .fetch_one(&h.pool)
            .await?;
            assert_eq!(invented_reward, 0);
            owner.drain_monster_melee_at(now).await;
            assert_eq!(
                receipts(&h, begin.death_key()).await?,
                rows,
                "replaying the canonical owner preserves event, transaction, Item and intent identities"
            );
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    })?;
    if mismatched_reset_map {
        eprintln!(
            "SRX3_POOR_SOUL_WORLD_CORPSE_RESET_MAP_REFUSED_POSTGRES_CASE_EXECUTED duration={:?}",
            started.elapsed()
        );
    } else {
        eprintln!(
            "SRX3_POOR_SOUL_WORLD_CORPSE_POSTGRES_CASE_EXECUTED duration={:?}",
            started.elapsed()
        );
    }
    Ok(())
}

async fn assert_fresh_mint_rejected(
    h: &Harness,
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    native: &crate::content::native_gameplay::NativeGameplayState,
    begin: &oteryn_gameplay_entities::carrier::native_source_death::SourceCreatureDeathSeal,
) -> TestResult {
    use crate::durability::item_mint::{
        CORPSE_MATERIALIZATION_PURPOSE_KEY, GroundPlacement, ItemMintCause, ItemMintError,
        ItemMintRequest, TypedDefinitionRef,
    };

    assert!(receipts(h, begin.death_key()).await?.is_empty());
    let permit = owner
        .spell_lane_permit()
        .await
        .ok_or("source corpse MINT lane")?;
    let completed = {
        let mut runtime = owner.runtime.lock().await;
        assert!(
            begin.events().is_empty(),
            "no copied handler may be skipped"
        );
        runtime
            .pin_uncredited_source_corpse_projection(begin)
            .map_err(debug)?;
        runtime
            .finish_source_creature_death_task(begin)
            .map_err(debug)?
    };
    let row = native
        .reward_table()
        .row(begin.definition_key())
        .map_err(debug)?;
    let corpse = row
        .corpse_item
        .as_ref()
        .ok_or("actual Poor Soul corpse definition")?;
    assert_eq!(
        Some(corpse.production_key.as_str()),
        begin.uncredited_corpse_item_key()
    );
    let item = TypedDefinitionRef {
        family: corpse.family.clone(),
        production_key: corpse.production_key.clone(),
        revision_ref: corpse.revision_ref.clone(),
    };
    let position = begin.position();
    let mut spatial_position = Vec::with_capacity(10);
    spatial_position.extend_from_slice(&position.x.to_be_bytes());
    spatial_position.extend_from_slice(&position.y.to_be_bytes());
    spatial_position.extend_from_slice(&position.floor.to_be_bytes());
    let mut candidate = owner
        .root
        .freeze_item_mint(
            owner.character,
            owner.holder,
            ItemMintRequest {
                cause: ItemMintCause::from_creature_death(
                    begin.death_key(),
                    item.clone(),
                    CORPSE_MATERIALIZATION_PURPOSE_KEY.into(),
                    0,
                )
                .map_err(debug)?,
                item,
                quantity: 1,
                ground: GroundPlacement {
                    spatial_position,
                    corpse_ref: crate::durability::party_kill_xp::encode_death_key(
                        begin.death_key(),
                    ),
                    map_revision: format!("sha256:{}", hex(begin.map_digest())),
                    content_revision: format!("sha256:{}", hex(begin.content_digest())),
                    native_room_placement_context: begin.frame_digest().to_vec(),
                },
                content_revision: format!("sha256:{}", hex(begin.content_digest())),
                ruleset_revision: crate::interaction_chest_use::entry_chest::RULESET_REVISION
                    .into(),
                sim_revision: crate::interaction_chest_use::entry_chest::SIM_REVISION.into(),
            },
        )
        .await
        .map_err(debug)?;
    assert_eq!(mint_effect_counts(h, &candidate).await?, (0, 0, 0, 0, 0));
    let result = owner
        .root
        .commit_uncredited_source_corpse_mint(
            &permit,
            owner.character,
            owner.holder,
            native,
            &completed,
            &mut candidate,
        )
        .await;
    // Commit errors deliberately erase SQLSTATE/message. Require the typed
    // deferred-SQL rejection, not an earlier Rust refusal or receipt replay.
    assert!(
        matches!(
            &result,
            Err(ItemMintError::Unavailable(
                crate::durability::DurabilityError::CommitRejected
            ))
        ),
        "fresh source corpse MINT must be rejected at COMMIT: {result:?}"
    );
    assert_eq!(mint_effect_counts(h, &candidate).await?, (0, 0, 0, 0, 0));
    assert!(receipts(h, begin.death_key()).await?.is_empty());
    assert!(
        owner
            .runtime
            .lock()
            .await
            .corpse_projection_pinned(begin.actor())
    );
    Ok(())
}

async fn mint_effect_counts(
    h: &Harness,
    candidate: &crate::durability::item_mint::ItemMintCandidate,
) -> TestResult<(i64, i64, i64, i64, i64)> {
    Ok(sqlx::query_as(
        "SELECT (SELECT count(*) FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid), \
         (SELECT count(*) FROM game_item_ground_locations WHERE item_instance_id=encode($1,'hex')::uuid), \
         (SELECT count(*) FROM game_item_mint_receipts WHERE transaction_id=encode($2,'hex')::uuid), \
         (SELECT count(*) FROM game_source_corpse_mints WHERE transaction_id=encode($2,'hex')::uuid), \
         (SELECT count(*) FROM game_item_audit_outbox WHERE transaction_id=encode($2,'hex')::uuid)",
    )
    .bind(candidate.item_instance_id().as_slice())
    .bind(candidate.transaction_id().as_slice())
    .fetch_one(&h.pool).await?)
}

async fn issue_world_pins(
    h: &Harness,
    entry: &ChannelContentPin,
    current: &ChannelContentPin,
    world: &crate::map::boot::BundleWorld,
    mismatched_reset_map: bool,
) -> TestResult {
    let scope = super::fixture::fence(1)?.runtime_scope;
    let channel = scope.channel_id().ok_or("current World Channel")?;
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(scope.world_id().as_bytes().as_slice()).bind(channel.as_bytes().as_slice())
        .execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(scope.world_id().as_bytes().as_slice()).bind(channel.as_bytes().as_slice())
        .bind(entry.server_artifact_digest().as_slice()).bind(entry.client_artifact_digest().as_slice())
        .bind(entry.frame_binding_digest().as_slice()).execute(&h.pool).await?;
    let mut reset_map_revision = world.map_revision().to_owned();
    if mismatched_reset_map {
        // Intentionally invalid reset metadata for this negative case only.
        // Actual runtime, activation and source-death pins remain unchanged;
        // no accepted authority or source completion is fabricated.
        let last = reset_map_revision
            .pop()
            .ok_or("actual Bundle map revision")?;
        reset_map_revision.push(if last == '0' { '1' } else { '0' });
        assert_ne!(reset_map_revision, world.map_revision());
    }
    sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,0,$2,$3,$4,$5,$6,'TIBIA_SERVER_SAVE_BERLIN_06',1,clock_timestamp(),'2026-10')")
        .bind(scope.world_id().as_bytes().as_slice()).bind(world.base().digest().as_slice())
        .bind(&reset_map_revision).bind(current.server_artifact_digest().as_slice())
        .bind(current.client_artifact_digest().as_slice()).bind(current.frame_binding_digest().as_slice())
        .execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,0)")
        .bind(scope.world_id().as_bytes().as_slice())
        .execute(&h.pool)
        .await?;
    if mismatched_reset_map {
        return Ok(());
    }
    let (old_entry_match, current_world_match): (bool, bool) = sqlx::query_as(
        "SELECT c.frame_binding_digest=$3, \
         game_world_reset_map_epoch_current(active.world_id,active.reset_epoch) \
           AND sha256(convert_to(epoch.map_revision,'UTF8'))=$4 \
           AND game_elemental_placement_frame_matches(active.world_id,epoch.map_revision,$3,c.frame_binding_digest) \
         FROM game_content_activations c JOIN game_world_reset_active active ON active.world_id=c.world_id \
         JOIN game_world_reset_epochs epoch ON epoch.world_id=active.world_id AND epoch.reset_epoch=active.reset_epoch \
         WHERE c.world_id=encode($1,'hex')::uuid AND c.channel_id=encode($2,'hex')::uuid",
    )
    .bind(scope.world_id().as_bytes().as_slice()).bind(channel.as_bytes().as_slice())
    .bind(current.frame_binding_digest().as_slice()).bind(current.map_revision_digest().as_slice())
    .fetch_one(&h.pool).await?;
    assert!(
        !old_entry_match && current_world_match,
        "the old activation-only predicate refuses exactly this independently proven current World frame"
    );
    Ok(())
}

type MintReceipt = (
    String,
    String,
    String,
    String,
    String,
    String,
    i64,
    String,
    String,
);

async fn receipts(h: &Harness, death: CreatureDeathOccurrenceKey) -> TestResult<Vec<MintReceipt>> {
    let scope = death.runtime_scope();
    Ok(sqlx::query_as(
        "SELECT r.loot_purpose_key,r.event_id::text,r.transaction_id::text,r.item_instance_id::text, \
         encode(r.intent_binding,'hex'),i.definition_production_key,i.quantity::bigint, \
         coalesce(r.corpse_top_damage_character_id::text,''),r.xmin::text \
         FROM game_item_mint_receipts r JOIN game_item_instances i USING(item_instance_id) \
         WHERE r.death_scope_kind=1 AND r.death_world_id=encode($1,'hex')::uuid \
           AND r.death_channel_id=encode($2,'hex')::uuid AND r.death_scope_ownership_generation=$3::text::numeric(20,0) \
           AND r.death_actor_local_id=$4 AND r.death_actor_local_generation=$5::text::numeric(20,0) \
         ORDER BY r.loot_purpose_key,r.event_id",
    )
    .bind(scope.world_id().as_bytes().as_slice())
    .bind(scope.channel_id().ok_or("World source death Channel")?.as_bytes().as_slice())
    .bind(death.scope_ownership_generation().get().to_string())
    .bind(i64::from(death.actor_local_id())).bind(death.actor_local_generation().to_string())
    .fetch_all(&h.pool).await?)
}

fn hex(bytes: [u8; 32]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
