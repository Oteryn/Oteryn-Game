//! HOUSE-1a ordinary-property storage (HOUSE-OWN-0 §3; HOUSE-TENURE §4.2/Q1a).
//!
//! Uninstalled migration fragment: no runtime grants, migration number or writer activation.
//! The bank/escrow/Premium/event transaction is a separate incomplete acquisition boundary.
//! These constraints conserve canonical property, personal slot and the existing 0038 anchor;
//! they do not turn a receipt UUID or an administrator's fixture into acquisition authority.
//! Guildhall ownership belongs to its later allocated lifecycle integration.

/// Install atomically at the CP slot, before admitting any acquisition writer. Existing legacy
/// owners need explicit reconciliation: 0038 has no first-rent or paid-until facts to import.
/// Never guess those facts, delete a legacy anchor, or change its 0047 instance binding.
#[allow(
    dead_code,
    reason = "Uninstalled owned migration fragment; CP controls registration"
)]
pub(crate) const ORDINARY_PROPERTY_SCHEMA_SQL: &str = r#"
DO $$ BEGIN
 IF EXISTS (SELECT 1 FROM game_house_ownership) THEN
  RAISE EXCEPTION 'legacy House ownership requires explicit tenancy reconciliation' USING ERRCODE='23514';
 END IF;
END $$;

CREATE TABLE game_house_properties (
 world_id UUID NOT NULL CHECK(game_character_is_uuid_v7(world_id)),
 house_key TEXT NOT NULL CHECK(game_house_scope_key_valid(house_key)),
 state TEXT NOT NULL CHECK(state IN('VACANT','AUCTION','OWNED','MOVE_OUT_PENDING','DISPOSITION','RETIRED')),
 owner_kind TEXT CHECK(owner_kind='CHARACTER'),
 owner_character_id UUID REFERENCES game_character_roots(character_id),
 owner_account_id UUID REFERENCES game_character_account_guards(account_id),
 -- Reserved shape only; this fragment cannot grant a guildhall owner.
 owner_guild_id UUID CHECK(owner_guild_id IS NULL),
 paid_until BIGINT, grace_until BIGINT, move_out_at BIGINT,
 revision NUMERIC(20,0) NOT NULL CHECK(revision BETWEEN 1 AND 18446744073709551615),
 acl_revision NUMERIC(20,0) NOT NULL CHECK(acl_revision BETWEEN 0 AND 18446744073709551615),
 -- Ownership generation is separate from lifecycle/ACL revisions, preserving the 0038 ABI.
 ownership_revision NUMERIC(20,0) NOT NULL CHECK(ownership_revision BETWEEN 0 AND 18446744073709551615),
 acquisition_receipt_ref UUID CHECK(acquisition_receipt_ref IS NULL OR game_character_is_uuid_v7(acquisition_receipt_ref)),
 content_fence_operation UUID CHECK(content_fence_operation IS NULL OR game_character_is_uuid_v7(content_fence_operation)),
 PRIMARY KEY(world_id,house_key),
 CHECK(((owner_kind IS NULL AND owner_character_id IS NULL AND owner_account_id IS NULL
         AND state IN('VACANT','AUCTION','RETIRED') AND paid_until IS NULL
         AND grace_until IS NULL AND move_out_at IS NULL)
     OR (owner_kind='CHARACTER' AND owner_character_id IS NOT NULL AND owner_account_id IS NOT NULL
         AND state IN('OWNED','MOVE_OUT_PENDING','DISPOSITION') AND paid_until IS NOT NULL AND paid_until>0)) IS TRUE),
 CHECK((ownership_revision=0)=(acquisition_receipt_ref IS NULL)),
 CHECK(grace_until IS NULL OR (paid_until IS NOT NULL AND grace_until>paid_until)),
 CHECK(move_out_at IS NULL OR move_out_at>0)
);
CREATE TABLE game_house_tiles (
 world_id UUID NOT NULL, house_key TEXT NOT NULL,
 tile_x INTEGER NOT NULL CHECK(tile_x BETWEEN 0 AND 65535),
 tile_y INTEGER NOT NULL CHECK(tile_y BETWEEN 0 AND 65535),
 tile_floor SMALLINT NOT NULL CHECK(tile_floor BETWEEN -15 AND 0),
 PRIMARY KEY(world_id,house_key,tile_x,tile_y,tile_floor),
 -- A World tile cannot belong to two physical properties.
 UNIQUE(world_id,tile_x,tile_y,tile_floor),
 FOREIGN KEY(world_id,house_key) REFERENCES game_house_properties(world_id,house_key)
);
CREATE TABLE game_account_housing_slots (
 account_id UUID NOT NULL REFERENCES game_character_account_guards(account_id),
 world_id UUID NOT NULL, house_key TEXT NOT NULL,
 kind TEXT NOT NULL CHECK(kind='PHYSICAL_HOUSE'),
 PRIMARY KEY(account_id,world_id), UNIQUE(world_id,house_key),
 FOREIGN KEY(world_id,house_key) REFERENCES game_house_properties(world_id,house_key)
);

