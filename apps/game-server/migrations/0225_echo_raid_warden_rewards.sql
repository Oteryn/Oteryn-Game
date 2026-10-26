-- ECHORAID: immutable first-Warden-per-species grant and corpse reward receipt.
-- The admission receipt is the independently persisted session-generation fence. A retry with
-- the same occurrence reconciles; another Warden of the species cannot grant the charm again.
CREATE TABLE game_echo_raid_warden_rewards (
 occurrence_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(occurrence_id)),
 character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 game_session_id UUID NOT NULL REFERENCES game_durability_fresh_admission_receipts(game_session_id),
 expected_session_generation NUMERIC(20,0) NOT NULL CHECK(expected_session_generation BETWEEN 1 AND 18446744073709551615),
 species_key TEXT NOT NULL CHECK(octet_length(species_key) BETWEEN 1 AND 512),
 charm_granted BOOLEAN NOT NULL,
 imbuement_scroll_count SMALLINT NOT NULL CHECK(imbuement_scroll_count=1),
 catalyst_count SMALLINT NOT NULL CHECK(catalyst_count=1),
 forge_dust_count SMALLINT NOT NULL CHECK(forge_dust_count=50),
 created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
 UNIQUE(character_id,species_key),
 UNIQUE(game_session_id,occurrence_id)
);

CREATE FUNCTION game_echo_raid_warden_reward_guard() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
DECLARE admitted NUMERIC(20,0);
BEGIN
 SELECT connection_generation INTO admitted FROM public.game_durability_fresh_admission_receipts
  WHERE game_session_id=NEW.game_session_id AND character_id=NEW.character_id FOR KEY SHARE;
 IF admitted IS NULL OR admitted<>NEW.expected_session_generation THEN
   RAISE EXCEPTION 'stale echo raid session generation' USING ERRCODE='40001';
 END IF;
 IF NOT NEW.charm_granted THEN
   RAISE EXCEPTION 'first species Warden must grant charm' USING ERRCODE='23514';
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_echo_raid_warden_reward_fenced BEFORE INSERT ON game_echo_raid_warden_rewards
 FOR EACH ROW EXECUTE FUNCTION game_echo_raid_warden_reward_guard();
CREATE TRIGGER game_echo_raid_warden_reward_immutable BEFORE UPDATE OR DELETE ON game_echo_raid_warden_rewards
 FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_echo_raid_warden_reward_no_truncate BEFORE TRUNCATE ON game_echo_raid_warden_rewards
 EXECUTE FUNCTION game_item_immutable();
REVOKE ALL ON game_echo_raid_warden_rewards FROM PUBLIC;
REVOKE ALL ON FUNCTION game_echo_raid_warden_reward_guard() FROM PUBLIC;
GRANT SELECT,INSERT ON game_echo_raid_warden_rewards TO oteryn_game_runtime;
GRANT SELECT ON game_echo_raid_warden_rewards TO oteryn_game_control;
