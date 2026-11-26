use super::fixture::{Harness, debug, id};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;

use super::{AdmittedSession, ComposedFreshAdmission, TestResult};

use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};

use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;

const TRACK: &str = "oteryn:quest-progress/questpc7/tokens/programme";
const TOKEN_FIXTURE_CELL: Position = Position {
    x: 33293,
    y: 31677,
    floor: -15,
};

async fn progress(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult<i64> {
    let f = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let state = owner
        .root
        .read_character_quest_state(owner.character, f.character_id)
        .await
        .map_err(debug)?;
    Ok(state.tracks().get(TRACK).copied().unwrap_or(-1))
}

async fn count(h: &Harness, item: u32) -> TestResult<i64> {
    super::questpc7_outfit_items_fixtures::quantity(h, item).await
}

async fn mint(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    quantity: u16,
    nonce: u64,
) -> TestResult {
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    super::quest_world_callbacks_fixtures::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: TOKEN_FIXTURE_CELL,
            item: "oteryn:item.tibia.i19083".into(),
            count: quantity,
        },
        nonce,
        true,
    )
    .await
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let version: String = sqlx::query_scalar("SHOW server_version")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("QUESTPC7_TOKENS POSTGRES_STARTED version={version}");
    let mut selected = admitted
        .continuity
        .selected_capabilities
        .as_slice()
        .to_vec();
    selected.extend([3, 6, 25]);
    selected.sort_unstable();
    selected.dedup();
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered, &selected)
            .ok_or("capabilities")?;
    let [jorge] =
        super::classic_quests_fixtures::install(owner, admitted, ["oteryn:npc.jorge"]).await?;
    assert_eq!(
        progress(owner, admitted).await?,
        -1,
        "no subject quest progress seeds"
    );
    use super::qengine_rewards_fixtures::talk;

    assert_eq!(
        talk(owner, admitted, jorge, 6010, "hi").await?,
        NpcIntentDisposition::Ok
    );
    assert_eq!(
        talk(owner, admitted, jorge, 6011, "I would like a souvenir, please").await?,
        NpcIntentDisposition::Ok
    );
    assert_eq!(progress(owner, admitted).await?, 0);
    talk(owner, admitted, jorge, 6012, "finish the commission").await?;
    assert_eq!(
        progress(owner, admitted).await?,
        0,
        "completion cannot bypass the exchange"
    );

    mint(owner, admitted, 19, 6000).await?;
    assert_eq!(
        talk(owner, admitted, jorge, 6013, "abacus").await?,
        NpcIntentDisposition::Rejected
    );
    assert_eq!(count(h, 19083).await?, 19);
    assert_eq!(count(h, 19151).await?, 0);
    assert_eq!(progress(owner, admitted).await?, 0);

    let fence = owner.current_quest_fence(admitted.game_session_id).await
        .map_err(|()| "fence")?.ok_or("fence")?;
    super::quest_world_callbacks_fixtures::actions::nest_key(
        owner, h, fence.item_fence(), TOKEN_FIXTURE_CELL, "oteryn:item.tibia.i19083",
    ).await?;
    mint(owner, admitted, 5, 6001).await?;
    let quantities: Vec<i64> = sqlx::query_scalar(
        "SELECT quantity::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i19083' ORDER BY quantity",
    ).fetch_all(&h.pool).await?;
    assert_eq!(quantities, [5, 19], "the first purchase must span two stacks");
    assert_eq!(
        talk(owner, admitted, jorge, 6014, "souvenir").await?,
        NpcIntentDisposition::Ok
    );
    assert_eq!(
        talk(owner, admitted, jorge, 6015, "An Abacus, please").await?,
        NpcIntentDisposition::Ok
    );
    assert_eq!(count(h, 19083).await?, 4, "consume exactly twenty across carried stacks");
    assert_eq!(count(h, 19151).await?, 1);
    assert_eq!(progress(owner, admitted).await?, 1);
    talk(owner, admitted, jorge, 6015, "An Abacus, please").await?;
    assert_eq!(
        count(h, 19083).await?,
        4,
        "a duplicate command must not charge again"
    );
    assert_eq!(count(h, 19151).await?, 1);

    assert_eq!(
        talk(owner, admitted, jorge, 6016, "finish the commission").await?,
        NpcIntentDisposition::Ok
    );
    assert_eq!(progress(owner, admitted).await?, 2);
    let completed:bool=sqlx::query_scalar("SELECT completed_character_revision IS NOT NULL FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key='oteryn:quest.hunting_for_tokens'").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert!(completed);
    talk(owner, admitted, jorge, 6016, "finish the commission").await?;
    assert_eq!(count(h, 19083).await?, 4);
    assert_eq!(count(h, 19151).await?, 1);

    mint(owner, admitted, 40, 6002).await?;
    for (menu, purchase) in [(6017, 6018), (6019, 6020)] {
        assert_eq!(
            talk(owner, admitted, jorge, menu, "Do you have any souvenirs?").await?,
            NpcIntentDisposition::Ok
        );
        assert_eq!(
            talk(owner, admitted, jorge, purchase, "An Assassin Doll, please").await?,
            NpcIntentDisposition::Ok
        );
    }
    assert_eq!(count(h, 19083).await?, 4);
    assert_eq!(count(h, 19151).await?, 1);
    assert_eq!(
        count(h, 28897).await?,
        2,
        "the same alternative remains available after completion"
    );
    let grants: i64 = sqlx::query_scalar("SELECT count(*) FROM game_quest_reward_sources WHERE character_id=encode($1,'hex')::uuid AND transition_key='oteryn:quest-reward/questpc7-tokens-abacus-replica'")
        .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(grants, 1, "replay preserves one Abacus source receipt");
    assert_eq!(progress(owner, admitted).await?, 2);
    Ok(())
}

