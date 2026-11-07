//! WCONTRIB batch 2: real earned-point exchanges, collar MINT and fenced escort.
use crate::bestiary_postgres_harness as fixture;
use crate::domain::CharacterId;
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_transfer::{ItemWeightLookup, ItemWeightRefused};
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::durability::reward_claim_mint::quest::QuestRewardAdmission;
use crate::durability::world_event::changes::actions::WorldActionContext;
use crate::foundation::{
    ChannelId, ChannelRuntimeV1, CharacterLease, CommandId, CommandRef, ExactActorRef,
    GameSessionId, MovementLocalPosition, NodeId,
};
use oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin;
use oteryn_gameplay_content::content::native_gameplay::{NativeGameplayInput, NativeGameplayState};
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_gameplay_quests::quest::{QuestRefusal, QuestStateCatalogue, noodles, rathleton};
use sha2::{Digest, Sha256};
use std::sync::Arc;

#[test]
fn wcontrib_noodles_pg_collar_mint_present_escort_claim_and_next_save() -> fixture::TestResult {
    threaded("wcontrib-noodles", false)
}
#[test]
fn wcontrib_rathleton_pg_chavis_consumes_roots_and_earns_voting_custody() -> fixture::TestResult {
    threaded("wcontrib-chavis", true)
}
#[test]
fn wcontrib_swamp_pg_ottokar_repeat_exchange_and_personal_achievement() -> fixture::TestResult {
    threaded("wcontrib-ottokar", false)
}
fn threaded(name: &'static str, chavis: bool) -> fixture::TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .name(name.into())
        .stack_size(64 << 20)
        .spawn(move || {
            fixture::runtime()
                .map_err(fixture::debug)?
                .block_on(exercise(admin, name, chavis))
                .map_err(fixture::debug)
        })?
        .join()
        .map_err(|_| "world reward thread failed")??;
    Ok(())
}
async fn exercise(admin: String, name: &str, chavis: bool) -> fixture::TestResult {
    let started = std::time::Instant::now();
    let h = fixture::Harness::create(admin, &name.replace('-', "_"), true).await?;
    let result = if name == "wcontrib-ottokar" {
        ottokar(&h).await
    } else if chavis {
        earned_points(&h).await
    } else {
        escort(&h).await
    };
    h.cleanup().await?;
    result?;
    eprintln!(
        "{}_PG_EXECUTED: actual fenced reward transaction and custody proof; elapsed={:.3}s",
        if name == "wcontrib-ottokar" {
            "WCONTRIB_OTTOKAR"
        } else if chavis {
            "WCONTRIB_CHAVIS"
        } else {
            "WCONTRIB_NOODLES"
        },
        started.elapsed().as_secs_f64()
    );
    Ok(())
}
fn native() -> fixture::TestResult<NativeGameplayState> {
    let manifest =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/spells.manifest.json");
    let mut input = NativeGameplayInput::from_manifest(&manifest).map_err(fixture::debug)?;
    input
        .build_training
        .as_mut()
        .ok_or("training policy")?
        .content_revision = "content-1".into();
    let cosmetics = &mut input.cosmetics;
    let mut document: serde_json::Value = serde_json::from_slice(&cosmetics.bytes)?;
    if document["availability"] == "authored" {
        document["authoring"]["document"]["content_revision"] = "content-1".into();
        cosmetics.bytes = serde_json::to_vec(&document)?;
        cosmetics.sha256 = Sha256::digest(&cosmetics.bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
    }
    let room = oteryn_gameplay_content::content::qualify_selected_native_gameplay_room(
        fixture::scope()?.world_id(),
        &input,
    )
    .map_err(fixture::debug)?;
    Ok(
        oteryn_gameplay_content::content::qualified_native_gameplay_test_state(room.compiled())
            .map_err(fixture::debug)?,
    )
}
struct Weights<'a>(&'a NativeGameplayState);
impl ItemWeightLookup for Weights<'_> {
    fn generation_digest(&self) -> [u8; 32] {
        self.0.source_digest()
    }
    fn weight_of(&self, definition: &TypedDefinitionRef) -> Result<u32, ItemWeightRefused> {
        oteryn_gameplay_items::item_ref_admission::generation_item_facts(self.0, definition)
            .map_err(|_| ItemWeightRefused::DefinitionMismatch)?
            .weight
            .ok_or(ItemWeightRefused::Unknown)
    }
}
fn catalogue() -> fixture::TestResult<Arc<QuestStateCatalogue>> {
    Ok(Arc::new(
        oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
            .map_err(fixture::debug)?
            .catalogue()
            .clone(),
    ))
}
fn request(
    key: &str,
    command: u64,
    session: u8,
    use_item: bool,
) -> fixture::TestResult<QuestTransitionRequest> {
    let cause = CommandRef::new(
        GameSessionId::decode(&fixture::id(session)).map_err(fixture::debug)?,
        CommandId::new(command).map_err(fixture::debug)?,
    );
    Ok(QuestTransitionRequest {
        transition_key: key.into(),
        cause: if use_item {
            QuestCause::Use(cause)
        } else {
            QuestCause::Command(cause)
        },
    })
}
async fn fence(
    h: &fixture::Harness,
    character: u8,
    session: u8,
) -> fixture::TestResult<CurrentCharacterGameplayFence> {
    let revision:String=sqlx::query_scalar("SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid").bind(fixture::id(character).as_slice()).fetch_one(&h.pool).await?;
    let mut fence = fixture::fence(revision.parse()?)?;
    fence.character_id = CharacterId::from_bytes(fixture::id(character)).map_err(fixture::debug)?;
    fence.game_session_id = GameSessionId::decode(&fixture::id(session)).map_err(fixture::debug)?;
    Ok(fence)
}
async fn reward(
    h: &fixture::Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    catalogue: &Arc<QuestStateCatalogue>,
    native: &NativeGameplayState,
    key: &str,
    command: u64,
) -> fixture::TestResult<QuestTransitionOutcome> {
    let weights = Weights(native);
    Ok(h.root
        .commit_character_quest_reward(
            authority,
            &h.node,
            fence(h, fixture::CHARACTER, fixture::SESSION).await?,
            request(key, command, fixture::SESSION, false)?,
            Arc::clone(catalogue),
            QuestRewardAdmission {
                native,
                weights: &weights,
            },
        )
        .await
        .map_err(|error| format!("reward {key} command {command}: {error:?}"))?)
}
async fn bag(h: &fixture::Harness, character: u8, seed: u8) -> fixture::TestResult {
    // Historical starting backpack DATA, not the reward effect being qualified.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state=2 WHERE character_id=encode($1,'hex')::uuid").bind(fixture::id(character).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item','oteryn:item.tibia.i2854','definition-r1',1,1,encode($3,'hex')::uuid)")
        .bind(fixture::id(seed).as_slice()).bind(fixture::id(fixture::WORLD).as_slice()).bind(fixture::id(seed+1).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_container_slots(character_id,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid)")
        .bind(fixture::id(character).as_slice()).bind(fixture::id(seed).as_slice()).bind(fixture::id(fixture::WORLD).as_slice()).bind(fixture::id(seed+2).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}
async fn owned(h: &fixture::Harness, character: u8, key: &str) -> fixture::TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce(sum(i.quantity),0)::bigint FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid) c JOIN game_item_instances i ON i.item_instance_id=c.item WHERE i.lifecycle=1 AND i.definition_production_key=$3")
        .bind(fixture::id(character).as_slice()).bind(fixture::id(fixture::WORLD).as_slice()).bind(key).fetch_one(&h.pool).await?)
}
async fn points(h: &fixture::Harness) -> fixture::TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce((SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2),-1)").bind(fixture::id(fixture::CHARACTER).as_slice()).bind(rathleton::POINTS_TRACK).fetch_one(&h.pool).await?)
}
async fn earned_points(h: &fixture::Harness) -> fixture::TestResult {
    bag(h, fixture::CHARACTER, 90).await?;
    let native = native()?;
    let catalogue = catalogue()?;
    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(fixture::debug)?;
    let first = "oteryn:quest-reward/chavis-food-first";
    let repeat = "oteryn:quest-reward/chavis-food-repeat";
    assert_eq!(points(h).await?, -1);
    assert_eq!(
        reward(h, &authority, &catalogue, &native, first, 1).await?,
        QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch)
    );
    // Starting harvested roots are historical custody DATA. The actual five-root
    // debit, point earning, Quest receipt and exact replay below use Qengine owners.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item','oteryn:item.tibia.i21291','definition-r1',10,1,encode($3,'hex')::uuid)")
        .bind(fixture::id(100).as_slice()).bind(fixture::id(fixture::WORLD).as_slice()).bind(fixture::id(101).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,1,encode($5,'hex')::uuid)")
        .bind(fixture::id(100).as_slice()).bind(fixture::id(fixture::WORLD).as_slice()).bind(fixture::id(fixture::CHARACTER).as_slice()).bind(fixture::id(90).as_slice()).bind(fixture::id(102).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    assert!(matches!(
        reward(h, &authority, &catalogue, &native, first, 2).await?,
        QuestTransitionOutcome::Committed(_)
    ));
    assert_eq!(
        (
            points(h).await?,
            owned(h, fixture::CHARACTER, "oteryn:item.tibia.i21291").await?
        ),
        (1, 5)
    );
    assert!(matches!(
        reward(h, &authority, &catalogue, &native, first, 2).await?,
        QuestTransitionOutcome::AlreadyCommitted(_)
    ));
    assert_eq!(
        (
            points(h).await?,
            owned(h, fixture::CHARACTER, "oteryn:item.tibia.i21291").await?
        ),
        (1, 5)
    );
    assert!(matches!(
        reward(h, &authority, &catalogue, &native, repeat, 3).await?,
        QuestTransitionOutcome::Committed(_)
    ));
    assert_eq!(
        (
            points(h).await?,
            owned(h, fixture::CHARACTER, "oteryn:item.tibia.i21291").await?
        ),
        (2, 0)
    );
    let count:i64=sqlx::query_scalar("SELECT count(*) FROM game_quest_callback_consumptions WHERE character_id=encode($1,'hex')::uuid").bind(fixture::id(fixture::CHARACTER).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(count, 2);
    drop(authority);
    drop(seal);
    Ok(())
}
async fn other_claimant(h: &fixture::Harness) -> fixture::TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_character_account_guards VALUES(encode($1,'hex')::uuid)")
        .bind(fixture::id(62).as_slice())
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_character_roots(character_id,account_id,world_id,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,name,sex) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,1,'profile-1','ruleset-1','content-1','starter-1','Other Escort',2)")
        .bind(fixture::id(61).as_slice()).bind(fixture::id(62).as_slice()).bind(fixture::id(fixture::WORLD).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_character_progression_state SELECT encode($1,'hex')::uuid,1,level,total_experience,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision FROM game_character_progression_state WHERE character_id=encode($2,'hex')::uuid")
        .bind(fixture::id(61).as_slice()).bind(fixture::id(fixture::CHARACTER).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_durability_reconnect_sessions(game_session_id,account_id,character_id,world_id,runtime_scope_kind,runtime_scope_world_id,runtime_scope_channel_id,control_loss_epoch,original_grace_deadline,predecessor_generation,character_lease_generation,scope_ownership_generation,current_generation,current_transport_ref,session_state) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,1,encode($4,'hex')::uuid,encode($5,'hex')::uuid,1,999999,1,1,1,1,$6,2)")
        .bind(fixture::id(63).as_slice()).bind(fixture::id(62).as_slice()).bind(fixture::id(61).as_slice()).bind(fixture::id(fixture::WORLD).as_slice()).bind(fixture::id(fixture::CHANNEL).as_slice()).bind([8_u8;16].as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_durability_admission_account_guards VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,'test',1,'account-current',1,0,'{}')")
        .bind(fixture::id(62).as_slice()).bind(fixture::id(61).as_slice()).bind(fixture::id(63).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_durability_admission_character_guards VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,true,1,encode($4,'hex')::uuid,1,'test',1,'character-current',1,0,'{}')")
        .bind(fixture::id(61).as_slice()).bind(fixture::id(62).as_slice()).bind(fixture::id(fixture::WORLD).as_slice()).bind(fixture::id(63).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    bag(h, 61, 140).await?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,'definition-r1',1,1,encode($4,'hex')::uuid)")
        .bind(fixture::id(150).as_slice()).bind(fixture::id(fixture::WORLD).as_slice()).bind(noodles::COLLAR_ITEM).bind(fixture::id(151).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,1,encode($5,'hex')::uuid)")
        .bind(fixture::id(150).as_slice()).bind(fixture::id(fixture::WORLD).as_slice()).bind(fixture::id(61).as_slice()).bind(fixture::id(140).as_slice()).bind(fixture::id(152).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}
async fn action(
    h: &fixture::Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    catalogue: &Arc<QuestStateCatalogue>,
    (character, session): (u8, u8),
    key: &str,
    command: u64,
    context: WorldActionContext,
) -> fixture::TestResult<QuestTransitionOutcome> {
    Ok(h.root
        .commit_character_world_action(
            authority,
            &h.node,
            fence(h, character, session).await?,
            request(key, command, session, key != noodles::RETURN)?,
            Arc::clone(catalogue),
            context,
        )
        .await
        .map_err(|error| {
            format!("action {key} command {command} character {character}: {error:?}")
        })?)
}
async fn escort(h: &fixture::Harness) -> fixture::TestResult {
    bag(h, fixture::CHARACTER, 90).await?;
    let native = native()?;
    let catalogue = catalogue()?;
    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(fixture::debug)?;
    // Real daily snapshots choose the disappearing day; no test-only phase grant.
    let world = fixture::scope()?.world_id();
    let (mut runtime, player) = runtime()?;
    let mut saw_palace = false;
    let mut missing = None;
    for epoch in 0..128_i64 {
        sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,$2,$3,'map-1',$3,$3,$3,'TIBIA_SERVER_SAVE_BERLIN_06',1,to_timestamp(0)+$4::bigint*interval '1 day','1970-01')")
            .bind(fixture::id(fixture::WORLD).as_slice()).bind(epoch).bind([1_u8;32].as_slice()).bind(epoch).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,$2) ON CONFLICT(world_id) DO UPDATE SET reset_epoch=EXCLUDED.reset_epoch")
            .bind(fixture::id(fixture::WORLD).as_slice()).bind(epoch).execute(&h.pool).await?;
        h.root
            .world_changes(world, u64::try_from(epoch)?)
            .await?
            .ok_or("daily world")?;
        let saved = h
            .root
            .shared_world_changes(world, u64::try_from(epoch)?)
            .await?
            .ok_or("shared day")?;
        let live = h
            .root
            .live_world_changes(world, u64::try_from(epoch)?)
            .await?
            .ok_or("live daily NPC state")?;
        project(&mut runtime, &live)?;
        if !saved.noodles.missing {
            assert_eq!(
                positions(&runtime, noodles::NOODLES),
                vec![local(noodles::PALACE)]
            );
            saw_palace = true;
        }
        if saved.noodles.missing {
            missing = Some((u64::try_from(epoch)?, saved.noodles.roaming_index));
            break;
        }
    }
    let (epoch, index) = missing.ok_or("bounded actual missing-day draw")?;
    let (x, y, floor) = noodles::ROAMING[usize::from(index)];
    runtime
        .initialize_pinned_test_position(player, local((x - 1, y, floor)))
        .map_err(fixture::debug)?;
    assert!(
        saw_palace,
        "actual earlier saved day projected palace Noodles"
    );
    assert_eq!(
        positions(&runtime, noodles::NOODLES),
        vec![local((x, y, floor))]
    );
    assert!(positions(&runtime, noodles::DOG_LADY).is_empty());
    let context = WorldActionContext {
        actor: Position { x: x - 1, y, floor },
        target_npc: noodles::NOODLES.into(),
        target_position: Position { x, y, floor },
    };
    assert_eq!(
        action(
            h,
            &authority,
            &catalogue,
            (fixture::CHARACTER, fixture::SESSION),
            noodles::ESCORT,
            1,
            context.clone(),
        )
        .await?,
        QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch)
    );
    assert!(matches!(
        reward(
            h,
            &authority,
            &catalogue,
            &native,
            noodles::COLLAR_REWARD,
            2
        )
        .await?,
        QuestTransitionOutcome::Committed(_)
    ));
    assert_eq!(owned(h, fixture::CHARACTER, noodles::COLLAR_ITEM).await?, 1);
    assert!(matches!(
        reward(
            h,
            &authority,
            &catalogue,
            &native,
            noodles::COLLAR_REWARD,
            2
        )
        .await?,
        QuestTransitionOutcome::AlreadyCommitted(_)
    ));
    assert_eq!(owned(h, fixture::CHARACTER, noodles::COLLAR_ITEM).await?, 1);
    let minted:i64=sqlx::query_scalar("SELECT count(*) FROM game_reward_claim_mint_receipts WHERE character_id=encode($1,'hex')::uuid").bind(fixture::id(fixture::CHARACTER).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(minted, 1);
    assert!(matches!(
        action(
            h,
            &authority,
            &catalogue,
            (fixture::CHARACTER, fixture::SESSION),
            noodles::ESCORT,
            3,
            context.clone(),
        )
        .await?,
        QuestTransitionOutcome::Committed(_)
    ));
    let live = h
        .root
        .live_world_changes(world, epoch)
        .await?
        .ok_or("live escort")?;
    assert_eq!(live.escort_character, Some(fixture::id(fixture::CHARACTER)));
    project(&mut runtime, &live)?;
    let dog = positions(&runtime, noodles::NOODLES);
    assert_eq!(dog.len(), 1);
    let player_position = runtime
        .bound_character_position(&fixture::id(fixture::CHARACTER))
        .ok_or("lease-bound escort position")?;
    assert_eq!(dog[0].floor, player_position.floor);
    assert!(dog[0].x.abs_diff(player_position.x) <= 1 && dog[0].y.abs_diff(player_position.y) <= 1);
    for _ in 0..4 {
        let before = runtime
            .read_actor_position(player)
            .map_err(fixture::debug)?;
        let next = MovementLocalPosition {
            x: before.position().x - 1,
            ..before.position()
        };
        runtime
            .borrow_movement_position()
            .commit_cardinal(before, next)
            .map_err(fixture::debug)?;
    }
    project(&mut runtime, &live)?;
    let moved = runtime
        .bound_character_position(&fixture::id(fixture::CHARACTER))
        .ok_or("moved bound escort")?;
    let followed = positions(&runtime, noodles::NOODLES);
    assert_eq!(followed.len(), 1);
    assert!(followed[0].x.abs_diff(moved.x) <= 1 && followed[0].y.abs_diff(moved.y) <= 1);

    other_claimant(h).await?;
    assert_eq!(
        action(
            h,
            &authority,
            &catalogue,
            (61, 63),
            noodles::ESCORT,
            4,
            context,
        )
        .await?,
        QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch)
    );
    assert_eq!(
        h.root
            .live_world_changes(world, epoch)
            .await?
            .ok_or("retained claim")?
            .escort_character,
        live.escort_character
    );
    let king = WorldActionContext {
        actor: Position {
            x: 32311,
            y: 32171,
            floor: -6,
        },
        target_npc: noodles::KING.into(),
        target_position: Position {
            x: 32311,
            y: 32172,
            floor: -6,
        },
    };
    let mut remote = king.clone();
    remote.actor.x -= 5;
    assert_eq!(
        action(
            h,
            &authority,
            &catalogue,
            (fixture::CHARACTER, fixture::SESSION),
            noodles::RETURN,
            5,
            remote,
        )
        .await?,
        QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch)
    );
    assert!(matches!(
        action(
            h,
            &authority,
            &catalogue,
            (fixture::CHARACTER, fixture::SESSION),
            noodles::RETURN,
            6,
            king.clone(),
        )
        .await?,
        QuestTransitionOutcome::Committed(_)
    ));
    assert!(matches!(
        action(
            h,
            &authority,
            &catalogue,
            (fixture::CHARACTER, fixture::SESSION),
            noodles::RETURN,
            6,
            king.clone(),
        )
        .await?,
        QuestTransitionOutcome::AlreadyCommitted(_)
    ));
    let character =
        CharacterId::from_bytes(fixture::id(fixture::CHARACTER)).map_err(fixture::debug)?;
    let original = request(noodles::RETURN, 6, fixture::SESSION, false)?;
    assert!(
        h.root
            .reconcile_character_world_action(&authority, character, original.clone(), &king,)
            .await?
            .is_some()
    );
    let mut altered = king.clone();
    altered.actor.x += 1;
    let conflict = h
        .root
        .reconcile_character_world_action(&authority, character, original.clone(), &altered)
        .await
        .expect_err("changed spatial binding cannot reconcile original action");
    assert!(format!("{conflict:?}").contains("ConflictingOccurrence"));
    let ordinary = h
        .root
        .reconcile_character_quest_transition(&authority, character, original)
        .await
        .expect_err("ordinary binding cannot authorize a spatial action receipt");
    assert!(format!("{ordinary:?}").contains("ConflictingOccurrence"));
    let returned = h
        .root
        .live_world_changes(world, epoch)
        .await?
        .ok_or("returned Noodles")?;
    assert!(!returned.noodles.missing && returned.noodles.returned && !returned.noodles.dog_lady);
    assert!(returned.escort_character.is_none());
    project(&mut runtime, &returned)?;
    assert_eq!(
        positions(&runtime, noodles::NOODLES),
        vec![local(noodles::PALACE)]
    );
    assert!(positions(&runtime, noodles::DOG_LADY).is_empty());
    assert_eq!(
        owned(h, fixture::CHARACTER, noodles::COLLAR_ITEM).await?,
        1,
        "present-only collar survives escort and return"
    );
    let next = epoch + 1;
    sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3,'map-1',$3,$3,$3,'TIBIA_SERVER_SAVE_BERLIN_06',1,to_timestamp(0)+$4::bigint*interval '1 day','1970-01')")
        .bind(fixture::id(fixture::WORLD).as_slice()).bind(next.to_string()).bind([1_u8;32].as_slice()).bind(i64::try_from(next)?).execute(&h.pool).await?;
    sqlx::query("UPDATE game_world_reset_active SET reset_epoch=$2::text::numeric WHERE world_id=encode($1,'hex')::uuid").bind(fixture::id(fixture::WORLD).as_slice()).bind(next.to_string()).execute(&h.pool).await?;
    h.root
        .world_changes(world, next)
        .await?
        .ok_or("next save")?;
    assert!(
        h.root
            .shared_world_changes(world, next)
            .await?
            .ok_or("Dog Lady save")?
            .noodles
            .dog_lady
    );
    let saved_live = h
        .root
        .live_world_changes(world, next)
        .await?
        .ok_or("saved Dog Lady live projection")?;
    project(&mut runtime, &saved_live)?;
    assert_eq!(
        positions(&runtime, noodles::DOG_LADY),
        vec![local(noodles::DOG_LADY_POSITION)]
    );
    // A new genuine disappearing day cannot bypass the individual 14-day return
    // cooldown. Saved-day timestamps advance, but the prior return was committed
    // only seconds ago according to the database clock used by the action owner.
    let mut repeat_day = None;
    for day in (next + 1)..(next + 129) {
        sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3,'map-1',$3,$3,$3,'TIBIA_SERVER_SAVE_BERLIN_06',1,to_timestamp(0)+$4::bigint*interval '1 day','1970-01')")
            .bind(fixture::id(fixture::WORLD).as_slice()).bind(day.to_string()).bind([1_u8;32].as_slice()).bind(i64::try_from(day)?).execute(&h.pool).await?;
        sqlx::query("UPDATE game_world_reset_active SET reset_epoch=$2::text::numeric WHERE world_id=encode($1,'hex')::uuid")
            .bind(fixture::id(fixture::WORLD).as_slice()).bind(day.to_string()).execute(&h.pool).await?;
        h.root
            .world_changes(world, day)
            .await?
            .ok_or("repeat saved day")?;
        let live = h
            .root
            .live_world_changes(world, day)
            .await?
            .ok_or("repeat live day")?;
        if live.noodles.missing {
            assert!(live.escort_character.is_none());
            repeat_day = Some((day, live));
            break;
        }
    }
    let (day, before) = repeat_day.ok_or("bounded second genuine disappearing day")?;
    let (x, y, floor) = noodles::ROAMING[usize::from(before.noodles.roaming_index)];
    let revision: String = sqlx::query_scalar("SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
        .bind(fixture::id(fixture::CHARACTER).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        action(
            h,
            &authority,
            &catalogue,
            (fixture::CHARACTER, fixture::SESSION),
            noodles::ESCORT,
            7,
            WorldActionContext {
                actor: Position { x: x - 1, y, floor },
                target_npc: noodles::NOODLES.into(),
                target_position: Position { x, y, floor },
            },
        )
        .await?,
        QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch)
    );
    let after = h
        .root
        .live_world_changes(world, day)
        .await?
        .ok_or("cooldown refused state")?;
    assert!(after.noodles.missing && after.escort_character.is_none());
    assert_eq!(owned(h, fixture::CHARACTER, noodles::COLLAR_ITEM).await?, 1);
    let after_revision: String = sqlx::query_scalar("SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
        .bind(fixture::id(fixture::CHARACTER).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(after_revision, revision);
    drop(authority);
    drop(seal);
    Ok(())
}

fn local((x, y, floor): (u16, u16, i8)) -> MovementLocalPosition {
    MovementLocalPosition {
        x: i32::from(x),
        y: i32::from(y),
        floor: -i16::from(floor),
    }
}
fn runtime() -> fixture::TestResult<(ChannelRuntimeV1, ExactActorRef)> {
    let world = fixture::scope()?.world_id();
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        world,
        ChannelId::decode(&fixture::id(fixture::CHANNEL)).map_err(fixture::debug)?,
        NodeId::decode(&fixture::id(1)).map_err(fixture::debug)?,
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        64,
        ChannelContentPin::test(world),
    )
    .map_err(fixture::debug)?;
    let session = GameSessionId::decode(&fixture::id(fixture::SESSION)).map_err(fixture::debug)?;
    let reservation = runtime
        .reserve_fresh_session(session)
        .map_err(fixture::debug)?;
    let actor = runtime
        .commit_fresh_session(reservation)
        .map_err(fixture::debug)?;
    let character = crate::foundation::CharacterId::decode(&fixture::id(fixture::CHARACTER))
        .map_err(fixture::debug)?;
    runtime
        .bind_attacker_lease(
            actor,
            session,
            CharacterLease::new(character, 1).map_err(fixture::debug)?,
        )
        .map_err(fixture::debug)?;
    Ok((runtime, actor))
}
fn project(
    runtime: &mut ChannelRuntimeV1,
    state: &crate::durability::world_event::changes::actions::LiveChanges,
) -> fixture::TestResult {
    // Explicit open-terrain fixture. Typed NPC admission/presentation remains
    // with the source catalogue; this exercises the real named NPC carrier.
    crate::gameplay_transport::noodles::project(runtime, state, |_| true, |_| true)
        .map_err(fixture::debug)?;
    Ok(())
}
fn positions(runtime: &ChannelRuntimeV1, key: &str) -> Vec<MovementLocalPosition> {
    runtime
        .visible_entities()
        .npcs
        .iter()
        .filter(|npc| runtime.npc_key(npc.actor) == Some(key))
        .map(|npc| npc.position)
        .collect()
}

