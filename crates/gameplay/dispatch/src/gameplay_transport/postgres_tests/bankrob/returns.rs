use super::*;
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::foundation::{
    ChannelId, MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements,
};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::gameplay_transport::npc_command::NpcTalkOutcome;
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::sync::Arc;

pub(super) const BOARD: &str = "oteryn:item.tibia.i19236";

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Case {
    Announcements,
    Rewards,
    Fifth,
    LostAck,
    Second,
    Synchronized,
    Stale,
    PayoutFailure,
}

impl Case {
    pub(super) fn double_loot(self) -> bool {
        matches!(self, Self::Second | Self::Synchronized)
    }
}
macro_rules! case {
    ($name:ident, $case:ident) => {
        #[test]
        fn $name() -> TestResult {
            let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
                return Ok(());
            };
            super::super::kill_reward_e2e_fixtures::scenario_thread::run_async(async move |_| {
                let h = Harness::create_with_admission_seed(admin, "bankrob_return", false, false)
                    .await?;
                eprintln!(concat!("bankrob-marker: ", stringify!($name), " db ready"));
                let result = exercise(&h, Case::$case).await;
                h.cleanup().await?;
                result
            })
        }
    };
}

case!(bankrob_announcements_and_bank_suspension, Announcements);
#[test]
fn bankrob_hand_in_rewards() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::super::kill_reward_e2e_fixtures::scenario_thread::run_async(async move |_| {
        for case in [Case::Rewards, Case::Fifth] {
            let h =
                Harness::create_with_admission_seed(admin.clone(), "bankrob_rewards", false, false)
                    .await?;
            eprintln!("bankrob-marker: bankrob_hand_in_rewards db ready");
            let result = exercise(&h, case).await;
            h.cleanup().await?;
            result?;
        }
        Ok(())
    })
}
case!(bankrob_replay_after_lost_ack, LostAck);
case!(bankrob_second_hand_in, Second);
case!(bankrob_synchronized_distinct_bags, Synchronized);
case!(bankrob_stale_session, Stale);
case!(bankrob_failed_payout_rolls_back, PayoutFailure);