CREATE FUNCTION game_house_property_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='DELETE' THEN
  RAISE EXCEPTION 'House property is retained through retirement' USING ERRCODE='23514';
 END IF;
 IF TG_OP='INSERT' THEN
  IF NEW.state<>'VACANT' OR NEW.owner_kind IS NOT NULL OR NEW.revision<>1
     OR NEW.acl_revision<>0 OR NEW.ownership_revision<>0 OR NEW.content_fence_operation IS NOT NULL THEN
   RAISE EXCEPTION 'House property must start vacant' USING ERRCODE='23514';
  END IF;
 ELSE
  IF (NEW.world_id,NEW.house_key) IS DISTINCT FROM (OLD.world_id,OLD.house_key) THEN
   RAISE EXCEPTION 'House identity is immutable' USING ERRCODE='23514';
  END IF;
  IF OLD.state='RETIRED' AND NEW.state<>'RETIRED' THEN
   RAISE EXCEPTION 'Retired House cannot return to auction' USING ERRCODE='23514';
  END IF;
  -- ACL edits advance only the property-global ACL revision; lifecycle writes advance revision.
  IF (to_jsonb(NEW)-'acl_revision')=(to_jsonb(OLD)-'acl_revision') THEN
   IF NEW.acl_revision<>OLD.acl_revision+1 THEN
    RAISE EXCEPTION 'House ACL requires exact revision successor' USING ERRCODE='23514';
   END IF;
  ELSIF NEW.revision<>OLD.revision+1 OR NEW.acl_revision<>OLD.acl_revision THEN
   RAISE EXCEPTION 'House lifecycle requires exact revision successor' USING ERRCODE='23514';
  END IF;
  IF (NEW.owner_kind,NEW.owner_character_id,NEW.owner_account_id)
       IS DISTINCT FROM (OLD.owner_kind,OLD.owner_character_id,OLD.owner_account_id) THEN
   IF NEW.ownership_revision<>OLD.ownership_revision+1 THEN
    RAISE EXCEPTION 'House ownership requires exact generation successor' USING ERRCODE='23514';
   END IF;
   IF NEW.owner_kind IS NOT NULL THEN
    IF OLD.owner_kind IS NOT NULL OR OLD.state<>'AUCTION' OR NEW.state<>'OWNED'
       OR NEW.acquisition_receipt_ref IS NULL
       OR NEW.acquisition_receipt_ref IS NOT DISTINCT FROM OLD.acquisition_receipt_ref THEN
     RAISE EXCEPTION 'House acquisition must succeed an auction with new receipt' USING ERRCODE='23514';
    END IF;
   ELSIF OLD.state<>'DISPOSITION' OR NEW.state NOT IN('VACANT','RETIRED')
         OR NEW.acquisition_receipt_ref IS DISTINCT FROM OLD.acquisition_receipt_ref THEN
    RAISE EXCEPTION 'House release must retain its acquisition anchor' USING ERRCODE='23514';
   END IF;
  ELSIF (NEW.ownership_revision,NEW.acquisition_receipt_ref)
       IS DISTINCT FROM (OLD.ownership_revision,OLD.acquisition_receipt_ref) THEN
   RAISE EXCEPTION 'House ownership evidence cannot change without ownership' USING ERRCODE='23514';
  END IF;
 END IF;
 IF NEW.owner_kind='CHARACTER' AND NOT EXISTS (
  SELECT 1 FROM game_character_roots r WHERE r.character_id=NEW.owner_character_id
   AND r.account_id=NEW.owner_account_id AND r.world_id=NEW.world_id AND r.lifecycle=1
 ) THEN
  RAISE EXCEPTION 'House owner must be current Character of its Account and World' USING ERRCODE='23514';
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_house_property_guard BEFORE INSERT OR UPDATE OR DELETE
 ON game_house_properties FOR EACH ROW EXECUTE FUNCTION game_house_property_guard();

