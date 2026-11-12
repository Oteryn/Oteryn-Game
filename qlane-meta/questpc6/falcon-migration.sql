-- Complete Falcon section for root-owned migration0969; not a separate migration.
CREATE TABLE game_character_falcon_chest_cycles (
 character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 chest_key TEXT NOT NULL CHECK (chest_key IN (
 'oteryn:reward-claim.quest.u11_80.the_secret_library.falcon_bastion.chests_timer.coast',
 'oteryn:reward-claim.quest.u11_80.the_secret_library.falcon_bastion.chests_timer.island',
 'oteryn:reward-claim.quest.u11_80.the_secret_library.falcon_bastion.chests_timer.throne_hall',
 'oteryn:reward-claim.quest.u11_80.the_secret_library.falcon_bastion.chests_timer.shortcut',
 'oteryn:reward-claim.quest.u11_80.the_secret_library.falcon_bastion.chests_timer.lower_bastion',
 'oteryn:reward-claim.quest.u11_80.the_secret_library.falcon_bastion.chests_timer.underground_bastion')),
 cycle BIGINT NOT NULL CHECK(cycle>0),
 entropy BYTEA NOT NULL CHECK(octet_length(entropy)=32),
 selected_index SMALLINT NOT NULL CHECK(selected_index BETWEEN 0 AND 6),
 drawn_at BIGINT NOT NULL,
 claimed_at BIGINT NULL,
 transaction_id UUID NULL REFERENCES game_reward_claim_mint_receipts(transaction_id),
 CHECK((claimed_at IS NULL) = (transaction_id IS NULL)),
 CHECK(claimed_at IS NULL OR claimed_at>=drawn_at),
 PRIMARY KEY(character_id,chest_key,cycle)
);
CREATE UNIQUE INDEX game_falcon_cycle_pending ON game_character_falcon_chest_cycles(character_id,chest_key) WHERE claimed_at IS NULL;
CREATE UNIQUE INDEX game_falcon_cycle_transaction ON game_character_falcon_chest_cycles(transaction_id) WHERE transaction_id IS NOT NULL;
REVOKE ALL ON game_character_falcon_chest_cycles FROM PUBLIC;
GRANT SELECT,INSERT,UPDATE ON game_character_falcon_chest_cycles TO oteryn_game_runtime;
GRANT SELECT ON game_character_falcon_chest_cycles TO oteryn_game_control;
CREATE FUNCTION game_falcon_now_ms() RETURNS BIGINT
LANGUAGE SQL VOLATILE AS $$
    SELECT floor(extract(epoch FROM statement_timestamp())*1000)::bigint
$$;

CREATE FUNCTION game_falcon_draw_index(p_entropy BYTEA) RETURNS SMALLINT
LANGUAGE plpgsql IMMUTABLE STRICT AS $$
DECLARE seed_block BYTEA := p_entropy; ordinal INTEGER; sample_byte INTEGER;
BEGIN
    IF octet_length(seed_block) <> 32 THEN
        RAISE EXCEPTION 'Falcon entropy must be 32 bytes' USING ERRCODE='23514';
    END IF;
    LOOP
        FOR ordinal IN 0..31 LOOP
            sample_byte := get_byte(seed_block,ordinal);
            IF sample_byte < 252 THEN RETURN (sample_byte % 7)::smallint; END IF;
        END LOOP;
        seed_block := sha256(seed_block);
    END LOOP;
END;
$$;