async fn ottokar(h: &fixture::Harness) -> fixture::TestResult {
    const TRACK: &str = "oteryn:quest-progress/swamp-fever/ottokar-exchanges";
    const MEDICINE: &str = "oteryn:item.tibia.i12517";
    const BELONGINGS: &str = "oteryn:item.tibia.i12413";
    let first = "oteryn:quest-reward/ottokar-medicine-first";
    let repeat = "oteryn:quest-reward/ottokar-medicine-repeat";
    let hundredth = "oteryn:quest-reward/ottokar-medicine-hundredth";
    let afterwards = "oteryn:quest-reward/ottokar-medicine-afterwards";
    bag(h, fixture::CHARACTER, 90).await?;
    let native = native()?;
    let catalogue = catalogue()?;
    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(fixture::debug)?;
    assert_eq!(
        reward(h, &authority, &catalogue, &native, first, 1).await?,
        QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch)
    );
    // Historical dropped medicine DATA. Every exchange debit and belongings MINT
    // below is performed by the real custody-backed generic reward transaction.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,'definition-r1',5,1,encode($4,'hex')::uuid)")
        .bind(fixture::id(100).as_slice()).bind(fixture::id(fixture::WORLD).as_slice()).bind(MEDICINE).bind(fixture::id(101).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,1,encode($5,'hex')::uuid)")
        .bind(fixture::id(100).as_slice()).bind(fixture::id(fixture::WORLD).as_slice()).bind(fixture::id(fixture::CHARACTER).as_slice()).bind(fixture::id(90).as_slice()).bind(fixture::id(102).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    for (key, command, remaining, granted) in
        [(first, 2, 4, 1), (repeat, 3, 3, 2), (repeat, 4, 2, 3)]
    {
        assert!(matches!(
            reward(h, &authority, &catalogue, &native, key, command).await?,
            QuestTransitionOutcome::Committed(_)
        ));
        assert_eq!(owned(h, fixture::CHARACTER, MEDICINE).await?, remaining);
        assert_eq!(owned(h, fixture::CHARACTER, BELONGINGS).await?, granted);
        assert!(matches!(
            reward(h, &authority, &catalogue, &native, key, command).await?,
            QuestTransitionOutcome::AlreadyCommitted(_)
        ));
        assert_eq!(owned(h, fixture::CHARACTER, MEDICINE).await?, remaining);
        assert_eq!(owned(h, fixture::CHARACTER, BELONGINGS).await?, granted);
    }
    let counter: i64 = sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2")
        .bind(fixture::id(fixture::CHARACTER).as_slice()).bind(TRACK).fetch_one(&h.pool).await?;
    assert_eq!(counter, 3);
    let achievements: i64 = sqlx::query_scalar("SELECT count(*) FROM game_account_achievements WHERE achievement_key='oteryn:achievement/doctor_doctor'").fetch_one(&h.pool).await?;
    assert_eq!(achievements, 0);
    // Compress only the earlier personal hand-in history after a lawful anchor;
    // the threshold hand-in and its achievement grant remain actual transactions.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_character_quest_tracks SET value=99 WHERE character_id=encode($1,'hex')::uuid AND track_key=$2")
        .bind(fixture::id(fixture::CHARACTER).as_slice()).bind(TRACK).execute(&mut *tx).await?;
    tx.commit().await?;
    for (key, command) in [(hundredth, 5), (afterwards, 6)] {
        assert!(matches!(
            reward(h, &authority, &catalogue, &native, key, command).await?,
            QuestTransitionOutcome::Committed(_)
        ));
    }
    assert_eq!(owned(h, fixture::CHARACTER, MEDICINE).await?, 0);
    assert_eq!(owned(h, fixture::CHARACTER, BELONGINGS).await?, 5);
    let counter: i64 = sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2")
        .bind(fixture::id(fixture::CHARACTER).as_slice()).bind(TRACK).fetch_one(&h.pool).await?;
    assert_eq!(counter, 101);
    let achievements: i64 = sqlx::query_scalar("SELECT count(*) FROM game_account_achievements WHERE achievement_key='oteryn:achievement/doctor_doctor'").fetch_one(&h.pool).await?;
    assert_eq!(achievements, 1);
    assert_eq!(
        reward(h, &authority, &catalogue, &native, afterwards, 7).await?,
        QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch)
    );
    assert_eq!(owned(h, fixture::CHARACTER, BELONGINGS).await?, 5);
    let tally: String = sqlx::query_scalar("SELECT coalesce(sum(tally),0)::text FROM game_world_change_tallies WHERE world_id=encode($1,'hex')::uuid AND resource='medicine'")
        .bind(fixture::id(fixture::WORLD).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        tally, "0",
        "Ottokar personal exchange never contributes shared medicine"
    );
    let consumptions: i64 = sqlx::query_scalar("SELECT count(*) FROM game_quest_callback_consumptions WHERE character_id=encode($1,'hex')::uuid")
        .bind(fixture::id(fixture::CHARACTER).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(consumptions, 5);
    drop(authority);
    drop(seal);
    Ok(())
}