async fn exercise(h: &Harness, case: Case) -> TestResult {
    use oteryn_gameplay_foundation::scheduled_events::{Schedule, test_clock};
    let schedule = Schedule::parse(r#"{"events":[{"name":"double","startdate":"01/01/2030","enddate":"01/02/2030","ingame":{"lootrate":200}}]}"#).map_err(debug)?;
    let _clock = case
        .double_loot()
        .then(|| test_clock::set(schedule, 1_893_456_000));
    scenario(h, &bank_robbery::cities()?[0], false, Some(case)).await
}

pub(super) struct Bank {
    pub(super) runtime: tokio::sync::Mutex<ChannelRuntimeV1>,
    actor: ExactActorRef,
    naji: EntityRef,
    suzy: EntityRef,
    origin: MovementLocalPosition,
}

impl Bank {
    pub(super) fn new(
        scope: RuntimeScopeRefV1,
        pin: ChannelContentPin,
        session: GameSessionId,
        origin: MovementLocalPosition,
    ) -> TestResult<Self> {
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            scope.world_id(),
            scope.channel_id().ok_or("channel")?,
            NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            16,
            pin,
        )
        .map_err(debug)?;
        let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
        let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(actor, origin)
            .map_err(debug)?;
        let keys = [
            "oteryn:npc.naji",
            "oteryn:npc.suzy",
            bank_robbery::TOWNCRYER,
        ];
        let npcs = keys
            .iter()
            .enumerate()
            .map(|(i, key)| {
                Ok(NpcPlacements {
                    key: (*key).into(),
                    placements: vec![NpcPlacement {
                        x: u16::try_from(origin.x + i32::try_from(i)? + 1)?,
                        y: u16::try_from(origin.y)?,
                        floor: i8::try_from(-origin.floor)?,
                        direction: MovementFacing::South,
                    }],
                })
            })
            .collect::<TestResult<Vec<_>>>()?;
        let placed = runtime
            .place_npcs(&NpcPlacementTable { npcs }, |_| true, true)
            .map_err(debug)?;
        let entity = |index: usize| EntityRef {
            identity: placed.actors[index].placement_identity(),
            generation: placed.actors[index].actor_local_generation(),
        };
        Ok(Self {
            runtime: tokio::sync::Mutex::new(runtime),
            actor,
            naji: entity(0),
            suzy: entity(1),
            origin,
        })
    }

    pub(super) fn install(
        &self,
        owner: &ComposedFreshAdmission<'_, '_, '_>,
        session: GameSessionId,
    ) -> TestResult {
        use crate::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};
        use crate::gameplay_transport::npc_talk::NpcTalkBook;
        let catalogue = NpcDataCatalogue::load(
            std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
            std::ffi::OsStr::new("world"),
            npc_catalogue_preproduction_limits(),
        )
        .map_err(debug)?;
        let model = catalogue.service_model_for_project().map_err(debug)?;
        let mut talk = NpcTalkBook::from_model(&model);
        talk.bind_source_voices(&catalogue, &model, |text| {
            crate::chat::ChatText::parse(text)
                .ok()
                .map(|text| text.as_str().to_owned())
        })
        .map_err(debug)?;
        owner.states.chat.install_npc_talk(Arc::new(talk));
        owner
            .states
            .chat
            .admit(session, "Fixture Hero", owner.owner_now());
        owner.states.chat.set_npc_service(session);
        Ok(())
    }

    pub(super) async fn before_robbery(
        &self,
        owner: &ComposedFreshAdmission<'_, '_, '_>,
        fence: CurrentCharacterItemFence,
    ) -> TestResult {
        assert_eq!(
            self.talk(owner, fence, self.naji, 10, "hi")
                .await
                .disposition,
            NpcIntentDisposition::Ok
        );
        let prompt = self
            .talk(owner, fence, self.naji, 11, "withdraw 1 gold")
            .await;
        assert!(line(&prompt).contains('?'), "{}", line(&prompt));
        let completed = self.talk(owner, fence, self.naji, 27, "yes").await;
        assert!(
            !completed.unknown && line(&completed).contains("enough gold"),
            "{}",
            line(&completed)
        );
        let fresh = self
            .talk(owner, fence, self.naji, 28, "withdraw 1 gold")
            .await;
        assert!(line(&fresh).contains('?'), "{}", line(&fresh));
        // Park a second accepted confirmation at the native Bank settlement boundary.
        // Its durable resume must recheck closure even though enqueue happened while open.
        use crate::durability::bank::BankNpcSource;
        let conversations = &owner.states.npc_conversations;
        let pending = conversations
            .take_bank(fence.game_session_id)
            .ok_or("withdraw prompt")?;
        let request = crate::gameplay_transport::npc_command::bank_request(
            &crate::gameplay_transport::bank_npc::BankNpcOperation::Withdraw(1),
            pending.coins.clone(),
            fence.game_session_id,
            26,
        )
        .ok_or("withdraw request")?;
        let source = BankNpcSource {
            command: CommandRef::new(fence.game_session_id, CommandId::new(26).map_err(debug)?),
            npc_identity: self.naji.identity,
            npc_key: "oteryn:npc.naji".into(),
            npc_generation: self.naji.generation,
            catalogue_revision: pending.catalogue_revision,
            prompt_generation: owner
                .states
                .chat
                .npc_conversation_generation(fence.game_session_id)
                .ok_or("prompt generation")?,
            recipient_name: None,
        };
        owner
            .root
            .enqueue_npc_bank(owner.character, owner.holder, fence, source, request)
            .await?;
        assert!(conversations.prompt_bank(fence.game_session_id, pending));
        Ok(())
    }

    pub(super) async fn exercise(
        &self,
        case: Case,
        owner: &mut ComposedFreshAdmission<'_, '_, '_>,
        fence: CurrentCharacterItemFence,
        h: &Harness,
        first_bag: [u8; 16],
        second_bag: Option<[u8; 16]>,
    ) -> TestResult {
        owner.clock_origin -= std::time::Duration::from_secs(2);
        if case == Case::Announcements {
            return self.announcements(owner, fence, h).await;
        }
        if case == Case::Fifth {
            seed_return_count(h, fence).await?;
        }
        if case == Case::PayoutFailure {
            full_backpack(h, first_bag).await?;
        }
        if case == Case::Synchronized {
            return self
                .synchronized(owner, fence, h, first_bag, second_bag.ok_or("second bag")?)
                .await;
        }
        let hi = self.talk(owner, fence, self.naji, 20, "hi").await;
        assert!(line(&hi).contains("what's that bag?"), "{}", line(&hi));
        if case == Case::Stale {
            sqlx::query("UPDATE game_durability_reconnect_sessions SET current_generation=current_generation+1
                        WHERE game_session_id=encode($1,'hex')::uuid")
                .bind(fence.game_session_id.as_bytes().as_slice()).execute(&h.pool).await?;
        }
        if case == Case::LostAck {
            owner
                .states
                .npc_conversations
                .fixture_lose_robbery_ack(fence.game_session_id, 21)?;
        }
        let yes = self.talk(owner, fence, self.naji, 21, "yes").await;
        match case {
            Case::Stale | Case::PayoutFailure => {
                assert!(!yes.unknown && !yes.inventory_changed, "{}", line(&yes));
                assert_eq!(return_facts(h, first_bag).await?, (2, 1, 0, 0, 0));
                return Ok(());
            }
            Case::LostAck => {
                assert!(yes.unknown, "{}", line(&yes));
                let replay = owner
                    .resume_npc_bank(self.actor, command(fence, 21))
                    .await
                    .ok_or("return receipt replay")?;
                assert!(
                    replay.inventory_changed && line(&replay).contains("Good job"),
                    "{}",
                    line(&replay)
                );
                let mismatch = self.talk(owner, fence, self.naji, 21, "YES").await;
                assert_eq!(mismatch.disposition, NpcIntentDisposition::Rejected);
            }
            _ => assert!(
                yes.inventory_changed && line(&yes).contains("Good job"),
                "{}",
                line(&yes)
            ),
        }
        if case == Case::Fifth {
            let (count, goldhunter): (i64, bool) = sqlx::query_as("SELECT return_count,
                EXISTS(SELECT 1 FROM game_account_achievements WHERE achievement_key='oteryn:achievement/goldhunter')
                FROM game_bank_robbery_returns WHERE bag_id=encode($1,'hex')::uuid")
                .bind(first_bag.as_slice()).fetch_one(&h.pool).await?;
            assert_eq!((count, goldhunter), (5, true));
            assert_eq!(return_facts(h, first_bag).await?, (3, 2, 5, 10, 0));
            return Ok(());
        }
        assert_eq!(return_facts(h, first_bag).await?, (3, 2, 1, 10, 1));
        if case == Case::Second {
            owner.clock_origin -= std::time::Duration::from_secs(2);
            let again = self.talk(owner, fence, self.naji, 22, "yes").await;
            assert!(!again.inventory_changed, "{}", line(&again));
            let hi = self.talk(owner, fence, self.suzy, 23, "hi").await;
            assert!(line(&hi).contains("back in business"), "{}", line(&hi));
            let balance = self.talk(owner, fence, self.suzy, 25, "balance").await;
            assert!(
                line(&balance).contains("account balance"),
                "{}",
                line(&balance)
            );
            let read = owner
                .world_event(command(fence, 24))
                .await
                .ok_or("query outcome")?
                .map_err(|()| "event query rejected")?;
            let state = oteryn_protocol_oteryn::world_event::decode_world_event_state(&read)
                .map_err(debug)?;
            assert!(!state.announcement.contains(bank_robbery::BOARD_LINE));
            assert_eq!(return_facts(h, first_bag).await?, (3, 2, 1, 10, 1));
        }
        if case == Case::Rewards {
            sqlx::query("UPDATE game_bank_robbery_occurrences SET boot_ms=boot_ms-3600000")
                .execute(&h.pool)
                .await?;
            assert!(
                bank_robbery::with_test_roll(
                    "carlin",
                    h.root.poll_bank_robbery(
                        owner.character,
                        &h.node,
                        fence.runtime_scope,
                        fence.scope_ownership_generation,
                        0,
                        [true; 4],
                    )
                )
                .await?
                .is_none()
            );
            assert_eq!(return_facts(h, first_bag).await?.0, 3);
        }
        Ok(())
    }

    async fn announcements(
        &self,
        owner: &mut ComposedFreshAdmission<'_, '_, '_>,
        fence: CurrentCharacterItemFence,
        h: &Harness,
    ) -> TestResult {
        let blocked = self.talk(owner, fence, self.naji, 12, "yes").await;
        assert!(
            line(&blocked).contains("Ancient Temple"),
            "{}",
            line(&blocked)
        );
        for (index, banker) in [self.naji, self.suzy].into_iter().enumerate() {
            owner.clock_origin -= std::time::Duration::from_secs(2);
            let hi = self
                .talk(owner, fence, banker, 13 + u64::try_from(index)? * 2, "hi")
                .await;
            assert!(line(&hi).contains("Ancient Temple"), "{}", line(&hi));
            let closed = self
                .talk(
                    owner,
                    fence,
                    banker,
                    14 + u64::try_from(index)? * 2,
                    "balance",
                )
                .await;
            assert!(
                line(&closed).contains("Ancient Temple"),
                "{}",
                line(&closed)
            );
        }
        for key in ["oteryn:npc.naji", "oteryn:npc.suzy"] {
            assert!(matches!(
                owner
                    .root
                    .preview_named_npc_bank(
                        owner.character,
                        owner.holder,
                        fence,
                        None,
                        None,
                        key.into()
                    )
                    .await,
                Err(crate::durability::bank::BankError::Suspended)
            ));
        }
        let committed = owner
            .resume_npc_bank(self.actor, command(fence, 27))
            .await
            .ok_or("committed Bank receipt")?;
        assert!(
            !committed.unknown && line(&committed).contains("enough gold"),
            "{}",
            line(&committed)
        );
        let parked = owner
            .resume_npc_bank(self.actor, command(fence, 26))
            .await
            .ok_or("parked Bank confirmation")?;
        assert!(!parked.unknown && parked.disposition == NpcIntentDisposition::Ok);
        assert!(line(&parked).contains("Ancient Temple"));
        let replay = owner
            .resume_npc_bank(self.actor, command(fence, 26))
            .await
            .ok_or("suspended Bank replay")?;
        assert_eq!(replay.disposition, parked.disposition);
        assert_eq!(line(&replay), line(&parked));
        let suspended: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_npc_bank_robbery_suspensions")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(suspended, 1);
        assert_other_channel_announcement(owner, fence, h).await?;
        let operations: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_account_bank_operations")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(operations, 1);
        let result = owner
            .world_event(command(fence, 17))
            .await
            .ok_or("query outcome")?
            .map_err(|()| "event query rejected")?;
        let state = oteryn_protocol_oteryn::world_event::decode_world_event_state(&result)
            .map_err(debug)?;
        assert_eq!(state.reset_epoch, 0);
        assert!(!state.active);
        assert!(state.announcement.is_empty());
        let map = owner.bundle_map().ok_or("World Board map")?;
        let pos = crate::map::overlay::TilePos {
            x: u16::try_from(self.origin.x)?,
            y: u16::try_from(self.origin.y + 1)?,
            floor: i8::try_from(-self.origin.floor)?,
        };
        let placement = crate::map::view::placement_key(pos, 1).ok_or("Board placement")?;
        let result = owner
            .use_object(
                self.actor,
                command(fence, 18),
                oteryn_protocol_oteryn::world_object::WorldObjectTarget {
                    placement: [
                        map.overlay.base().digest().as_slice(),
                        &placement.to_be_bytes(),
                    ]
                    .concat(),
                    expected_revision: 0,
                },
            )
            .await;
        assert_eq!(
            result.disposition,
            oteryn_protocol_oteryn::world_object::UseDisposition::Committed
        );
        assert!(
            owner
                .states
                .chat
                .drain(fence.game_session_id)
                .iter()
                .any(|value| format!("{value:?}").contains(bank_robbery::BOARD_LINE))
        );
        let mut cycle = {
            let runtime = self.runtime.lock().await;
            let talk = owner.states.chat.npc_talk().ok_or("Towncryer voice book")?;
            crate::gameplay_transport::npc_voice::NpcVoiceCycle::new(&runtime, talk)
        };
        for tick in 1..=90 {
            owner.emit_npc_voices(&mut cycle, tick * 1_000_000).await;
        }
        assert!(
            owner
                .states
                .chat
                .drain(fence.game_session_id)
                .iter()
                .any(|value| format!("{value:?}").contains(bank_robbery::TOWNCRYER_LINE))
        );
        Ok(())
    }

    async fn talk(
        &self,
        owner: &ComposedFreshAdmission<'_, '_, '_>,
        fence: CurrentCharacterItemFence,
        banker: EntityRef,
        id: u64,
        text: &str,
    ) -> NpcTalkOutcome {
        owner
            .npc_talk(
                self.actor,
                command(fence, id),
                NpcTalkIntent {
                    npc_actor: banker,
                    text: text.into(),
                },
            )
            .await
    }
}

async fn assert_other_channel_announcement(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    fence: CurrentCharacterItemFence,
    h: &Harness,
) -> TestResult {
    use crate::durability::runtime_scope_assignment::{
        AssignmentCommand, AssignmentOutcome, AssignmentRequest, ControlActor, OperationKey,
        RuntimeScopeAssignmentWriter,
    };
    let world = fence.runtime_scope.world_id();
    let channel = ChannelId::decode(&id(84)).map_err(debug)?;
    let scope = RuntimeScopeRefV1::channel(world, channel);
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,1)")
        .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).execute(&h.pool).await?;
    let writer = RuntimeScopeAssignmentWriter::open(h.root.clone(), "sequencer-writer")
        .await
        .map_err(debug)?;
    assert!(matches!(
        writer
            .submit(&AssignmentRequest {
                operation_key: OperationKey::from_bytes([84; 32]),
                actor: ControlActor::new("oteryn_test_admin").map_err(debug)?,
                command: AssignmentCommand::Assign {
                    scope,
                    target: h.node.fact()
                },
            })
            .await
            .map_err(debug)?,
        AssignmentOutcome::Committed(_)
    ));
    crate::bestiary_postgres_harness::publish_readiness_for_scope(
        &h.pool, &h.root, &h.node, 1, scope,
    )
    .await?;
    assert!(
        h.root
            .bank_robbery_announcement(
                owner.character,
                &h.node,
                scope,
                ScopeOwnershipGeneration::new(1).map_err(debug)?,
                0,
            )
            .await?
    );
    Ok(())
}

