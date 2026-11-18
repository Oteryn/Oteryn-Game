-- The shared sarcophagus cooldown and rolled receipt include uses that mint no key.
CREATE TABLE game_haunted_tomb_uses (
    game_session_id UUID NOT NULL,
    command_id NUMERIC(20,0) NOT NULL CHECK (command_id BETWEEN 1 AND 18446744073709551615),
    character_id UUID NOT NULL,
    world_id UUID NOT NULL,
    channel_id UUID NOT NULL,
    scope_generation NUMERIC(20,0) NOT NULL CHECK (scope_generation BETWEEN 1 AND 18446744073709551615),
    map_digest BYTEA NOT NULL CHECK (octet_length(map_digest)=32),
    placement BYTEA NOT NULL CHECK (octet_length(placement)=8),
    source_revision NUMERIC(20,0) NOT NULL CHECK (source_revision BETWEEN 0 AND 18446744073709551615),
    roll SMALLINT NOT NULL CHECK (roll BETWEEN 1 AND 10),
    occurred_at BIGINT NOT NULL CHECK (occurred_at>0),
    expires_at BIGINT NOT NULL CHECK (expires_at=occurred_at+30000),
    reward_transaction_id UUID UNIQUE REFERENCES game_reward_claim_mint_receipts(transaction_id),
    created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
    CHECK (reward_transaction_id IS NULL OR roll>=7),
    PRIMARY KEY (game_session_id,command_id)
);
CREATE INDEX game_haunted_tomb_cooldown ON game_haunted_tomb_uses
    (world_id,channel_id,scope_generation,map_digest,placement,expires_at);
REVOKE ALL ON game_haunted_tomb_uses FROM PUBLIC;
GRANT SELECT,INSERT ON game_haunted_tomb_uses TO oteryn_game_runtime;
GRANT SELECT ON game_haunted_tomb_uses TO oteryn_game_control;

CREATE FUNCTION game_haunted_tomb_deadline_proven(s game_spell_item_temporal_schedules)
RETURNS BOOLEAN LANGUAGE sql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
    SELECT s.definition_production_key='oteryn:item.tibia.i29351'
       AND s.definition_family='Item' AND s.definition_revision='definition-r1'
       AND s.target_family IS NULL AND s.source_ordinal=1 AND s.duration_millis=1800000
       AND s.created_xact_id=pg_current_xact_id()
       AND EXISTS (
           SELECT 1 FROM public.game_haunted_tomb_uses u
           JOIN public.game_reward_claim_mint_receipts r ON r.transaction_id=u.reward_transaction_id
           JOIN public.game_item_instances i ON i.item_instance_id=r.item_instance_id
           WHERE (r.game_session_id,r.command_id,r.character_id)
                   =(u.game_session_id,u.command_id,u.character_id)
             AND r.claim_family='RewardClaim' AND r.claim_revision_ref='haunted-tomb-r1'
             AND r.claim_production_key LIKE 'oteryn:reward-claim.haunted-tomb.%'
             AND r.transaction_id=s.source_transaction_id AND r.item_instance_id=s.item_instance_id
             AND r.quantity=1 AND i.lifecycle=1 AND i.quantity=1
             AND (i.definition_family,i.definition_production_key,i.definition_revision_ref)
                   =(s.definition_family,s.definition_production_key,s.definition_revision)
             AND s.expires_at_unix_ms=r.occurred_at+s.duration_millis
             AND u.created_xact_id=s.created_xact_id AND r.created_xact_id=s.created_xact_id
    )
$$;

DO $migration$
DECLARE body TEXT; begin_at INT;
BEGIN
    body:=pg_get_functiondef('game_spell_item_temporal_schedule_proven()'::regprocedure);
    begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
    IF begin_at=0 THEN RAISE EXCEPTION 'missing original temporal mint proof'; END IF;
    EXECUTE overlay(body placing
        E'BEGIN\n IF game_haunted_tomb_deadline_proven(NEW) THEN RETURN NULL; END IF;'
        from begin_at for 5);
END $migration$;
