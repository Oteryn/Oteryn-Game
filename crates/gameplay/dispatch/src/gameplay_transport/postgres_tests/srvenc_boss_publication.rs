//! Crystal 201189f boss_lever.lua:63-76,183-218: immutable qualified room
//! policy is read under the current participant's World/content interpretation.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::content::NativeEntryStart;
use crate::content::bosses::activity_template::QualifiedActivityRoomTemplate;
use crate::durability::activity_handoff::{ActivityContentPin, ActivityHandoffOwner};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::{ChannelId, RuntimeScopeRefV1, WorldId};
use crate::gameplay_transport::item_fence_of;

const BOSS: &str = "oteryn:boss.sugar_daddy";

fn templates(
    world: WorldId,
    channel: ChannelId,
    revisions: &[&str],
) -> TestResult<Vec<QualifiedActivityRoomTemplate>> {
    let (controller, activation) =
        crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None);
    let native = controller
        .active()
        .and_then(|active| active.native_gameplay())
        .ok_or("active source gameplay")?;
    let parent = activation.qualified_room();
    let candidate = crate::content::bosses::activity_template::captured_boss_levers()
        .map_err(debug)?
        .into_iter()
        .find(|candidate| candidate.creature_key.as_deref() == Some("oteryn:creature.sugar_daddy"))
        .ok_or("captured Sugar Daddy source")?;
    let bundle = super::srvenc_source_boss::source_bundle(world, channel, native, &candidate)?;
    let cells = bundle
        .movement_cells(parent.movement_cells())
        .map_err(debug)?;
    let room = parent
        .bundle_view(
            cells,
            bundle.frame_binding_digest(world),
            bundle.map_revision_digest(),
            NativeEntryStart {
                x: 33339,
                y: 32221,
                floor: 9,
            },
        )
        .ok_or("source bundle room")?;
    revisions
        .iter()
        .map(|revision| {
            controller
                .qualify_activity_template(parent, &room, revision, &candidate)
                .map_err(debug)
        })
        .collect()
}

fn pin(template: &QualifiedActivityRoomTemplate) -> ActivityContentPin {
    ActivityContentPin {
        coordinate_frame_digest: template.room().frame_binding().digest(),
        map_revision_digest: template.room().map_revision_digest(),
        server_artifact_digest: template.room().compiled().server_digest(),
    }
}

