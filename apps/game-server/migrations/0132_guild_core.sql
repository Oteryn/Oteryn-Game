-- GUILD-1: installed core; no wire/capability or Bank/Hall activation.
-- GUILD TABLES BEGIN

CREATE TABLE game_guilds(
 guild_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(guild_id)),
 world_id UUID NOT NULL CHECK(game_character_is_uuid_v7(world_id)),
 name TEXT NOT NULL CHECK(game_character_is_name(name)),
 name_key TEXT GENERATED ALWAYS AS(game_character_name_key(name)) STORED,
 state SMALLINT NOT NULL CHECK(state BETWEEN 1 AND 4),
 formation_deadline BIGINT NOT NULL,vice_deficit_since BIGINT NULL,premium_deficit_since BIGINT NULL,
 motd TEXT NOT NULL DEFAULT '' CHECK(char_length(motd)<=255),
 revision NUMERIC(20,0) NOT NULL CHECK(revision BETWEEN 1 AND 18446744073709551615),
 created_at BIGINT NOT NULL);
CREATE UNIQUE INDEX game_guild_live_name ON game_guilds(world_id,name_key) WHERE state<>4;
CREATE TABLE game_guild_ranks(
 guild_id UUID NOT NULL REFERENCES game_guilds,level SMALLINT NOT NULL CHECK(level BETWEEN 1 AND 20),
 name TEXT NOT NULL CHECK(name<>''),name_key TEXT GENERATED ALWAYS AS(game_character_name_key(name)) STORED,
 PRIMARY KEY(guild_id,level),CONSTRAINT game_guild_rank_name_unique UNIQUE(guild_id,name_key) DEFERRABLE INITIALLY IMMEDIATE);
CREATE TABLE game_guild_members(
 character_id UUID PRIMARY KEY REFERENCES game_character_roots,guild_id UUID NOT NULL REFERENCES game_guilds,
 rank_level SMALLINT NOT NULL,title TEXT NOT NULL DEFAULT '' CHECK(char_length(title)<=29),joined_at BIGINT NOT NULL,
 FOREIGN KEY(guild_id,rank_level) REFERENCES game_guild_ranks);
CREATE INDEX game_guild_member_cleanup ON game_guild_members(guild_id,character_id);
CREATE UNIQUE INDEX game_guild_one_leader ON game_guild_members(guild_id) WHERE rank_level=1;
CREATE TABLE game_guild_invitations(
 guild_id UUID NOT NULL REFERENCES game_guilds,character_id UUID NOT NULL REFERENCES game_character_roots,
 inviter UUID NOT NULL REFERENCES game_character_roots,expires_at BIGINT NOT NULL,PRIMARY KEY(guild_id,character_id));
CREATE INDEX game_guild_target_invitations ON game_guild_invitations(character_id,guild_id);
CREATE TABLE game_account_guild_leadership(
 account_id UUID PRIMARY KEY REFERENCES game_character_account_guards,guild_id UUID NOT NULL REFERENCES game_guilds,
 character_id UUID NOT NULL UNIQUE REFERENCES game_character_roots);
CREATE INDEX game_guild_leadership_cleanup ON game_account_guild_leadership(guild_id,account_id);
CREATE FUNCTION game_guild_rank_names_valid(names TEXT[]) RETURNS BOOLEAN
LANGUAGE sql IMMUTABLE AS $$
 SELECT coalesce(array_ndims(names)=1 AND array_lower(names,1)=1 AND cardinality(names) BETWEEN 3 AND 20
   AND NOT EXISTS(SELECT 1 FROM unnest(names) AS n(value) WHERE value IS NULL OR value='')
   AND (SELECT count(DISTINCT game_character_name_key(value)) FROM unnest(names) AS n(value))=cardinality(names),false)
$$;
CREATE TABLE game_guild_operations(
 occurrence UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(occurrence)),
 transaction_id UUID NOT NULL UNIQUE CHECK(game_character_is_uuid_v7(transaction_id)),
 requested_event_id UUID NOT NULL UNIQUE CHECK(game_character_is_uuid_v7(requested_event_id)),
 game_session_id UUID NOT NULL CHECK(game_character_is_uuid_v7(game_session_id)),
 command_id NUMERIC(20,0) NOT NULL CHECK(command_id BETWEEN 1 AND 18446744073709551615),
 actor UUID NOT NULL REFERENCES game_character_roots,world_id UUID NOT NULL,guild_id UUID NOT NULL,
 target UUID NOT NULL CHECK(game_character_is_uuid_v7(target)),action SMALLINT NOT NULL CHECK(action BETWEEN 1 AND 13),
 request_binding BYTEA NOT NULL CHECK(octet_length(request_binding)=32),result SMALLINT NOT NULL CHECK(result BETWEEN 0 AND 12),
 previous_revision NUMERIC(20,0),revision NUMERIC(20,0),event_id UUID UNIQUE,audit_sha256 BYTEA,
 found_name TEXT CHECK((action=6 AND found_name IS NOT NULL AND game_character_is_name(found_name)) OR (action<>6 AND found_name IS NULL)),
 expected_revision NUMERIC(20,0),edit_text TEXT,previous_rank SMALLINT,edit_rank SMALLINT,edit_rank_names TEXT[],disband_confirmation TEXT,
 CHECK(CASE WHEN action=13 AND result=0 THEN disband_confirmation IS NOT NULL AND char_length(disband_confirmation)<=29 ELSE disband_confirmation IS NULL END),
 CHECK(CASE WHEN action=11 AND result=0 THEN game_guild_rank_names_valid(edit_rank_names) ELSE edit_rank_names IS NULL END),
 CHECK(CASE WHEN action IN(7,8) AND result=0 THEN expected_revision IS NOT NULL AND expected_revision=previous_revision AND expected_revision>=1 AND edit_text IS NOT NULL
   AND ((action=7 AND char_length(edit_text)<=29) OR (action=8 AND char_length(edit_text) BETWEEN 1 AND 255 AND octet_length(edit_text)<=1020))
   WHEN action IN(9,10,11) AND result=0 THEN expected_revision IS NOT NULL AND expected_revision=previous_revision AND expected_revision>=1 AND edit_text IS NULL
   ELSE expected_revision IS NULL AND edit_text IS NULL END),
 CHECK(CASE WHEN action=9 AND result=0 THEN previous_rank BETWEEN 2 AND 20 AND previous_rank IS NOT NULL AND edit_rank BETWEEN 2 AND 20 AND edit_rank IS NOT NULL WHEN action=10 AND result=0 THEN previous_rank=2 AND previous_rank IS NOT NULL AND edit_rank=1 AND edit_rank IS NOT NULL ELSE previous_rank IS NULL AND edit_rank IS NULL END),
 CHECK(result<>12 OR action IN(7,8,9,10,11)),
 created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
 UNIQUE(game_session_id,command_id),UNIQUE(created_xact_id),
 CHECK(CASE WHEN result=0 THEN previous_revision IS NOT NULL AND revision IS NOT NULL
 AND event_id IS NOT NULL AND audit_sha256 IS NOT NULL
 AND ((action=6 AND previous_revision=0 AND revision=1) OR (action<>6 AND previous_revision>=1 AND revision=previous_revision+1)) AND event_id=requested_event_id AND octet_length(audit_sha256)=32
 ELSE previous_revision IS NOT DISTINCT FROM revision AND event_id IS NULL AND audit_sha256 IS NULL END));
CREATE TABLE game_guild_event_outbox(
 event_id UUID PRIMARY KEY,occurrence UUID NOT NULL UNIQUE REFERENCES game_guild_operations,
 guild_id UUID NOT NULL REFERENCES game_guilds DEFERRABLE INITIALLY DEFERRED,occurred_at BIGINT NOT NULL CHECK(occurred_at>0),
 payload BYTEA NOT NULL CHECK(octet_length(payload) BETWEEN 1 AND 4096),
 payload_sha256 BYTEA NOT NULL CHECK(payload_sha256=sha256(payload)),
 created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id());
-- Pending start is retained independently of the later D1098 terminal-step journal.
CREATE TABLE game_guild_job_checkpoints(
 guild_id UUID NOT NULL REFERENCES game_guilds,step SMALLINT NOT NULL CHECK(step=1),
 world_id UUID NOT NULL CHECK(game_character_is_uuid_v7(world_id)),
 reason SMALLINT NOT NULL CHECK(reason IN(1,2,3)),
 previous_revision NUMERIC(20,0) NOT NULL CHECK(previous_revision BETWEEN 1 AND 18446744073709551614),
 revision NUMERIC(20,0) NOT NULL CHECK(revision=previous_revision+1),
 occurred_at BIGINT NOT NULL CHECK(occurred_at>0),
 created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),PRIMARY KEY(guild_id,step));
CREATE TABLE game_guild_job_event_outbox(
 guild_id UUID NOT NULL,step SMALLINT NOT NULL,
 payload BYTEA NOT NULL CHECK(octet_length(payload) BETWEEN 1 AND 4096),
 payload_sha256 BYTEA NOT NULL CHECK(payload_sha256=sha256(payload)),
 created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),PRIMARY KEY(guild_id,step),
 FOREIGN KEY(guild_id,step) REFERENCES game_guild_job_checkpoints);