ALTER TABLE game_house_ownership ALTER COLUMN owner_character_id DROP NOT NULL;
CREATE OR REPLACE FUNCTION game_house_owner_in_world() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='UPDATE' AND ((NEW.world_id,NEW.house_key) IS DISTINCT FROM (OLD.world_id,OLD.house_key)
    OR NEW.ownership_revision<>OLD.ownership_revision+1) THEN
  RAISE EXCEPTION 'House anchor requires exact ownership successor' USING ERRCODE='23514';
 END IF;
 IF NOT EXISTS (SELECT 1 FROM game_house_properties p WHERE p.world_id=NEW.world_id AND p.house_key=NEW.house_key
   AND p.owner_character_id IS NOT DISTINCT FROM NEW.owner_character_id
   AND p.ownership_revision=NEW.ownership_revision AND p.acquisition_receipt_ref=NEW.acquisition_receipt_ref) THEN
  RAISE EXCEPTION 'House anchor must follow canonical property in the same transaction' USING ERRCODE='23514';
 END IF;
 IF NEW.owner_character_id IS NOT NULL THEN
  IF NOT EXISTS (SELECT 1 FROM game_character_roots r WHERE r.character_id=NEW.owner_character_id
      AND r.world_id=NEW.world_id AND r.lifecycle=1)
     OR (TG_OP='UPDATE' AND NEW.acquisition_receipt_ref=OLD.acquisition_receipt_ref) THEN
   RAISE EXCEPTION 'House acquisition requires active owner and new receipt' USING ERRCODE='23514';
  END IF;
 ELSIF TG_OP<>'UPDATE' OR OLD.owner_character_id IS NULL
       OR NEW.acquisition_receipt_ref<>OLD.acquisition_receipt_ref THEN
  RAISE EXCEPTION 'House release clears owner and retains receipt' USING ERRCODE='23514';
 END IF;
 RETURN NEW;
END $$;
CREATE FUNCTION game_house_anchor_retained() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN RAISE EXCEPTION 'House ownership anchor cannot be deleted' USING ERRCODE='23514'; END $$;
CREATE TRIGGER game_house_anchor_retained BEFORE DELETE ON game_house_ownership
 FOR EACH ROW EXECUTE FUNCTION game_house_anchor_retained();

