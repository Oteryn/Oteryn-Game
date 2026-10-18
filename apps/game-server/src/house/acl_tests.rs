//! Data-rule fixtures grant no authority. Native PG cases use genuine unchanged Root fixtures.
#![allow(clippy::expect_used, reason = "Explicit test data fixtures")]
use super::*;
use crate::bestiary_postgres_harness as native;
use crate::foundation::{CommandId, ConnectionGeneration, ScopeOwnershipGeneration, WorldId};
use native::{CHARACTER, Harness, TestResult, WORLD, debug, id, item_fence};
use sqlx::Executor;

fn cid(n: u8) -> CharacterId {
    CharacterId::from_bytes(id(n)).expect("native character")
}
fn gid(n: u8) -> GuildId {
    GuildId::from_bytes(id(n)).expect("guild UUID data")
}
fn rank(n: u8) -> Rank {
    Rank::new(n).expect("rank")
}
fn house() -> HouseId {
    HouseId::new(
        WorldId::decode(&id(WORLD)).expect("world"),
        "oteryn:content.house.acl_test",
    )
    .expect("house")
}
fn request(n: u64, list: List, revision: u64, entries: Vec<Entry>) -> AclSet {
    AclSet {
        house: house(),
        command: CommandRef::new(
            item_fence().expect("native fence").game_session_id,
            CommandId::new(n).expect("command"),
        ),
        list,
        expected_revision: revision,
        entries,
    }
}
fn set(v: Vec<Entry>) -> BTreeSet<Entry> {
    rules::checked_entries(&v).expect("valid entries")
}
fn blank() -> Sources {
    Sources {
        property_revision: 1,
        revision: 1,
        owner: None,
        owner_guild: None,
        entries: BTreeMap::new(),
        guilds: BTreeMap::new(),
        member: None,
        ranks: BTreeSet::new(),
    }
}