CREATE FUNCTION game_falcon_cycle_guard() RETURNS TRIGGER
LANGUAGE plpgsql AS $$
DECLARE previous_cycle BIGINT; previous_claimed BIGINT;
BEGIN
    IF TG_OP='DELETE' THEN
        RAISE EXCEPTION 'Falcon cycle receipts are retained' USING ERRCODE='23514';
    END IF;
    -- Serialize the six per-character cycle owners even for direct SQL calls.
    PERFORM 1 FROM game_character_roots WHERE character_id=NEW.character_id FOR UPDATE;
    IF NEW.selected_index <> game_falcon_draw_index(NEW.entropy) THEN
        RAISE EXCEPTION 'Falcon draw differs from frozen entropy' USING ERRCODE='23514';
    END IF;
    IF TG_OP='INSERT' THEN
        SELECT cycle,claimed_at INTO previous_cycle,previous_claimed
          FROM game_character_falcon_chest_cycles
         WHERE character_id=NEW.character_id AND chest_key=NEW.chest_key
         ORDER BY cycle DESC LIMIT 1;
        IF NEW.claimed_at IS NOT NULL OR NEW.transaction_id IS NOT NULL
           OR NEW.cycle <> COALESCE(previous_cycle,0)+1
           OR NEW.drawn_at NOT BETWEEN game_falcon_now_ms()-10000 AND game_falcon_now_ms()
           OR (previous_cycle IS NOT NULL AND
               (previous_claimed IS NULL OR
                NEW.drawn_at::numeric < previous_claimed::numeric+86400000)) THEN
            RAISE EXCEPTION 'Falcon cycle must follow successful 24h eligibility' USING ERRCODE='23514';
        END IF;
    ELSIF TG_OP='UPDATE' THEN
        IF OLD.claimed_at IS NOT NULL OR NEW.claimed_at IS NULL OR NEW.transaction_id IS NULL
           OR ROW(NEW.character_id,NEW.chest_key,NEW.cycle,NEW.entropy,NEW.selected_index,NEW.drawn_at)
              IS DISTINCT FROM ROW(OLD.character_id,OLD.chest_key,OLD.cycle,OLD.entropy,OLD.selected_index,OLD.drawn_at)
           OR NOT EXISTS (
                SELECT 1 FROM game_reward_claim_mint_receipts r
                 WHERE r.transaction_id=NEW.transaction_id
                   AND r.character_id=NEW.character_id
                   AND r.claim_family='RewardClaim'
                   AND r.claim_production_key=NEW.chest_key||':cycle:'||NEW.cycle::text
                   AND r.occurred_at=NEW.claimed_at
                   AND r.created_xact_id=pg_current_xact_id()) THEN
            RAISE EXCEPTION 'Falcon cycle may only acknowledge its atomic mint' USING ERRCODE='23514';
        END IF;
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER game_falcon_cycle_guard BEFORE INSERT OR UPDATE OR DELETE
ON game_character_falcon_chest_cycles FOR EACH ROW EXECUTE FUNCTION game_falcon_cycle_guard();
CREATE TRIGGER game_falcon_cycles_no_truncate BEFORE TRUNCATE
ON game_character_falcon_chest_cycles FOR EACH STATEMENT EXECUTE FUNCTION game_item_immutable();

CREATE FUNCTION game_falcon_receipt_guard() RETURNS TRIGGER
LANGUAGE plpgsql AS $$
BEGIN
    -- Internal operation identities remain subject to every existing generic
    -- mint proof. This additional deferred proof closes direct once-owner calls.
    IF NOT starts_with(NEW.claim_production_key,'oteryn:reward-claim.quest.u11_80.the_secret_library.falcon_bastion.chests_timer.') THEN
        RETURN NULL;
    END IF;
    IF NEW.claim_production_key !~ '^oteryn:reward-claim[.]quest[.]u11_80[.]the_secret_library[.]falcon_bastion[.]chests_timer[.](coast|island|throne_hall|shortcut|lower_bastion|underground_bastion):cycle:[1-9][0-9]*$' THEN
        RAISE EXCEPTION 'Falcon public claims require the repeat owner' USING ERRCODE='23514';
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM game_character_falcon_chest_cycles c
        JOIN game_item_instances i ON i.item_instance_id=NEW.item_instance_id
        WHERE c.character_id=NEW.character_id AND c.transaction_id=NEW.transaction_id
          AND NEW.claim_family='RewardClaim'
          AND NEW.claim_production_key=c.chest_key||':cycle:'||c.cycle::text
          AND c.claimed_at=NEW.occurred_at
          AND NEW.quantity=(ARRAY[1,1,1,1,1,1,10])[c.selected_index+1]
          AND i.definition_family='Item'
          AND i.definition_production_key='oteryn:item.tibia.i'||(ARRAY[9081,28821,28823,9058,6299,3052,3035])[c.selected_index+1]::text
          AND c.selected_index=game_falcon_draw_index(c.entropy)) THEN
        RAISE EXCEPTION 'Falcon mint requires its exact cycle and selected pool item' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END;
$$;
CREATE CONSTRAINT TRIGGER game_falcon_receipt_guard AFTER INSERT
ON game_reward_claim_mint_receipts DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION game_falcon_receipt_guard();

DO $$
DECLARE signature TEXT;
BEGIN
    FOREACH signature IN ARRAY ARRAY['game_falcon_now_ms()','game_falcon_draw_index(BYTEA)',
        'game_falcon_cycle_guard()','game_falcon_receipt_guard()'] LOOP
        EXECUTE format('ALTER FUNCTION %s SET search_path = %I, pg_temp',signature,current_schema());
    END LOOP;
END;
$$;
REVOKE ALL ON FUNCTION game_falcon_now_ms(),game_falcon_draw_index(BYTEA),game_falcon_cycle_guard(),game_falcon_receipt_guard() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_falcon_now_ms(),game_falcon_draw_index(BYTEA),game_falcon_cycle_guard(),game_falcon_receipt_guard() TO oteryn_game_runtime;
GRANT EXECUTE ON FUNCTION game_falcon_now_ms(),game_falcon_draw_index(BYTEA) TO oteryn_game_control;
