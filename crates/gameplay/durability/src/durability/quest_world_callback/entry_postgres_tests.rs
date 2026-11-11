//! M03: actual registered PoI STEP uses the production fenced quest writer.
//! Chosen requirements: content/quests/definitions/quests-00000-00099.json:1 record89.
use crate::bestiary_postgres_harness as h;
use crate::durability::character_progression::CharacterProgressionError;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef, GameSessionId};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

struct CurrentPremium(AtomicUsize);
impl crate::durability::quest_state::grizzly::GrizzlyPremiumGate for CurrentPremium {
    async fn task_permitted(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        account: [u8; 16],
    ) -> Result<bool, crate::durability::DurabilityError> {
        assert_eq!(account, h::id(40));
        let _: String = sqlx::query_scalar("SELECT pg_current_xact_id()::text")
            .fetch_one(&mut **tx)
            .await?;
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(true)
    }
}
fn command(number: u64) -> h::TestResult<CommandRef> {
    Ok(CommandRef::new(
        GameSessionId::decode(&h::id(h::SESSION)).map_err(h::debug)?,
        CommandId::new(number).map_err(h::debug)?,
    ))
}

#[test]
fn m03_postgres_registered_pits_entry_requires_live_premium_and_replays() -> h::TestResult {
    let Some(admin) = h::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            h::runtime()
                .map_err(h::debug)?
                .block_on(async {
                    let started = std::time::Instant::now();
                    let fixture = h::Harness::create(admin, "srvenc_m03_poi_entry", false).await?;
                    let result = exercise(&fixture).await;
                    fixture.cleanup().await?;
                    result?;
                    eprintln!(
                        "M03 REGISTERED POI PG EXECUTED duration={:?}",
                        started.elapsed()
                    );
                    assert!(started.elapsed().as_secs_f64() > 0.01);
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(h::debug)
        })?
        .join()
        .map_err(|_| "M03 PoI thread panicked")?
        .map_err(Into::into)
}

async fn exercise(f: &h::Harness) -> h::TestResult {
    // Standard cold bootstrap fixture, with level80 instead of the harness's level50.
    // It never changes a live progression row or bypasses a revision receipt.
    sqlx::query("INSERT INTO game_character_progression_state VALUES (encode($1,'hex')::uuid,1,80,1000,'profile-1','ruleset-1','content-1','simulation-1','evidence-1','declaration-1','policy-1','reward-1')")
        .bind(h::id(h::CHARACTER).as_slice()).execute(&f.pool).await?;
    let (controller, pin) =
        oteryn_gameplay_content::content::native_gameplay::test_support::activated_with_loot_tables(
            h::scope()?.world_id(),
            None,
        );
    let native = controller
        .active()
        .and_then(|generation| generation.native_gameplay())
        .ok_or("native generation")?;
    sqlx::query("INSERT INTO game_control_scope_grants(control_role,world_id,channel_id,operation) VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(h::id(h::WORLD).as_slice()).bind(h::id(h::CHANNEL).as_slice()).execute(&f.pool).await?;
    assert!(
        f.root
            .record_content_activation(
                &crate::durability::content_activation::ContentActivationRequest {
                    world_id: h::scope()?.world_id(),
                    channel_id: crate::foundation::ChannelId::decode(&h::id(h::CHANNEL))
                        .map_err(h::debug)?,
                    activation_sequence: 1,
                    previous_sequence: None,
                    server_artifact_digest: native.source_digest(),
                    client_artifact_digest: pin.identity().client_artifact_digest(),
                    frame_binding_digest: pin.frame_binding().digest(),
                }
            )
            .await
            .map_err(h::debug)?
    );
    let seal = f.recovery.seal_current().map_err(h::debug)?;
    let authority = f
        .root
        .open_character_authority(&seal)
        .await
        .map_err(h::debug)?;
    let permit =
        crate::durability::spell_owner_commit::SpellLanePermit::of_fresh_scope(h::scope()?)
            .await
            .map_err(h::debug)?;
    let context = super::QuestWorldContext {
        map_revision: "map-1".into(),
        content_revision: format!(
            "sha256:{}",
            native
                .source_digest()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        ),
        placement_context: pin.frame_binding().digest().to_vec(),
        reset_epoch: 0,
        actor_placement: h::id(17),
        movement_revision: 1,
        party: Vec::new(),
    };
    let callbacks = oteryn_gameplay_quests::quest::world_callbacks::load()?;
    let callback = callbacks
        .iter()
        .find(|callback| callback.key == "oteryn:quest-callback/m03/pits-of-inferno-entry")
        .ok_or("registered PoI entry")?;
    let catalogue = Arc::new(
        oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
            .map_err(h::debug)?
            .catalogue()
            .clone(),
    );
    let request = |number| -> h::TestResult<_> {
        Ok(QuestTransitionRequest {
            transition_key: callback.key.clone(),
            cause: QuestCause::Step(command(number)?),
        })
    };
    assert!(
        matches!(
            f.root
                .commit_character_world_callback_with_items(
                    &authority,
                    &f.node,
                    h::fence(1)?,
                    request(600)?,
                    catalogue.clone(),
                    &permit,
                    context.clone(),
                    Some(native),
                    None
                )
                .await
                .map_err(h::debug)?,
            QuestTransitionOutcome::Refused(_)
        ),
        "legacy caller has no Premium authority"
    );
    assert!(
        matches!(
            f.root
                .commit_character_quest_transition(
                    &authority,
                    &f.node,
                    h::fence(1)?,
                    request(601)?,
                    catalogue.clone()
                )
                .await,
            Err(CharacterProgressionError::InvalidInput)
        ),
        "generic quest caller cannot impersonate the STEP"
    );
    let premium = CurrentPremium(AtomicUsize::new(0));
    assert!(matches!(
        f.root
            .commit_character_world_callback_with_items_and_premium(
                &authority,
                &f.node,
                h::fence(1)?,
                request(602)?,
                catalogue.clone(),
                &permit,
                context.clone(),
                Some(native),
                None,
                &premium
            )
            .await
            .map_err(h::debug)?,
        QuestTransitionOutcome::Committed(_)
    ));
    assert!(
        matches!(
            f.root
                .commit_character_world_callback_with_items(
                    &authority,
                    &f.node,
                    h::fence(1)?,
                    request(602)?,
                    catalogue.clone(),
                    &permit,
                    context.clone(),
                    Some(native),
                    None
                )
                .await
                .map_err(h::debug)?,
            QuestTransitionOutcome::AlreadyCommitted(_)
        ),
        "historical replay does not need a fresh Premium grant"
    );
    assert!(
        matches!(
            f.root
                .commit_character_world_callback_with_items(
                    &authority,
                    &f.node,
                    h::fence(2)?,
                    request(603)?,
                    catalogue.clone(),
                    &permit,
                    context.clone(),
                    Some(native),
                    None
                )
                .await
                .map_err(h::debug)?,
            QuestTransitionOutcome::Refused(_)
        ),
        "fresh repeat must recheck Premium"
    );
    let value: i64 =
        sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1")
            .bind(&callback.storage[0].track)
            .fetch_one(&f.pool)
            .await?;
    assert_eq!(value, 1);
    let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
        .fetch_one(&f.pool)
        .await?;
    assert_eq!(receipts, 1);
    assert_eq!(premium.0.load(Ordering::SeqCst), 1);
    Ok(())
}