#[test]
fn owner_subowner_guest_and_exclusion_hierarchy() {
    let mut s = blank();
    s.owner = Some(cid(41));
    s.entries.insert(
        slot(List::Subowners),
        set(vec![Entry::Character(cid(41)), Entry::Exclude(cid(41))]),
    );
    assert_eq!(
        s.view(cid(41), None).expect("view").role,
        HouseAccess::Owner
    );
    s.owner = Some(cid(61));
    assert_eq!(
        s.view(cid(41), None).expect("view").role,
        HouseAccess::NotInvited
    );
    s.entries
        .insert(slot(List::Subowners), set(vec![Entry::Character(cid(41))]));
    assert_eq!(
        s.view(cid(41), None).expect("view").role,
        HouseAccess::Subowner
    );
    assert!(rules::may_edit(HouseAccess::Subowner, List::Guests));
    assert!(!rules::may_edit(HouseAccess::Subowner, List::Subowners));
    assert!(!rules::may_edit(HouseAccess::Guest, List::Guests));
    s.entries
        .insert(slot(List::Guests), set(vec![Entry::Exclude(cid(41))]));
    assert_eq!(
        s.view(cid(41), None).expect("view").role,
        HouseAccess::NotInvited
    );
    s.owner = None;
    s.owner_guild = Some(gid(71));
    s.member = Some(Membership {
        guild: gid(71),
        level: rank(1),
        state: GuildState::Active,
    });
    assert_eq!(
        s.view(cid(41), None).expect("guild owner").role,
        HouseAccess::Owner
    );
    s.member = Some(Membership {
        guild: gid(71),
        level: rank(2),
        state: GuildState::Active,
    });
    assert_eq!(
        s.view(cid(41), None).expect("former guild leader").role,
        HouseAccess::NotInvited
    );
}
#[test]
fn selected_door_grant_requires_invitation_and_preserves_native_bounds() {
    let p = TilePos {
        x: u16::MAX,
        y: u16::MAX,
        floor: -15,
    };
    let mut s = blank();
    s.entries
        .insert(slot(List::Door(p)), set(vec![Entry::Character(cid(41))]));
    assert_eq!(
        s.view(cid(41), Some(p)).expect("view").door_list_granted,
        Some(false)
    );
    s.entries
        .insert(slot(List::Guests), set(vec![Entry::Character(cid(41))]));
    assert_eq!(
        s.view(cid(41), Some(p)).expect("view").door_list_granted,
        Some(true)
    );
    assert_eq!(
        s.view(cid(41), Some(TilePos { x: 1, ..p }))
            .expect("view")
            .door_list_granted,
        Some(false)
    );
    assert!(!rules::may_edit(HouseAccess::Subowner, List::Door(p)));
    s.owner = Some(cid(41));
    assert_eq!(
        s.view(cid(41), Some(TilePos { x: 1, ..p }))
            .expect("view")
            .door_list_granted,
        Some(true)
    );
    assert_eq!(slot(List::Door(p)), (3, 65535, 65535, -15));
    assert!(List::Door(TilePos { floor: 1, ..p }).validate().is_err());
}
#[test]
fn guild_exact_at_least_removed_rank_and_disbanded_rules() {
    let mut member = Membership {
        guild: gid(71),
        level: rank(2),
        state: GuildState::Active,
    };
    let exact = set(vec![Entry::Guild {
        guild: gid(71),
        matching: Match::Exact,
        level: rank(2),
    }]);
    let ranks = BTreeSet::from([rank(1), rank(2), rank(20)]);
    assert_eq!(
        rules::match_list(&exact, cid(41), Some(member), &ranks),
        ListMatch::Granted
    );
    assert_eq!(
        rules::match_list(&exact, cid(41), Some(member), &BTreeSet::new()),
        ListMatch::None
    );
    let atleast = set(vec![Entry::Guild {
        guild: gid(71),
        matching: Match::AtLeast,
        level: rank(3),
    }]);
    assert_eq!(
        rules::match_list(&atleast, cid(41), Some(member), &ranks),
        ListMatch::Granted
    );
    member.level = rank(4);
    assert_eq!(
        rules::match_list(&atleast, cid(41), Some(member), &ranks),
        ListMatch::None
    );
    member.level = rank(2);
    member.state = GuildState::Disbanding;
    assert_eq!(
        rules::match_list(&exact, cid(41), Some(member), &ranks),
        ListMatch::Granted
    );
    member.state = GuildState::Disbanded;
    assert_eq!(
        rules::match_list(&exact, cid(41), Some(member), &ranks),
        ListMatch::None
    );
    member.state = GuildState::Active;
    member.guild = gid(72);
    assert_eq!(
        rules::match_list(&exact, cid(41), Some(member), &ranks),
        ListMatch::None
    );
    assert!(Rank::new(0).is_err());
    assert!(Rank::new(21).is_err());
    assert!(GuildId::from_bytes([0; 16]).is_err());
}
#[test]
fn list_cap_duplicates_and_canonical_occurrence_binding() {
    let entries = (1..=200)
        .map(|n| Entry::Character(cid(n)))
        .collect::<Vec<_>>();
    assert_eq!(rules::checked_entries(&entries).expect("200").len(), 200);
    let mut over = entries.clone();
    over.push(Entry::Character(cid(201)));
    assert!(rules::checked_entries(&over).is_err());
    assert!(rules::checked_entries(&[Entry::Character(cid(41)); 2]).is_err());
    let a = request(1, List::Guests, 4, vec![]);
    let b = set(vec![Entry::Character(cid(41)), Entry::Exclude(cid(61))]);
    assert_ne!(binding(cid(41), &a, &b), binding(cid(61), &a, &b));
    let mut altered = a.clone();
    altered.expected_revision = 5;
    assert_ne!(binding(cid(41), &a, &b), binding(cid(41), &altered, &b));
    altered = a.clone();
    altered.list = List::Subowners;
    assert_ne!(binding(cid(41), &a, &b), binding(cid(41), &altered, &b));
}

