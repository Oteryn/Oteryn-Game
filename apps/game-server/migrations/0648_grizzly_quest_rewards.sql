-- W15c: Grizzly progress and success receipts use the existing QuestState owner.
-- Preserve a rule refusal for the same command/death, and make the holy icon a
-- durable quest obligation consumed by the existing RewardClaim MINT owner.
CREATE TABLE game_character_grizzly_refusals (
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    cause_kind TEXT NOT NULL CHECK (cause_kind IN ('command','step','creature_death')),
    cause_id UUID NOT NULL CHECK (game_character_is_uuid_v7(cause_id)),
    cause_ordinal NUMERIC(20,0) NOT NULL
        CHECK (cause_ordinal BETWEEN 0 AND 18446744073709551615),
    transition_key TEXT NOT NULL
        CHECK (transition_key ~ '^oteryn:[A-Za-z0-9._:/-]+$' AND octet_length(transition_key) <= 128),
    request_binding BYTEA NOT NULL CHECK (octet_length(request_binding) = 33),
    result_code TEXT NOT NULL CHECK (result_code IN
        ('STAGE_MISMATCH','REVISION_MISMATCH','OUT_OF_RANGE','NOT_SUPPORTED','CAPACITY_EXCEEDED')),
    CHECK ((cause_kind = 'creature_death') = (cause_ordinal = 0)),
    PRIMARY KEY (character_id,cause_id,cause_ordinal,transition_key)
);
REVOKE ALL ON game_character_grizzly_refusals FROM PUBLIC;
GRANT SELECT, INSERT ON game_character_grizzly_refusals TO oteryn_game_runtime;
CREATE TRIGGER game_character_grizzly_refusals_immutable
BEFORE UPDATE OR DELETE ON game_character_grizzly_refusals
FOR EACH ROW EXECUTE FUNCTION game_character_immutable();

CREATE TABLE game_character_grizzly_reward_entitlements (
    character_id UUID PRIMARY KEY REFERENCES game_character_roots(character_id),
    transition_key TEXT NOT NULL CHECK (octet_length(transition_key) BETWEEN 1 AND 128),
    cause_id UUID NOT NULL,
    cause_ordinal NUMERIC(20,0) NOT NULL,
    item_source_id INTEGER NOT NULL CHECK (item_source_id = 9388),
    claim_key TEXT NOT NULL CHECK (claim_key = 'oteryn:reward-claim.grizzly-holy-icon'),
    content_revision TEXT NOT NULL
        CHECK (content_revision ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    state TEXT NOT NULL DEFAULT 'PENDING' CHECK (state IN ('PENDING','DELIVERED')),
    FOREIGN KEY (character_id,cause_id,cause_ordinal,transition_key)
        REFERENCES game_character_quest_receipts(character_id,cause_id,cause_ordinal,transition_key)
);
REVOKE ALL ON game_character_grizzly_reward_entitlements FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE ON game_character_grizzly_reward_entitlements TO oteryn_game_runtime;

CREATE FUNCTION game_grizzly_reward_entitlement_guard() RETURNS trigger
LANGUAGE plpgsql SET search_path=pg_catalog,public,pg_temp AS $$
BEGIN
    IF TG_OP = 'INSERT' AND NEW.state = 'PENDING' AND EXISTS (
        SELECT 1 FROM game_character_quest_receipts r
        WHERE (r.character_id,r.cause_id,r.cause_ordinal,r.transition_key)
            = (NEW.character_id,NEW.cause_id,NEW.cause_ordinal,NEW.transition_key)
          AND r.quest_key = 'oteryn:quest.killing_in_the_name_of_quest'
          AND r.cause_kind = 'command' AND r.created_xact_id = pg_current_xact_id()
          -- Crystal lib/quests/killing_in_the_name_of.lua:886-899, NPC:515.
          AND r.values_after[array_position(r.track_keys,'oteryn:quest-progress/storage/1542')] = 2
          AND r.values_after[array_position(r.track_keys,'oteryn:quest-progress/storage/41300')] = 1
    ) THEN
        RETURN NEW;
    ELSIF TG_OP = 'UPDATE' AND OLD.state = 'PENDING' AND NEW.state = 'DELIVERED'
        AND (to_jsonb(NEW) - 'state') = (to_jsonb(OLD) - 'state') AND EXISTS (
        SELECT 1 FROM game_reward_claim_mint_receipts r
        WHERE r.character_id = NEW.character_id
          AND r.claim_family = 'RewardClaim' AND r.claim_production_key = NEW.claim_key
          AND r.created_xact_id = pg_current_xact_id()
    ) THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'Grizzly entitlement requires its quest or MINT owner receipt'
        USING ERRCODE = '23514';
END;
$$;
REVOKE ALL ON FUNCTION game_grizzly_reward_entitlement_guard() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_grizzly_reward_entitlement_guard() TO oteryn_game_runtime;
CREATE TRIGGER game_grizzly_reward_entitlement_guard
BEFORE INSERT OR UPDATE OR DELETE ON game_character_grizzly_reward_entitlements
FOR EACH ROW EXECUTE FUNCTION game_grizzly_reward_entitlement_guard();

-- A frozen boss death cannot credit a counter reset by a later Grizzly command.
-- The guard reads only this Character's command receipts after the observed
-- revision, excluding the unbounded stream of ordinary creature-death receipts.
CREATE INDEX game_character_grizzly_command_revisions
ON game_character_quest_receipts(character_id,committed_character_revision)
INCLUDE(track_keys,values_after)
WHERE cause_kind='command'
  AND quest_key='oteryn:quest.killing_in_the_name_of_quest'
  AND transition_key LIKE 'oteryn:quest-transition/grizzly/%';