-- D1098: immutable ordinal steps, separate from the already retained start receipt.
CREATE TABLE game_guild_disband_steps(
 guild_id UUID NOT NULL REFERENCES game_guilds,step INTEGER NOT NULL CHECK(step BETWEEN 2 AND 29),
 phase SMALLINT NOT NULL CHECK(phase BETWEEN 2 AND 6),world_id UUID NOT NULL,
 previous_revision NUMERIC(20,0) NOT NULL CHECK(previous_revision BETWEEN 1 AND 18446744073709551614),
 revision NUMERIC(20,0) NOT NULL CHECK(revision=previous_revision+1),
 payout_transaction_id UUID NULL,
 member_ids UUID[] NOT NULL,invitation_ids UUID[] NOT NULL,leadership_accounts UUID[] NOT NULL,
 CHECK(cardinality(member_ids)+cardinality(invitation_ids)<=100),
 CHECK(cardinality(leadership_accounts)<=cardinality(member_ids)),
 occurred_at BIGINT NOT NULL CHECK(occurred_at>0),created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
 PRIMARY KEY(guild_id,step),UNIQUE(guild_id,revision),UNIQUE(created_xact_id));
CREATE TABLE game_guild_disband_step_events(
 guild_id UUID NOT NULL,step INTEGER NOT NULL,
 payload BYTEA NOT NULL CHECK(octet_length(payload) BETWEEN 1 AND 4096),
 payload_sha256 BYTEA NOT NULL CHECK(payload_sha256=sha256(payload)),
 created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),PRIMARY KEY(guild_id,step),
 FOREIGN KEY(guild_id,step) REFERENCES game_guild_disband_steps);

-- GUILD GUARDS BEGIN

CREATE FUNCTION game_guild_rows_immutable() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN RAISE EXCEPTION 'retained Guild operation/event cannot be changed' USING ERRCODE='23514';END $$;
CREATE TRIGGER game_guild_operation_immutable BEFORE UPDATE OR DELETE ON game_guild_operations FOR EACH ROW EXECUTE FUNCTION game_guild_rows_immutable();
CREATE TRIGGER game_guild_event_immutable BEFORE UPDATE OR DELETE ON game_guild_event_outbox FOR EACH ROW EXECUTE FUNCTION game_guild_rows_immutable();
CREATE TRIGGER game_guild_operation_stamp BEFORE INSERT ON game_guild_operations FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();
CREATE TRIGGER game_guild_event_stamp BEFORE INSERT ON game_guild_event_outbox FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();
DO $$ DECLARE tab TEXT;BEGIN
 FOREACH tab IN ARRAY ARRAY['game_guilds','game_guild_ranks','game_guild_members','game_guild_invitations','game_account_guild_leadership','game_guild_operations','game_guild_event_outbox'] LOOP
  EXECUTE format('CREATE TRIGGER %I BEFORE TRUNCATE ON %I FOR EACH STATEMENT EXECUTE FUNCTION game_guild_rows_immutable()',tab||'_no_truncate',tab);
 END LOOP;
END $$;

