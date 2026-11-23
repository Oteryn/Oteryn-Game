-- Extend the fenced 0941 rent tick at the warning mint. Keep the text and its
-- revision in the same transaction as the warning, item and Inbox delivery.
DO $migration$
DECLARE
 tick CONSTANT regprocedure := 'game_house_rent_tick(uuid,uuid,numeric,bytea)'::regprocedure;
 definition TEXT := pg_get_functiondef(tick);
 old_mint CONSTANT TEXT := $old$
  INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id)
   VALUES(letter,w,'Item','oteryn:item.tibia.i3506','house-rent-warning',1,1,letter_tx);
  PERFORM game_character_inbox_deliver(letter,p.owner_character_id,w,'HOUSE_RENT_WARNING',letter::text);
$old$;
 new_mint CONSTANT TEXT := $new$
  INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id)
   VALUES(letter,w,'Item','oteryn:item.tibia.i3506','definition-r1',1,1,letter_tx);
  INSERT INTO game_item_text_revisions(item_instance_id,last_revision) VALUES(letter,1);
  -- Crystal formats the warning using the count before incrementing it.
  -- Use the stable house key when the world catalogue has no display name.
  INSERT INTO game_item_texts(item_instance_id,text,writer_character_id,written_at,text_revision)
   VALUES(letter,format(
    E'Warning! \nThe %s rent of %s gold for your house "%s" is payable. Have it within %s days or you will lose this house.',
    CASE period WHEN interval '1 day' THEN 'daily' WHEN interval '7 days' THEN 'weekly'
     WHEN interval '30 days' THEN 'monthly' WHEN interval '365 days' THEN 'annual' END,
    p.rent_gold,
    COALESCE((SELECT c.name FROM game_public_house_catalogue c
              WHERE c.world_id=w AND c.house_key=p.house_key),p.house_key),
    8-s.warnings),NULL,statement_timestamp(),1);
  PERFORM game_character_inbox_deliver(letter,p.owner_character_id,w,'HOUSE_RENT_WARNING',letter::text);
$new$;
 old_count INTEGER := cardinality(string_to_array(definition,old_mint));
 new_count INTEGER := cardinality(string_to_array(definition,new_mint));
BEGIN
 IF old_count = 2 AND new_count = 1 THEN
  EXECUTE replace(definition,old_mint,new_mint);
 ELSIF old_count IS DISTINCT FROM 1 OR new_count IS DISTINCT FROM 2 THEN
  RAISE EXCEPTION 'expected one 0941 house rent warning mint in %',tick;
 END IF;
 EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',tick,current_schema());
END $migration$;

REVOKE ALL ON FUNCTION game_house_rent_tick(UUID,UUID,NUMERIC,BYTEA) FROM PUBLIC,oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_house_rent_tick(UUID,UUID,NUMERIC,BYTEA) TO oteryn_game_runtime;
