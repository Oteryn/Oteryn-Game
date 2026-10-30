-- MONKQUEST3: a shrine flag and its item entitlement commit together. The existing
-- RewardClaim mint owns delivery; a full bag leaves the entitlement PENDING.
CREATE TABLE game_character_monk_reward_entitlements (
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id) ON DELETE CASCADE,
    transition_key TEXT NOT NULL CHECK (octet_length(transition_key) BETWEEN 1 AND 128),
    cause_id UUID NOT NULL,
    cause_ordinal NUMERIC(20,0) NOT NULL,
    item_source_id INTEGER NOT NULL CHECK (item_source_id IN (50267,50271,50269,50273,50274,50272,50268)),
    claim_key TEXT NOT NULL CHECK (octet_length(claim_key) BETWEEN 1 AND 128),
    content_revision TEXT NOT NULL CHECK (octet_length(content_revision) BETWEEN 1 AND 128),
    state TEXT NOT NULL DEFAULT 'PENDING' CHECK (state IN ('PENDING','DELIVERED')),
    PRIMARY KEY (character_id, transition_key),
    UNIQUE (character_id, claim_key),
    FOREIGN KEY (character_id, cause_id, cause_ordinal, transition_key)
        REFERENCES game_character_quest_receipts(character_id,cause_id,cause_ordinal,transition_key)
);
REVOKE ALL ON game_character_monk_reward_entitlements FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE ON game_character_monk_reward_entitlements TO oteryn_game_runtime;
GRANT SELECT ON game_character_monk_reward_entitlements TO oteryn_game_control;

-- Stable Oteryn title ids 1..4 retain their meanings. Pensive Wanderer is id 5.
ALTER TABLE game_character_titles DROP CONSTRAINT game_character_titles_title_check;
ALTER TABLE game_character_titles ADD CONSTRAINT game_character_titles_title_check CHECK (title BETWEEN 1 AND 5);
ALTER TABLE game_character_title_selection DROP CONSTRAINT game_character_title_selection_title_check;
ALTER TABLE game_character_title_selection ADD CONSTRAINT game_character_title_selection_title_check CHECK (title IS NULL OR title BETWEEN 1 AND 5);

-- Identity never changes. Earning proves the flag receipt in this transaction;
-- delivery proves the existing MINT receipt in its transaction.
CREATE FUNCTION game_monk_reward_entitlement_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'INSERT' AND NEW.state = 'PENDING' AND EXISTS (
        SELECT 1 FROM game_character_quest_receipts r
        WHERE (r.character_id,r.cause_id,r.cause_ordinal,r.transition_key)
            = (NEW.character_id,NEW.cause_id,NEW.cause_ordinal,NEW.transition_key)
          AND r.quest_key = 'oteryn:quest.the_way_of_the_monk_quest'
          AND r.cause_kind = 'use' AND r.created_xact_id = pg_current_xact_id()) THEN
        RETURN NEW;
    ELSIF TG_OP = 'UPDATE' AND OLD.state = 'PENDING' AND NEW.state = 'DELIVERED'
        AND (to_jsonb(NEW) - 'state') = (to_jsonb(OLD) - 'state') AND EXISTS (
        SELECT 1 FROM game_reward_claim_mint_receipts r
        WHERE r.character_id = NEW.character_id
          AND r.claim_family = 'RewardClaim' AND r.claim_production_key = NEW.claim_key
          AND r.created_xact_id = pg_current_xact_id()) THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'Monk entitlement requires its quest or MINT owner receipt'
        USING ERRCODE = '23514';
END;
$$;
CREATE TRIGGER game_monk_reward_entitlement_guard
BEFORE INSERT OR UPDATE OR DELETE ON game_character_monk_reward_entitlements
FOR EACH ROW EXECUTE FUNCTION game_monk_reward_entitlement_guard();
