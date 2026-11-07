-- PARTY-1 local review candidate: PostgreSQL is the sole World-scoped membership owner.
CREATE TABLE game_parties (
 party_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(party_id)),
 world_id UUID NOT NULL CHECK(game_character_is_uuid_v7(world_id)),
 leader_character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 revision NUMERIC(20,0) NOT NULL CHECK(revision BETWEEN 1 AND 18446744073709551615),
 next_seq NUMERIC(20,0) NOT NULL CHECK(next_seq BETWEEN 2 AND 18446744073709551615),
 shared_xp_enabled BOOLEAN NOT NULL DEFAULT FALSE,
 created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp()
);
CREATE TABLE game_party_members (
 character_id UUID PRIMARY KEY REFERENCES game_character_roots(character_id),
 party_id UUID NOT NULL REFERENCES game_parties(party_id) ON DELETE CASCADE,
 seq NUMERIC(20,0) NOT NULL CHECK(seq>0),
 joined_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
 presence_revision NUMERIC(20,0) NOT NULL DEFAULT 1 CHECK(presence_revision>0),
 presence_state SMALLINT NOT NULL CHECK(presence_state IN(1,2,3)),
 presence_channel_id UUID,
 absence_observed_at TIMESTAMPTZ,
 UNIQUE(party_id,seq),
 CHECK((presence_state=1 AND presence_channel_id IS NOT NULL) OR (presence_state<>1 AND presence_channel_id IS NULL))
);
CREATE TABLE game_party_invitations (
 party_id UUID NOT NULL REFERENCES game_parties(party_id) ON DELETE CASCADE,
 invitee UUID NOT NULL REFERENCES game_character_roots(character_id),
 seq NUMERIC(20,0) NOT NULL CHECK(seq>0),
 created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
 expires_at TIMESTAMPTZ NOT NULL,
 PRIMARY KEY(party_id,invitee),UNIQUE(party_id,seq),CHECK(expires_at>created_at)
);
CREATE INDEX game_party_invitation_deadlines ON game_party_invitations(expires_at,party_id,invitee);
CREATE INDEX game_party_worlds ON game_parties(world_id,party_id);
CREATE TABLE game_character_social_settings (
 character_id UUID PRIMARY KEY REFERENCES game_character_roots(character_id),
 party_invites SMALLINT NOT NULL CHECK(party_invites IN(1,2)),
 channel_visibility SMALLINT NOT NULL CHECK(channel_visibility IN(1,2)),
 source_policy TEXT NOT NULL CHECK(source_policy='PARTYPVP0-PARTIES-AND-PVP-V1')
);
CREATE TABLE game_character_social_blocks (
 character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 blocked_character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 PRIMARY KEY(character_id,blocked_character_id),CHECK(character_id<>blocked_character_id)
);
-- Explicit migration/admission policy initialization, not a missing-row permission fallback.
INSERT INTO game_character_social_settings SELECT character_id,1,1,'PARTYPVP0-PARTIES-AND-PVP-V1' FROM game_character_roots;
CREATE TABLE game_world_party_receipts (
 game_session_id UUID NOT NULL,command_id NUMERIC(20,0) NOT NULL CHECK(command_id>0),
 receipt_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(receipt_id)),
 world_id UUID NOT NULL,character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 connection_generation NUMERIC(20,0) NOT NULL CHECK(connection_generation>0),
 lease_generation NUMERIC(20,0) NOT NULL CHECK(lease_generation>0),
 scope_generation NUMERIC(20,0) NOT NULL CHECK(scope_generation>0),
 intent BYTEA NOT NULL CHECK(octet_length(intent) BETWEEN 1 AND 16384),
 binding BYTEA NOT NULL CHECK(octet_length(binding)=32),
 result JSONB NOT NULL CHECK(jsonb_typeof(result)='object'),
 created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 UNIQUE(game_session_id,command_id)
);
CREATE TABLE game_world_party_audit_outbox (
 receipt_id UUID PRIMARY KEY REFERENCES game_world_party_receipts(receipt_id),
 envelope BYTEA NOT NULL CHECK(octet_length(envelope) BETWEEN 1 AND 16384)
);
CREATE FUNCTION game_world_party_guard() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE p RECORD; touched UUID; touched_invitee UUID;
BEGIN
 touched:=CASE WHEN TG_OP='DELETE' THEN OLD.party_id ELSE NEW.party_id END;
 FOR p IN SELECT * FROM game_parties WHERE party_id=touched LOOP
  IF NOT EXISTS(SELECT 1 FROM game_party_members m JOIN game_character_roots r USING(character_id)
   WHERE m.party_id=p.party_id AND m.character_id=p.leader_character_id AND r.world_id=p.world_id)
   OR EXISTS(SELECT 1 FROM game_party_members m JOIN game_character_roots r USING(character_id)
    WHERE m.party_id=p.party_id AND r.world_id<>p.world_id)
   OR EXISTS(SELECT 1 FROM game_party_invitations i JOIN game_character_roots r ON r.character_id=i.invitee
    WHERE i.party_id=p.party_id AND r.world_id<>p.world_id)
   OR (SELECT count(*) FROM game_party_members WHERE party_id=p.party_id)>50
   OR (SELECT count(*) FROM game_party_invitations WHERE party_id=p.party_id AND expires_at>clock_timestamp())>50 THEN
   RAISE EXCEPTION 'World party membership invariant' USING ERRCODE='23514';
  END IF;
 END LOOP;
 IF TG_TABLE_NAME='game_party_invitations' THEN
  touched_invitee:=CASE WHEN TG_OP='DELETE' THEN OLD.invitee ELSE NEW.invitee END;
  IF (SELECT count(*) FROM game_party_invitations i WHERE i.invitee=touched_invitee AND expires_at>clock_timestamp())>20 THEN
   RAISE EXCEPTION 'World party invitee resource bound' USING ERRCODE='23514';
  END IF;
 END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_world_party_proven AFTER INSERT OR UPDATE OR DELETE ON game_parties
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_world_party_guard();
CREATE CONSTRAINT TRIGGER game_world_party_members_proven AFTER INSERT OR UPDATE OR DELETE ON game_party_members
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_world_party_guard();
CREATE CONSTRAINT TRIGGER game_world_party_invitations_proven AFTER INSERT OR UPDATE OR DELETE ON game_party_invitations
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_world_party_guard();
CREATE FUNCTION game_world_party_receipt_immutable() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN RAISE EXCEPTION 'World party receipts immutable' USING ERRCODE='23514'; END $$;
CREATE TRIGGER game_world_party_receipt_immutable BEFORE UPDATE OR DELETE ON game_world_party_receipts
 FOR EACH ROW EXECUTE FUNCTION game_world_party_receipt_immutable();
REVOKE ALL ON game_parties,game_party_members,game_party_invitations,game_character_social_settings,
 game_character_social_blocks,game_world_party_receipts,game_world_party_audit_outbox FROM PUBLIC;
GRANT SELECT,INSERT,UPDATE,DELETE ON game_parties,game_party_members,game_party_invitations,
 game_character_social_settings,game_character_social_blocks TO oteryn_game_runtime;
GRANT SELECT,INSERT ON game_world_party_receipts,game_world_party_audit_outbox TO oteryn_game_runtime;
GRANT SELECT ON game_parties,game_party_members,game_party_invitations,game_character_social_settings,
 game_character_social_blocks,game_world_party_receipts,game_world_party_audit_outbox TO oteryn_game_control;
DO $$ DECLARE n TEXT; BEGIN FOREACH n IN ARRAY ARRAY['game_world_party_guard','game_world_party_receipt_immutable'] LOOP
 EXECUTE format('ALTER FUNCTION %I() SET search_path = %I, pg_temp',n,current_schema());
 EXECUTE format('REVOKE ALL ON FUNCTION %I() FROM PUBLIC',n);
END LOOP; END $$;