CREATE FUNCTION game_house_property_consistent(p_world UUID,p_house TEXT) RETURNS VOID LANGUAGE plpgsql AS $$
DECLARE p game_house_properties; a game_house_ownership;
BEGIN
 SELECT * INTO p FROM game_house_properties WHERE world_id=p_world AND house_key=p_house;
 IF NOT FOUND THEN
  RAISE EXCEPTION 'House anchor/slot has no canonical property' USING ERRCODE='23514';
 END IF;
 SELECT * INTO a FROM game_house_ownership WHERE world_id=p_world AND house_key=p_house;
 IF p.ownership_revision=0 THEN
  IF FOUND THEN RAISE EXCEPTION 'Never-owned House has legacy ownership' USING ERRCODE='23514'; END IF;
 ELSIF NOT FOUND OR a.owner_character_id IS DISTINCT FROM p.owner_character_id
       OR a.ownership_revision<>p.ownership_revision OR a.acquisition_receipt_ref<>p.acquisition_receipt_ref THEN
  RAISE EXCEPTION 'House property and legacy ownership disagree' USING ERRCODE='23514';
 END IF;
 IF p.owner_kind='CHARACTER' THEN
  IF NOT EXISTS (SELECT 1 FROM game_account_housing_slots s WHERE s.world_id=p_world AND s.house_key=p_house
      AND s.account_id=p.owner_account_id AND s.kind='PHYSICAL_HOUSE') THEN
   RAISE EXCEPTION 'Owned House requires its Account World slot' USING ERRCODE='23514';
  END IF;
 ELSIF EXISTS (SELECT 1 FROM game_account_housing_slots WHERE world_id=p_world AND house_key=p_house) THEN
  RAISE EXCEPTION 'Unowned House cannot retain personal slot' USING ERRCODE='23514';
 END IF;
END $$;
CREATE FUNCTION game_house_property_commit_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 -- An updated slot can remove the old property's slot; validate BOTH keys, not only NEW.
 IF TG_OP IN('UPDATE','DELETE') THEN PERFORM game_house_property_consistent(OLD.world_id,OLD.house_key); END IF;
 IF TG_OP IN('INSERT','UPDATE') THEN PERFORM game_house_property_consistent(NEW.world_id,NEW.house_key); END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_house_property_commit AFTER INSERT OR UPDATE ON game_house_properties
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_house_property_commit_guard();
CREATE CONSTRAINT TRIGGER game_house_anchor_commit AFTER INSERT OR UPDATE ON game_house_ownership
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_house_property_commit_guard();
CREATE CONSTRAINT TRIGGER game_house_slot_commit AFTER INSERT OR UPDATE OR DELETE ON game_account_housing_slots
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_house_property_commit_guard();

