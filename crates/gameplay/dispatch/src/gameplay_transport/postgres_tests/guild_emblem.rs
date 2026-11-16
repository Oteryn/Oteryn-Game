//! GUILD-EMBLEM-1: Tibia's five emblems from live guild membership and active guild wars,
//! through the durable reader and the real composed observation of the connection.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::domain::{CharacterId, guild::GuildId};
use crate::durability::guild_chat_fixture as guild_fixture;
use crate::gameplay_transport::capabilities::{OfferedCapability, SelectedCapabilities};
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use oteryn_protocol_oteryn::guild_emblem::Emblem;

fn gid(n: u32) -> TestResult<GuildId> {
    GuildId::from_bytes(guild_fixture::sid(n, 3)).map_err(debug)
}

fn select(capabilities: &[u32]) -> TestResult<SelectedCapabilities> {
    Ok(SelectedCapabilities::select(
        &[
            OfferedCapability {
                id: 6,
                requires: &[],
            },
            OfferedCapability {
                id: 509,
                requires: &[6],
            },
        ],
        capabilities,
    )
    .ok_or("guild emblem capability selection")?)
}

async fn war(h: &Harness, n: u32, declaring: u32, opposing: u32, state: i16) -> TestResult {
    sqlx::query(
        "INSERT INTO game_guild_wars(war_id,world_id,declaring_guild,opposing_guild,state,frag_limit,duration_days,declared_at,started_at,ends_at) \
         VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5,10,7,1,\
         CASE WHEN $5=2 THEN 1 END,CASE WHEN $5=2 THEN 9999999999999 END)",
    )
    .bind(guild_fixture::sid(n, 40).as_slice())
    .bind(id(42).as_slice())
    .bind(gid(declaring)?.as_bytes().as_slice())
    .bind(gid(opposing)?.as_bytes().as_slice())
    .bind(state)
    .execute(&h.pool)
    .await?;
    Ok(())
}

#[test]
fn guild_emblem_relations_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "guild emblem scenario panicked")??;
    Ok(())
}

fn scenario() -> TestResult {
    run(async |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "guild_emblem", true, false).await?;
        prepare_current_session(&h).await?;
        guild_fixture::prepare(&h.pool).await?;
        // Guilds 90-92 and 94 are led by characters 90-92 and 94; 93 has no guild.
        for n in [90, 91, 92, 93, 94] {
            let character = CharacterId::from_bytes(guild_fixture::sid(n, 1)).map_err(debug)?;
            guild_fixture::historical_character(
                &h.pool,
                n,
                character,
                guild_fixture::sid(n, 2),
                id(42),
                &format!("Emblem Fixture {}", char::from(b'A' + (n - 90) as u8)),
            )
            .await?;
            if n != 93 {
                guild_fixture::initial_guild(
                    &h.pool,
                    gid(n)?,
                    id(42),
                    &format!("Emblem Guild {}", char::from(b'A' + (n - 90) as u8)),
                    character,
                    guild_fixture::sid(n, 2),
                )
                .await?;
            }
        }
        // The session's character is an ordinary member of guild 90.
        guild_fixture::initial_member(
            &h.pool,
            gid(90)?,
            CharacterId::from_bytes(id(41)).map_err(debug)?,
            id(40),
            3,
        )
        .await?;
        guild_fixture::guard(&h.pool).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let deps = Dependencies::new()?;
        deps.runtime
            .lock()
            .await
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        let owner = deps.owner(&h, &authority);
        deps.initialize(&owner).await;
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(deps.admitted.game_session_id)
                .await?
                .0;
        deps.runtime
            .lock()
            .await
            .bind_attacker_lease(
                deps.actor,
                deps.admitted.game_session_id,
                current.current_character_lease(),
            )
            .map_err(debug)?;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = crate::gameplay_transport::item_fence_of(current);
        let fence = admitted.item_fence.ok_or("item fence")?;
        let viewed = [
            id(41),
            guild_fixture::sid(90, 1),
            guild_fixture::sid(91, 1),
            guild_fixture::sid(92, 1),
            guild_fixture::sid(93, 1),
        ];
        let read = async || -> TestResult<Vec<([u8; 16], Emblem)>> {
            let mut emblems = h
                .root
                .read_guild_emblems(&authority, &h.node, fence, viewed.to_vec())
                .await
                .map_err(debug)?
                .ok_or("current fence")?;
            emblems.sort();
            Ok(emblems)
        };
        let expect = |pairs: &[([u8; 16], Emblem)]| {
            let mut pairs = pairs.to_vec();
            pairs.sort();
            pairs
        };
        let (own, mate, rival, other) = (
            id(41),
            guild_fixture::sid(90, 1),
            guild_fixture::sid(91, 1),
            guild_fixture::sid(92, 1),
        );
        // No war: own guild is Member, any other guild is Other, no guild has no emblem.
        assert_eq!(
            read().await?,
            expect(&[
                (own, Emblem::Member),
                (mate, Emblem::Member),
                (rival, Emblem::Other),
                (other, Emblem::Other)
            ])
        );
        // A pending declaration is no war yet.
        war(&h, 1, 90, 91, 1).await?;
        assert_eq!(
            read().await?,
            expect(&[
                (own, Emblem::Member),
                (mate, Emblem::Member),
                (rival, Emblem::Other),
                (other, Emblem::Other)
            ])
        );
        // Active wars: 90 against 91 and 92 against 94 (not the observer's war).
        sqlx::query("UPDATE game_guild_wars SET state=2,started_at=1,ends_at=9999999999999 WHERE war_id=encode($1,'hex')::uuid")
            .bind(guild_fixture::sid(1, 40).as_slice())
            .execute(&h.pool)
            .await?;
        war(&h, 2, 92, 94, 2).await?;
        assert_eq!(
            read().await?,
            expect(&[
                (own, Emblem::Ally),
                (mate, Emblem::Ally),
                (rival, Emblem::Enemy),
                (other, Emblem::Neutral)
            ])
        );
        // The connection's real observation: the positioned own player, green while at war.
        admitted.continuity.selected_capabilities = select(&[6])?;
        assert_eq!(owner.observe_guild_emblems(admitted).await, None);
        admitted.continuity.selected_capabilities = select(&[6, 509])?;
        let observed = owner
            .observe_guild_emblems(admitted)
            .await
            .ok_or("observed emblems")?;
        assert_eq!(observed.len(), 1);
        assert_eq!(observed[0].entity.identity, deps.actor.placement_identity());
        assert_eq!(observed[0].emblem, Emblem::Ally);
        let mut stale = admitted;
        stale.continuity.connection_generation += 1;
        assert_eq!(owner.observe_guild_emblems(stale).await, None);
        // An ended war returns to Member.
        sqlx::query("UPDATE game_guild_wars SET state=3,ended_reason=1 WHERE war_id=encode($1,'hex')::uuid")
            .bind(guild_fixture::sid(1, 40).as_slice())
            .execute(&h.pool)
            .await?;
        let observed = owner
            .observe_guild_emblems(admitted)
            .await
            .ok_or("observed after war")?;
        assert_eq!(observed.iter().map(|e| e.emblem).collect::<Vec<_>>(), [Emblem::Member]);
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