CREATE FUNCTION game_guild_command_mutation() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE o game_guild_operations%ROWTYPE;r SMALLINT;w UUID;n BIGINT;now_millis BIGINT;rank_state SMALLINT;rank_deficit BIGINT;j game_guild_job_checkpoints%ROWTYPE;d game_guild_disband_steps%ROWTYPE;bound_guild UUID;
BEGIN
 IF TG_OP='DELETE' THEN bound_guild=OLD.guild_id;ELSE bound_guild=NEW.guild_id;END IF;
 SELECT * INTO d FROM game_guild_disband_steps WHERE guild_id=bound_guild AND created_xact_id=pg_current_xact_id();
 IF FOUND THEN
  IF TG_TABLE_NAME='game_guilds' AND TG_OP='UPDATE' THEN
   IF OLD.state<>3 OR NEW.state<>(CASE WHEN d.phase=6 THEN 4 ELSE 3 END)
      OR OLD.world_id<>d.world_id OR OLD.revision<>d.previous_revision OR NEW.revision<>d.revision
      OR (to_jsonb(OLD)-'state'-'revision'-'name_key') IS DISTINCT FROM (to_jsonb(NEW)-'state'-'revision'-'name_key') THEN
    RAISE EXCEPTION 'Disband changes only its exact current state/revision' USING ERRCODE='23514';END IF;
   RETURN NEW;
  ELSIF TG_TABLE_NAME='game_guild_members' THEN
   IF TG_OP='DELETE' AND d.phase IN(4,6) AND OLD.character_id=ANY(d.member_ids) THEN RETURN OLD;END IF;
  ELSIF TG_TABLE_NAME='game_guild_invitations' THEN
   IF TG_OP='DELETE' AND d.phase=5 AND OLD.character_id=ANY(d.invitation_ids) THEN RETURN OLD;END IF;
  ELSIF TG_TABLE_NAME='game_account_guild_leadership' THEN
   IF TG_OP='DELETE' AND d.phase IN(4,6) AND OLD.account_id=ANY(d.leadership_accounts) THEN RETURN OLD;END IF;
  ELSIF TG_TABLE_NAME='game_guild_ranks' THEN
   IF TG_OP='DELETE' AND d.phase=6 THEN RETURN OLD;END IF;
  END IF;
  RAISE EXCEPTION 'Disband permits only its exact bounded row set' USING ERRCODE='23514';
 END IF;
 IF TG_TABLE_NAME='game_guilds' AND TG_OP='UPDATE' THEN
  SELECT * INTO j FROM game_guild_job_checkpoints WHERE guild_id=OLD.guild_id AND step=1 AND created_xact_id=pg_current_xact_id();
  IF FOUND THEN
   IF OLD.state NOT IN(1,2) OR NEW.state<>3 OR OLD.world_id<>j.world_id
      OR OLD.revision<>j.previous_revision OR NEW.revision<>j.revision
      OR (to_jsonb(OLD)-'state'-'revision'-'name_key') IS DISTINCT FROM (to_jsonb(NEW)-'state'-'revision'-'name_key') THEN
    RAISE EXCEPTION 'Guild job only freezes its due Guild at the exact next revision' USING ERRCODE='23514';END IF;
   RETURN NEW;
  END IF;
 END IF;
 SELECT * INTO o FROM game_guild_operations WHERE created_xact_id=pg_current_xact_id() AND result=0;
 IF NOT FOUND THEN RAISE EXCEPTION 'Guild mutation requires its same-transaction operation' USING ERRCODE='23514';END IF;
 IF TG_TABLE_NAME='game_guilds' THEN
  IF TG_OP='INSERT' THEN
   n=floor(extract(epoch FROM transaction_timestamp())*1000)::bigint;
   IF o.action<>6 OR NEW.guild_id<>o.guild_id OR NEW.world_id<>o.world_id OR NEW.name<>o.found_name
      OR NEW.state<>1 OR NEW.revision<>1 OR o.previous_revision<>0 OR o.revision<>1
      OR NEW.created_at<>n OR NEW.formation_deadline<>n+259200000
      OR NEW.vice_deficit_since IS NOT NULL OR NEW.premium_deficit_since IS NOT NULL OR NEW.motd<>'' THEN
    RAISE EXCEPTION 'Found must create exactly its bound initial Guild' USING ERRCODE='23514';END IF;
   RETURN NEW;
  END IF;
  IF o.action=12 THEN
   IF TG_OP<>'UPDATE' OR OLD.guild_id<>o.guild_id OR OLD.world_id<>o.world_id OR OLD.state NOT IN(1,2)
      OR o.actor<>o.target OR OLD.revision<>o.previous_revision OR NEW.revision<>o.revision
      OR (to_jsonb(OLD)-'revision'-'name_key') IS DISTINCT FROM (to_jsonb(NEW)-'revision'-'name_key') THEN
    RAISE EXCEPTION 'Decline changes only its locked Guild revision' USING ERRCODE='23514';END IF;
   RETURN NEW;
  END IF;
  IF o.action=11 THEN
   IF TG_OP<>'UPDATE' OR OLD.guild_id<>o.guild_id OR OLD.world_id<>o.world_id OR OLD.state NOT IN(1,2)
      OR OLD.revision IS DISTINCT FROM o.expected_revision OR OLD.revision IS DISTINCT FROM o.previous_revision
      OR NEW.revision IS DISTINCT FROM o.revision OR o.target<>o.actor
      OR (to_jsonb(OLD)-'revision'-'name_key') IS DISTINCT FROM (to_jsonb(NEW)-'revision'-'name_key')
      OR NOT EXISTS(SELECT 1 FROM game_guild_members WHERE guild_id=o.guild_id AND character_id=o.actor AND rank_level=1) THEN
    RAISE EXCEPTION 'EditRanks changes only its current leader Guild revision' USING ERRCODE='23514';END IF;
   RETURN NEW;
  END IF;
  IF o.action=10 THEN
   IF TG_OP<>'UPDATE' OR OLD.guild_id<>o.guild_id OR OLD.world_id<>o.world_id OR OLD.state NOT IN(1,2)
      OR OLD.revision<>o.expected_revision OR OLD.revision<>o.previous_revision OR NEW.revision<>o.revision
      OR (to_jsonb(OLD)-'revision'-'name_key')<>(to_jsonb(NEW)-'revision'-'name_key')
      OR NOT EXISTS(SELECT 1 FROM game_guild_members WHERE guild_id=o.guild_id AND character_id=o.actor AND rank_level=2)
      OR NOT EXISTS(SELECT 1 FROM game_guild_members WHERE guild_id=o.guild_id AND character_id=o.target AND rank_level=1) THEN
    RAISE EXCEPTION 'Resign changes only revision after its exact fixed role swap' USING ERRCODE='23514';END IF;
   RETURN NEW;
  END IF;
  IF o.action=9 THEN
   SELECT rank_level INTO r FROM game_guild_members WHERE guild_id=o.guild_id AND character_id=o.actor;
   SELECT count(*) INTO n FROM game_guild_members WHERE guild_id=o.guild_id AND rank_level=2;
   now_millis=floor(extract(epoch FROM transaction_timestamp())*1000)::bigint;
   rank_state=CASE WHEN OLD.state=1 AND n>=4 THEN 2 ELSE OLD.state END;
   rank_deficit=CASE WHEN rank_state=2 AND n>=4 THEN NULL WHEN rank_state=2 THEN coalesce(OLD.vice_deficit_since,now_millis) ELSE OLD.vice_deficit_since END;
   IF TG_OP<>'UPDATE' OR OLD.guild_id<>o.guild_id OR OLD.world_id<>o.world_id OR OLD.state NOT IN(1,2)
      OR OLD.revision<>o.expected_revision OR OLD.revision<>o.previous_revision OR NEW.revision<>o.revision
      OR NOT coalesce(r<o.previous_rank AND r<o.edit_rank,false)
      OR NEW.state<>rank_state OR NEW.vice_deficit_since IS DISTINCT FROM rank_deficit
      OR (to_jsonb(OLD)-'revision'-'state'-'vice_deficit_since'-'name_key')<>(to_jsonb(NEW)-'revision'-'state'-'vice_deficit_since'-'name_key')
      OR (n>=4 AND ((OLD.state=1 AND now_millis>OLD.formation_deadline)
          OR (OLD.state=2 AND OLD.vice_deficit_since IS NOT NULL AND now_millis::numeric-OLD.vice_deficit_since::numeric>1209600000))) THEN
    RAISE EXCEPTION 'rank edit requires actual roles/revision and exact synchronous lifecycle' USING ERRCODE='23514';END IF;
   RETURN NEW;
  END IF;
  IF o.action IN(7,8) THEN
   SELECT rank_level INTO r FROM game_guild_members WHERE guild_id=o.guild_id AND character_id=o.actor;
   IF TG_OP<>'UPDATE' OR OLD.guild_id<>o.guild_id OR OLD.world_id<>o.world_id OR OLD.state NOT IN(1,2)
      OR OLD.revision<>o.expected_revision OR OLD.revision<>o.previous_revision OR NEW.revision<>o.revision
      OR NOT coalesce((o.action=7 AND r=1 AND EXISTS(SELECT 1 FROM game_guild_members WHERE guild_id=o.guild_id AND character_id=o.target))
         OR (o.action=8 AND r<=2 AND o.target=o.actor),false) THEN
    RAISE EXCEPTION 'text edit requires its actual current Guild/role/revision' USING ERRCODE='23514';END IF;
   IF o.action=7 THEN
    IF (to_jsonb(OLD)-'revision'-'name_key')<>(to_jsonb(NEW)-'revision'-'name_key') THEN
     RAISE EXCEPTION 'title edit changes no other Guild field' USING ERRCODE='23514';END IF;
   ELSE
    IF NEW.motd IS DISTINCT FROM o.edit_text OR (to_jsonb(OLD)-'revision'-'name_key'-'motd')<>(to_jsonb(NEW)-'revision'-'name_key'-'motd') THEN
     RAISE EXCEPTION 'message edit changes only its exact bound text' USING ERRCODE='23514';END IF;
   END IF;
   RETURN NEW;
  END IF;
  -- Stored generated columns are NULL in NEW until AFTER BEFORE triggers. Its immutable
  -- source name stays in the comparison; the generated expression and unique index stay active.
  IF TG_OP<>'UPDATE' OR OLD.guild_id<>o.guild_id OR OLD.state NOT IN(1,2)
    OR OLD.revision<>o.previous_revision OR NEW.revision<>o.revision
    OR (to_jsonb(OLD)-'revision'-'vice_deficit_since'-'name_key')<>(to_jsonb(NEW)-'revision'-'vice_deficit_since'-'name_key') THEN
   RAISE EXCEPTION 'Guild mutation changes only the locked revision/vice deficit' USING ERRCODE='23514';END IF;
  SELECT count(*) INTO n FROM game_guild_members WHERE guild_id=o.guild_id AND rank_level=2;
  IF NEW.vice_deficit_since IS DISTINCT FROM (CASE WHEN NEW.state=2 AND n<4 THEN coalesce(OLD.vice_deficit_since,floor(extract(epoch FROM transaction_timestamp())*1000)::bigint) ELSE OLD.vice_deficit_since END) THEN
   RAISE EXCEPTION 'vice deficit must retain its first time' USING ERRCODE='23514';END IF;
  RETURN NEW;
 ELSIF TG_TABLE_NAME='game_guild_members' THEN
  IF TG_OP='INSERT' THEN
   SELECT world_id INTO w FROM game_character_roots WHERE character_id=NEW.character_id AND lifecycle=1;
   IF o.action=6 THEN
    IF NEW.character_id<>o.actor OR NEW.guild_id<>o.guild_id OR NEW.rank_level<>1 OR NEW.title<>''
       OR NEW.joined_at<>floor(extract(epoch FROM transaction_timestamp())*1000)::bigint OR w IS DISTINCT FROM o.world_id THEN
     RAISE EXCEPTION 'Found must create its actual same-world founder' USING ERRCODE='23514';END IF;
    RETURN NEW;
   END IF;
   IF o.action<>3 OR NEW.character_id<>o.actor OR NEW.guild_id<>o.guild_id
      OR NEW.rank_level<>(SELECT max(level) FROM game_guild_ranks WHERE guild_id=o.guild_id)
      OR NEW.rank_level<3 OR w IS DISTINCT FROM o.world_id
      OR NOT EXISTS(SELECT 1 FROM game_guild_invitations WHERE guild_id=o.guild_id AND character_id=o.actor AND expires_at>floor(extract(epoch FROM transaction_timestamp())*1000)::bigint) THEN
    RAISE EXCEPTION 'join must consume the actual live invitation into the lowest rank' USING ERRCODE='23514';END IF;
   RETURN NEW;
  ELSIF TG_OP='UPDATE' AND o.action=11 THEN
   IF OLD.guild_id<>o.guild_id OR OLD.rank_level<=cardinality(o.edit_rank_names)
      OR NEW.rank_level IS DISTINCT FROM cardinality(o.edit_rank_names)
      OR (to_jsonb(OLD)-'rank_level') IS DISTINCT FROM (to_jsonb(NEW)-'rank_level')
      OR NOT EXISTS(SELECT 1 FROM game_guilds WHERE guild_id=o.guild_id AND world_id=o.world_id AND state IN(1,2) AND revision=o.expected_revision)
      OR NOT EXISTS(SELECT 1 FROM game_guild_members WHERE guild_id=o.guild_id AND character_id=o.actor AND rank_level=1) THEN
    RAISE EXCEPTION 'EditRanks moves only removed-tail members to exact new lowest' USING ERRCODE='23514';END IF;
   RETURN NEW;
  ELSIF TG_OP='UPDATE' AND o.action=10 THEN
   IF OLD.guild_id<>o.guild_id OR (to_jsonb(OLD)-'rank_level')<>(to_jsonb(NEW)-'rank_level')
      OR NOT EXISTS(SELECT 1 FROM game_guilds WHERE guild_id=o.guild_id AND world_id=o.world_id AND state IN(1,2) AND revision=o.expected_revision)
      OR NOT EXISTS(SELECT 1 FROM game_character_roots WHERE character_id=OLD.character_id AND world_id=o.world_id AND lifecycle=1)
      OR NOT ((OLD.character_id=o.actor AND OLD.rank_level=1 AND NEW.rank_level=2
                AND EXISTS(SELECT 1 FROM game_guild_members WHERE guild_id=o.guild_id AND character_id=o.target AND rank_level=2))
          OR (OLD.character_id=o.target AND OLD.rank_level=2 AND NEW.rank_level=1
                AND EXISTS(SELECT 1 FROM game_guild_members WHERE guild_id=o.guild_id AND character_id=o.actor AND rank_level=2)
                AND NOT EXISTS(SELECT 1 FROM game_guild_members WHERE guild_id=o.guild_id AND rank_level=1))) THEN
    RAISE EXCEPTION 'Resign requires exact ordered current leader/vice swap' USING ERRCODE='23514';END IF;
   RETURN NEW;
  ELSIF TG_OP='UPDATE' AND o.action=9 THEN
   SELECT rank_level INTO r FROM game_guild_members WHERE guild_id=o.guild_id AND character_id=o.actor;
   IF OLD.guild_id<>o.guild_id OR OLD.character_id<>o.target OR OLD.rank_level<>o.previous_rank OR NEW.rank_level<>o.edit_rank
      OR NOT coalesce(r<OLD.rank_level AND r<NEW.rank_level,false)
      OR (to_jsonb(OLD)-'rank_level')<>(to_jsonb(NEW)-'rank_level')
      OR NOT EXISTS(SELECT 1 FROM game_guilds WHERE guild_id=o.guild_id AND world_id=o.world_id AND state IN(1,2) AND revision=o.expected_revision)
      OR NOT EXISTS(SELECT 1 FROM game_character_roots WHERE character_id=o.target AND world_id=o.world_id AND lifecycle=1) THEN
    RAISE EXCEPTION 'rank edit requires its exact current member/role/bound levels' USING ERRCODE='23514';END IF;
   RETURN NEW;
  ELSIF TG_OP='UPDATE' AND o.action=7 THEN
   SELECT rank_level INTO r FROM game_guild_members WHERE guild_id=o.guild_id AND character_id=o.actor;
   IF r IS DISTINCT FROM 1 OR OLD.guild_id<>o.guild_id OR OLD.character_id<>o.target OR NEW.title IS DISTINCT FROM o.edit_text
      OR (to_jsonb(OLD)-'title')<>(to_jsonb(NEW)-'title')
      OR NOT EXISTS(SELECT 1 FROM game_guilds WHERE guild_id=o.guild_id AND world_id=o.world_id AND state IN(1,2) AND revision=o.expected_revision) THEN
    RAISE EXCEPTION 'title requires the current leader and exact member/text' USING ERRCODE='23514';END IF;
   RETURN NEW;
  ELSIF TG_OP='DELETE' THEN
   SELECT rank_level INTO r FROM game_guild_members WHERE character_id=o.actor AND guild_id=o.guild_id;
   IF OLD.guild_id<>o.guild_id OR OLD.character_id<>o.target OR OLD.rank_level=1
      OR NOT coalesce((o.action=4 AND o.actor=OLD.character_id) OR (o.action=5 AND r<=2 AND r<OLD.rank_level),false) THEN
    RAISE EXCEPTION 'leave/exclude requires the actual lower-ranked member' USING ERRCODE='23514';END IF;
   RETURN OLD;
  END IF;
 ELSIF TG_TABLE_NAME='game_guild_invitations' THEN
  SELECT rank_level INTO r FROM game_guild_members WHERE character_id=o.actor AND guild_id=o.guild_id;
  IF TG_OP='INSERT' THEN
   SELECT world_id INTO w FROM game_character_roots WHERE character_id=NEW.character_id AND lifecycle=1;
   IF o.action<>1 OR NEW.guild_id<>o.guild_id OR NEW.character_id<>o.target OR NEW.inviter<>o.actor
      OR r IS NULL OR r>2 OR w IS DISTINCT FROM o.world_id
      OR NEW.expires_at<>floor(extract(epoch FROM transaction_timestamp())*1000)::bigint+2592000000
      OR EXISTS(SELECT 1 FROM game_guild_members WHERE character_id=NEW.character_id AND guild_id=NEW.guild_id) THEN
    RAISE EXCEPTION 'invitation needs current same-world target and leader/vice' USING ERRCODE='23514';END IF;
   RETURN NEW;
  ELSIF TG_OP='DELETE' AND OLD.character_id=o.target AND (
      (o.action=1 AND OLD.expires_at<=floor(extract(epoch FROM transaction_timestamp())*1000)::bigint)
      OR (o.action=2 AND OLD.guild_id=o.guild_id AND r<=2)
      OR (o.action=12 AND o.actor=o.target AND OLD.guild_id=o.guild_id)
      OR (o.action=3 AND o.actor=o.target)) THEN RETURN OLD;
  END IF;
 ELSIF TG_TABLE_NAME='game_guild_ranks' AND o.action=11 THEN
  IF NOT game_guild_rank_names_valid(o.edit_rank_names) OR o.target<>o.actor
     OR NOT EXISTS(SELECT 1 FROM game_guilds WHERE guild_id=o.guild_id AND world_id=o.world_id AND state IN(1,2) AND revision=o.expected_revision)
     OR NOT EXISTS(SELECT 1 FROM game_guild_members WHERE guild_id=o.guild_id AND character_id=o.actor AND rank_level=1) THEN
   RAISE EXCEPTION 'EditRanks definitions require their current leader and bound array' USING ERRCODE='23514';END IF;
  IF TG_OP='DELETE' THEN
   IF OLD.guild_id<>o.guild_id OR OLD.level<=cardinality(o.edit_rank_names) THEN
    RAISE EXCEPTION 'EditRanks deletes only removed-tail definitions' USING ERRCODE='23514';END IF;
   RETURN OLD;
  END IF;
  IF NEW.guild_id<>o.guild_id OR NEW.level>cardinality(o.edit_rank_names)
     OR NEW.name IS DISTINCT FROM o.edit_rank_names[NEW.level] THEN
   RAISE EXCEPTION 'EditRanks definition must equal exact ordered name' USING ERRCODE='23514';END IF;
  IF TG_OP='UPDATE' AND (to_jsonb(OLD)-'name'-'name_key') IS DISTINCT FROM (to_jsonb(NEW)-'name'-'name_key') THEN
   RAISE EXCEPTION 'EditRanks retains level and Guild identity' USING ERRCODE='23514';END IF;
  RETURN NEW;
 ELSIF TG_TABLE_NAME='game_guild_ranks' AND TG_OP='INSERT' THEN
  IF o.action<>6 OR NEW.guild_id<>o.guild_id OR NEW.name IS DISTINCT FROM (CASE NEW.level WHEN 1 THEN 'Leader' WHEN 2 THEN 'Vice Leader' WHEN 3 THEN 'Member' END) THEN
   RAISE EXCEPTION 'Found must create its three bound default ranks' USING ERRCODE='23514';END IF;
  RETURN NEW;
 ELSIF TG_TABLE_NAME='game_account_guild_leadership' AND TG_OP='INSERT' THEN
  IF o.action=9 AND o.previous_rank<>2 AND o.edit_rank=2 AND NEW.guild_id=o.guild_id AND NEW.character_id=o.target
     AND NEW.account_id=(SELECT account_id FROM game_character_roots WHERE character_id=o.target)
     AND EXISTS(SELECT 1 FROM game_guild_members WHERE guild_id=o.guild_id AND character_id=o.target AND rank_level=2) THEN RETURN NEW;END IF;
  IF o.action=6 AND NEW.guild_id=o.guild_id AND NEW.character_id=o.actor
     AND NEW.account_id=(SELECT account_id FROM game_character_roots WHERE character_id=o.actor) THEN RETURN NEW;END IF;
 ELSIF TG_TABLE_NAME='game_account_guild_leadership' AND TG_OP='DELETE' THEN
  IF o.action=9 AND o.previous_rank=2 AND o.edit_rank<>2 AND OLD.guild_id=o.guild_id AND OLD.character_id=o.target
     AND OLD.account_id=(SELECT account_id FROM game_character_roots WHERE character_id=o.target)
     AND EXISTS(SELECT 1 FROM game_guild_members WHERE guild_id=o.guild_id AND character_id=o.target AND rank_level=o.edit_rank) THEN RETURN OLD;END IF;
  IF o.action IN(4,5) AND OLD.guild_id=o.guild_id AND OLD.character_id=o.target
     AND OLD.account_id=(SELECT account_id FROM game_character_roots WHERE character_id=o.target) THEN RETURN OLD;END IF;
 END IF;
 RAISE EXCEPTION 'this Guild operation does not own that row transition' USING ERRCODE='23514';
