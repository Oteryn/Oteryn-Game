-- Two Guild ledger successors are backed by one immutable, same-World operation.
ALTER TABLE game_guild_bank_operations
 ADD COLUMN transfer_target UUID REFERENCES game_guilds,
 ADD COLUMN transfer_target_before BIGINT NOT NULL DEFAULT 0 CHECK(transfer_target_before BETWEEN 0 AND 999999999999),
 ADD COLUMN transfer_target_after BIGINT NOT NULL DEFAULT 0 CHECK(transfer_target_after BETWEEN 0 AND 999999999999),
 ADD COLUMN transfer_npc_identity BYTEA,
 ADD COLUMN transfer_npc_generation NUMERIC(20,0),
 ADD CONSTRAINT game_guild_bank_transfer_values CHECK(
  CASE WHEN kind='GUILD_TRANSFER' THEN
   amount>0 AND transfer_npc_identity IS NOT NULL AND octet_length(transfer_npc_identity)=16
   AND transfer_npc_generation IS NOT NULL AND transfer_npc_generation BETWEEN 1 AND 18446744073709551615
   AND transfer_target IS DISTINCT FROM guild_id
   AND bank_before=0 AND bank_after=0 AND claim_before=0 AND claim_after=0
   AND CASE WHEN result=1 THEN transfer_target IS NOT NULL
    AND guild_before-guild_after=amount AND transfer_target_after-transfer_target_before=amount
    ELSE transfer_target_before=0 AND transfer_target_after=0 END
  ELSE transfer_target IS NULL AND transfer_target_before=0 AND transfer_target_after=0
   AND transfer_npc_identity IS NULL AND transfer_npc_generation IS NULL END);
CREATE INDEX game_guild_bank_transfer_target ON game_guild_bank_operations(transfer_target)
 WHERE transfer_target IS NOT NULL;

DO $$ DECLARE c RECORD; BEGIN
 FOR c IN SELECT conname,pg_get_expr(conbin,conrelid) AS expr FROM pg_constraint
  WHERE conrelid='game_guild_bank_operations'::regclass AND contype='c'
    AND pg_get_expr(conbin,conrelid) LIKE '%GUILD_DEPOSIT%' LOOP
  EXECUTE format('ALTER TABLE game_guild_bank_operations DROP CONSTRAINT %I',c.conname);
  EXECUTE format('ALTER TABLE game_guild_bank_operations ADD CONSTRAINT %I CHECK(kind=''GUILD_TRANSFER'' OR (%s))',c.conname,c.expr);
 END LOOP;
END $$;

ALTER TABLE game_guild_bank_entries DROP CONSTRAINT game_guild_bank_entries_operation_id_key,
 ADD UNIQUE(operation_id,guild_id);
DO $$ DECLARE c RECORD; BEGIN
 FOR c IN SELECT conname,pg_get_expr(conbin,conrelid) AS expr FROM pg_constraint
  WHERE conrelid='game_guild_bank_entries'::regclass AND contype='c'
    AND pg_get_expr(conbin,conrelid) LIKE '%GUILD_DEPOSIT%' LOOP
  EXECUTE format('ALTER TABLE game_guild_bank_entries DROP CONSTRAINT %I',c.conname);
  EXECUTE format('ALTER TABLE game_guild_bank_entries ADD CONSTRAINT %I CHECK(CASE WHEN kind=''GUILD_TRANSFER'' THEN abs(balance_after-balance_before)=amount ELSE (%s) END)',c.conname,c.expr);
 END LOOP;
END $$;