// Exact candidate prerequisite slice from GUILD_BANK_SCHEMA owner, not an installed/current
// guild lifecycle producer. Source/hash is retained in the qualification manifest.
const GUILD_PREREQUISITE: &str = r#"
CREATE TABLE game_guilds(guild_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(guild_id)),
 world_id UUID NOT NULL CHECK(game_character_is_uuid_v7(world_id)),
 state SMALLINT NOT NULL CHECK(state IN(1,2,3,4)),revision NUMERIC(20,0) NOT NULL CHECK(revision BETWEEN 1 AND 18446744073709551615));
CREATE TABLE game_guild_ranks(guild_id UUID NOT NULL REFERENCES game_guilds,
 level SMALLINT NOT NULL CHECK(level BETWEEN 1 AND 20),PRIMARY KEY(guild_id,level));
CREATE TABLE game_guild_members(character_id UUID PRIMARY KEY REFERENCES game_character_roots,
 guild_id UUID NOT NULL REFERENCES game_guilds,rank_level SMALLINT NOT NULL,
 FOREIGN KEY(guild_id,rank_level) REFERENCES game_guild_ranks(guild_id,level));
CREATE UNIQUE INDEX game_guild_one_leader ON game_guild_members(guild_id) WHERE rank_level=1;
"#;
fn run<F>(tag: &'static str, body: F) -> TestResult
where
    F: AsyncFnOnce(&Harness) -> TestResult,
{
    let admin = native::configured_admin().ok_or("canonical PG17.6 required; no silent skip")?;
    let runtime = native::runtime()?;
    let h = runtime.block_on(Harness::create(admin, tag, false))?;
    let result =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| runtime.block_on(body(&h))));
    runtime.block_on(h.cleanup())?;
    match result {
        Ok(r) => r,
        Err(e) => std::panic::resume_unwind(e),
    }
}
async fn fixture(h: &Harness) -> TestResult {
    h.pool
        .execute(sqlx::raw_sql(PROPERTY_PREREQUISITE_SCHEMA_PROPOSAL))
        .await?;
    h.pool
        .execute(sqlx::raw_sql(HOUSE_ACL_SCHEMA_PROPOSAL))
        .await?;
    h.pool.execute(sqlx::raw_sql(GUILD_PREREQUISITE)).await?;
    sqlx::query("INSERT INTO game_house_properties VALUES(encode($1,'hex')::uuid,$2,'CHARACTER',encode($3,'hex')::uuid,encode($4,'hex')::uuid,NULL,1,1,'OWNED',NULL)")
        .bind(id(WORLD).as_slice()).bind(house().house_key()).bind(id(CHARACTER).as_slice()).bind(id(40).as_slice()).execute(&h.pool).await?;
    Ok(())
}
async fn counts(h: &Harness) -> TestResult<(i64, i64, String)> {
    Ok((
        h.count("game_house_acl_operations").await?,
        h.count("game_house_acl_change_outbox").await?,
        sqlx::query_scalar("SELECT acl_revision::text FROM game_house_properties")
            .fetch_one(&h.pool)
            .await?,
    ))
}