END $$;
CREATE TRIGGER game_guild_row_mutation BEFORE INSERT OR UPDATE OR DELETE ON game_guilds FOR EACH ROW EXECUTE FUNCTION game_guild_command_mutation();
CREATE TRIGGER game_guild_member_mutation BEFORE INSERT OR UPDATE OR DELETE ON game_guild_members FOR EACH ROW EXECUTE FUNCTION game_guild_command_mutation();
CREATE TRIGGER game_guild_invitation_mutation BEFORE INSERT OR UPDATE OR DELETE ON game_guild_invitations FOR EACH ROW EXECUTE FUNCTION game_guild_command_mutation();
CREATE TRIGGER game_guild_leadership_mutation BEFORE INSERT OR UPDATE OR DELETE ON game_account_guild_leadership FOR EACH ROW EXECUTE FUNCTION game_guild_command_mutation();
CREATE TRIGGER game_guild_rank_not_owned BEFORE INSERT OR UPDATE OR DELETE ON game_guild_ranks FOR EACH ROW EXECUTE FUNCTION game_guild_command_mutation();

CREATE FUNCTION game_guild_snapshot_valid() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE g UUID;s SMALLINT;members BIGINT;leaders BIGINT;ranks BIGINT;
BEGIN
 IF TG_OP='DELETE' THEN g=OLD.guild_id;ELSE g=NEW.guild_id;END IF;
 SELECT state INTO s FROM game_guilds WHERE guild_id=g;
 SELECT count(*),count(*) FILTER(WHERE rank_level=1) INTO members,leaders FROM game_guild_members WHERE guild_id=g;
 SELECT count(*) INTO ranks FROM game_guild_ranks WHERE guild_id=g;
 IF members>2000 OR (s<>4 AND (ranks NOT BETWEEN 3 AND 20
    OR (SELECT min(level) FROM game_guild_ranks WHERE guild_id=g) IS DISTINCT FROM 1
    OR (SELECT max(level) FROM game_guild_ranks WHERE guild_id=g) IS DISTINCT FROM ranks))
    OR (s<>4 AND leaders<>1)
    OR (s=4 AND (members<>0 OR ranks<>0 OR EXISTS(SELECT 1 FROM game_guild_invitations WHERE guild_id=g)
       OR EXISTS(SELECT 1 FROM game_account_guild_leadership WHERE guild_id=g)))
    OR EXISTS(SELECT 1 FROM game_guild_members m JOIN game_character_roots c USING(character_id) JOIN game_guilds x USING(guild_id) WHERE m.guild_id=g AND (c.world_id<>x.world_id OR c.lifecycle<>1))
    OR EXISTS(SELECT 1 FROM game_guild_members m JOIN game_character_roots c USING(character_id) WHERE m.guild_id=g AND m.rank_level<=2 AND NOT EXISTS(SELECT 1 FROM game_account_guild_leadership l WHERE l.guild_id=m.guild_id AND l.character_id=m.character_id AND l.account_id=c.account_id))
    OR EXISTS(SELECT 1 FROM game_account_guild_leadership l JOIN game_character_roots c USING(character_id) WHERE l.guild_id=g AND (l.account_id<>c.account_id OR NOT EXISTS(SELECT 1 FROM game_guild_members m WHERE m.character_id=l.character_id AND m.guild_id=g AND m.rank_level<=2))) THEN
  RAISE EXCEPTION 'Guild snapshot violates membership/rank/leadership invariants' USING ERRCODE='23514';END IF;
 RETURN NULL;