-- Keep the original successor and Hall custody proofs. Only the tuple matched
-- to the operation and the number of successors differ for a peer transfer.
DO $$ DECLARE definition TEXT; updated TEXT; BEGIN
 SELECT pg_get_functiondef('game_guild_bank_entry_proven()'::regprocedure) INTO definition;
 updated:=replace(definition,
  '(o.guild_id,o.kind,o.amount,o.guild_before,o.guild_after)',
  '(CASE WHEN o.kind=''GUILD_TRANSFER'' AND NEW.guild_id=o.transfer_target THEN o.transfer_target ELSE o.guild_id END,o.kind,o.amount,
    CASE WHEN o.kind=''GUILD_TRANSFER'' AND NEW.guild_id=o.transfer_target THEN o.transfer_target_before ELSE o.guild_before END,
    CASE WHEN o.kind=''GUILD_TRANSFER'' AND NEW.guild_id=o.transfer_target THEN o.transfer_target_after ELSE o.guild_after END)');
 IF updated=definition THEN RAISE EXCEPTION 'missing Guild Bank entry proof'; END IF;
 EXECUTE updated;

 SELECT pg_get_functiondef('game_guild_bank_operation_proven()'::regprocedure) INTO definition;
 updated:=replace(definition,
  'WHEN o.kind=''GUILD_DISBAND_CLAIM'' THEN 1',
  'WHEN o.kind IN(''GUILD_DISBAND_CLAIM'',''GUILD_TRANSFER'') THEN 1');
 IF updated=definition THEN RAISE EXCEPTION 'missing Guild Bank audit count'; END IF;
 definition:=updated;
 updated:=replace(definition,
  'CASE WHEN o.kind=''GUILD_DISBAND_CLAIM'' THEN ARRAY[''BANK'']',
  'CASE WHEN o.kind=''GUILD_TRANSFER'' THEN ARRAY[''GUILD''] WHEN o.kind=''GUILD_DISBAND_CLAIM'' THEN ARRAY[''BANK'']');
 IF updated=definition THEN RAISE EXCEPTION 'missing Guild Bank audit owners'; END IF;
 definition:=updated;
 updated:=replace(definition,
  'CASE WHEN o.result=1 AND o.guild_before<>o.guild_after THEN 1 ELSE 0 END',
  'CASE WHEN o.result=1 AND o.kind=''GUILD_TRANSFER'' THEN 2 WHEN o.result=1 AND o.guild_before<>o.guild_after THEN 1 ELSE 0 END');
 IF updated=definition THEN RAISE EXCEPTION 'missing Guild Bank successor count'; END IF;
 definition:=updated;
 updated:=replace(definition,
  'IF o.kind=''GUILD_DISBAND_PAYOUT'' THEN',
  'IF o.kind=''GUILD_TRANSFER'' AND NOT (
    EXISTS(SELECT 1 FROM game_guilds WHERE guild_id=o.guild_id AND world_id=o.world_id
      AND state IN(1,2) AND revision=o.guild_revision)
    AND EXISTS(SELECT 1 FROM game_guilds WHERE guild_id=o.transfer_target AND world_id=o.world_id AND state IN(1,2))
    AND EXISTS(SELECT 1 FROM game_guild_members WHERE guild_id=o.guild_id AND character_id=o.actor AND rank_level IN(1,2))
   ) THEN RAISE EXCEPTION ''Guild transfer requires current officer and same-World guilds'' USING ERRCODE=''23514''; END IF;
   IF o.kind=''GUILD_DISBAND_PAYOUT'' THEN');
 IF updated=definition THEN RAISE EXCEPTION 'missing Guild Bank operation proof'; END IF;
 EXECUTE updated;
END $$;

CREATE OR REPLACE FUNCTION game_guild_bank_payload(o game_guild_bank_operations,own TEXT) RETURNS JSONB LANGUAGE sql IMMUTABLE AS $$
 SELECT jsonb_build_object('version',1,'class',CASE WHEN o.hall_burn>0 THEN 'BURN' ELSE 'TRANSFER' END,'kind',o.kind,'owner',own,
  'guild',o.guild_id,'account',o.account_id,'world',o.world_id,'actor',o.actor,
  'bank_before',o.bank_before,'bank_after',o.bank_after,'guild_before',o.guild_before,'guild_after',o.guild_after,
  'claim_before',o.claim_before,'claim_after',o.claim_after,'amount',o.amount)
  ||CASE WHEN o.hall_house IS NULL THEN '{}'::jsonb ELSE jsonb_build_object('house',o.hall_house,'auction',o.hall_auction,
   'escrow_guild_before',o.hall_guild_before,'escrow_guild_after',o.hall_guild_after,
   'escrow_account_before',o.hall_account_before,'escrow_account_after',o.hall_account_after,'burn',o.hall_burn) END
  ||CASE WHEN o.kind='GUILD_TRANSFER' THEN jsonb_build_object('recipient_guild',o.transfer_target,
   'recipient_before',o.transfer_target_before,'recipient_after',o.transfer_target_after) ELSE '{}'::jsonb END
$$;