pub(super) async fn prepare_earned_journey(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
) -> TestResult {
    let pin = owner.runtime.lock().await.content_pin().clone();
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(owner.world_id.as_bytes().as_slice()).bind(owner.channel_id.as_bytes().as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(owner.world_id.as_bytes().as_slice()).bind(owner.channel_id.as_bytes().as_slice())
        .bind(pin.server_artifact_digest().as_slice()).bind(pin.client_artifact_digest().as_slice())
        .bind(pin.frame_binding_digest().as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)")
        .bind(id(41).as_slice()).execute(&h.pool).await?;
    super::item_look_fixtures::seed_main_backpack(h, &[("oteryn:item.tibia.i19083", 19)]).await
}

pub(super) async fn earned_journey(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    zushuka: crate::foundation::ExactActorRef,
) -> TestResult {
    use super::qengine_rewards_fixtures::talk;
    use crate::durability::reward_claim_mint::boss_reward::{
        BossRewardCollectRequest, BossRewardCollectionStatus, BossRewardSelection,
    };
    use crate::foundation::runtime_actor_spell_types::{
        OwnerCombatBatch, OwnerCombatChange, OwnerCombatEffect, SpellOccurrenceBinding,
    };
    use crate::foundation::{CharacterId, CommandId, CommandRef};
    use crate::gameplay_transport::kill_reward::{DrainOutcome, KillSink, record_batch_kills};
    use oteryn_protocol_oteryn::boss_rewards::{
        COMMAND_TYPE_BOSS_REWARD_TAKE, TakeRequest, encode_take_request,
    };

    let version: String = sqlx::query_scalar("SHOW server_version")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("QUESTPC7_EARNED_TOKEN POSTGRES_STARTED version={version}");
    admitted.item_fence = Some(
        owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "raid item fence")?
            .ok_or("raid item fence")?
            .item_fence(),
    );
    owner.admit_progression_session(admitted).await;
    owner.quest_catalogue = Some(std::sync::Arc::new(
        oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
            .map_err(debug)?
            .catalogue()
            .clone(),
    ));
    owner.admit_quest_session(admitted).await;
    let offerings = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            &offerings,
            &[3, 6, 25],
        )
        .ok_or("Jorge capabilities")?;
    let [jorge] =
        super::classic_quests_fixtures::install(owner, admitted, ["oteryn:npc.jorge"]).await?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?
        .item_fence();
    assert_eq!(
        talk(owner, admitted, jorge, 6110, "hi").await?,
        NpcIntentDisposition::Ok
    );
    assert_eq!(
        talk(owner, admitted, jorge, 6111, "souvenir").await?,
        NpcIntentDisposition::Ok
    );
    assert_eq!(
        talk(owner, admitted, jorge, 6112, "abacus").await?,
        NpcIntentDisposition::Rejected
    );

    let native = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .ok_or("activated raid loot owner")?;
    let now = owner.owner_now();
    {
        let mut runtime = owner.runtime.lock().await;
        let actor = admitted.runtime_actor.ok_or("player actor")?;
        owner.states.spell_states.lock().await.initialize(
            &runtime, actor, admitted.game_session_id,
            crate::gameplay_transport::actor_spell::tests::FACTS, (0, 0), now,
        ).ok_or("raid player spell state")?;
        let creature = runtime.companion_snapshot(zushuka).map_err(debug)?;
        let batch = OwnerCombatBatch {
            caster: actor,
            attacker: CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?,
            current_lease_generation: fence.character_lease_generation,
            command: CommandRef::new(
                admitted.game_session_id,
                CommandId::new(6120).map_err(debug)?,
            ),
            occurrence: SpellOccurrenceBinding {
                id: "questpc7:zushuka-lethal".into(),
                revisions: ["r1"; 5].map(str::to_owned),
            },
            binding: b"questpc7-zushuka-lethal".to_vec(),
            anchor: None,
            now_us: Some(now),
            now_ms: now.get() / 1000,
            effects: vec![OwnerCombatEffect {
                target: zushuka,
                sub_ordinal: 0,
                change: OwnerCombatChange::Damage {
                    target_atom: runtime.creature_spell_target_atom(zushuka).map_err(debug)?,
                    magnitude: creature.maximum_health,
                },
            }],
            deferred: None,
        };
        let staged = runtime.stage_spell_batch(&batch).map_err(debug)?;
        let receipt = runtime.commit_spell_batch(staged).map_err(debug)?;
        assert!(receipt.applied);
        let attack = owner.states.attack.lock().await;
        record_batch_kills(
            &mut runtime,
            KillSink {
                prey: None,
                activity: &[],
                queue: attack.kills(),
                rewards: native.reward_table(),
                charms: None,
            },
            &receipt,
            now.get() / 1000,
        );
        assert!(attack.kills().has_session(admitted.game_session_id));
    }
    assert_eq!(
        owner.drain_kill_rewards(admitted.game_session_id).await,
        DrainOutcome::Drained
    );
    assert_eq!(
        count(h, 19083).await?,
        19,
        "uncollected raid loot cannot pay Jorge"
    );
    let (death, ordinal): (Vec<u8>, i32) = sqlx::query_as(
        "SELECT uuid_send(death_id),item_ordinal FROM game_boss_reward_items WHERE definition_production_key='oteryn:item.tibia.i19083' AND quantity=1",
    ).fetch_one(&h.pool).await?;
    let death: [u8; 16] = death.try_into().map_err(|_| "native death UUID")?;
    let ordinal = u16::try_from(ordinal)?;
    let mut intent = COMMAND_TYPE_BOSS_REWARD_TAKE.to_be_bytes().to_vec();
    intent.extend_from_slice(&encode_take_request(&TakeRequest { death, ordinal }).map_err(debug)?);
    let content_revision = format!(
        "sha256:{}",
        native
            .source_digest()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    );
    let content = crate::gameplay_transport::item_ref_admission::NativeItemWeights(native);
    let permit = owner.spell_lane_permit().await.ok_or("raid lane")?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?
        .item_fence();
    let collected = h
        .root
        .collect_boss_rewards(
            &permit,
            owner.character,
            owner.holder,
            fence,
            BossRewardCollectRequest {
                command: CommandRef::new(
                    admitted.game_session_id,
                    CommandId::new(6121).map_err(debug)?,
                ),
                selection: BossRewardSelection::Item { death, ordinal },
                intent,
                source_placement: "oteryn:reward-chest.raid_token_fixture".into(),
                content_revision,
                ruleset_revision: crate::interaction_chest_use::entry_chest::RULESET_REVISION
                    .into(),
                sim_revision: crate::interaction_chest_use::entry_chest::SIM_REVISION.into(),
            },
            &content,
            false,
        )
        .await?
        .ok_or("native token collection")?;
    assert_eq!(
        (collected.status, collected.moved),
        (BossRewardCollectionStatus::Collected, 1)
    );
    assert_eq!(count(h, 19083).await?, 20);
    assert_eq!(
        talk(owner, admitted, jorge, 6122, "souvenir").await?,
        NpcIntentDisposition::Ok
    );
    assert_eq!(
        talk(owner, admitted, jorge, 6123, "abacus").await?,
        NpcIntentDisposition::Ok
    );
    assert_eq!(count(h, 19083).await?, 0);
    assert_eq!(count(h, 19151).await?, 1);
    assert_eq!(
        talk(owner, admitted, jorge, 6124, "finish the commission").await?,
        NpcIntentDisposition::Ok
    );
    assert_eq!(progress(owner, admitted).await?, 2);
    let completed: bool = sqlx::query_scalar("SELECT completed_character_revision IS NOT NULL FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key='oteryn:quest.hunting_for_tokens'")
        .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert!(completed);
    Ok(())
}
