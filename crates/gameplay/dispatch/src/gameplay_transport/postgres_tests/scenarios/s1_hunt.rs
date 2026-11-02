//! S1: Prey and daily selection precede the real kill, party split and stamina debit.
use super::*;

#[test]
fn s1_hunt_prey_stamina_daily_party_loot_and_relogin() -> TestResult {
    timed("s1-hunt", scenario)
}

fn scenario() -> TestResult {
    world_scenario!("xmech_s1", |h,
                                 request,
                                 seal,
                                 authority,
                                 world_id,
                                 native,
                                 world,
                                 cells,
                                 room,
                                 pin,
                                 deps,
                                 actor,
                                 session,
                                 owner,
                                 fence,
                                 admitted| {
        use crate::durability::character_build::{
            BuildCause, BuildChangeRequest, BuildOccurrence, convert_vocation,
        };
        let formula = native.training_formula().ok_or("training formula")?;
        let before = h
            .root
            .read_character_build_state(&authority, fence.character_id)
            .await
            .map_err(debug)?;
        let after = convert_vocation(formula, &before, "monk").map_err(debug)?;
        let current = owner
            .current_quest_fence(session)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?;
        owner
            .revision_sequencer
            .acquire(fence.character_id)
            .await
            .commit_build(
                owner.root,
                owner.character,
                owner.holder,
                current,
                BuildChangeRequest {
                    occurrence: BuildOccurrence::from_bytes(id(130)).map_err(debug)?,
                    cause: BuildCause::VocationChoice,
                    before,
                    after,
                    pruned_stance: None,
                },
                formula,
                None,
            )
            .await
            .map_err(debug)?;
        let other = peer(&h, &mut owner, admitted).await?;
        join_party(&h, &owner, admitted, other).await?;
        let toggle = oteryn_protocol_oteryn::chat::ChatIntent::Say {
            mode: oteryn_protocol_oteryn::chat::ChatSpeechMode::Say,
            text: "/sharedxp on".into(),
        };
        assert_eq!(
            owner
                .party_shared_xp_chat_command(actor, session, 802, &toggle)
                .await
                .ok_or("shared xp")?
                .disposition,
            oteryn_protocol_oteryn::chat::ChatDisposition::Ok
        );
        assert_eq!(
            count(
                &h,
                "SELECT count(*) FROM game_parties WHERE shared_xp_enabled"
            )
            .await?,
            1
        );
        // A retained selectable rat, activated through PREY_V1; deterministic grade-10 XP.
        sqlx::query("INSERT INTO game_character_prey_slots(character_id,slot,state,creature_list,hunting_s_left,slot_epoch) VALUES(encode($1,'hex')::uuid,1,'SELECTION',ARRAY['oteryn:creature.rat'],0,1)").bind(id(41).as_slice()).execute(&h.pool).await?;
        use sha2::{Digest, Sha256};
        let prey_command = admitted.continuity.next_command_id;
        let occurrence = crate::gameplay_transport::npc_trade_command::derived(
            b"prey:command:v1",
            session,
            prey_command,
        );
        let secret = (0..4096_u32)
            .map(|n| Sha256::digest(n.to_be_bytes()).to_vec())
            .find(|secret| {
                let draw = |n: u64| {
                    let hash = Sha256::digest(
                        [
                            secret.as_slice(),
                            occurrence.as_slice(),
                            b"oteryn.prey.bonus.v1",
                            &n.to_be_bytes(),
                        ]
                        .concat(),
                    );
                    let mut bytes = [0; 8];
                    bytes.copy_from_slice(&hash[..8]);
                    u64::from_be_bytes(bytes)
                };
                draw(0) % 4 == 2 && draw(1) % 9 == 8
            })
            .ok_or("prey entropy")?;
        sqlx::query("UPDATE game_prey_decision_root SET secret=$1")
            .bind(secret)
            .execute(&h.pool)
            .await?;
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            &crate::gameplay_transport::prey::offered(owner.offered_capabilities(), true),
            &[6, 140],
        )
        .ok_or("prey capability")?;
        let select = oteryn_protocol_oteryn::prey::encode_request(
            &oteryn_protocol_oteryn::prey::Request::Select { slot: 1, index: 1 },
        )
        .map_err(debug)?;
        let (status, bytes) = exchange(
            &owner,
            &mut admitted,
            oteryn_protocol_oteryn::prey::COMMAND_TYPE_PREY,
            &select,
        )
        .await?;
        assert_eq!(status, oteryn_protocol_oteryn::CommandStatus::Accepted);
        let reply = oteryn_protocol_oteryn::prey::decode_reply(&bytes).map_err(debug)?;
        assert_eq!(
            (
                reply.slots[0].bonus,
                reply.slots[0].grade,
                reply.slots[0].hunting_seconds
            ),
            (3, 10, 7200)
        );
        let durable_prey: (String, i16, i32) = sqlx::query_as(
            "SELECT creature_key,grade,hunting_s_left FROM game_character_prey_slots WHERE slot=1",
        )
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(durable_prey, ("oteryn:creature.rat".into(), 10, 7200));
        assert!(matches!(
            owner.prey_wire(admitted, prey_command, &select).await,
            crate::gameplay_transport::guild_intent::GuildReply::Payload(_)
        ));
        assert_eq!(
            count(
                &h,
                "SELECT count(*) FROM game_character_prey_slots WHERE state='ACTIVE'"
            )
            .await?,
            1
        );
        // The only eligible creature in this isolated server-save fixture is the rat.
        let digest = room.map_revision_digest();
        sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,0,$2,'xmech-hunt',$2,$2,$2,'TIBIA_SERVER_SAVE_BERLIN_06',1,clock_timestamp(),'2026-11')").bind(id(42).as_slice()).bind(digest.as_slice()).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,0)")
            .bind(id(42).as_slice())
            .execute(&h.pool)
            .await?;
        {
            let permit = owner.spell_lane_permit().await.ok_or("boost lane")?;
            let choice = h
                .root
                .prepare_daily_boost(
                    &authority,
                    &h.node,
                    &permit,
                    crate::durability::daily_boost::DailyBoostBinding {
                        scope: fence.runtime_scope,
                        generation: 1,
                        bundle: digest,
                        epoch: 0,
                    },
                    (
                        vec!["oteryn:creature.rat".into()],
                        vec!["fixture:unmatched-boss".into()],
                    ),
                )
                .await
                .map_err(debug)?;
            assert_eq!(choice.creature, "oteryn:creature.rat");
            owner
                .runtime
                .lock()
                .await
                .install_daily_boost(choice)
                .map_err(debug)?;
        }
        assert_eq!(
            count(&h, "SELECT count(*) FROM game_world_daily_boosts").await?,
            1
        );
        // The shared fixture stops before first_entry's semantic clock load. No hunt
        // has happened yet: initialize both admission clocks at the real owner time.
        assert_eq!(
            count(&h, "SELECT count(*) FROM game_character_xp_receipts").await?,
            0
        );
        {
            let runtime = owner.runtime.lock().await;
            let mut states = owner.states.spell_states.lock().await;
            states.hunting_stamina.clear();
            for player in [admitted, other] {
                states.load_stamina_at(
                    &runtime,
                    player.runtime_actor.ok_or("actor")?,
                    player.game_session_id,
                    2520,
                    owner.owner_now().get(),
                );
            }
        }
        // Both members must have real recent activity. Let the peer hit the rat before the
        // leader kills it; both activity and the lethal reward use the real attack owner.
        step_west(&mut owner, actor, session).await?;
        step_west(&mut owner, actor, session).await?;
        kill_rat(&mut owner, admitted, other, pin.clone()).await?;
        let awards:Vec<(i64,i16,i32)> = sqlx::query_as("SELECT xp_awarded,prey_percent,stamina_minutes FROM game_party_kill_xp_members ORDER BY character_id").fetch_all(&h.pool).await?;
        assert_eq!(
            awards.len(),
            2,
            "the party shares the boosted kill: {awards:?}"
        );
        assert_eq!(
            count(
                &h,
                "SELECT count(*) FROM game_party_kill_xp_receipts WHERE shared"
            )
            .await?,
            1
        );
        // Explicit numerical order: rat 5 -> daily 10 -> party pool/split -> each member's
        // personal Prey, low-level multiplier, then stamina, with one final truncation.
        let leader:(i64,i16,i32) = sqlx::query_as("SELECT xp_awarded,prey_percent,stamina_minutes FROM game_party_kill_xp_members WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(leader, (19, 40, 2519));
        assert_eq!(awards, [(19, 40, 2519), (14, 0, 2519)]);
        assert_eq!(
            count(&h, "SELECT base_xp FROM game_party_kill_xp_receipts").await?,
            10
        );
        // Hunting debits are live until the production checkpoint, rather than part of
        // the XP transaction. Check both sides of that persistence boundary and its replay.
        assert_eq!(
            h.root
                .read_hunting_stamina(&authority, fence.character_id)
                .await
                .map_err(debug)?,
            2520
        );
        for player in [admitted, other] {
            let actor = player.runtime_actor.ok_or("actor")?;
            assert_eq!(
                owner
                    .states
                    .spell_states
                    .lock()
                    .await
                    .live_stamina(actor, player.game_session_id),
                Some(2519)
            );
            assert_eq!(
                owner
                    .save_hunting_stamina(actor, player.game_session_id)
                    .await,
                crate::gameplay_transport::actor_spell::TrainingSave::Saved
            );
            let saved = snapshot(&h).await?;
            assert_eq!(
                owner
                    .save_hunting_stamina(actor, player.game_session_id)
                    .await,
                crate::gameplay_transport::actor_spell::TrainingSave::NotApplicable
            );
            assert_eq!(snapshot(&h).await?, saved);
        }
        assert_eq!(
            h.root
                .read_hunting_stamina(&authority, fence.character_id)
                .await
                .map_err(debug)?,
            2519
        );
        assert_eq!(
            count(
                &h,
                "SELECT chosen_kills FROM game_character_prey_slots WHERE slot=1"
            )
            .await?,
            1
        );
        assert_eq!(
            count(&h, "SELECT count(*) FROM game_character_xp_receipts").await?,
            2
        );
        assert_eq!(ledger(&h).await?.corpse_mints, 1);
        let after = snapshot(&h).await?;
        assert_eq!(
            owner.drain_kill_rewards(session).await,
            DrainOutcome::Drained
        );
        assert_eq!(snapshot(&h).await?, after);
        let corpse = owner
            .runtime
            .lock()
            .await
            .visible_corpses()
            .into_iter()
            .next()
            .ok_or("corpse")?;
        let contents = h
            .root
            .read_corpse_contents(&authority, corpse.item.item_instance_id)
            .await
            .map_err(debug)?
            .ok_or("loot")?;
        let loot = contents.first().ok_or("rat loot")?;
        owner
            .take_corpse_entry(
                actor,
                command(admitted, 950),
                ItemKey::Entity(corpse.identity),
                ItemKey::Instance(loot.item.item_instance_id),
            )
            .await
            .map_err(debug)?;
        let location:(i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid),(SELECT quantity::bigint FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid)").bind(loot.item.item_instance_id.as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(location, (1, i64::from(loot.item.quantity)));
        let looted = snapshot(&h).await?;
        let _ = owner
            .take_corpse_entry(
                actor,
                command(admitted, 950),
                ItemKey::Entity(corpse.identity),
                ItemKey::Instance(loot.item.item_instance_id),
            )
            .await;
        assert_eq!(snapshot(&h).await?, looted);
        logout_relogin(&h, &mut owner, admitted).await?;
        assert_eq!(
            count(&h, "SELECT count(*) FROM game_party_members").await?,
            0,
            "terminal logout removes the member and dissolves the two-player party"
        );
        assert_eq!(
            h.root
                .read_hunting_stamina(&authority, fence.character_id)
                .await
                .map_err(debug)?,
            2519
        );
        assert_eq!(count(&h,"SELECT count(*) FROM game_character_prey_slots WHERE creature_key='oteryn:creature.rat'").await?,1);
        drop(owner);
        drop(authority);
        drop(seal);
        drop(request);
        h.cleanup().await
    })
}