fn command(fence: CurrentCharacterItemFence, command_id: u64) -> UseCommand {
    UseCommand {
        game_session_id: fence.game_session_id,
        command_id,
        item_fence: Some(fence),
    }
}

fn line(outcome: &NpcTalkOutcome) -> String {
    super::super::npc_paid_travel_fixtures::line(outcome)
}

async fn return_facts(h: &Harness, bag: [u8; 16]) -> TestResult<(i16, i16, i64, i64, i64)> {
    Ok(sqlx::query_as("SELECT o.state, i.lifecycle, (SELECT count(*) FROM game_bank_robbery_returns),
        (SELECT coalesce(sum(r.quantity),0)::bigint FROM game_reward_claim_mint_receipts r JOIN game_bank_robbery_returns USING(game_session_id,command_id)),
        (SELECT count(*) FROM game_account_achievements WHERE achievement_key='oteryn:achievement/honest_finder')
        FROM game_bank_robbery_bags b JOIN game_bank_robbery_occurrences o USING(world_id,reset_epoch)
        JOIN game_item_instances i USING(item_instance_id) WHERE b.item_instance_id=encode($1,'hex')::uuid")
        .bind(bag.as_slice()).fetch_one(&h.pool).await?)
}

async fn seed_return_count(h: &Harness, fence: CurrentCharacterItemFence) -> TestResult {
    // Import the count history without manufacturing a current bag or a payout.
    // The fifth return still goes through the real NPC and native MINT writers.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    for count in 1..=4i64 {
        let bag = id(u8::try_from(130 + count)?);
        let epoch = count + 10;
        sqlx::query("INSERT INTO game_bank_robbery_returns(game_session_id,command_id,world_id,reset_epoch,
            bag_id,character_id,npc_identity,npc_generation,npc_key,prompt_generation,request_text,return_count)
            VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,$4,encode($5,'hex')::uuid,
                encode($6,'hex')::uuid,$5,1,'oteryn:npc.naji',1,'yes',$2)")
            .bind(id(129).as_slice()).bind(count).bind(fence.runtime_scope.world_id().as_bytes().as_slice()).bind(epoch)
            .bind(bag.as_slice()).bind(fence.character_id.as_bytes().as_slice()).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(())
}