#[test]
fn srvenc_boss_publication_postgres_scopes_replay_conflict_and_current_reader() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("scoped-boss-publication", || {
        run(async |admin| {
            reset();
            let h =
                Harness::create_with_admission_seed(admin, "scoped_boss_publication", true, false)
                    .await?;
            prepare_current_session(&h).await?;
            let deps = Dependencies::new()?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let owner = deps.owner(&h, &authority);
            deps.initialize(&owner).await;
            // The fresh admission above advanced the actual lease generation
            // to two; the original fixture fence is deliberately stale now.
            let (current_session, _) = FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(deps.admitted.game_session_id)
                .await?;
            let fence = item_fence_of(current_session).ok_or("current admitted item fence")?;
            let RuntimeScopeRefV1::Channel {
                world_id,
                channel_id,
            } = fence.runtime_scope
            else {
                return Err("current Channel fence".into());
            };
            let qualified = templates(
                world_id,
                channel_id,
                &["publication-future", "content-1", "publication-conflict"],
            )?;
            let origin = pin(&qualified[0]);
            let publisher = ActivityHandoffOwner::new(h.root.clone());
            publisher
                .publish_qualified_boss_rooms(owner.holder, &qualified[..1])
                .await?;
            assert!(
                publisher
                    .read_boss_room_with_pin(&authority, owner.holder, fence, BOSS, origin)
                    .await?
                    .is_none(),
                "a current node's future publication is not the participant's current interpretation"
            );
            publisher
                .publish_qualified_boss_rooms(owner.holder, &qualified[1..2])
                .await?;
            publisher
                .publish_qualified_boss_rooms(owner.holder, &qualified[1..2])
                .await?;
            let current = publisher
                .read_boss_room_with_pin(&authority, owner.holder, fence, BOSS, origin)
                .await?
                .ok_or("current scoped policy")?;
            assert_eq!(current.content_revision, "content-1");
            assert!(
                publisher
                    .read_boss_room(&authority, owner.holder, fence, BOSS)
                    .await?
                    .is_none(),
                "an unpinned caller cannot fall back to production identity metadata"
            );
            let mut foreign_pin = origin;
            foreign_pin.coordinate_frame_digest[0] ^= 1;
            assert!(
                publisher
                    .read_boss_room_with_pin(&authority, owner.holder, fence, BOSS, foreign_pin)
                    .await?
                    .is_none(),
                "a missing physical namespace never falls back"
            );
            let other_world = WorldId::decode(&id(44)).map_err(debug)?;
            let other_channel = ChannelId::decode(&id(45)).map_err(debug)?;
            let other = templates(other_world, other_channel, &["content-1"])?;
            publisher
                .publish_qualified_boss_rooms(owner.holder, &other)
                .await?;
            assert_eq!(
                publisher
                    .read_boss_room_with_pin(&authority, owner.holder, fence, BOSS, origin)
                    .await?
                    .ok_or("original World policy")?,
                current,
                "publication in another World cannot replace a current policy"
            );
            assert_eq!(
                sqlx::query_scalar::<_, i64>(
                    "SELECT count(*) FROM game_boss_room_publications WHERE boss_key=$1"
                )
                .bind(BOSS)
                .fetch_one(&h.pool)
                .await?,
                3
            );
            assert_eq!(
                sqlx::query_scalar::<_, String>(
                    "SELECT content_revision FROM game_boss_room_catalogue WHERE boss_key=$1"
                )
                .bind(BOSS)
                .fetch_one(&h.pool)
                .await?,
                "publication-future",
                "global FK metadata is insert-once, never the active policy pointer"
            );
            for query in [
                "UPDATE game_boss_room_publications SET after_kill_seconds=after_kill_seconds+1 WHERE boss_key=$1",
                "DELETE FROM game_boss_room_publications WHERE boss_key=$1",
                "UPDATE game_boss_room_catalogue SET content_revision='overwritten' WHERE boss_key=$1",
            ] {
                let error = sqlx::query(query)
                    .bind(BOSS)
                    .execute(&h.pool)
                    .await
                    .err()
                    .ok_or("immutable publication mutation was accepted")?;
                assert_eq!(
                    error
                        .as_database_error()
                        .and_then(|error| error.code())
                        .as_deref(),
                    Some("23514")
                );
            }
            // Simulate a conflicting independently retained publication at a fresh
            // revision. The real publisher must refuse it without rewriting history.
            sqlx::query("INSERT INTO game_boss_room_publications SELECT q.* FROM game_boss_room_publications p CROSS JOIN LATERAL jsonb_populate_record(NULL::game_boss_room_publications,to_jsonb(p)||'{\"content_revision\":\"publication-conflict\",\"after_kill_seconds\":1}'::jsonb) q WHERE p.world_id=encode($1,'hex')::uuid AND p.boss_key=$2 AND p.content_revision='content-1'")
            .bind(world_id.as_bytes().as_slice()).bind(BOSS).execute(&h.pool).await?;
            assert!(
                publisher
                    .publish_qualified_boss_rooms(owner.holder, &qualified[2..])
                    .await
                    .is_err(),
                "nonidentical replay must refuse"
            );
            assert_eq!(sqlx::query_scalar::<_, i64>("SELECT after_kill_seconds FROM game_boss_room_publications WHERE world_id=encode($1,'hex')::uuid AND boss_key=$2 AND content_revision='publication-conflict'")
            .bind(world_id.as_bytes().as_slice()).bind(BOSS).fetch_one(&h.pool).await?, 1);
            eprintln!("SRVENC_BOSS_PUBLICATION POSTGRES_CASE_COMPLETED");
            Ok(())
        })
    })
}
