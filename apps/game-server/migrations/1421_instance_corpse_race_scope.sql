-- Main 1100 owns exact nullable Channel/Instance race provenance. Preserve its
-- complete proof and canonical tagged BYTEA namespace; add no admission arm.
DO $compatibility$
DECLARE body TEXT; expected TEXT; relation_name TEXT; expression TEXT;
BEGIN
    SELECT p.prosrc INTO STRICT body FROM pg_proc p
      WHERE p.oid='game_corpse_source_race_admit()'::regprocedure;
    expected:=$proof$
BEGIN
 IF NOT EXISTS(SELECT 1 FROM game_item_mint_receipts r
   JOIN game_item_instances i ON i.item_instance_id=r.item_instance_id
   JOIN game_item_ground_locations g ON g.item_instance_id=i.item_instance_id
   WHERE r.transaction_id=NEW.mint_transaction_id AND r.item_instance_id=NEW.item_instance_id
    AND r.created_xact_id=pg_current_xact_id() AND r.loot_purpose_key='CORPSE_MATERIALIZATION'
    AND r.draw_ordinal=0 AND r.destination_parent_item_instance_id IS NULL
    AND i.minted_transaction_id=r.transaction_id AND i.lifecycle=1 AND i.quantity=1
    AND i.world_id=r.death_world_id AND g.world_id=r.death_world_id
    AND (g.channel_id IS NOT DISTINCT FROM r.death_channel_id AND g.instance_id IS NOT DISTINCT FROM r.death_instance_id)
    AND g.runtime_scope_ownership_generation=r.death_scope_ownership_generation
    AND i.definition_family=r.loot_table_family
    AND i.definition_production_key=r.loot_table_production_key
    AND i.definition_revision_ref=r.loot_table_revision_ref)
 THEN RAISE EXCEPTION 'corpse race requires this transaction exact corpse mint' USING ERRCODE='23514'; END IF;
 NEW.created_xact_id:=pg_current_xact_id();
 RETURN NEW;
END $proof$;
    IF regexp_replace(body,'\s','','g') IS DISTINCT FROM
       regexp_replace(expected,'\s','','g') THEN
        RAISE EXCEPTION 'main exact corpse race provenance proof changed';
    END IF;
    FOREACH relation_name IN ARRAY ARRAY['game_item_mint_reservations','game_item_mint_receipts'] LOOP
        SELECT pg_get_expr(d.adbin,d.adrelid) INTO STRICT expression
        FROM pg_attribute a JOIN pg_attrdef d
          ON d.adrelid=a.attrelid AND d.adnum=a.attnum
        WHERE a.attrelid=relation_name::regclass AND a.attname='death_scope_id'
          AND NOT a.attisdropped AND a.atttypid='bytea'::regtype
          AND a.attgenerated='s' AND a.attnotnull;
        IF regexp_replace(expression,'[\s()]','','g') IS DISTINCT FROM
           regexp_replace($expression$CASE WHEN death_channel_id IS NOT NULL
             THEN decode('01'::text,'hex'::text)||uuid_send(death_channel_id)
             ELSE decode('02'::text,'hex'::text)||uuid_send(death_instance_id) END$expression$,
             '[\s()]','','g') THEN
            RAISE EXCEPTION 'main tagged corpse death scope expression changed: %',relation_name;
        END IF;
        IF NOT EXISTS (SELECT 1 FROM pg_constraint c JOIN pg_attribute a
            ON a.attrelid=c.conrelid AND a.attnum=ANY(c.conkey)
            WHERE c.conrelid=relation_name::regclass AND c.contype='p'
              AND a.attname='death_scope_id') THEN
            RAISE EXCEPTION 'main corpse cause primary key lacks tagged scope: %',relation_name;
        END IF;
    END LOOP;
END $compatibility$;