async fn full_backpack(h: &Harness, bag: [u8; 16]) -> TestResult {
    // Fixture setup fills all twenty slots while leaving the credited bag nested
    // in one existing slot. Native hand-in and payout use normal PostgreSQL guards.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    let parent: String = sqlx::query_scalar("SELECT parent_item_instance_id::text FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(bag.as_slice()).fetch_one(&mut *tx).await?;
    sqlx::query(
        "DELETE FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(bag.as_slice())
    .execute(&mut *tx)
    .await?;
    for index in 1..=20i64 {
        let fixture = id(u8::try_from(100 + index)?);
        let key = if index == 1 {
            "oteryn:item.tibia.i2854"
        } else {
            "oteryn:item.tibia.i3031"
        };
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id)
            SELECT encode($1,'hex')::uuid, world_id, 'Item',$3,'definition-r1',1,1,encode($1,'hex')::uuid
            FROM game_item_instances WHERE item_instance_id=$2::uuid")
            .bind(fixture.as_slice()).bind(&parent).bind(key).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_item_container_entries(character_id,world_id,parent_item_instance_id,item_instance_id,placement_ordinal,placed_transaction_id)
            SELECT character_id,world_id,item_instance_id,encode($1,'hex')::uuid,$3,encode($1,'hex')::uuid
            FROM game_item_container_slots WHERE item_instance_id=$2::uuid")
            .bind(fixture.as_slice()).bind(&parent).bind(index).execute(&mut *tx).await?;
    }
    sqlx::query("INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id)
        SELECT item_instance_id,world_id,encode($2,'hex')::uuid,1,encode($2,'hex')::uuid
        FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(bag.as_slice()).bind(id(101).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

impl Bank {
    async fn synchronized(
        &self,
        owner: &mut ComposedFreshAdmission<'_, '_, '_>,
        fence: CurrentCharacterItemFence,
        h: &Harness,
        first_bag: [u8; 16],
        second_bag: [u8; 16],
    ) -> TestResult {
        let now: i64 =
            sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
                .fetch_one(&h.pool)
                .await?;
        use crate::world_party_runtime_postgres_cases::{
            FreshPlayerOwners, committed_fresh_player_from_source_owned,
        };
        let (peer, _) = committed_fresh_player_from_source_owned(
            &FreshPlayerOwners {
                root: h.root.clone(),
                node: h.node.clone(),
                recovery: &h.recovery,
            },
            61,
            64,
            71,
            now,
            Some("platform-test"),
        )
        .await
        .map_err(|error| format!("Bank Robbery peer Fresh admission: {error}"))?;
        let peer_fence = crate::gameplay_transport::item_fence_of(peer).ok_or("peer fence")?;
        initialize_peer_progression(owner, peer_fence).await?;
        super::super::player_trade_fixtures::seed_trade_backpack(
            &h.pool,
            *peer.commit().character_id().as_bytes(),
            92,
            93,
            "oteryn:item.tibia.i3031",
            1,
        )
        .await?;
        let peer_actor = {
            let mut runtime = self.runtime.lock().await;
            runtime
                .bind_attacker_lease(
                    self.actor,
                    fence.game_session_id,
                    crate::foundation::CharacterLease::new(
                        crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                            .map_err(debug)?,
                        fence.character_lease_generation,
                    )
                    .map_err(debug)?,
                )
                .map_err(debug)?;
            let reservation = runtime
                .reserve_fresh_session(peer_fence.game_session_id)
                .map_err(debug)?;
            let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
            runtime
                .bind_attacker_lease(
                    actor,
                    peer_fence.game_session_id,
                    peer.current_character_lease(),
                )
                .map_err(debug)?;
            runtime
                .initialize_pinned_test_position(
                    actor,
                    MovementLocalPosition {
                        y: self.origin.y - 1,
                        ..self.origin
                    },
                )
                .map_err(debug)?;
            actor
        };
        owner
            .states
            .chat
            .admit(peer_fence.game_session_id, "Peer Hero", owner.owner_now());
        owner
            .states
            .chat
            .set_npc_service(peer_fence.game_session_id);
        swap_bag(
            owner,
            [(self.actor, fence), (peer_actor, peer_fence)],
            first_bag,
        )
        .await?;

        let ask = |actor, fence, npc, id, text: &str| {
            owner.npc_talk(
                actor,
                command(fence, id),
                NpcTalkIntent {
                    npc_actor: npc,
                    text: text.into(),
                },
            )
        };
        assert!(
            line(&ask(self.actor, fence, self.naji, 20, "hi").await).contains("what's that bag?")
        );
        assert!(
            line(&ask(peer_actor, peer_fence, self.suzy, 20, "hi").await)
                .contains("what's that bag?")
        );
        assert!(
            ask(self.actor, fence, self.naji, 21, "yes")
                .await
                .inventory_changed
        );
        assert!(
            ask(peer_actor, peer_fence, self.suzy, 21, "yes")
                .await
                .inventory_changed
        );
        for bag in [first_bag, second_bag] {
            assert_eq!(return_facts(h, bag).await?, (3, 2, 2, 20, 2));
        }
        let credited: Vec<(Vec<u8>, Vec<u8>)> = sqlx::query_as(
            "SELECT uuid_send(bag_id),uuid_send(character_id) FROM game_bank_robbery_returns ORDER BY character_id"
        ).fetch_all(&h.pool).await?;
        let mut expected = [
            (second_bag.to_vec(), fence.character_id.as_bytes().to_vec()),
            (first_bag.to_vec(), peer_fence.character_id.as_bytes().to_vec()),
        ];
        expected.sort_unstable_by(|(_, left), (_, right)| left.cmp(right));
        assert_eq!(credited, expected);
        Ok(())
    }
}

async fn initialize_peer_progression(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    item: CurrentCharacterItemFence,
) -> TestResult {
    use crate::domain::progression::{FiniteProgressionPolicy, LevelThreshold};
    use crate::durability::character_progression::{
        CurrentCharacterGameplayFence, ProgressionInitializationRequest,
    };
    use oteryn_simulation_determinism::{ExactI64, RoundingMode};
    let context = crate::bestiary_postgres_harness::context();
    owner
        .root
        .initialize_character_progression(
            owner.character,
            owner.holder,
            CurrentCharacterGameplayFence {
                character_id: item.character_id,
                game_session_id: item.game_session_id,
                connection_generation: item.connection_generation,
                character_lease_generation: item.character_lease_generation,
                runtime_scope: item.runtime_scope,
                scope_ownership_generation: item.scope_ownership_generation,
                expected_character_revision: crate::domain::CharacterRevision::new(1)
                    .map_err(debug)?,
            },
            ProgressionInitializationRequest {
                context: context.clone(),
                policy_revision: "policy-1".into(),
                reward_revision: "reward-1".into(),
                policy: FiniteProgressionPolicy {
                    context,
                    policy_revision: "policy-1".into(),
                    reward_revision: "reward-1".into(),
                    death_policy_revision: "death-1".into(),
                    declared_difference_revision: "declaration-1".into(),
                    thresholds: [
                        LevelThreshold {
                            level: 1,
                            minimum_experience: ExactI64::new(0),
                        },
                        LevelThreshold {
                            level: 2,
                            minimum_experience: ExactI64::new(100),
                        },
                    ],
                    terminal_exclusive_experience: ExactI64::new(200),
                    death_loss_numerator: 1,
                    death_loss_denominator: 10,
                    death_loss_rounding: RoundingMode::Floor,
                },
            },
        )
        .await?;
    Ok(())
}

async fn swap_bag(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    parties: [(ExactActorRef, CurrentCharacterItemFence); 2],
    bag: [u8; 16],
) -> TestResult {
    use crate::durability::fresh_admission::FreshAdmissionStore;
    use crate::gameplay_transport::connection::{
        AdmittedSession, ControllerBinding, FirstEntryOutcome, SessionContinuity,
    };
    use crate::gameplay_transport::session_ingress::SessionCommandStream;
    use oteryn_protocol_oteryn::player_trade::{Intent, ResultCode};
    let mut sessions = Vec::new();
    let store = FreshAdmissionStore::from_root(owner.root.clone());
    for ((actor, fence), account) in parties.into_iter().zip([40, 64]) {
        let current = store
            .current_session(fence.game_session_id)
            .await
            .map_err(debug)?;
        let admitted = AdmittedSession {
            game_session_id: fence.game_session_id,
            world_id: owner.world_id,
            channel_id: owner.channel_id,
            runtime_actor: Some(actor),
            first_entry: FirstEntryOutcome::Positioned,
            controller: Some(ControllerBinding {
                account_id: id(account),
                transport: current.current_transport().ok_or("trade controller")?,
            }),
            continuity: SessionContinuity {
                connection_generation: fence.connection_generation.get(),
                ..SessionContinuity::FRESH
            },
            item_fence: Some(fence),
        };
        owner.trade_open(admitted).await.ok_or("trade open")?;
        let stream = SessionCommandStream::open(Some(&owner.states.command_ingress), &admitted)
            .ok_or("trade stream")?;
        sessions.push((admitted, stream));
    }
    for (at, item) in [bag, id(93)].into_iter().enumerate() {
        let partner = parties[1 - at].0;
        let (admitted, stream) = &mut sessions[at];
        let outcome = trade(
            owner,
            *admitted,
            stream,
            1,
            Intent::Offer {
                partner: EntityRef {
                    identity: partner.placement_identity(),
                    generation: partner.actor_local_generation(),
                },
                item: 1,
            },
            Some(item),
        )
        .await?;
        assert_eq!(outcome, ResultCode::Offered);
    }
    let (_, state, _) = owner
        .trade_observation(parties[0].1.game_session_id)
        .await
        .ok_or("trade state")?;
    let trade_id = state.trade_id.ok_or("trade id")?;
    for ((admitted, stream), expected) in sessions
        .iter_mut()
        .zip([ResultCode::Accepted, ResultCode::Completed])
    {
        assert_eq!(
            trade(
                owner,
                *admitted,
                stream,
                2,
                Intent::Accept { trade_id },
                None
            )
            .await?,
            expected
        );
    }
    Ok(())
}

async fn trade(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: crate::gameplay_transport::connection::AdmittedSession,
    stream: &mut crate::gameplay_transport::session_ingress::SessionCommandStream,
    id: u64,
    intent: oteryn_protocol_oteryn::player_trade::Intent,
    item: Option<[u8; 16]>,
) -> TestResult<oteryn_protocol_oteryn::player_trade::ResultCode> {
    use crate::gameplay_transport::{item_view::ItemKey, session_ingress::CommandAction};
    assert!(matches!(
        stream.begin(id, 1, id - 1, false),
        Ok(CommandAction::Execute)
    ));
    let result = owner
        .trade_command(admitted, id, intent, item.map(ItemKey::Instance))
        .await
        .ok_or("trade command")?;
    let frame = oteryn_protocol_oteryn::encode_command_result(
        1,
        id,
        id,
        oteryn_protocol_oteryn::CommandStatus::Accepted,
        &result.payload(),
    )?;
    stream
        .terminalize_with(id, &frame, || {
            owner.player_trade_terminalized(admitted.game_session_id, id);
        })
        .ok_or("trade terminalization")?;
    Ok(result)
}