END $$;
DO $$ DECLARE tab TEXT;BEGIN
 FOREACH tab IN ARRAY ARRAY['game_guilds','game_guild_ranks','game_guild_members','game_account_guild_leadership'] LOOP
  EXECUTE format('CREATE CONSTRAINT TRIGGER %I AFTER INSERT OR UPDATE OR DELETE ON %I DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_guild_snapshot_valid()',tab||'_snapshot',tab);
 END LOOP;
END $$;
CREATE FUNCTION game_guild_invitation_caps() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF (SELECT count(*) FROM game_guild_invitations WHERE guild_id=NEW.guild_id)>500
 OR (SELECT count(*) FROM game_guild_invitations WHERE character_id=NEW.character_id)>50 THEN
  RAISE EXCEPTION 'Guild invitation row cap exceeded' USING ERRCODE='23514';END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_guild_invitation_caps AFTER INSERT ON game_guild_invitations DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_guild_invitation_caps();

-- The real writer has already locked the Guild and this invitation in accepted order.
-- A newly proposed success must also see that existing row, not merely a final empty snapshot.
CREATE FUNCTION game_guild_membership_precondition() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE acting SMALLINT;subject SMALLINT;n BIGINT;
BEGIN
 IF NEW.result<>0 OR NEW.action NOT BETWEEN 1 AND 5 THEN RETURN NEW;END IF;
 IF NOT EXISTS(SELECT 1 FROM game_guilds WHERE guild_id=NEW.guild_id AND world_id=NEW.world_id
    AND state IN(1,2) AND revision=NEW.previous_revision) THEN
  RAISE EXCEPTION 'Membership requires its current Guild/revision' USING ERRCODE='23514';END IF;
 SELECT rank_level INTO acting FROM game_guild_members WHERE guild_id=NEW.guild_id AND character_id=NEW.actor;
 SELECT rank_level INTO subject FROM game_guild_members WHERE guild_id=NEW.guild_id AND character_id=NEW.target;
 n=floor(extract(epoch FROM transaction_timestamp())*1000)::bigint;
 IF NOT coalesce(CASE NEW.action
   WHEN 1 THEN acting<=2 AND subject IS NULL AND EXISTS(SELECT 1 FROM game_character_roots
      WHERE character_id=NEW.target AND world_id=NEW.world_id AND lifecycle=1)
      AND NOT EXISTS(SELECT 1 FROM game_guild_invitations WHERE guild_id=NEW.guild_id AND character_id=NEW.target AND expires_at>n)
   WHEN 2 THEN acting<=2 AND EXISTS(SELECT 1 FROM game_guild_invitations WHERE guild_id=NEW.guild_id AND character_id=NEW.target)
   WHEN 3 THEN NEW.actor=NEW.target AND NOT EXISTS(SELECT 1 FROM game_guild_members WHERE character_id=NEW.actor)
      AND EXISTS(SELECT 1 FROM game_guild_invitations WHERE guild_id=NEW.guild_id AND character_id=NEW.actor AND expires_at>n)
   WHEN 4 THEN NEW.actor=NEW.target AND subject>1
   WHEN 5 THEN acting<=2 AND subject>acting
   ELSE false END,false) THEN
  RAISE EXCEPTION 'Membership success requires its actual actor/target/invitation' USING ERRCODE='23514';END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_guild_membership_precondition BEFORE INSERT ON game_guild_operations FOR EACH ROW EXECUTE FUNCTION game_guild_membership_precondition();

CREATE FUNCTION game_guild_disband_precondition() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.action=13 AND NEW.result=0 AND (NEW.actor<>NEW.target OR NOT EXISTS(
    SELECT 1 FROM game_guilds g JOIN game_guild_members m USING(guild_id)
    WHERE g.guild_id=NEW.guild_id AND g.world_id=NEW.world_id AND g.state IN(1,2)
      AND g.revision=NEW.previous_revision AND g.name=NEW.disband_confirmation AND m.character_id=NEW.actor AND m.rank_level=1)) THEN
  RAISE EXCEPTION 'Disband requires exact current leader and bound name confirmation' USING ERRCODE='23514';END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_guild_disband_precondition BEFORE INSERT ON game_guild_operations FOR EACH ROW EXECUTE FUNCTION game_guild_disband_precondition();

CREATE FUNCTION game_guild_decline_precondition() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.action=12 AND NEW.result=0 AND (NEW.actor<>NEW.target OR NOT EXISTS(
    SELECT 1 FROM game_guild_invitations WHERE guild_id=NEW.guild_id AND character_id=NEW.actor)) THEN
  RAISE EXCEPTION 'Decline success requires its actual invited actor' USING ERRCODE='23514';END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_guild_decline_precondition BEFORE INSERT ON game_guild_operations FOR EACH ROW EXECUTE FUNCTION game_guild_decline_precondition();