REVOKE ALL ON game_house_properties,game_house_tiles,game_account_housing_slots FROM PUBLIC,oteryn_game_runtime;
-- 0038 gave control a standalone ownership writer. It cannot outlive canonical acquisition.
REVOKE INSERT,UPDATE ON game_house_ownership FROM oteryn_game_control;
DO $$ DECLARE n TEXT; BEGIN
 FOREACH n IN ARRAY ARRAY['game_house_property_guard','game_house_owner_in_world',
  'game_house_anchor_retained','game_house_property_commit_guard'] LOOP
  EXECUTE format('ALTER FUNCTION %I() SET search_path=%I,pg_temp',n,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %I() FROM PUBLIC',n);
 END LOOP;
 EXECUTE format('ALTER FUNCTION game_house_property_consistent(UUID,TEXT) SET search_path=%I,pg_temp',current_schema());
 REVOKE ALL ON FUNCTION game_house_property_consistent(UUID,TEXT) FROM PUBLIC;
END $$;
"#;

#[cfg(test)]
mod tests {
    use super::ORDINARY_PROPERTY_SCHEMA_SQL;
    use sqlx::{Connection, Executor, PgConnection, Postgres, Row, Transaction};

    type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
    const HOUSE: &str = "oteryn:content.house.thais_1";
    const OTHER_HOUSE: &str = "oteryn:content.house.thais_2";

    struct Database {
        admin_url: String,
        name: String,
        url: String,
    }

    impl Database {
        async fn create(admin_url: String) -> TestResult<Self> {
            if !admin_url.starts_with("postgresql://oteryn_test_admin:")
                || !admin_url.ends_with("@127.0.0.1:5432/postgres")
            {
                return Err("unsafe PostgreSQL test admin URL".into());
            }
            let mut admin = PgConnection::connect(&admin_url).await?;
            let version: String = sqlx::query_scalar("SHOW server_version_num")
                .fetch_one(&mut admin)
                .await?;
            if version != "170006" {
                return Err("House ownership qualification requires PostgreSQL17.6".into());
            }
            let suffix = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos();
            let name = format!("house_ownership_{}_{suffix}", std::process::id());
            admin
                .execute(sqlx::query(sqlx::AssertSqlSafe(format!(
                    "CREATE DATABASE {name}"
                ))))
                .await?;
            admin.close().await?;
            let prefix = admin_url.strip_suffix("/postgres").ok_or("admin URL")?;
            let url = format!("{prefix}/{name}");
            Ok(Self {
                admin_url,
                name,
                url,
            })
        }

        async fn cleanup(self) -> TestResult {
            let mut admin = PgConnection::connect(&self.admin_url).await?;
            admin
                .execute(sqlx::query(sqlx::AssertSqlSafe(format!(
                    "DROP DATABASE {} WITH (FORCE)",
                    self.name
                ))))
                .await?;
            admin.close().await?;
            Ok(())
        }
    }

    fn run(
        install: bool,
        case: impl AsyncFnOnce(&mut PgConnection, &str) -> TestResult,
    ) -> TestResult {
        let admin = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL").map_err(
            |_| "House ownership PostgreSQL case was not executed: approved admin URL is required",
        )?;
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(async move {
                let database = Database::create(admin).await?;
                let outcome = async {
                    let mut connection = PgConnection::connect(&database.url).await?;
                    sqlx::migrate!("./migrations").run(&mut connection).await?;
                    if install {
                        let mut tx = connection.begin().await?;
                        tx.execute(ORDINARY_PROPERTY_SCHEMA_SQL).await?;
                        tx.commit().await?;
                    }
                    case(&mut connection, &database.url).await
                }
                .await;
                database.cleanup().await?;
                outcome
            })
    }

    struct Character {
        world: String,
        account: String,
        character: String,
    }

    /// Administrative source fixture only, never a session, Premium or acquisition proof.
    /// Native identity tables/migrations are unchanged; only root seeding bypasses triggers.
    async fn seed_character(connection: &mut PgConnection) -> TestResult<Character> {
        let mut tx = connection.begin().await?;
        tx.execute("SET LOCAL session_replication_role=replica")
            .await?;
        let world: String = sqlx::query_scalar("SELECT game_character_uuid_v7()::text")
            .fetch_one(&mut *tx)
            .await?;
        let account: String = sqlx::query_scalar(
            "INSERT INTO game_character_account_guards(account_id) VALUES(game_character_uuid_v7()) RETURNING account_id::text",
        ).fetch_one(&mut *tx).await?;
        let character: String = sqlx::query_scalar(
            "INSERT INTO game_character_roots(character_id,account_id,world_id,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,name)
             VALUES(game_character_uuid_v7(),$1::uuid,$2::uuid,1,1,'p1','r1','c1','s1','House'||translate(substr(md5(random()::text),1,12),'0123456789','ghijklmnop')) RETURNING character_id::text",
        ).bind(&account).bind(&world).fetch_one(&mut *tx).await?;
        tx.commit().await?;
        Ok(Character {
            world,
            account,
            character,
        })
    }

    async fn acquire(
        tx: &mut Transaction<'_, Postgres>,
        actor: &Character,
        house: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("INSERT INTO game_house_properties(world_id,house_key,state,revision,acl_revision,ownership_revision) VALUES($1::uuid,$2,'VACANT',1,0,0)")
            .bind(&actor.world).bind(house).execute(&mut **tx).await?;
        sqlx::query("UPDATE game_house_properties SET state='AUCTION',revision=2 WHERE world_id=$1::uuid AND house_key=$2")
            .bind(&actor.world).bind(house).execute(&mut **tx).await?;
        sqlx::query("UPDATE game_house_properties SET state='OWNED',owner_kind='CHARACTER',owner_character_id=$3::uuid,owner_account_id=$4::uuid,paid_until=2592000001,revision=3,ownership_revision=1,acquisition_receipt_ref=game_character_uuid_v7() WHERE world_id=$1::uuid AND house_key=$2")
            .bind(&actor.world).bind(house).bind(&actor.character).bind(&actor.account).execute(&mut **tx).await?;
        sqlx::query("INSERT INTO game_house_ownership(world_id,house_key,owner_character_id,ownership_revision,acquisition_receipt_ref) SELECT world_id,house_key,owner_character_id,ownership_revision,acquisition_receipt_ref FROM game_house_properties WHERE world_id=$1::uuid AND house_key=$2")
            .bind(&actor.world).bind(house).execute(&mut **tx).await?;
        sqlx::query("INSERT INTO game_account_housing_slots(account_id,world_id,house_key,kind) VALUES($1::uuid,$2::uuid,$3,'PHYSICAL_HOUSE')")
            .bind(&actor.account).bind(&actor.world).bind(house).execute(&mut **tx).await?;
        Ok(())
    }

    fn sqlstate<T>(outcome: Result<T, sqlx::Error>) -> TestResult<String> {
        match outcome {
            Ok(_) => Err("expected database refusal".into()),
            Err(error) => error
                .as_database_error()
                .and_then(|database| database.code())
                .map(|code| code.into_owned())
                .ok_or_else(|| format!("not a database error: {error}").into()),
        }
    }

    #[test]
    fn house_ownership_release_retains_instance_and_receipt_after_reload() -> TestResult {
        run(true, async |connection, url| {
            let actor = seed_character(connection).await?;
            let mut tx = connection.begin().await?;
            acquire(&mut tx, &actor, HOUSE).await?;
            tx.commit().await?;
            sqlx::query("INSERT INTO game_world_house_instances(world_id,house_key,house_revision,instance_id,compatible_content_digest,qualified_placement_digest,binding_receipt_ref) VALUES($1::uuid,$2,'r1',game_character_uuid_v7(),decode(repeat('00',32),'hex'),decode(repeat('01',32),'hex'),game_character_uuid_v7())")
                .bind(&actor.world).bind(HOUSE).execute(&mut *connection).await?;
            sqlx::query("INSERT INTO game_house_acl_receipts(receipt_id,world_id,house_key,list_id,character_id,game_session_id,command_id,ownership_revision,revision_before,revision_after,allow_everyone,members) VALUES(game_character_uuid_v7(),$1::uuid,$2,-1,$3::uuid,game_character_uuid_v7(),1,1,0,1,false,ARRAY[]::uuid[])")
                .bind(&actor.world).bind(HOUSE).bind(&actor.character).execute(&mut *connection).await?;
            let mut tx = connection.begin().await?;
            sqlx::query("UPDATE game_house_properties SET state='DISPOSITION',revision=4 WHERE world_id=$1::uuid AND house_key=$2")
                .bind(&actor.world).bind(HOUSE).execute(&mut *tx).await?;
            sqlx::query("UPDATE game_house_properties SET state='VACANT',owner_kind=NULL,owner_character_id=NULL,owner_account_id=NULL,paid_until=NULL,revision=5,ownership_revision=2 WHERE world_id=$1::uuid AND house_key=$2")
                .bind(&actor.world).bind(HOUSE).execute(&mut *tx).await?;
            sqlx::query("UPDATE game_house_ownership SET owner_character_id=NULL,ownership_revision=2 WHERE world_id=$1::uuid AND house_key=$2")
                .bind(&actor.world).bind(HOUSE).execute(&mut *tx).await?;
            sqlx::query(
                "DELETE FROM game_account_housing_slots WHERE world_id=$1::uuid AND house_key=$2",
            )
            .bind(&actor.world)
            .bind(HOUSE)
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
            let mut reopened = PgConnection::connect(url).await?;
            let row=sqlx::query("SELECT p.state,p.owner_character_id IS NULL AND a.owner_character_id IS NULL AS cleared,p.ownership_revision=a.ownership_revision AND a.ownership_revision=2 AS generation,p.acquisition_receipt_ref=a.acquisition_receipt_ref AS receipt,(SELECT count(*) FROM game_world_house_instances) AS bindings,(SELECT count(*) FROM game_house_acl_receipts) AS receipts,(SELECT count(*) FROM game_account_housing_slots) AS slots,r.character_revision::text FROM game_house_properties p JOIN game_house_ownership a USING(world_id,house_key) CROSS JOIN game_character_roots r WHERE p.world_id=$1::uuid AND p.house_key=$2 AND r.character_id=$3::uuid")
                .bind(&actor.world).bind(HOUSE).bind(&actor.character).fetch_one(&mut reopened).await?;
            assert_eq!(row.try_get::<String, _>("state")?, "VACANT");
            for field in ["cleared", "generation", "receipt"] {
                assert!(row.try_get::<bool, _>(field)?);
            }
            assert_eq!(row.try_get::<i64, _>("bindings")?, 1);
            assert_eq!(row.try_get::<i64, _>("receipts")?, 1);
            assert_eq!(row.try_get::<i64, _>("slots")?, 0);
            assert_eq!(row.try_get::<String, _>("character_revision")?, "1");
            Ok(())
        })
    }

    #[test]
    fn house_ownership_unsynchronized_release_rolls_back() -> TestResult {
        run(true, async |connection, _| {
            let actor = seed_character(connection).await?;
            let mut tx = connection.begin().await?;
            acquire(&mut tx, &actor, HOUSE).await?;
            tx.commit().await?;
            let mut tx = connection.begin().await?;
            sqlx::query("UPDATE game_house_properties SET state='DISPOSITION',revision=4 WHERE world_id=$1::uuid AND house_key=$2")
                .bind(&actor.world).bind(HOUSE).execute(&mut *tx).await?;
            sqlx::query("UPDATE game_house_properties SET state='VACANT',owner_kind=NULL,owner_character_id=NULL,owner_account_id=NULL,paid_until=NULL,revision=5,ownership_revision=2 WHERE world_id=$1::uuid AND house_key=$2")
                .bind(&actor.world).bind(HOUSE).execute(&mut *tx).await?;
            assert_eq!(sqlstate(tx.commit().await)?, "23514");
            let retained: bool=sqlx::query_scalar("SELECT p.state='OWNED' AND p.revision=3 AND p.ownership_revision=1 AND p.owner_character_id=a.owner_character_id AND EXISTS(SELECT 1 FROM game_account_housing_slots s WHERE s.world_id=p.world_id AND s.house_key=p.house_key) FROM game_house_properties p JOIN game_house_ownership a USING(world_id,house_key) WHERE p.world_id=$1::uuid AND p.house_key=$2")
                .bind(&actor.world).bind(HOUSE).fetch_one(&mut *connection).await?;
            assert!(retained);
            let mut tx = connection.begin().await?;
            let change=sqlx::query("UPDATE game_house_ownership SET owner_character_id=NULL,ownership_revision=2 WHERE world_id=$1::uuid AND house_key=$2")
                .bind(&actor.world).bind(HOUSE).execute(&mut *tx).await;
            assert_eq!(sqlstate(change)?, "23514");
            tx.rollback().await?;
            let deletion = sqlx::query(
                "DELETE FROM game_house_ownership WHERE world_id=$1::uuid AND house_key=$2",
            )
            .bind(&actor.world)
            .bind(HOUSE)
            .execute(&mut *connection)
            .await;
            assert_eq!(sqlstate(deletion)?, "23514");
            Ok(())
        })
    }

    #[test]
    fn house_ownership_concurrent_properties_compete_for_one_account_world_slot() -> TestResult {
        run(true, async |connection, url| {
            let actor = seed_character(connection).await?;
            let attempt = async |house: &str| -> Result<(), sqlx::Error> {
                let mut writer = PgConnection::connect(url).await?;
                let mut tx = writer.begin().await?;
                acquire(&mut tx, &actor, house).await?;
                tx.commit().await
            };
            let (left, right) = tokio::join!(attempt(HOUSE), attempt(OTHER_HOUSE));
            match (left, right) {
                (Ok(()), loser) | (loser, Ok(())) => assert_eq!(sqlstate(loser)?, "23505"),
                _ => return Err("expected exactly one committed property acquisition".into()),
            }
            let counts:(i64,i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM game_house_properties),(SELECT count(*) FROM game_house_ownership),(SELECT count(*) FROM game_account_housing_slots)")
                .fetch_one(&mut *connection).await?;
            assert_eq!(counts, (1, 1, 1));
            Ok(())
        })
    }

    #[test]
    fn house_ownership_retired_property_cannot_be_reopened() -> TestResult {
        run(true, async |connection, _| {
            let world: String = sqlx::query_scalar("SELECT game_character_uuid_v7()::text")
                .fetch_one(&mut *connection)
                .await?;
            sqlx::query("INSERT INTO game_house_properties(world_id,house_key,state,revision,acl_revision,ownership_revision) VALUES($1::uuid,$2,'VACANT',1,0,0)")
                .bind(&world).bind(HOUSE).execute(&mut *connection).await?;
            sqlx::query("UPDATE game_house_properties SET state='RETIRED',revision=2 WHERE world_id=$1::uuid AND house_key=$2")
                .bind(&world).bind(HOUSE).execute(&mut *connection).await?;
            for state in ["VACANT", "AUCTION"] {
                let changed=sqlx::query("UPDATE game_house_properties SET state=$3,revision=3 WHERE world_id=$1::uuid AND house_key=$2")
                    .bind(&world).bind(HOUSE).bind(state).execute(&mut *connection).await;
                assert_eq!(sqlstate(changed)?, "23514");
            }
            let retained:bool=sqlx::query_scalar("SELECT state='RETIRED' AND revision=2 AND ownership_revision=0 FROM game_house_properties WHERE world_id=$1::uuid AND house_key=$2")
                .bind(&world).bind(HOUSE).fetch_one(&mut *connection).await?;
            assert!(retained);
            Ok(())
        })
    }

    #[test]
    fn house_ownership_refuses_unreconciled_legacy_rows_without_guessing_rent() -> TestResult {
        run(false, async |connection, _| {
            let actor = seed_character(connection).await?;
            sqlx::query("INSERT INTO game_house_ownership(world_id,house_key,owner_character_id,ownership_revision,acquisition_receipt_ref) VALUES($1::uuid,$2,$3::uuid,1,game_character_uuid_v7())")
                .bind(&actor.world).bind(HOUSE).bind(&actor.character).execute(&mut *connection).await?;
            let mut tx = connection.begin().await?;
            assert_eq!(
                sqlstate(tx.execute(ORDINARY_PROPERTY_SCHEMA_SQL).await)?,
                "23514"
            );
            tx.rollback().await?;
            let retained:bool=sqlx::query_scalar("SELECT to_regclass('game_house_properties') IS NULL AND EXISTS(SELECT 1 FROM game_house_ownership WHERE world_id=$1::uuid AND house_key=$2 AND owner_character_id=$3::uuid AND ownership_revision=1)")
                .bind(&actor.world).bind(HOUSE).bind(&actor.character).fetch_one(&mut *connection).await?;
            assert!(retained);
            Ok(())
        })
    }
}
