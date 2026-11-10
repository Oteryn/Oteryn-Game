//! docs/reference/tibia-manual/combat.md:262: current-rotation standings and lifetime cumulative levels.
use super::*;

#[path = "../bazaar_native_custody.rs"]
mod native_custody;

#[derive(serde::Serialize)]
struct AcceptedLockReceipt<'a> {
    contract_version: u8,
    operation: &'static str,
    source_authority: &'static str,
    command: &'static str,
    operation_id: &'a str,
    outcome: &'static str,
    result_code: &'static str,
    result_data: Option<serde_json::Value>,
    recovery_generation: &'static str,
    decided_at: String,
}

// Projection-consumer fixture data only, not a complete Bazaar custody or recovery
// fixture. These synthetic roots are never admitted as Character authority. The
// normal SQL guards run; full custody/source proof is in the real-owner test below.
async fn lock_auction_characters(h: &Harness, characters: &[[u8; 16]]) -> TestResult {
    use oteryn_game_server::native_admission_source::bazaar as wire;
    use sha2::{Digest, Sha256};
    for character in characters {
        let label = hex(character);
        let mut tx = h
            .pool
            .begin()
            .await
            .map_err(|error| format!("accepted Bazaar {label} begin: {error:?}"))?;
        let (account, world, revision, lifecycle): (String, String, String, i16) = sqlx::query_as("SELECT account_id::text,world_id::text,character_revision::text,lifecycle FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
            .bind(character.as_slice()).fetch_one(&mut *tx).await
            .map_err(|error| format!("accepted Bazaar {label} current root: {error:?}"))?;
        assert_eq!(lifecycle, 1);
        let revision = revision.parse::<u64>()?;
        let next = revision.checked_add(1).ok_or("Bazaar revision overflow")?;
        let (operation, subject, now): (String, String, i64) = sqlx::query_as("SELECT game_character_uuid_v7()::text,encode($1,'hex')::uuid::text,floor(extract(epoch FROM statement_timestamp()))::bigint")
            .bind(character.as_slice()).fetch_one(&mut *tx).await
            .map_err(|error| format!("accepted Bazaar {label} clock/operation: {error:?}"))?;
        let intent: wire::LockBazaarCharacter = serde_json::from_value(serde_json::json!({
            "contract_version":2,"command":"LockBazaarCharacter","variant":"PLATFORM_BAZAAR",
            "issuer_authority":wire::ISSUER,"issuer_decision_id":operation,"source_revision":"1",
            "operation_id":operation,"listing_id":operation,"seller_account_id":account,
            "world_id":world,"character_id":subject,"expected_character_revision":revision.to_string(),
            "eligibility_revision":"1","snapshot_schema_version":1,
            "issued_at_source":(now-1).to_string(),"expires_at_source":(now+120).to_string(),
            "audience":wire::AUDIENCE,
        }))?;
        let intent = serde_json::to_vec(&intent)?;
        wire::decode_intent(&intent, "LockBazaarCharacter", &operation).map_err(debug)?;
        // bazaar/mod.rs:489-538: the accepted consumer fact has no invented
        // capture/signature. Its real receipt bytes and generation bind the audit payload.
        let receipt = serde_json::to_vec(&AcceptedLockReceipt {
            contract_version: 2,
            operation: "PublishCharacterCommandReceiptV1",
            source_authority: "oteryn:character-authority:primary",
            command: "LockBazaarCharacter",
            operation_id: &operation,
            outcome: "COMMITTED",
            result_code: "CHAR_CMD_OK",
            result_data: None,
            recovery_generation: "1",
            decided_at: now.to_string(),
        })?;
        let mut payload = vec![0x0a, 32];
        payload.extend_from_slice(&Sha256::digest(&intent));
        payload.extend_from_slice(&[0x12, 32]);
        payload.extend_from_slice(&Sha256::digest(&receipt));
        payload.extend_from_slice(&[0x18, 1]);
        sqlx::query("INSERT INTO game_bazaar_decisions(issuer,operation_id,command,intent,receipt,result_code,recovery_generation,decided_at,event_id,transaction_id,world_id,payload) VALUES($1,$2::uuid,'LockBazaarCharacter',$3,$4,'CHAR_CMD_OK',1,$5,game_character_uuid_v7(),game_character_uuid_v7(),$6::uuid,$7)")
            .bind(wire::ISSUER).bind(&operation).bind(intent).bind(receipt).bind(now).bind(&world).bind(payload).execute(&mut *tx).await
            .map_err(|error| format!("accepted Bazaar {label} decision/audit: {error:?}"))?;
        sqlx::query("INSERT INTO game_bazaar_character_transitions(operation_id,character_id,old_account,new_account,old_lifecycle,new_lifecycle,old_revision,new_revision,issuer) VALUES($1::uuid,$2::uuid,$3::uuid,$3::uuid,1,2,$4::text::numeric,$5::text::numeric,$6)")
            .bind(&operation).bind(&subject).bind(&account).bind(revision.to_string()).bind(next.to_string()).bind(wire::ISSUER).execute(&mut *tx).await
            .map_err(|error| format!("accepted Bazaar {label} transition/capture: {error:?}"))?;
        let changed = sqlx::query("UPDATE game_character_roots SET lifecycle=2,character_revision=$2::text::numeric WHERE character_id=$1::uuid")
            .bind(&subject).bind(next.to_string()).execute(&mut *tx).await
            .map_err(|error| format!("accepted Bazaar {label} guarded root update: {error:?}"))?.rows_affected();
        assert_eq!(changed, 1);
        tx.commit()
            .await
            .map_err(|error| format!("accepted Bazaar {label} commit: {error:?}"))?;
    }
    Ok(())
}

#[test]
fn tibiadrome_projection_consumer_withdraws_auction_rows_paging_and_own_rank() -> TestResult {
    run("hsdromeauction", async |h| {
        dataset(h, 300, 51, "druid", 1000, world())
            .await
            .map_err(|error| format!("native snapshot projection dataset: {error:?}"))?;
        sqlx::query("INSERT INTO game_tibiadrome_scores(world_id,rotation,character_id,level,experience) SELECT world_id,floor(extract(epoch FROM statement_timestamp())/1209600)::bigint,character_id,CASE WHEN character_id=encode($2,'hex')::uuid THEN 1 ELSE 100 END,1000 FROM game_character_roots WHERE character_id=ANY($1::text[]::uuid[])")
            .bind((300..351).map(|n| hex(&generated(n))).collect::<Vec<_>>())
            .bind(generated(350).as_slice()).execute(&h.pool).await?;
        h.root
            .refresh_highscore_schedule(world(), complete_catalogues()?, true)
            .await
            .map_err(|error| format!("native snapshot initial refresh/read: {error:?}"))?;
        let target = CharacterId::decode(&generated(350))
            .map_err(|error| format!("native snapshot initial refresh/read: {error:?}"))?;
        let mut captured = Vec::new();
        for category in [14, 17] {
            let p = h
                .root
                .read_highscore_page(world(), category, None, 1, 0, target)
                .await
                .map_err(|error| format!("native snapshot auction read: {error:?}"))?;
            assert_eq!(p.rows.len(), 50);
            assert!(p.has_more);
            assert_eq!(
                p.own_row.as_ref().map(|r| (r.rank, r.value.0)),
                Some((51, 1))
            );
            captured.push((category, p));
        }
        let generic = h
            .root
            .read_highscore_page(world(), 1, None, 1, 0, target)
            .await
            .map_err(|error| format!("native snapshot auction read: {error:?}"))?;
        assert!(generic.own_row.is_some());
        lock_auction_characters(h, &[generated(300), generated(350)])
            .await
            .map_err(|error| format!("native snapshot accepted auction transition: {error:?}"))?;
        for (category, before) in &captured {
            let p = h
                .root
                .read_highscore_page(
                    world(),
                    *category,
                    None,
                    1,
                    before.snapshot.snapshot_id,
                    target,
                )
                .await
                .map_err(|error| format!("native snapshot auction read: {error:?}"))?;
            assert_eq!(p.snapshot, before.snapshot);
            let visible: Vec<_> = before
                .rows
                .iter()
                .filter(|row| row.character_id.as_bytes() != &generated(300))
                .cloned()
                .collect();
            assert_eq!(
                p.rows.len(),
                49,
                "auction withdrawal leaves a page-position gap"
            );
            assert_eq!(
                p.rows, visible,
                "retained ranks and page positions stay unchanged"
            );
            assert!(!p.has_more, "the only later row is auctioned");
            assert!(p.own_row.is_none());
            assert!(
                h.root
                    .read_highscore_page(
                        world(),
                        *category,
                        None,
                        2,
                        before.snapshot.snapshot_id,
                        target
                    )
                    .await
                    .map_err(|error| format!("native snapshot auction page two: {error:?}"))?
                    .rows
                    .is_empty()
            );
        }
        let after = h
            .root
            .read_highscore_page(world(), 1, None, 1, generic.snapshot.snapshot_id, target)
            .await
            .map_err(|error| format!("native snapshot auction read: {error:?}"))?;
        assert_eq!(
            after, generic,
            "Drome withdrawal does not change generic snapshot policy"
        );
        Ok(())
    })
}

// Same fenced durability reader/commit called by native Drome Status; this
// supported Reference authority fixture does not claim a native actor boot.
async fn drome_status(
    h: &Harness,
    authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    ordinal: u64,
) -> TestResult<oteryn_protocol_oteryn::arena::DromeResult> {
    use oteryn_protocol_oteryn::arena as wire;
    let mut fence = harness::fence(1)?;
    let revision:String=sqlx::query_scalar("SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(CHARACTER).as_slice()).fetch_one(&h.pool).await?;
    fence.expected_character_revision =
        crate::domain::CharacterRevision::new(revision.parse()?).map_err(debug)?;
    let command = crate::foundation::CommandRef::new(
        fence.game_session_id,
        crate::foundation::CommandId::new(ordinal).map_err(debug)?,
    );
    let payload = wire::encode_match_request(wire::ArenaAction::Drome(wire::DromeAction::Status))
        .map_err(debug)?;
    let read = h
        .root
        .read_tibiadrome(authority, &h.node, fence, command, &payload)
        .await
        .map_err(debug)?;
    let before = read.state;
    let mut after = before.clone();
    let rotation = read.now / crate::durability::tibiadrome::ROTATION_SECONDS;
    if after == crate::durability::tibiadrome::Record::default() {
        after.progress.rotation = rotation;
    }
    after.progress.rotate(rotation);
    let reply = h
        .root
        .commit_tibiadrome(
            authority, &h.node, fence, command, payload, before, after, 1, None, None,
        )
        .await
        .map_err(debug)?;
    Ok(wire::DromeResult::decode(&reply).map_err(debug)?)
}

#[test]
fn srvenc_tibiadrome_postgres_auction_excludes_standing_and_expired_rewards() -> TestResult {
    native_profile_withdrawal("hsdromenative")
}

#[test]
fn tibiadrome_native_profile_withdrawal_keeps_public_highscore_retries_identical() -> TestResult {
    native_profile_withdrawal("hsdromeprofile")
}

fn native_profile_withdrawal(tag: &'static str) -> TestResult {
    // Both tests run this scenario concurrently under cargo test; isolate their recovery files.
    run(tag, async |h| {
        // The registered authority target owns the real bootstrap producer;
        // create a distinct offline Character through that existing API.
        let (artifact, _) = crate::bazaar_items::artifact()?;
        let view = oteryn_gameplay_content::content::ReferencePlayableArtifactView::load(
            &artifact,
            oteryn_gameplay_content::content::ReferenceArtifactProjection::ServerAuthoritative,
        )?;
        let custody = native_custody::NativeBazaar::prepare(
            &h.pool,
            &h.root,
            &h.node,
            crate::durability::content_activation::ContentActivationRequest {
                world_id: world(),
                channel_id: crate::foundation::ChannelId::decode(&id(harness::CHANNEL))
                    .map_err(debug)?,
                activation_sequence: 1,
                previous_sequence: None,
                server_artifact_digest: view.artifact_digest(),
                client_artifact_digest: [3; 32],
                frame_binding_digest: [4; 32],
            },
            &artifact,
        )
        .await
        .map_err(|error| format!("public fixture genuine Bazaar setup: {error}"))?;
        let seal = h
            .recovery
            .seal_current()
            .map_err(|error| format!("public fixture recovery seal: {error:?}"))?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(|error| format!("public fixture normal Character authority: {error:?}"))?;
        crate::allow(&h.root, &h.node, 100).await?;
        // The CHARM harness already consumed issuer revision one. Advance the
        // real retained bootstrap floor rather than replaying another decision.
        let bootstrap_revision: i64 = sqlx::query_scalar(
            "SELECT COALESCE(max(source_revision),0)+1 FROM game_character_bootstrap_intent_floors WHERE issuer_scope=1",
        )
        .fetch_one(&h.pool)
        .await?;
        let mut bootstrap = crate::Wire::new(105, 100, bootstrap_revision)?;
        bootstrap.world = WORLD;
        let created = h
            .root
            .bootstrap_character(&authority, &h.node, &bootstrap.decode()?)
            .await
            .map_err(|error| format!("public fixture genuine offline bootstrap: {error:?}"))?;
        let observer = *created.character_id.as_bytes();
        sqlx::query("INSERT INTO game_character_progression_state(character_id,character_revision,level,total_experience,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision) VALUES(encode($1,'hex')::uuid,1,50,1000,'profile-1','ruleset-1','content-1','simulation-1','evidence-1','declaration-1','policy-1','reward-1')")
            .bind(observer.as_slice()).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_tibiadrome_scores(world_id,rotation,character_id,level,experience) SELECT world_id,floor(extract(epoch FROM statement_timestamp())/1209600)::bigint,character_id,7,1000 FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
            .bind(observer.as_slice()).execute(&h.pool).await?;
        h.root
            .refresh_highscore_schedule(world(), complete_catalogues()?, true)
            .await
            .map_err(|error| format!("public fixture initial snapshot: {error:?}"))?;
        let target_text: String = sqlx::query_scalar("SELECT character_id::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
            .bind(observer.as_slice()).fetch_one(&h.pool).await?;
        let world_text: String = sqlx::query_scalar("SELECT world_id::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
            .bind(observer.as_slice()).fetch_one(&h.pool).await?;
        let held_profiles: Vec<String> = sqlx::query_scalar("SELECT character_id::text FROM game_character_roots WHERE character_id<>encode($1,'hex')::uuid ORDER BY character_id LIMIT 1000")
            .bind(observer.as_slice()).fetch_all(&h.pool).await?;
        assert_eq!(held_profiles.len(), 1);
        let profiles = h
            .root
            .next_public_profiles(&authority, &held_profiles, 32)
            .await
            .map_err(|error| format!("public profile before auction: {error:?}"))?;
        assert_eq!(profiles.len(), 1);
        let profile_before = profiles[0].clone();
        assert_eq!(profile_before.character_id, target_text);
        assert!(profile_before.profile.is_some());
        let mut public_captured = Vec::new();
        for category in [14, 17] {
            let held: Vec<_> = (1..=12)
                .chain([14, 16, 17])
                .filter(|c| *c != category)
                .map(|c| format!("{world_text}:{c}"))
                .collect();
            let batch = h
                .root
                .next_public_highscores(&authority, &held)
                .await
                .map_err(|error| {
                    format!("public highscore before auction category {category}: {error:?}")
                })?
                .ok_or("Drome public snapshot outbox missing")?;
            assert_eq!(batch.category, category);
            assert_eq!(batch.lists[0].len(), 1);
            public_captured.push((held, batch));
        }
        // Manual combat.md:263: actual current auction withdrawal, while a
        // listing after the old rotation cutoff preserves already earned prizes.
        let now: i64 =
            sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
                .fetch_one(&h.pool)
                .await?;
        let rotation = u32::try_from(now)? / crate::durability::tibiadrome::ROTATION_SECONDS;
        let state = crate::durability::tibiadrome::Record {
            progress: crate::durability::tibiadrome::Progress {
                rotation: rotation - 1,
                level: 25,
                start: 25,
                waves: 1,
                ..Default::default()
            },
            ..Default::default()
        };
        sqlx::query("INSERT INTO game_character_tibiadrome(character_id,body) VALUES(encode($1,'hex')::uuid,$2)")
            .bind(id(CHARACTER).as_slice()).bind(serde_json::to_value(state)?).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_tibiadrome_scores(world_id,rotation,character_id,level,experience) VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,3,1000),(encode($1,'hex')::uuid,$4,encode($3,'hex')::uuid,25,1234),(encode($1,'hex')::uuid,$4,encode($5,'hex')::uuid,25,2000)")
            .bind(id(WORLD).as_slice()).bind(i64::from(rotation)).bind(id(CHARACTER).as_slice()).bind(i64::from(rotation-1)).bind(observer.as_slice()).execute(&h.pool).await?;
        let receipt = custody
            .command(&h.pool, &h.root, &authority, &h.node, &observer, true)
            .await
            .map_err(|error| format!("public fixture genuine Game lock: {error}"))?;
        assert_eq!(receipt["result_code"], "CHAR_CMD_OK");
        assert!(receipt["result_data"]["signed_snapshot"].is_object());
        let valid = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(|error| {
                format!("public fixture recovery validity after real capture: {error:?}")
            })?;
        let standing = drome_status(h, &valid, 4001).await?;
        assert_eq!(
            (standing.rank, standing.points, standing.potions),
            (1, 250, 3),
            "fenced Drome Status excludes an actually auction-locked rival"
        );
        let previous:(i64,i32)=sqlx::query_as("SELECT rank,potions FROM game_tibiadrome_awards WHERE character_id=encode($1,'hex')::uuid AND rotation=$2")
            .bind(observer.as_slice()).bind(i64::from(rotation-1)).fetch_one(&h.pool).await?;
        assert_eq!(
            previous,
            (2, 3),
            "later listing preserves prior rotation prize"
        );

        drop(valid);
        // PUBLIC_PROJECTIONS_V1.md:195-200,233-240: immutable ranking revisions;
        // website withdrawal follows the newer profile:[] through its existing join.
        let profiles = h
            .root
            .next_public_profiles(&authority, &held_profiles, 32)
            .await
            .map_err(|error| format!("public profile after auction: {error:?}"))?;
        assert_eq!(profiles.len(), 1);
        let withdrawn = &profiles[0];
        assert_eq!(withdrawn.character_id, target_text);
        assert!(withdrawn.profile.is_none());
        assert_eq!(withdrawn.projection_epoch, profile_before.projection_epoch);
        assert!(withdrawn.projection_revision > profile_before.projection_revision);
        for (held, before) in public_captured {
            let after = h
                .root
                .next_public_highscores(&authority, &held)
                .await
                .map_err(|error| {
                    format!(
                        "public highscore after auction category {}: {error:?}",
                        before.category
                    )
                })?
                .ok_or("Drome public snapshot outbox disappeared")?;
            assert_eq!(
                after, before,
                "auction profile withdrawal preserves pending ranking revisions"
            );
            for filter in 0..6 {
                let encode = oteryn_game_server::native_admission_source::public_projections::encode_highscores;
                assert_eq!(
                    encode(
                        "oteryn:character-authority:primary",
                        &after.snapshot(filter)
                    )
                    .map_err(debug)?,
                    encode(
                        "oteryn:character-authority:primary",
                        &before.snapshot(filter)
                    )
                    .map_err(debug)?,
                    "same highscore revision remains byte-identical on retry",
                );
            }
        }
        // combat.md:262-263: historical rotation eligibility uses accepted
        // Game auction history, independently of the current lifecycle/profile.
        let lock_at = receipt["decided_at"]
            .as_str()
            .ok_or("public fixture actual lock clock")?
            .parse::<i64>()?;
        let before_lock: i16 = sqlx::query_scalar(
            "SELECT game_tibiadrome_lifecycle_at_cut(encode($1,'hex')::uuid,$2)",
        )
        .bind(observer.as_slice())
        .bind(lock_at)
        .fetch_one(&h.pool)
        .await
        .map_err(|error| format!("Drome cutoff before genuine lock: {error:?}"))?;
        assert_eq!(
            before_lock, 1,
            "later auctions preserve earlier eligibility"
        );
        let waiting = std::time::Instant::now();
        loop {
            let now: i64 =
                sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
                    .fetch_one(&h.pool)
                    .await?;
            if now > lock_at {
                break;
            }
            assert!(waiting.elapsed() < std::time::Duration::from_secs(3));
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        let unlocked = custody
            .command(&h.pool, &h.root, &authority, &h.node, &observer, false)
            .await
            .map_err(|error| format!("public fixture genuine Game unlock: {error}"))?;
        let unlock_at = unlocked["decided_at"]
            .as_str()
            .ok_or("public fixture actual unlock clock")?
            .parse::<i64>()?;
        assert!(unlock_at > lock_at);
        for (cut, expected) in [(unlock_at, 2_i16), (unlock_at + 1, 1_i16)] {
            let actual: i16 = sqlx::query_scalar(
                "SELECT game_tibiadrome_lifecycle_at_cut(encode($1,'hex')::uuid,$2)",
            )
            .bind(observer.as_slice())
            .bind(cut)
            .fetch_one(&h.pool)
            .await
            .map_err(|error| format!("Drome cutoff {cut} after genuine unlock: {error:?}"))?;
            assert_eq!(
                actual, expected,
                "accepted lock at cutoff survives later unlock"
            );
        }
        h.root
            .open_character_authority(&seal)
            .await
            .map_err(|error| {
                format!("public fixture recovery validity after real unlock: {error:?}")
            })?;
        let restored = drome_status(h, &authority, 4002).await?;
        assert_eq!(
            (restored.rank, restored.points, restored.potions),
            (2, 250, 3),
            "genuine unlock restores the current rank and never doubles rotation points"
        );
        drop(authority);
        drop(seal);
        Ok(())
    })
}

#[test]
fn tibiadrome_highscores_publish_lifetime_current_rotation_and_private_own_row() -> TestResult {
    run("hsdrome", async |h| {
        dataset(h, 300, 5, "druid", 1000, world()).await?;
        let rotation: i64 = sqlx::query_scalar(
            "SELECT floor(extract(epoch FROM statement_timestamp())/1209600)::bigint",
        )
        .fetch_one(&h.pool)
        .await?;
        for (w, r, who, level, experience) in [
            (world(), rotation - 1, generated(300), 10_i64, 200_i64),
            (world(), rotation, generated(300), 5, 200),
            (
                WorldId::decode(&id(WORLD + 1)).map_err(debug)?,
                rotation - 2,
                generated(300),
                7,
                200,
            ),
            (world(), rotation + 1, generated(300), 100, 200),
            (world(), rotation - 1, generated(301), 20, 300),
            (world(), rotation, generated(301), 6, 300),
            (world(), rotation, generated(302), 6, 100),
            (world(), rotation, generated(303), 6, 100),
            (world(), rotation, id(CHARACTER), 4, 500),
        ] {
            sqlx::query("INSERT INTO game_tibiadrome_scores(world_id,rotation,character_id,level,experience) VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,$4,$5)")
                .bind(w.as_bytes().as_slice()).bind(r).bind(who.as_slice()).bind(level).bind(experience)
                .execute(&h.pool).await?;
        }
        let published = h
            .root
            .refresh_highscore_schedule(world(), complete_catalogues()?, true)
            .await
            .map_err(debug)?;
        assert!(published.snapshots.iter().any(|s| s.category == 14));
        assert!(published.snapshots.iter().any(|s| s.category == 17));
        let lifetime = page(h, 14, None, 1, 0).await?;
        assert_eq!(
            lifetime.rows.len(),
            5,
            "unplayed Characters have no Drome ranking"
        );
        assert_eq!(
            (
                lifetime.rows[0].character_id.as_bytes(),
                lifetime.rows[0].value.0
            ),
            (&generated(301), 26)
        );
        assert_eq!(
            (
                lifetime.rows[1].character_id.as_bytes(),
                lifetime.rows[1].value.0
            ),
            (&generated(300), 22),
            "lifetime follows the Character across Worlds and excludes future rotations"
        );
        let current = page(h, 17, None, 1, 0).await?;
        assert_eq!(current.rows.len(), 5);
        assert_eq!(
            current.rows[0].character_id.as_bytes(),
            &generated(302),
            "equal levels use lower captured XP first"
        );
        assert_eq!(current.rows[1].character_id.as_bytes(), &generated(303));
        assert_eq!(
            (current.rows[0].rank, current.rows[1].rank),
            (1, 2),
            "exact score/XP ties retain distinct Drome standings"
        );
        assert_eq!(current.rows[2].character_id.as_bytes(), &generated(301));
        assert_eq!(current.rows[3].value.0, 5);
        assert_eq!(
            current.own_row.as_ref().map(|r| (r.rank, r.value.0)),
            Some((5, 4))
        );
        for category in [14, 17] {
            let HighscoresReply::Result(bytes) = client_reply(h, category, 1, 1, 0).await? else {
                return Err("Drome query through the session handler was refused".into());
            };
            let reply = decode_highscores_result(&bytes).map_err(debug)?;
            assert_eq!(reply.category, category);
            assert_eq!(reply.rows.len(), 1);
            assert_eq!(reply.own_row.as_ref().map(|r| r.value), Some(4));
        }
        sqlx::query("UPDATE game_tibiadrome_scores SET level=8 WHERE world_id=encode($1,'hex')::uuid AND rotation=$2 AND character_id=encode($3,'hex')::uuid")
            .bind(world().as_bytes().as_slice()).bind(rotation).bind(generated(300).as_slice()).execute(&h.pool).await?;
        h.root
            .refresh_highscore_schedule(world(), complete_catalogues()?, true)
            .await
            .map_err(debug)?;
        assert_eq!(
            page(h, 17, None, 1, current.snapshot.snapshot_id).await?,
            current
        );
        assert_eq!(page(h, 17, None, 1, 0).await?.rows[0].value.0, 8);
        Ok(())
    })
}

/// Preserve real main 14/17 data across the branch highscore DDL and cleanup.
#[test]
fn tibiadrome_main_snapshots_survive_branch_upgrade_postgres() -> TestResult {
    run("hsdromeupgrade", async |h| {
        dataset(h, 300, 1, "druid", 1000, world()).await?;
        sqlx::query("INSERT INTO game_tibiadrome_scores(world_id,rotation,character_id,level,experience) VALUES(encode($1,'hex')::uuid,floor(extract(epoch FROM statement_timestamp())/1209600)::bigint,encode($2,'hex')::uuid,5,200)")
            .bind(world().as_bytes().as_slice()).bind(generated(300).as_slice())
            .execute(&h.pool).await?;
        let published = h
            .root
            .refresh_highscore_schedule(world(), complete_catalogues()?, true)
            .await
            .map_err(debug)?;
        for category in [14, 17] {
            assert!(
                published
                    .snapshots
                    .iter()
                    .any(|snapshot| snapshot.category == category)
            );
        }
        async fn retained(h: &Harness) -> TestResult<Vec<(String, String)>> {
            let mut retained = Vec::new();
            for table in [
                "game_world_highscore_snapshots",
                "game_world_highscore_heads",
                "game_public_highscores",
            ] {
                let rows: String = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
                    "SELECT COALESCE(jsonb_agg(body ORDER BY body::text),'[]'::jsonb)::text FROM (SELECT to_jsonb(t) AS body FROM {table} t WHERE world_id=encode($1,'hex')::uuid AND category IN(14,17)) captured"
                ))).bind(world().as_bytes().as_slice()).fetch_one(&h.pool).await?;
                assert_ne!(
                    rows, "[]",
                    "actual job must populate {table} before upgrade"
                );
                retained.push((table.to_owned(), rows));
            }
            Ok(retained)
        }
        let before = retained(h).await?;
        let lifetime = page(h, 14, None, 1, 0).await?;
        let rotation = page(h, 17, None, 1, 0).await?;
        assert!(!lifetime.rows.is_empty());
        assert!(!rotation.rows.is_empty());
        let installed: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM _sqlx_migrations WHERE version=2170 AND success)",
        )
        .fetch_one(&h.pool)
        .await?;
        assert!(
            installed,
            "the normal embedded migrator installed forward 2170"
        );
        // Current Runtime requires its complete schema. Exercise the exact source
        // fragment on real, already-published data rather than weakening that gate.
        let migration = include_str!("../../../migrations/1423_primal_pod_scoped_items.sql");
        assert_eq!(
            migration
                .matches("-- SRX-3 W17: canonical job keeps its main grants.")
                .count(),
            1
        );
        let start = migration
            .find("-- SRX-3 W17: canonical job keeps its main grants.")
            .ok_or("highscore fragment")?;
        let end = start
            + migration[start..]
                .find("-- SRX-3 W11:")
                .ok_or("highscore fragment end")?;
        let fragment = &migration[start..end];
        assert!(fragment.contains("REVOKE SELECT ON game_character_tibiadrome"));
        assert!(!migration.contains("GRANT SELECT ON game_character_tibiadrome"));
        assert!(
            !include_str!("../../../migrations/1420_tibiadrome_highscore_grant.sql")
                .contains("GRANT SELECT ON game_character_tibiadrome")
        );
        eprintln!("SRX3_MAIN_DROME_UPGRADE_REACHED actual14_17=true exact_source_fragment=true");
        let mut upgrade = h.pool.begin().await?;
        // Reproduce the old branch's actual grant without retaining it in the
        // corrected migration. The source fragment must retire it on its own.
        (&mut *upgrade)
            .execute("GRANT SELECT ON game_character_tibiadrome TO oteryn_game_highscore_job")
            .await?;
        let granted: bool = sqlx::query_scalar("SELECT has_table_privilege('oteryn_game_highscore_job','game_character_tibiadrome','SELECT')")
            .fetch_one(&mut *upgrade).await?;
        assert!(granted, "the legacy upgrade control really grants access");
        (&mut *upgrade)
            .execute(sqlx::AssertSqlSafe(fragment.to_owned()))
            .await?;
        let granted: bool = sqlx::query_scalar("SELECT has_table_privilege('oteryn_game_highscore_job','game_character_tibiadrome','SELECT')")
            .fetch_one(&mut *upgrade).await?;
        assert!(
            !granted,
            "the unmerged migration itself removes legacy access before 2170"
        );
        (&mut *upgrade)
            .execute(include_str!(
                "../../../migrations/2170_srx3_canonical_drome_highscores.sql"
            ))
            .await?;
        upgrade.commit().await?;
        assert_eq!(
            retained(h).await?,
            before,
            "upgrade preserves complete committed snapshot/head/public bodies"
        );
        assert_eq!(
            page(h, 14, None, 1, lifetime.snapshot.snapshot_id).await?,
            lifetime
        );
        assert_eq!(
            page(h, 17, None, 1, rotation.snapshot.snapshot_id).await?,
            rotation
        );
        Ok(())
    })
}