CREATE FUNCTION game_guild_operation_proven() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.result=0 THEN
  IF NEW.created_xact_id<>pg_current_xact_id() OR NOT EXISTS(SELECT 1 FROM game_guilds g WHERE g.guild_id=NEW.guild_id AND g.world_id=NEW.world_id AND ((NEW.action<>13 AND g.state IN(1,2)) OR (NEW.action=13 AND g.state=3)) AND g.revision=NEW.revision)
     OR NOT EXISTS(SELECT 1 FROM game_guild_event_outbox e WHERE e.occurrence=NEW.occurrence AND e.event_id=NEW.event_id AND e.guild_id=NEW.guild_id AND e.payload_sha256=NEW.audit_sha256 AND e.created_xact_id=pg_current_xact_id())
     OR (NEW.action=13 AND (NEW.actor<>NEW.target
       OR NOT EXISTS(SELECT 1 FROM game_guild_members WHERE guild_id=NEW.guild_id AND character_id=NEW.actor AND rank_level=1)
       OR NOT EXISTS(SELECT 1 FROM game_guilds WHERE guild_id=NEW.guild_id AND name=NEW.disband_confirmation AND state=3)
       OR NOT EXISTS(SELECT 1 FROM game_guild_job_checkpoints WHERE guild_id=NEW.guild_id AND step=1 AND reason=3 AND world_id=NEW.world_id AND previous_revision=NEW.previous_revision AND revision=NEW.revision AND created_xact_id=pg_current_xact_id())))
     OR (NEW.action=1 AND NOT EXISTS(SELECT 1 FROM game_guild_invitations WHERE guild_id=NEW.guild_id AND character_id=NEW.target AND inviter=NEW.actor))
     OR (NEW.action=2 AND EXISTS(SELECT 1 FROM game_guild_invitations WHERE guild_id=NEW.guild_id AND character_id=NEW.target))
     OR (NEW.action=12 AND (NEW.actor<>NEW.target OR EXISTS(SELECT 1 FROM game_guild_invitations WHERE guild_id=NEW.guild_id AND character_id=NEW.actor)))
     OR (NEW.action=3 AND (NOT EXISTS(SELECT 1 FROM game_guild_members WHERE guild_id=NEW.guild_id AND character_id=NEW.actor AND rank_level>=3) OR EXISTS(SELECT 1 FROM game_guild_invitations WHERE character_id=NEW.actor)))
     OR (NEW.action IN(4,5) AND EXISTS(SELECT 1 FROM game_guild_members WHERE character_id=NEW.target))
     OR (NEW.action=7 AND (NOT EXISTS(SELECT 1 FROM game_guild_members WHERE guild_id=NEW.guild_id AND character_id=NEW.target AND title=NEW.edit_text)
       OR NOT EXISTS(SELECT 1 FROM game_guild_members WHERE guild_id=NEW.guild_id AND character_id=NEW.actor AND rank_level=1)))
     OR (NEW.action=8 AND (NEW.target<>NEW.actor OR NOT EXISTS(SELECT 1 FROM game_guilds WHERE guild_id=NEW.guild_id AND motd=NEW.edit_text)
       OR NOT EXISTS(SELECT 1 FROM game_guild_members WHERE guild_id=NEW.guild_id AND character_id=NEW.actor AND rank_level<=2)))
     OR (NEW.action IN(7,8,9,10,11) AND NEW.expected_revision IS DISTINCT FROM NEW.previous_revision)
     OR (NEW.action=9 AND (NOT EXISTS(SELECT 1 FROM game_guild_members WHERE guild_id=NEW.guild_id AND character_id=NEW.target AND rank_level=NEW.edit_rank)
       OR NOT EXISTS(SELECT 1 FROM game_guild_members WHERE guild_id=NEW.guild_id AND character_id=NEW.actor AND rank_level<NEW.previous_rank AND rank_level<NEW.edit_rank)))
     OR (NEW.action=11 AND (NEW.target<>NEW.actor OR NOT game_guild_rank_names_valid(NEW.edit_rank_names)
       OR NOT EXISTS(SELECT 1 FROM game_guild_members WHERE guild_id=NEW.guild_id AND character_id=NEW.actor AND rank_level=1)
       OR (SELECT array_agg(name ORDER BY level) FROM game_guild_ranks WHERE guild_id=NEW.guild_id) IS DISTINCT FROM NEW.edit_rank_names
       OR (SELECT count(*) FROM game_guild_ranks WHERE guild_id=NEW.guild_id)<>cardinality(NEW.edit_rank_names)
       OR EXISTS(SELECT 1 FROM game_guild_ranks WHERE guild_id=NEW.guild_id AND level>cardinality(NEW.edit_rank_names))))
     OR (NEW.action=10 AND (NEW.actor=NEW.target OR NEW.previous_rank<>2 OR NEW.edit_rank<>1
       OR NOT EXISTS(SELECT 1 FROM game_guild_members WHERE guild_id=NEW.guild_id AND character_id=NEW.actor AND rank_level=2)
       OR NOT EXISTS(SELECT 1 FROM game_guild_members WHERE guild_id=NEW.guild_id AND character_id=NEW.target AND rank_level=1)))
     OR (NEW.action=6 AND (
       NEW.actor<>NEW.target OR NEW.previous_revision<>0 OR NEW.revision<>1
       OR NOT EXISTS(SELECT 1 FROM game_guilds WHERE guild_id=NEW.guild_id AND name=NEW.found_name AND state=1 AND vice_deficit_since IS NULL AND premium_deficit_since IS NULL AND motd='')
       OR (SELECT count(*) FROM game_guild_members WHERE guild_id=NEW.guild_id)<>1
       OR NOT EXISTS(SELECT 1 FROM game_guild_members WHERE guild_id=NEW.guild_id AND character_id=NEW.actor AND rank_level=1 AND title='')
       OR (SELECT count(*) FROM game_guild_ranks WHERE guild_id=NEW.guild_id)<>3
       OR NOT EXISTS(SELECT 1 FROM game_guild_ranks WHERE guild_id=NEW.guild_id AND level=1 AND name='Leader')
       OR NOT EXISTS(SELECT 1 FROM game_guild_ranks WHERE guild_id=NEW.guild_id AND level=2 AND name='Vice Leader')
       OR NOT EXISTS(SELECT 1 FROM game_guild_ranks WHERE guild_id=NEW.guild_id AND level=3 AND name='Member')
       OR EXISTS(SELECT 1 FROM game_guild_invitations WHERE guild_id=NEW.guild_id)
       OR NOT EXISTS(SELECT 1 FROM game_account_guild_leadership l JOIN game_character_roots c ON c.character_id=l.character_id WHERE l.guild_id=NEW.guild_id AND l.character_id=NEW.actor AND l.account_id=c.account_id))) THEN
   RAISE EXCEPTION 'Guild operation must commit its transition and one exact event' USING ERRCODE='23514';END IF;
 ELSIF EXISTS(SELECT 1 FROM game_guild_event_outbox WHERE occurrence=NEW.occurrence) THEN
  RAISE EXCEPTION 'refusal must not manufacture a Guild mutation event' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_guild_operation_proven AFTER INSERT ON game_guild_operations DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_guild_operation_proven();
-- The writer inserts its event before mutating rows, so audit ranks/removal counts can be
-- reconstructed from current locked state rather than trusted from the proposed JSON.
CREATE FUNCTION game_guild_event_allowed() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE o game_guild_operations%ROWTYPE;expected JSONB;before_rank SMALLINT;after_rank SMALLINT;removed BIGINT;
BEGIN
 SELECT * INTO o FROM game_guild_operations WHERE occurrence=NEW.occurrence AND result=0
   AND created_xact_id=pg_current_xact_id();
 IF NOT FOUND OR NEW.guild_id<>o.guild_id OR NEW.event_id<>o.event_id OR NEW.payload_sha256<>o.audit_sha256 THEN
  RAISE EXCEPTION 'Guild event has no matching operation' USING ERRCODE='23514';END IF;
 before_rank=NULL;after_rank=NULL;removed=0;
 IF o.action IN(4,5) THEN
  SELECT rank_level INTO before_rank FROM game_guild_members WHERE guild_id=o.guild_id AND character_id=o.target;
 ELSIF o.action=3 THEN
  SELECT max(level) INTO after_rank FROM game_guild_ranks WHERE guild_id=o.guild_id;
 ELSIF o.action=6 THEN after_rank=1;
 ELSIF o.action IN(9,10) THEN before_rank=o.previous_rank;after_rank=o.edit_rank;
 END IF;
 IF o.action=1 THEN
  SELECT count(*) INTO removed FROM game_guild_invitations WHERE character_id=o.target
    AND expires_at<=floor(extract(epoch FROM transaction_timestamp())*1000)::bigint;
 ELSIF o.action=3 THEN
  SELECT count(*) INTO removed FROM game_guild_invitations WHERE character_id=o.target;
 ELSIF o.action IN(2,12) THEN removed=1;
 END IF;
 expected=jsonb_build_object('version',1,'action',o.action,'previous_revision',o.previous_revision,
   'revision',o.revision,'rank_before',before_rank,'rank_after',after_rank,
   'removed_invitations',removed,'occurred_at_unix_ms',NEW.occurred_at)
   || (SELECT jsonb_object_agg(field_name,encoded_bytes) FROM
       (SELECT field_name,jsonb_agg(get_byte(uuid_send(uuid_value),byte_index) ORDER BY byte_index) AS encoded_bytes
        FROM (VALUES ('occurrence',o.occurrence),('transaction_id',o.transaction_id),('event_id',o.event_id),
          ('guild_id',o.guild_id),('world_id',o.world_id),('actor',o.actor),('target',o.target)) AS ids(field_name,uuid_value)
        CROSS JOIN generate_series(0,15) AS bytes(byte_index) GROUP BY field_name,uuid_value) AS encoded_ids);
 IF o.action IN(7,8,9,10,11) THEN expected=expected||jsonb_build_object('expected_revision',o.expected_revision);END IF;
 IF o.action IN(7,8) THEN expected=expected||jsonb_build_object('text',o.edit_text);END IF;
 IF o.action=11 THEN expected=expected||jsonb_build_object('names',to_jsonb(o.edit_rank_names));END IF;
 IF o.action=13 THEN expected=expected||jsonb_build_object('confirmation',o.disband_confirmation);END IF;
 IF convert_from(NEW.payload,'UTF8')::jsonb IS DISTINCT FROM expected THEN
  RAISE EXCEPTION 'Guild event must equal its whole bound payload' USING ERRCODE='23514';END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_guild_event_allowed BEFORE INSERT ON game_guild_event_outbox FOR EACH ROW EXECUTE FUNCTION game_guild_event_allowed();
CREATE FUNCTION game_guild_event_proven() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.created_xact_id<>pg_current_xact_id() OR NOT EXISTS(SELECT 1 FROM game_guild_operations o
    WHERE o.occurrence=NEW.occurrence AND o.result=0 AND o.guild_id=NEW.guild_id AND o.event_id=NEW.event_id
      AND o.audit_sha256=NEW.payload_sha256 AND o.created_xact_id=pg_current_xact_id()) THEN
  RAISE EXCEPTION 'Guild event has no matching operation' USING ERRCODE='23514';END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_guild_event_proven AFTER INSERT ON game_guild_event_outbox DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_guild_event_proven();