// Administrative historical root fixture uses the actual native bootstrap decoder.
async fn fixture_character(h: &Harness, seed: u8, owner: u8) -> TestResult {
    sqlx::query("INSERT INTO game_character_account_guards VALUES(encode($1,'hex')::uuid) ON CONFLICT DO NOTHING").bind(id(owner).as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_character_roots(character_id,account_id,world_id,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,name) SELECT encode($1,'hex')::uuid,encode($2,'hex')::uuid,world_id,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,$3 FROM game_character_roots WHERE character_id=encode($4,'hex')::uuid")
        .bind(id(seed).as_slice()).bind(id(owner).as_slice()).bind(format!("Vip Presence {}{}",char::from(b'A'+seed/26),char::from(b'A'+seed%26))).bind(id(CHARACTER).as_slice()).execute(&h.pool).await?;
    let distinct = |tag| {
        let mut bytes = id(seed);
        bytes[15] ^= tag;
        bytes
    };
    let operation = distinct(0x80);
    let decision = distinct(0x81);
    let uuid = |bytes: [u8; 16]| {
        let s = bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
        format!(
            "{}-{}-{}-{}-{}",
            &s[..8],
            &s[8..12],
            &s[12..16],
            &s[16..20],
            &s[20..]
        )
    };
    let raw = serde_json::to_vec(
        &serde_json::json!({"contract_version":2,"variant":"OPERATOR_CONTROL_PLANE_BOOTSTRAP","issuer_authority":"OTERYN_PLATFORM_CHARACTER_AUTHORITY","issuer_decision_id":uuid(decision),"source_revision":seed.to_string(),"operation_id":uuid(operation),"operation":"INITIAL_CHARACTER_BOOTSTRAP","account_id":uuid(id(owner)),"target_world_id":uuid(id(WORLD)),"requested_name":format!("Vip Presence {}{}",char::from(b'A'+seed/26),char::from(b'A'+seed%26)),"interpretation_context":{"profile_revision":"profile-1","ruleset_revision":"ruleset-1","content_revision":"content-1","starter_template_revision":"starter-1"},"issued_at_source":"1","expires_at_source":"120","audience":"OTERYN_GAME_CHARACTER_AUTHORITY"}),
    )?;
    let intent = crate::character_bootstrap_intent::decode_producer_response(&raw, operation)
        .map_err(debug)?;
    sqlx::query("INSERT INTO game_character_operation_receipts(operation_id,command_binding,account_id,character_id,world_id,character_revision,event_id,occurred_at,server_build_id,transaction_id,issuer_decision_id,intent_source_revision,issued_at_source,expires_at_source) VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,encode($4,'hex')::uuid,encode($5,'hex')::uuid,1,encode($6,'hex')::uuid,1,'test/1',encode($7,'hex')::uuid,encode($8,'hex')::uuid,$9,1,120)")
        .bind(operation.as_slice()).bind(intent.binding()).bind(id(owner).as_slice()).bind(id(seed).as_slice()).bind(id(WORLD).as_slice()).bind(distinct(0x82).as_slice()).bind(distinct(0x83).as_slice()).bind(decision.as_slice()).bind(i64::from(seed)).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_character_bootstrap_intent_floors(issuer_scope,source_revision,issuer_decision_id,intent_binding) SELECT 1,intent_source_revision,issuer_decision_id,command_binding FROM game_character_operation_receipts ORDER BY intent_source_revision DESC LIMIT 1 ON CONFLICT(issuer_scope) DO UPDATE SET source_revision=EXCLUDED.source_revision,issuer_decision_id=EXCLUDED.issuer_decision_id,intent_binding=EXCLUDED.intent_binding").execute(&h.pool).await?;
    Ok(())
}

#[test]
fn native_acl_unowned_property_returns_not_owner_without_receipt_or_decode_error() -> TestResult {
    run("houseaclunowned", async |h| {
        // An unowned canonical property has a NULL owner kind as well as NULL owners.
        // This fixture remains the explicitly uninstalled prerequisite schema above.
        h.pool
            .execute(sqlx::raw_sql(PROPERTY_PREREQUISITE_SCHEMA_PROPOSAL))
            .await?;
        h.pool
            .execute(sqlx::raw_sql(HOUSE_ACL_SCHEMA_PROPOSAL))
            .await?;
        h.pool.execute(sqlx::raw_sql(GUILD_PREREQUISITE)).await?;
        sqlx::query("INSERT INTO game_house_properties(world_id,house_key,revision,acl_revision,state) VALUES(encode($1,'hex')::uuid,$2,1,0,'VACANT')")
            .bind(id(WORLD).as_slice()).bind(house().house_key()).execute(&h.pool).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        for state in ["VACANT", "AUCTION", "RETIRED"] {
            sqlx::query("UPDATE game_house_properties SET state=$1 WHERE world_id=encode($2,'hex')::uuid AND house_key=$3")
                .bind(state).bind(id(WORLD).as_slice()).bind(house().house_key()).execute(&h.pool).await?;
            assert!(matches!(
                h.root
                    .read_current_house_acl(&authority, &h.node, item_fence()?, house(), None,)
                    .await,
                Err(AclError::NotOwner)
            ));
            assert!(matches!(
                h.root
                    .replace_house_acl(
                        &authority,
                        &h.node,
                        item_fence()?,
                        request(1, List::Guests, 0, vec![Entry::Character(cid(41))]),
                    )
                    .await,
                Err(AclError::NotOwner)
            ));
            assert_eq!(counts(h).await?, (0, 0, "0".into()));
        }
        Ok(())
    })
}

#[test]
fn native_acl_revision_replay_rollback_and_evidence_are_atomic() -> TestResult {
    run("houseaclcas", async |h| {
        fixture(h).await?;
        let version: String = sqlx::query_scalar("SHOW server_version")
            .fetch_one(&h.pool)
            .await?;
        assert!(version.starts_with("17.6"));
        println!("HOUSE ACL native PostgreSQL {version}");
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let a = request(1, List::Guests, 1, vec![Entry::Character(cid(41))]);
        let receipt = h
            .root
            .replace_house_acl(&authority, &h.node, item_fence()?, a.clone())
            .await
            .map_err(debug)?;
        assert_eq!(receipt.revision, 2);
        assert!(!receipt.replayed);
        assert_eq!(counts(h).await?, (1, 1, "2".into()));
        let replay = h
            .root
            .replace_house_acl(&authority, &h.node, item_fence()?, a.clone())
            .await
            .map_err(debug)?;
        assert_eq!(replay.operation, receipt.operation);
        assert!(replay.replayed);
        let mut changed = a.clone();
        changed.entries.clear();
        assert!(matches!(
            h.root
                .replace_house_acl(&authority, &h.node, item_fence()?, changed)
                .await,
            Err(AclError::ConflictingOccurrence)
        ));
        assert!(matches!(
            h.root
                .replace_house_acl(
                    &authority,
                    &h.node,
                    item_fence()?,
                    request(2, List::Subowners, 1, vec![])
                )
                .await,
            Err(AclError::StaleRevision)
        ));
        assert_eq!(counts(h).await?, (1, 1, "2".into()));
        // Native rows and exact receipt must roll back together when evidence insertion fails.
        h.pool.execute(sqlx::raw_sql("CREATE FUNCTION reject_acl_evidence() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'qualification injected evidence failure';END $$;CREATE TRIGGER reject_acl_evidence BEFORE INSERT ON game_house_acl_change_outbox FOR EACH ROW EXECUTE FUNCTION reject_acl_evidence();")).await?;
        assert!(matches!(
            h.root
                .replace_house_acl(
                    &authority,
                    &h.node,
                    item_fence()?,
                    request(3, List::Guests, 2, vec![])
                )
                .await,
            Err(AclError::Unavailable(_))
        ));
        assert_eq!(counts(h).await?, (1, 1, "2".into()));
        assert_eq!(h.count("game_house_acl_entries").await?, 1);
        h.pool
            .execute(sqlx::raw_sql(
                "DROP TRIGGER reject_acl_evidence ON game_house_acl_change_outbox",
            ))
            .await?;
        let p = TilePos {
            x: 65535,
            y: 65535,
            floor: -15,
        };
        h.root
            .replace_house_acl(
                &authority,
                &h.node,
                item_fence()?,
                request(4, List::Door(p), 2, vec![]),
            )
            .await
            .map_err(debug)?;
        let position:serde_json::Value=sqlx::query_scalar("SELECT o.change_data->'list' FROM game_house_acl_change_outbox o JOIN game_house_acl_operations op USING(operation_id) WHERE command_id=4").fetch_one(&h.pool).await?;
        assert_eq!(position, serde_json::json!([3, 65535, 65535, -15]));
        let revision:String=sqlx::query_scalar("SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid").bind(id(CHARACTER).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(revision, "1");
        assert!(
            sqlx::query("DELETE FROM game_house_acl_operations")
                .execute(&h.pool)
                .await
                .is_err()
        );
        assert!(
            sqlx::query("UPDATE game_house_acl_change_outbox SET change_data='{}'")
                .execute(&h.pool)
                .await
                .is_err()
        );
        // Historical receipt data remains readable despite stale current generation, but new
        // commands must still pass the genuine native fence.
        let mut stale = item_fence()?;
        stale.connection_generation = ConnectionGeneration::new(2).map_err(debug)?;
        assert!(
            h.root
                .replace_house_acl(&authority, &h.node, stale, a)
                .await
                .map_err(debug)?
                .replayed
        );
        assert!(matches!(
            h.root
                .replace_house_acl(
                    &authority,
                    &h.node,
                    stale,
                    request(5, List::Guests, 3, vec![])
                )
                .await,
            Err(AclError::AuthorityRejected)
        ));
        assert_eq!(counts(h).await?, (2, 2, "3".into()));
        Ok(())
    })
}

#[test]
fn native_owner_subowner_exclusions_and_independent_session_fences() -> TestResult {
    run("houseaclpermissions", async |h| {
        fixture(h).await?;
        fixture_character(h, 61, 60).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        // Genuine current native session is valid; independently change only property owner.
        sqlx::query("UPDATE game_house_properties SET owner_character_id=encode($1,'hex')::uuid,owner_account_id=encode($2,'hex')::uuid,revision=2").bind(id(61).as_slice()).bind(id(60).as_slice()).execute(&h.pool).await?;
        assert!(matches!(
            h.root
                .replace_house_acl(
                    &authority,
                    &h.node,
                    item_fence()?,
                    request(1, List::Guests, 1, vec![])
                )
                .await,
            Err(AclError::NotOwner)
        ));
        assert_eq!(counts(h).await?, (0, 0, "1".into()));
        sqlx::query("INSERT INTO game_house_acl_entries VALUES(encode($1,'hex')::uuid,$2,1,-1,-1,-99,1,encode($3,'hex')::uuid,0,0)").bind(id(WORLD).as_slice()).bind(house().house_key()).bind(id(CHARACTER).as_slice()).execute(&h.pool).await?;
        h.root
            .replace_house_acl(
                &authority,
                &h.node,
                item_fence()?,
                request(2, List::Guests, 1, vec![Entry::Character(cid(61))]),
            )
            .await
            .map_err(debug)?;
        for (n, list) in [
            (3, List::Subowners),
            (
                4,
                List::Door(TilePos {
                    x: 3,
                    y: 4,
                    floor: 0,
                }),
            ),
        ] {
            assert!(matches!(
                h.root
                    .replace_house_acl(
                        &authority,
                        &h.node,
                        item_fence()?,
                        request(n, list, 2, vec![])
                    )
                    .await,
                Err(AclError::NotOwner)
            ));
        }
        let baseline = counts(h).await?;
        let mut bad = item_fence()?;
        bad.character_lease_generation = 2;
        assert!(matches!(
            h.root
                .replace_house_acl(
                    &authority,
                    &h.node,
                    bad,
                    request(5, List::Guests, 2, vec![])
                )
                .await,
            Err(AclError::AuthorityRejected)
        ));
        bad = item_fence()?;
        bad.connection_generation = ConnectionGeneration::new(2).map_err(debug)?;
        assert!(matches!(
            h.root
                .replace_house_acl(
                    &authority,
                    &h.node,
                    bad,
                    request(6, List::Guests, 2, vec![])
                )
                .await,
            Err(AclError::AuthorityRejected)
        ));
        bad = item_fence()?;
        bad.scope_ownership_generation = ScopeOwnershipGeneration::new(2).map_err(debug)?;
        assert!(matches!(
            h.root
                .replace_house_acl(
                    &authority,
                    &h.node,
                    bad,
                    request(7, List::Guests, 2, vec![])
                )
                .await,
            Err(AclError::AuthorityRejected)
        ));
        let mut wrong = request(8, List::Guests, 2, vec![]);
        wrong.house = HouseId::new(
            WorldId::decode(&id(99)).map_err(debug)?,
            house().house_key(),
        )
        .ok_or("house")?;
        assert!(matches!(
            h.root
                .replace_house_acl(&authority, &h.node, item_fence()?, wrong)
                .await,
            Err(AclError::AuthorityRejected)
        ));
        let mut wrong = request(9, List::Guests, 2, vec![]);
        wrong.command = CommandRef::new(
            crate::foundation::GameSessionId::decode(&id(99)).map_err(debug)?,
            CommandId::new(9).map_err(debug)?,
        );
        assert!(matches!(
            h.root
                .replace_house_acl(&authority, &h.node, item_fence()?, wrong)
                .await,
            Err(AclError::AuthorityRejected)
        ));
        bad = item_fence()?;
        bad.character_id = cid(61);
        assert!(matches!(
            h.root
                .replace_house_acl(
                    &authority,
                    &h.node,
                    bad,
                    request(12, List::Guests, 2, vec![])
                )
                .await,
            Err(AclError::AuthorityRejected)
        ));
        bad = item_fence()?;
        bad.runtime_scope = RuntimeScopeRefV1::Channel {
            world_id: house().world_id(),
            channel_id: crate::foundation::ChannelId::decode(&id(99)).map_err(debug)?,
        };
        assert!(matches!(
            h.root
                .replace_house_acl(
                    &authority,
                    &h.node,
                    bad,
                    request(13, List::Guests, 2, vec![])
                )
                .await,
            Err(AclError::AuthorityRejected)
        ));
        let other_node = native::register(&h.root, 2).await?;
        assert!(matches!(
            h.root
                .replace_house_acl(
                    &authority,
                    &other_node,
                    item_fence()?,
                    request(11, List::Guests, 2, vec![])
                )
                .await,
            Err(AclError::AuthorityRejected)
        ));
        assert_eq!(counts(h).await?, baseline);
        // Exclusion defeats a valid subowner grant without editing that subowner entry.
        sqlx::query("INSERT INTO game_house_acl_entries VALUES(encode($1,'hex')::uuid,$2,2,-1,-1,-99,2,encode($3,'hex')::uuid,0,0)").bind(id(WORLD).as_slice()).bind(house().house_key()).bind(id(CHARACTER).as_slice()).execute(&h.pool).await?;
        assert_eq!(
            h.root
                .read_current_house_acl(&authority, &h.node, item_fence()?, house(), None)
                .await
                .map_err(debug)?
                .role,
            HouseAccess::NotInvited
        );
        assert!(matches!(
            h.root
                .replace_house_acl(
                    &authority,
                    &h.node,
                    item_fence()?,
                    request(10, List::Guests, 2, vec![])
                )
                .await,
            Err(AclError::NotOwner)
        ));
        assert_eq!(counts(h).await?, baseline);
        Ok(())
    })
}

#[test]
fn native_guild_current_membership_rank_and_leader_changes_share_acl_source() -> TestResult {
    run("houseaclguild", async |h| {
        fixture(h).await?;
        sqlx::query(
            "INSERT INTO game_guilds VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,2,1)",
        )
        .bind(id(71).as_slice())
        .bind(id(WORLD).as_slice())
        .execute(&h.pool)
        .await?;
        sqlx::query("INSERT INTO game_guild_ranks VALUES(encode($1,'hex')::uuid,1),(encode($1,'hex')::uuid,2),(encode($1,'hex')::uuid,20)").bind(id(71).as_slice()).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_guild_members VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1)").bind(id(CHARACTER).as_slice()).bind(id(71).as_slice()).execute(&h.pool).await?;
        sqlx::query("UPDATE game_house_properties SET owner_kind='GUILD',owner_character_id=NULL,owner_account_id=NULL,owner_guild_id=encode($1,'hex')::uuid").bind(id(71).as_slice()).execute(&h.pool).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let entry = Entry::Guild {
            guild: gid(71),
            matching: Match::AtLeast,
            level: rank(2),
        };
        h.root
            .replace_house_acl(
                &authority,
                &h.node,
                item_fence()?,
                request(1, List::Guests, 1, vec![entry]),
            )
            .await
            .map_err(debug)?;
        assert_eq!(
            h.root
                .read_current_house_acl(&authority, &h.node, item_fence()?, house(), None)
                .await
                .map_err(debug)?
                .role,
            HouseAccess::Owner
        );
        sqlx::raw_sql(
            "UPDATE game_guild_members SET rank_level=2;UPDATE game_guilds SET revision=2",
        )
        .execute(&h.pool)
        .await?;
        let view = h
            .root
            .read_current_house_acl(&authority, &h.node, item_fence()?, house(), None)
            .await
            .map_err(debug)?;
        assert_eq!(view.role, HouseAccess::Guest);
        assert_eq!(view.guild_revisions.get(&gid(71)), Some(&2));
        assert!(matches!(
            h.root
                .replace_house_acl(
                    &authority,
                    &h.node,
                    item_fence()?,
                    request(2, List::Guests, 2, vec![])
                )
                .await,
            Err(AclError::NotOwner)
        ));
        sqlx::query("UPDATE game_guild_members SET rank_level=20")
            .execute(&h.pool)
            .await?;
        assert_eq!(
            h.root
                .read_current_house_acl(&authority, &h.node, item_fence()?, house(), None)
                .await
                .map_err(debug)?
                .role,
            HouseAccess::NotInvited
        );
        sqlx::raw_sql(
            "UPDATE game_guild_members SET rank_level=1;UPDATE game_guilds SET state=3,revision=3",
        )
        .execute(&h.pool)
        .await?;
        assert!(matches!(
            h.root
                .replace_house_acl(
                    &authority,
                    &h.node,
                    item_fence()?,
                    request(3, List::Guests, 2, vec![])
                )
                .await,
            Err(AclError::GuildDisbanding)
        ));
        sqlx::query("UPDATE game_guilds SET state=4,revision=4")
            .execute(&h.pool)
            .await?;
        assert_eq!(
            h.root
                .read_current_house_acl(&authority, &h.node, item_fence()?, house(), None)
                .await
                .map_err(debug)?
                .role,
            HouseAccess::NotInvited
        );
        assert_eq!(counts(h).await?, (1, 1, "2".into()));
        Ok(())
    })
}

#[test]
fn native_concurrent_expected_revision_has_one_typed_loser_and_missing_schema_fails_closed()
-> TestResult {
    run("houseaclconcurrent", async |h| {
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        assert!(matches!(
            h.root
                .replace_house_acl(
                    &authority,
                    &h.node,
                    item_fence()?,
                    request(1, List::Guests, 1, vec![])
                )
                .await,
            Err(AclError::Unavailable(_))
        ));
        fixture(h).await?;
        // Independent genuine Root/pool: contention reaches actual native row locks, not only
        // the first Root fixture's single-connection queue.
        let second = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(second.maintain_ready_once().await?);
        let second_authority = second
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let a = h.root.replace_house_acl(
            &authority,
            &h.node,
            item_fence()?,
            request(1, List::Guests, 1, vec![]),
        );
        let b = second.replace_house_acl(
            &second_authority,
            &h.node,
            item_fence()?,
            request(2, List::Subowners, 1, vec![]),
        );
        // Tokio macros/features are unchanged; poll both genuine native futures concurrently.
        let mut a = Box::pin(a);
        let mut b = Box::pin(b);
        let mut ar = None;
        let mut br = None;
        let (a, b) = std::future::poll_fn(|cx| {
            if ar.is_none()
                && let std::task::Poll::Ready(v) = a.as_mut().poll(cx)
            {
                ar = Some(v)
            }
            if br.is_none()
                && let std::task::Poll::Ready(v) = b.as_mut().poll(cx)
            {
                br = Some(v)
            }
            if ar.is_some() && br.is_some() {
                std::task::Poll::Ready((ar.take().expect("a"), br.take().expect("b")))
            } else {
                std::task::Poll::Pending
            }
        })
        .await;
        assert!(
            matches!(
                (&a, &b),
                (Ok(_), Err(AclError::StaleRevision)) | (Err(AclError::StaleRevision), Ok(_))
            ),
            "{a:?} {b:?}"
        );
        assert_eq!(counts(h).await?, (1, 1, "2".into()));
        Ok(())
    })
}