-- GUILD0 §3.3/§3.4 start only. The same transaction retains its pending step and publication.
CREATE FUNCTION game_guild_job_start_allowed() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE g game_guilds%ROWTYPE;n BIGINT;
BEGIN
 SELECT * INTO g FROM game_guilds WHERE guild_id=NEW.guild_id;
 n=floor(extract(epoch FROM transaction_timestamp())*1000)::bigint;
 IF NOT FOUND OR g.world_id<>NEW.world_id OR g.revision<>NEW.previous_revision
    OR NEW.occurred_at<>n OR NEW.created_xact_id<>pg_current_xact_id()
    OR NOT ((NEW.reason=1 AND g.state=1 AND n>g.formation_deadline)
       OR (NEW.reason=2 AND g.state=2 AND g.vice_deficit_since IS NOT NULL
          AND n::numeric-g.vice_deficit_since::numeric>1209600000)
       OR (NEW.reason=3 AND g.state IN(1,2) AND EXISTS(
          SELECT 1 FROM game_guild_operations o WHERE o.created_xact_id=pg_current_xact_id()
            AND o.action=13 AND o.result=0 AND o.guild_id=g.guild_id AND o.world_id=g.world_id
            AND o.previous_revision=NEW.previous_revision AND o.revision=NEW.revision AND o.disband_confirmation=g.name)))
    OR (NEW.reason<>3 AND EXISTS(SELECT 1 FROM game_guild_operations WHERE created_xact_id=pg_current_xact_id())) THEN
  RAISE EXCEPTION 'Guild job requires its due locked Guild and exact database time' USING ERRCODE='23514';END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_guild_job_stamp BEFORE INSERT ON game_guild_job_checkpoints FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();
CREATE TRIGGER game_guild_job_start_allowed BEFORE INSERT ON game_guild_job_checkpoints FOR EACH ROW EXECUTE FUNCTION game_guild_job_start_allowed();
CREATE TRIGGER game_guild_job_event_stamp BEFORE INSERT ON game_guild_job_event_outbox FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();
CREATE TRIGGER game_guild_job_immutable BEFORE UPDATE OR DELETE ON game_guild_job_checkpoints FOR EACH ROW EXECUTE FUNCTION game_guild_rows_immutable();
CREATE TRIGGER game_guild_job_event_immutable BEFORE UPDATE OR DELETE ON game_guild_job_event_outbox FOR EACH ROW EXECUTE FUNCTION game_guild_rows_immutable();
CREATE TRIGGER game_guild_job_no_truncate BEFORE TRUNCATE ON game_guild_job_checkpoints FOR EACH STATEMENT EXECUTE FUNCTION game_guild_rows_immutable();
CREATE TRIGGER game_guild_job_event_no_truncate BEFORE TRUNCATE ON game_guild_job_event_outbox FOR EACH STATEMENT EXECUTE FUNCTION game_guild_rows_immutable();
CREATE FUNCTION game_guild_job_proven() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE j game_guild_job_checkpoints%ROWTYPE;expected JSONB;
BEGIN
 SELECT * INTO j FROM game_guild_job_checkpoints WHERE guild_id=NEW.guild_id AND step=NEW.step;
 IF NOT FOUND OR j.created_xact_id<>pg_current_xact_id()
    OR NOT EXISTS(SELECT 1 FROM game_guilds WHERE guild_id=j.guild_id AND world_id=j.world_id AND state=3 AND revision=j.revision) THEN
  RAISE EXCEPTION 'Guild job must commit its pending start' USING ERRCODE='23514';END IF;
 IF j.reason=3 THEN
  IF NOT EXISTS(SELECT 1 FROM game_guild_operations o JOIN game_guild_event_outbox e ON e.occurrence=o.occurrence
     WHERE o.guild_id=j.guild_id AND o.world_id=j.world_id AND o.action=13 AND o.result=0
       AND o.previous_revision=j.previous_revision AND o.revision=j.revision
       AND o.created_xact_id=pg_current_xact_id() AND e.created_xact_id=pg_current_xact_id()
       AND e.event_id=o.event_id AND e.payload_sha256=o.audit_sha256)
     OR EXISTS(SELECT 1 FROM game_guild_job_event_outbox WHERE guild_id=j.guild_id AND step=j.step) THEN
   RAISE EXCEPTION 'Player Disband start requires its one exact command event' USING ERRCODE='23514';END IF;
  RETURN NULL;
 END IF;
 expected=jsonb_build_object('version',1,'step',1,'reason',j.reason,'previous_revision',j.previous_revision,
   'revision',j.revision,'occurred_at_unix_ms',j.occurred_at)
   || (SELECT jsonb_object_agg(field_name,encoded_bytes) FROM
       (SELECT field_name,jsonb_agg(get_byte(uuid_send(uuid_value),byte_index) ORDER BY byte_index) AS encoded_bytes
        FROM (VALUES ('guild_id',j.guild_id),('world_id',j.world_id)) AS ids(field_name,uuid_value)
        CROSS JOIN generate_series(0,15) AS bytes(byte_index) GROUP BY field_name,uuid_value) AS encoded_ids);
 IF NOT EXISTS(SELECT 1 FROM game_guild_job_event_outbox e WHERE e.guild_id=j.guild_id AND e.step=j.step
    AND e.created_xact_id=pg_current_xact_id() AND e.payload_sha256=sha256(e.payload)
    AND convert_from(e.payload,'UTF8')::jsonb=expected) THEN
  RAISE EXCEPTION 'Guild job requires exactly bound same-transaction event DATA' USING ERRCODE='23514';END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_guild_job_proven AFTER INSERT ON game_guild_job_checkpoints DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_guild_job_proven();
CREATE CONSTRAINT TRIGGER game_guild_job_event_proven AFTER INSERT ON game_guild_job_event_outbox DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_guild_job_proven();

-- D1098 whole terminal flow. Missing genuine House closure fails closed in its owner port;
-- no retained financial receipt or caller-supplied boolean can satisfy that prerequisite.
-- CORE has no financial/property owner. Fail closed as soon as a later owner is installed;
-- its child must compose the real release/payout transaction before enabling cleanup.
CREATE FUNCTION game_guild_core_participants_installed() RETURNS BOOLEAN
LANGUAGE sql STABLE AS $$ SELECT EXISTS(SELECT 1 FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace
   WHERE n.nspname=current_schema() AND c.relkind IN('r','p')
     AND (c.relname LIKE 'game_guild_bank_%' OR c.relname LIKE 'game_guildhall_%'
       OR c.relname LIKE 'game_house_guild_%' OR c.relname='game_house_properties')) $$;
CREATE FUNCTION game_guild_core_assert_released(g UUID,w UUID) RETURNS VOID
LANGUAGE plpgsql AS $$
BEGIN
 IF NOT EXISTS(SELECT 1 FROM game_guilds WHERE guild_id=g AND world_id=w AND state IN(3,4)) THEN
  RAISE EXCEPTION 'Guild binding is neither pending nor terminal' USING ERRCODE='23514';END IF;
 IF game_guild_core_participants_installed() THEN
  RAISE EXCEPTION 'Guild terminal participants require their owner composition' USING ERRCODE='0A000';END IF;
END $$;
CREATE FUNCTION game_guild_disband_step_payload(d game_guild_disband_steps) RETURNS JSONB
LANGUAGE sql IMMUTABLE AS $$
 SELECT jsonb_build_object('version',1,'guild_id',d.guild_id,'world_id',d.world_id,'step',d.step,
  'phase',d.phase,'previous_revision',d.previous_revision,'revision',d.revision,
  'payout_transaction_id',d.payout_transaction_id,'occurred_at',d.occurred_at,
  'removed_members',cardinality(d.member_ids),'removed_invitations',cardinality(d.invitation_ids),
  'freed_leadership',cardinality(d.leadership_accounts),
  'member_ids_sha256',encode(sha256(convert_to(array_to_string(d.member_ids,','),'UTF8')),'hex'),
  'invitation_ids_sha256',encode(sha256(convert_to(array_to_string(d.invitation_ids,','),'UTF8')),'hex'),
  'leadership_accounts_sha256',encode(sha256(convert_to(array_to_string(d.leadership_accounts,','),'UTF8')),'hex'))
$$;
CREATE FUNCTION game_guild_disband_step_allowed() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE g game_guilds%ROWTYPE;last_step game_guild_disband_steps%ROWTYPE;has_last BOOLEAN;ids UUID[];accounts UUID[];
BEGIN
 SELECT * INTO g FROM game_guilds WHERE guild_id=NEW.guild_id;
 IF NOT FOUND OR g.state<>3 OR g.world_id<>NEW.world_id OR g.revision<>NEW.previous_revision
    OR NEW.created_xact_id<>pg_current_xact_id()
    OR NEW.occurred_at<>floor(extract(epoch FROM transaction_timestamp())*1000)::bigint
    OR EXISTS(SELECT 1 FROM game_guild_operations WHERE created_xact_id=pg_current_xact_id())
    OR NOT EXISTS(SELECT 1 FROM game_guild_job_checkpoints WHERE guild_id=g.guild_id AND world_id=g.world_id AND step=1 AND revision<=g.revision) THEN
  RAISE EXCEPTION 'Disband step requires its exact pending Guild/start/revision/time' USING ERRCODE='23514';END IF;
 SELECT * INTO last_step FROM game_guild_disband_steps WHERE guild_id=g.guild_id ORDER BY step DESC LIMIT 1;
 has_last=FOUND;
 IF NEW.step<>(CASE WHEN has_last THEN last_step.step+1 ELSE 2 END)
    OR (has_last AND last_step.revision<>NEW.previous_revision)
    OR NOT ((NEW.phase IN(4,5,6) AND NOT has_last)
       OR (NEW.phase=4 AND has_last AND last_step.phase=4)
       OR (NEW.phase IN(5,6) AND has_last AND last_step.phase IN(4,5))) THEN
  RAISE EXCEPTION 'Disband ordinal/phase cannot skip or replay a mutation' USING ERRCODE='23514';END IF;
 IF NEW.phase NOT IN(4,5,6) OR NEW.payout_transaction_id IS NOT NULL THEN
  RAISE EXCEPTION 'Core cannot fabricate Hall/payout phases or a payout identity' USING ERRCODE='23514';END IF;
 IF NEW.phase=4 THEN
  SELECT ARRAY(SELECT character_id FROM game_guild_members WHERE guild_id=g.guild_id AND rank_level<>1 ORDER BY character_id LIMIT 100) INTO ids;
  SELECT ARRAY(SELECT account_id FROM game_account_guild_leadership WHERE guild_id=g.guild_id AND character_id=ANY(ids) ORDER BY account_id LIMIT 101) INTO accounts;
  IF cardinality(ids)=0 OR NEW.member_ids IS DISTINCT FROM ids OR NEW.leadership_accounts IS DISTINCT FROM accounts OR cardinality(NEW.invitation_ids)<>0 THEN
   RAISE EXCEPTION 'Disband removes exactly its next bounded nonleader batch' USING ERRCODE='23514';END IF;
 ELSE
  IF EXISTS(SELECT 1 FROM game_guild_members WHERE guild_id=g.guild_id AND rank_level<>1) THEN
   RAISE EXCEPTION 'Disband must remove nonleaders before invitations/final step' USING ERRCODE='23514';END IF;
  IF NEW.phase=5 THEN
   SELECT ARRAY(SELECT character_id FROM game_guild_invitations WHERE guild_id=g.guild_id ORDER BY character_id LIMIT 100) INTO ids;
   IF cardinality(ids)=0 OR NEW.invitation_ids IS DISTINCT FROM ids OR cardinality(NEW.member_ids)+cardinality(NEW.leadership_accounts)<>0 THEN
    RAISE EXCEPTION 'Disband removes exactly its next bounded invitation batch' USING ERRCODE='23514';END IF;
  ELSE
   SELECT ARRAY(SELECT character_id FROM game_guild_members WHERE guild_id=g.guild_id AND rank_level=1 ORDER BY character_id LIMIT 2) INTO ids;
   SELECT ARRAY(SELECT account_id FROM game_account_guild_leadership WHERE guild_id=g.guild_id ORDER BY account_id LIMIT 2) INTO accounts;
   IF cardinality(ids)<>1 OR NEW.member_ids IS DISTINCT FROM ids OR NEW.leadership_accounts IS DISTINCT FROM accounts
      OR cardinality(NEW.invitation_ids)<>0 OR EXISTS(SELECT 1 FROM game_guild_invitations WHERE guild_id=g.guild_id) THEN
    RAISE EXCEPTION 'Final disband releases only its last leader and exact leadership set' USING ERRCODE='23514';END IF;
  END IF;
 END IF;
 -- Actual owner predicates; the Hall owner explicitly refuses while C Guild property release
 -- remains unavailable. No fixture House table, substitute participant or proof flag is read.
 PERFORM game_guild_core_assert_released(NEW.guild_id,NEW.world_id);
 RETURN NEW;
END $$;
CREATE TRIGGER game_guild_disband_step_allowed BEFORE INSERT ON game_guild_disband_steps FOR EACH ROW EXECUTE FUNCTION game_guild_disband_step_allowed();
CREATE TRIGGER game_guild_disband_step_stamp BEFORE INSERT ON game_guild_disband_steps FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();
CREATE TRIGGER game_guild_disband_step_event_stamp BEFORE INSERT ON game_guild_disband_step_events FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();
CREATE FUNCTION game_guild_disband_step_proven() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE d game_guild_disband_steps%ROWTYPE;
BEGIN
 SELECT * INTO d FROM game_guild_disband_steps WHERE guild_id=NEW.guild_id AND step=NEW.step;
 IF NOT FOUND OR d.created_xact_id<>pg_current_xact_id()
    OR NOT EXISTS(SELECT 1 FROM game_guilds WHERE guild_id=d.guild_id AND world_id=d.world_id AND revision=d.revision
      AND state=CASE WHEN d.phase=6 THEN 4 ELSE 3 END)
    OR NOT EXISTS(SELECT 1 FROM game_guild_disband_step_events e WHERE e.guild_id=d.guild_id AND e.step=d.step
      AND e.created_xact_id=pg_current_xact_id() AND e.payload_sha256=sha256(e.payload)
      AND convert_from(e.payload,'UTF8')::jsonb=game_guild_disband_step_payload(d)) THEN
  RAISE EXCEPTION 'Disband requires exact same-transaction state and whole private event' USING ERRCODE='23514';END IF;
 IF EXISTS(SELECT 1 FROM game_guild_members WHERE guild_id=d.guild_id AND character_id=ANY(d.member_ids))
    OR EXISTS(SELECT 1 FROM game_guild_invitations WHERE guild_id=d.guild_id AND character_id=ANY(d.invitation_ids))
    OR EXISTS(SELECT 1 FROM game_account_guild_leadership WHERE guild_id=d.guild_id AND account_id=ANY(d.leadership_accounts)) THEN
  RAISE EXCEPTION 'Disband must remove its complete bounded row set' USING ERRCODE='23514';END IF;
 PERFORM game_guild_core_assert_released(d.guild_id,d.world_id);
 IF d.phase=6 AND (EXISTS(SELECT 1 FROM game_guild_members WHERE guild_id=d.guild_id)
    OR EXISTS(SELECT 1 FROM game_guild_invitations WHERE guild_id=d.guild_id)
    OR EXISTS(SELECT 1 FROM game_guild_ranks WHERE guild_id=d.guild_id)
    OR EXISTS(SELECT 1 FROM game_account_guild_leadership WHERE guild_id=d.guild_id)) THEN
  RAISE EXCEPTION 'DISBANDED cannot retain members/invitations/ranks/leadership' USING ERRCODE='23514';END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_guild_disband_step_proven AFTER INSERT ON game_guild_disband_steps DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_guild_disband_step_proven();
CREATE CONSTRAINT TRIGGER game_guild_disband_step_event_proven AFTER INSERT ON game_guild_disband_step_events DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_guild_disband_step_proven();
-- Private D event data, not an admitted public EventEnvelope or authority capability.

DO $$ DECLARE tab TEXT;BEGIN
 FOREACH tab IN ARRAY ARRAY['game_guild_disband_steps','game_guild_disband_step_events'] LOOP
  EXECUTE format('CREATE TRIGGER %I BEFORE UPDATE OR DELETE ON %I FOR EACH ROW EXECUTE FUNCTION game_guild_rows_immutable()',tab||'_immutable',tab);
  EXECUTE format('CREATE TRIGGER %I BEFORE TRUNCATE ON %I FOR EACH STATEMENT EXECUTE FUNCTION game_guild_rows_immutable()',tab||'_no_truncate',tab);
 END LOOP;
END $$;
-- GUILD GRANTS BEGIN
-- Existing deployment roles remain unchanged; this migration creates no login or role membership.
REVOKE ALL ON game_guilds,game_guild_ranks,game_guild_members,game_guild_invitations,
 game_account_guild_leadership,game_guild_operations,game_guild_event_outbox,
 game_guild_job_checkpoints,game_guild_job_event_outbox,game_guild_disband_steps,
 game_guild_disband_step_events FROM PUBLIC;
GRANT SELECT,INSERT,UPDATE ON game_guilds TO oteryn_game_runtime;
GRANT SELECT,INSERT,UPDATE,DELETE ON game_guild_ranks,game_guild_members,
 game_guild_invitations,game_account_guild_leadership TO oteryn_game_runtime;
GRANT SELECT,INSERT ON game_guild_operations,game_guild_event_outbox,
 game_guild_job_checkpoints,game_guild_job_event_outbox,game_guild_disband_steps,
 game_guild_disband_step_events TO oteryn_game_runtime;
GRANT SELECT ON game_guilds,game_guild_ranks,game_guild_members,game_guild_invitations,
 game_account_guild_leadership,game_guild_operations,game_guild_event_outbox,
 game_guild_job_checkpoints,game_guild_job_event_outbox,game_guild_disband_steps,
 game_guild_disband_step_events TO oteryn_game_control;
