-- W05d continuation. Crystal201189f data-global/scripts/lib/a_piece_of_cake_config.lua:
-- 101-118,344-383,456-470; actions_cake_bite.lua:5-63;
-- globalevents_cake_eating_window.lua:29-80. Bridge Item48227 is outside this owner.
-- Draft: not mounted until the existing Type2 map/character caller is connected.
CREATE TABLE game_cake_phases (
 world_id UUID NOT NULL,event_year INTEGER NOT NULL,
 stage SMALLINT NOT NULL CHECK(stage IN(2,3)),layer SMALLINT NOT NULL CHECK(layer BETWEEN 1 AND 4),
 revision NUMERIC(20,0) NOT NULL DEFAULT 1 CHECK(revision BETWEEN 1 AND 18446744073709551615),
 window_generation NUMERIC(20,0) NOT NULL DEFAULT 0 CHECK(window_generation BETWEEN 0 AND 18446744073709551615),
 window_opened_at TIMESTAMPTZ,window_ends_at TIMESTAMPTZ,
 layer_progress SMALLINT NOT NULL DEFAULT 0 CHECK(layer_progress BETWEEN 0 AND 1116),
 pending_bonus BOOLEAN NOT NULL DEFAULT FALSE,completed_reset_epoch NUMERIC(20,0),
 bonus_activated_at TIMESTAMPTZ,bonus_until TIMESTAMPTZ,brutus_present BOOLEAN NOT NULL DEFAULT FALSE,
 event_end_cleaned_up BOOLEAN NOT NULL DEFAULT FALSE,
 PRIMARY KEY(world_id,event_year),FOREIGN KEY(world_id,event_year) REFERENCES game_cake_world_progress,
 CHECK((window_opened_at IS NULL)=(window_ends_at IS NULL)),
 CHECK(window_ends_at IS NULL OR window_ends_at=window_opened_at+interval '30 minutes'),
 CHECK(stage<>3 OR (layer=4 AND window_opened_at IS NULL)),
 CHECK(NOT pending_bonus OR (stage=3 AND bonus_activated_at IS NULL)),
 CHECK((bonus_activated_at IS NULL)=(bonus_until IS NULL)),
 CHECK(bonus_until IS NULL OR bonus_until=bonus_activated_at+interval '604800 seconds')
);
-- Immediate stage-one completion shares the original donation/lure physical transaction.
CREATE FUNCTION game_cake_enter_stage_two() RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.substances=200 AND NEW.golems=200 THEN
  INSERT INTO game_cake_phases(world_id,event_year,stage,layer) VALUES(NEW.world_id,NEW.event_year,2,1)
   ON CONFLICT(world_id,event_year) DO NOTHING;
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_cake_stage_two AFTER INSERT OR UPDATE OF substances,golems ON game_cake_world_progress
 FOR EACH ROW EXECUTE FUNCTION game_cake_enter_stage_two();
INSERT INTO game_cake_phases(world_id,event_year,stage,layer)
 SELECT world_id,event_year,2,1 FROM game_cake_world_progress WHERE substances=200 AND golems=200;

CREATE TABLE game_cake_eaters (
 world_id UUID NOT NULL,event_year INTEGER NOT NULL,character_id UUID NOT NULL REFERENCES game_character_roots,
 window_generation NUMERIC(20,0) NOT NULL,bites SMALLINT NOT NULL CHECK(bites BETWEEN 0 AND 12),last_bite_at TIMESTAMPTZ,
 PRIMARY KEY(world_id,event_year,character_id),FOREIGN KEY(world_id,event_year) REFERENCES game_cake_phases
);
CREATE TABLE game_cake_bite_receipts (
 game_session_id UUID NOT NULL,command_id NUMERIC(20,0) NOT NULL CHECK(command_id BETWEEN 1 AND 18446744073709551615),
 character_id UUID NOT NULL REFERENCES game_character_roots,world_id UUID NOT NULL,event_year INTEGER NOT NULL,
 window_generation NUMERIC(20,0) NOT NULL,layer SMALLINT NOT NULL CHECK(layer BETWEEN 1 AND 4),
 placement BYTEA NOT NULL CHECK(octet_length(placement)=40),
 source_item SMALLINT NOT NULL CHECK(source_item IN(12085,12086)),
 progress_after SMALLINT NOT NULL CHECK(progress_after BETWEEN 1 AND 1116),
 before_revision BIGINT NOT NULL CHECK(before_revision>=0),intent BYTEA NOT NULL CHECK(octet_length(intent) BETWEEN 1 AND 529),
 contributed_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 PRIMARY KEY(game_session_id,command_id),
 FOREIGN KEY(world_id,event_year) REFERENCES game_cake_phases
);
CREATE UNIQUE INDEX game_cake_bite_world_progress ON game_cake_bite_receipts(world_id,event_year,window_generation,progress_after);
CREATE UNIQUE INDEX game_cake_bite_world_placement ON game_cake_bite_receipts(world_id,event_year,window_generation,(substring(placement FROM 33 FOR 8)));
CREATE TRIGGER game_cake_bite_no_rewrite BEFORE UPDATE OR DELETE ON game_cake_bite_receipts
 FOR EACH ROW EXECUTE FUNCTION game_cake_lure_immutable();

-- Personal mark state is annual even before Brutus appears; started conversation is
-- informational, never a prerequisite (movements_cake_base_isle.lua:19-31).
CREATE TABLE game_cake_errands (
 world_id UUID NOT NULL,event_year INTEGER NOT NULL,character_id UUID NOT NULL REFERENCES game_character_roots,
 armed BOOLEAN NOT NULL DEFAULT FALSE,cities SMALLINT NOT NULL DEFAULT 0 CHECK(cities BETWEEN 0 AND 31),
 started BOOLEAN NOT NULL DEFAULT FALSE,item_claimed BOOLEAN NOT NULL DEFAULT FALSE,cupcake_claimed BOOLEAN NOT NULL DEFAULT FALSE,
 PRIMARY KEY(world_id,event_year,character_id),FOREIGN KEY(world_id,event_year) REFERENCES game_cake_phases
);
CREATE TABLE game_cake_errand_steps (
 world_id UUID NOT NULL,event_year INTEGER NOT NULL,character_id UUID NOT NULL,
 actor_placement UUID NOT NULL,movement_revision NUMERIC(20,0) NOT NULL,
 reset_epoch NUMERIC(20,0) NOT NULL,position BYTEA NOT NULL CHECK(octet_length(position)=12),
 city SMALLINT CHECK(city BETWEEN 0 AND 4),source_item INTEGER NOT NULL,
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 PRIMARY KEY(actor_placement,movement_revision,reset_epoch),
 FOREIGN KEY(world_id,event_year,character_id) REFERENCES game_cake_errands,
 CHECK((city IS NULL AND source_item BETWEEN 12090 AND 12095) OR (city IS NOT NULL AND source_item=8886))
);
CREATE TRIGGER game_cake_errand_step_no_rewrite BEFORE UPDATE OR DELETE ON game_cake_errand_steps
 FOR EACH ROW EXECUTE FUNCTION game_cake_lure_immutable();
CREATE TABLE game_cake_reward_claims (
 transaction_id UUID PRIMARY KEY REFERENCES game_reward_claim_mint_receipts(transaction_id),
 world_id UUID NOT NULL,event_year INTEGER NOT NULL,character_id UUID NOT NULL,
 category SMALLINT NOT NULL CHECK(category IN(1,2)),reward_key TEXT NOT NULL,
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 UNIQUE(world_id,event_year,character_id,category),
 FOREIGN KEY(world_id,event_year,character_id) REFERENCES game_cake_errands,
 CHECK((category=1 AND reward_key IN('oteryn:item.tibia.i14756','oteryn:item.tibia.i20347','oteryn:item.tibia.i20350'))
  OR (category=2 AND reward_key IN('oteryn:item.tibia.i28484','oteryn:item.tibia.i28485','oteryn:item.tibia.i28486')))
);
CREATE TRIGGER game_cake_reward_no_rewrite BEFORE UPDATE OR DELETE ON game_cake_reward_claims
 FOR EACH ROW EXECUTE FUNCTION game_cake_lure_immutable();
-- Bonus remains valid after seasonal-calendar removal. Closing the event removes
-- Brutus but does not erase the seven-day world benefit (config.lua:473-483).
CREATE FUNCTION game_cake_bonus_at(p_world UUID,p_at TIMESTAMPTZ) RETURNS BOOLEAN LANGUAGE sql STABLE AS $$
 SELECT EXISTS(SELECT 1 FROM game_cake_phases WHERE world_id=p_world
   AND bonus_activated_at<=p_at AND p_at<bonus_until)
$$;
REVOKE ALL ON game_cake_phases,game_cake_eaters,game_cake_bite_receipts,
 game_cake_errands,game_cake_errand_steps,game_cake_reward_claims FROM PUBLIC,oteryn_game_runtime;
REVOKE ALL ON FUNCTION game_cake_enter_stage_two(),game_cake_bonus_at(UUID,TIMESTAMPTZ) FROM PUBLIC;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_cake_enter_stage_two() SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_cake_bonus_at(UUID,TIMESTAMPTZ) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;

-- Used only within the existing current-character Type2 map transaction; if the
-- corresponding exact terrain mutation cannot commit, this entire transaction
-- rolls back, including an opened window and the eater contribution.
CREATE FUNCTION game_cake_bite_prepare(p_world UUID,p_year INTEGER,p_session UUID,p_command NUMERIC,
 p_character UUID,p_placement BYTEA,p_before_revision BIGINT,p_item INTEGER,p_floor SMALLINT,p_intent BYTEA)
RETURNS TEXT LANGUAGE plpgsql AS $$
DECLARE p game_cake_phases; prior game_cake_bite_receipts; e game_cake_eaters;
 sampled TIMESTAMPTZ:=date_trunc('second',clock_timestamp());
BEGIN
 SELECT * INTO prior FROM game_cake_bite_receipts WHERE game_session_id=p_session AND command_id=p_command;
 IF FOUND THEN
  IF prior.world_id=p_world AND prior.event_year=p_year AND prior.character_id=p_character
   AND prior.placement=p_placement AND prior.before_revision=p_before_revision AND prior.source_item=p_item AND prior.intent=p_intent
   THEN RETURN 'replay'; ELSE RETURN 'stale'; END IF;
 END IF;
 SELECT * INTO p FROM game_cake_phases WHERE world_id=p_world AND event_year=p_year FOR UPDATE;
 IF NOT FOUND OR p.stage<>2 THEN RETURN 'not_eating'; END IF;
 -- Source opens the window before checking floor, cooldown and edible item ID.
 IF p.window_opened_at IS NULL THEN
  IF p.window_generation=18446744073709551615 OR p.revision=18446744073709551615 THEN RETURN 'exhausted'; END IF;
  UPDATE game_cake_phases SET window_generation=window_generation+1,window_opened_at=sampled,
   window_ends_at=sampled+interval '30 minutes',layer_progress=0,revision=revision+1
   WHERE world_id=p_world AND event_year=p_year RETURNING * INTO p;
 END IF;
 IF p_floor<>7 THEN RETURN 'wrong_floor'; END IF;
 INSERT INTO game_cake_eaters(world_id,event_year,character_id,window_generation,bites)
  VALUES(p_world,p_year,p_character,p.window_generation,0) ON CONFLICT DO NOTHING;
 SELECT * INTO STRICT e FROM game_cake_eaters WHERE world_id=p_world AND event_year=p_year AND character_id=p_character FOR UPDATE;
 IF e.window_generation<>p.window_generation THEN
  UPDATE game_cake_eaters SET window_generation=p.window_generation,bites=0
   WHERE world_id=p_world AND event_year=p_year AND character_id=p_character RETURNING * INTO e;
 END IF;
 IF e.last_bite_at IS NOT NULL AND sampled-e.last_bite_at<interval '90 seconds' THEN RETURN 'cooldown'; END IF;
 IF e.bites>=12 THEN RETURN 'full'; END IF;
 IF p_item NOT IN(12085,12086) THEN RETURN 'wrong_item'; END IF;
 IF p.layer_progress>=1116 OR EXISTS(SELECT 1 FROM game_cake_bite_receipts WHERE world_id=p_world
  AND event_year=p_year AND window_generation=p.window_generation AND substring(placement FROM 33 FOR 8)=substring(p_placement FROM 33 FOR 8)) THEN RETURN 'stale'; END IF;
 INSERT INTO game_cake_bite_receipts(game_session_id,command_id,character_id,world_id,event_year,window_generation,
  layer,placement,source_item,progress_after,before_revision,intent,contributed_at)
  VALUES(p_session,p_command,p_character,p_world,p_year,p.window_generation,p.layer,p_placement,p_item,p.layer_progress+1,p_before_revision,p_intent,sampled);
 UPDATE game_cake_eaters SET bites=bites+1,last_bite_at=sampled WHERE world_id=p_world AND event_year=p_year AND character_id=p_character;
 UPDATE game_cake_phases SET layer_progress=layer_progress+1,revision=revision+1 WHERE world_id=p_world AND event_year=p_year;
 RETURN 'prepared';
END $$;

-- One-minute existing world cadence performs settlement; no wall-clock-only
-- client request can claim that a layer disappeared. Returned phase determines
-- the normal fixed-map projection for each currently fenced Channel separately.
CREATE FUNCTION game_cake_settle_window(p_world UUID,p_year INTEGER,p_reset_epoch NUMERIC)
RETURNS TEXT LANGUAGE plpgsql AS $$
DECLARE p game_cake_phases; sampled TIMESTAMPTZ:=clock_timestamp();
BEGIN
 IF NOT EXISTS(SELECT 1 FROM game_world_reset_active WHERE world_id=p_world AND reset_epoch=p_reset_epoch)
 THEN RETURN 'stale'; END IF;
 SELECT * INTO p FROM game_cake_phases WHERE world_id=p_world AND event_year=p_year FOR UPDATE;
 IF NOT FOUND OR p.stage<>2 OR p.window_opened_at IS NULL THEN RETURN 'none'; END IF;
 IF p.layer_progress<1116 AND sampled<p.window_ends_at THEN RETURN 'none'; END IF;
 IF p.layer_progress<1116 THEN
  UPDATE game_cake_phases SET window_opened_at=NULL,window_ends_at=NULL,revision=revision+1
   WHERE world_id=p_world AND event_year=p_year;
  RETURN 'restore';
 END IF;
 IF p.layer<4 THEN
  UPDATE game_cake_phases SET layer=layer+1,window_opened_at=NULL,window_ends_at=NULL,revision=revision+1
   WHERE world_id=p_world AND event_year=p_year;
  RETURN 'remove_floor_'||(p.layer+3)::text;
 END IF;
 UPDATE game_cake_phases SET stage=3,window_opened_at=NULL,window_ends_at=NULL,
  pending_bonus=TRUE,completed_reset_epoch=p_reset_epoch,revision=revision+1
  WHERE world_id=p_world AND event_year=p_year;
 RETURN 'finished';
END $$;

-- Reset caller runs AFTER immutable reset activation. A strictly later epoch is
-- essential: completing a window during a reset cannot pay its own next-save debt.
CREATE FUNCTION game_cake_reset_continuation(p_world UUID,p_epoch NUMERIC)
RETURNS VOID LANGUAGE plpgsql AS $$
DECLARE activated TIMESTAMPTZ; event_open BOOLEAN;
BEGIN
 SELECT r.activated_at,extract(month FROM r.activated_at AT TIME ZONE 'Europe/Berlin')=2
  AND extract(day FROM r.activated_at AT TIME ZONE 'Europe/Berlin') BETWEEN 21 AND 26
  INTO activated,event_open FROM game_world_reset_active a JOIN game_world_reset_epochs r USING(world_id,reset_epoch)
  WHERE a.world_id=p_world AND a.reset_epoch=p_epoch FOR SHARE OF a;
 IF NOT FOUND THEN RAISE EXCEPTION 'Cake continuation requires current reset'; END IF;
 -- server_save activates pending completion independently of the date window.
 UPDATE game_cake_phases SET pending_bonus=FALSE,bonus_activated_at=activated,
  bonus_until=activated+interval '604800 seconds',brutus_present=TRUE,revision=revision+1
  WHERE world_id=p_world AND pending_bonus AND completed_reset_epoch<p_epoch;
 IF NOT event_open THEN
  UPDATE game_cake_phases SET brutus_present=FALSE,event_end_cleaned_up=TRUE,revision=revision+1 WHERE world_id=p_world AND NOT event_end_cleaned_up;
 END IF;
END $$;
REVOKE ALL ON FUNCTION game_cake_bite_prepare(UUID,INTEGER,UUID,NUMERIC,UUID,BYTEA,BIGINT,INTEGER,SMALLINT,BYTEA),
 game_cake_settle_window(UUID,INTEGER,NUMERIC),game_cake_reset_continuation(UUID,NUMERIC) FROM PUBLIC;
DO $$ DECLARE n TEXT; BEGIN
 FOREACH n IN ARRAY ARRAY[
  'game_cake_bite_prepare(UUID,INTEGER,UUID,NUMERIC,UUID,BYTEA,BIGINT,INTEGER,SMALLINT,BYTEA)',
  'game_cake_settle_window(UUID,INTEGER,NUMERIC)','game_cake_reset_continuation(UUID,NUMERIC)'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',n,current_schema());
 END LOOP;
END $$;

-- Character movement owner supplies a current exact actor/movement/reset witness;
-- this routine records an idempotent source step, not a client-selected city.
CREATE FUNCTION game_cake_errand_step(p_world UUID,p_year INTEGER,p_character UUID,p_actor UUID,
 p_movement NUMERIC,p_epoch NUMERIC,p_position BYTEA,p_source INTEGER)
RETURNS TEXT LANGUAGE plpgsql AS $$
DECLARE p game_cake_phases;e game_cake_errands;city SMALLINT;px INTEGER;py INTEGER;pz INTEGER;
BEGIN
 IF octet_length(p_position)<>12 THEN RETURN 'invalid'; END IF;
 -- Native position is three signed big-endian i32 values, constrained by the
 -- actual selected source placement before this private owner is called.
 px:=get_byte(p_position,0)*16777216+get_byte(p_position,1)*65536+get_byte(p_position,2)*256+get_byte(p_position,3);
 py:=get_byte(p_position,4)*16777216+get_byte(p_position,5)*65536+get_byte(p_position,6)*256+get_byte(p_position,7);
 pz:=get_byte(p_position,8)*16777216+get_byte(p_position,9)*65536+get_byte(p_position,10)*256+get_byte(p_position,11);
 IF NOT EXISTS(SELECT 1 FROM game_world_reset_active WHERE world_id=p_world AND reset_epoch=p_epoch) THEN RETURN 'stale'; END IF;
 IF EXISTS(SELECT 1 FROM game_cake_errand_steps WHERE actor_placement=p_actor AND movement_revision=p_movement AND reset_epoch=p_epoch
  AND (world_id,event_year,character_id,position,source_item) IS DISTINCT FROM(p_world,p_year,p_character,p_position,p_source)) THEN RETURN 'stale'; END IF;
 IF EXISTS(SELECT 1 FROM game_cake_errand_steps WHERE actor_placement=p_actor AND movement_revision=p_movement AND reset_epoch=p_epoch) THEN RETURN 'replay'; END IF;
 SELECT * INTO p FROM game_cake_phases WHERE world_id=p_world AND event_year=p_year FOR SHARE;
 IF NOT FOUND OR p.stage<>3 THEN RETURN 'inactive'; END IF;
 IF p_source BETWEEN 12090 AND 12095 THEN city:=NULL;
 ELSIF p_source=8886 AND pz=7 THEN
  city:=CASE WHEN px=32346 AND py IN(31822,31823) THEN 0
   WHEN px=32350 AND py IN(32275,32276) THEN 1
   WHEN px=33251 AND py IN(31802,31803) THEN 2
   WHEN px=33191 AND py IN(32397,32398) THEN 3
   WHEN px=32377 AND py IN(32801,32802) THEN 4 END;
  IF city IS NULL THEN RETURN 'invalid'; END IF;
 ELSE RETURN 'invalid'; END IF;
 INSERT INTO game_cake_errands(world_id,event_year,character_id) VALUES(p_world,p_year,p_character) ON CONFLICT DO NOTHING;
 SELECT * INTO STRICT e FROM game_cake_errands WHERE world_id=p_world AND event_year=p_year AND character_id=p_character FOR UPDATE;
 IF e.cities=31 THEN RETURN 'complete'; END IF;
 IF city IS NULL AND e.armed THEN RETURN 'armed'; END IF;
 IF city IS NOT NULL THEN
  IF NOT e.armed THEN RETURN 'unarmed'; END IF;
  IF (e.cities & (1<<city))<>0 THEN RETURN 'visited'; END IF;
 END IF;
 INSERT INTO game_cake_errand_steps(world_id,event_year,character_id,actor_placement,movement_revision,reset_epoch,position,city,source_item)
  VALUES(p_world,p_year,p_character,p_actor,p_movement,p_epoch,p_position,city,p_source);
 UPDATE game_cake_errands SET armed=city IS NULL,cities=CASE WHEN city IS NULL THEN cities ELSE cities | (1<<city) END
  WHERE world_id=p_world AND event_year=p_year AND character_id=p_character;
 RETURN CASE WHEN city IS NULL THEN 'armed' ELSE 'visited_'||city::text END;
END $$;
REVOKE ALL ON FUNCTION game_cake_errand_step(UUID,INTEGER,UUID,UUID,NUMERIC,NUMERIC,BYTEA,INTEGER) FROM PUBLIC;
DO $$ BEGIN EXECUTE format('ALTER FUNCTION game_cake_errand_step(UUID,INTEGER,UUID,UUID,NUMERIC,NUMERIC,BYTEA,INTEGER) SET search_path=pg_catalog,%I,pg_temp',current_schema()); END $$;

-- The existing audited reward MINT participant calls this AFTER inserting its
-- normal receipt, before commit. It changes no Item custody or audit envelope.
CREATE FUNCTION game_cake_reward_record(p_transaction UUID,p_world UUID,p_year INTEGER,p_character UUID,p_category SMALLINT)
RETURNS VOID LANGUAGE plpgsql AS $$
DECLARE e game_cake_errands; reward TEXT; claim TEXT;
BEGIN
 IF p_category NOT IN(1,2) THEN RAISE EXCEPTION 'invalid Brutus reward category'; END IF;
 SELECT * INTO e FROM game_cake_errands WHERE world_id=p_world AND event_year=p_year AND character_id=p_character FOR UPDATE;
 IF NOT FOUND OR e.cities<>31 OR (p_category=1 AND e.item_claimed) OR (p_category=2 AND e.cupcake_claimed)
  OR NOT EXISTS(SELECT 1 FROM game_cake_phases WHERE world_id=p_world AND event_year=p_year AND stage=3 AND brutus_present)
 THEN RAISE EXCEPTION 'Brutus annual reward requires completed current errand' USING ERRCODE='23514'; END IF;
 claim:=CASE p_category WHEN 1 THEN 'oteryn:reward-claim.cake-brutus-item' ELSE 'oteryn:reward-claim.cake-brutus-cupcake' END;
 SELECT i.definition_production_key INTO reward FROM game_reward_claim_mint_receipts r
  JOIN game_item_instances i ON i.item_instance_id=r.item_instance_id
  WHERE r.transaction_id=p_transaction AND r.created_xact_id=pg_current_xact_id() AND r.character_id=p_character
   AND r.claim_family='RewardClaim' AND r.claim_production_key=claim AND r.claim_revision_ref='oteryn:cake-brutus.v1' AND r.quantity=1
   AND i.world_id=p_world AND i.definition_family='Item' AND i.definition_revision_ref='definition-r1';
 IF reward IS NULL OR NOT ((p_category=1 AND reward IN('oteryn:item.tibia.i14756','oteryn:item.tibia.i20347','oteryn:item.tibia.i20350'))
  OR (p_category=2 AND reward IN('oteryn:item.tibia.i28484','oteryn:item.tibia.i28485','oteryn:item.tibia.i28486')))
 THEN RAISE EXCEPTION 'Brutus reward must be exact normal MINT receipt' USING ERRCODE='23514'; END IF;
 INSERT INTO game_cake_reward_claims(transaction_id,world_id,event_year,character_id,category,reward_key)
  VALUES(p_transaction,p_world,p_year,p_character,p_category,reward);
 UPDATE game_cake_errands SET item_claimed=item_claimed OR p_category=1,cupcake_claimed=cupcake_claimed OR p_category=2
  WHERE world_id=p_world AND event_year=p_year AND character_id=p_character;
END $$;
REVOKE ALL ON FUNCTION game_cake_reward_record(UUID,UUID,INTEGER,UUID,SMALLINT) FROM PUBLIC;
DO $$ BEGIN EXECUTE format('ALTER FUNCTION game_cake_reward_record(UUID,UUID,INTEGER,UUID,SMALLINT) SET search_path=pg_catalog,%I,pg_temp',current_schema()); END $$;

-- Cake uses the existing fixed-map terrain owner. Its provenance extends that
-- owner's rows; no parallel placement registry or synthetic ItemInstance exists.
ALTER TABLE game_terrain_tool_transforms ADD COLUMN cake_event_year INTEGER,
 ADD COLUMN cake_source_item INTEGER,
 ADD CONSTRAINT game_terrain_tool_cake_source CHECK(
  (cake_event_year IS NULL AND cake_source_item IS NULL) OR
  (cake_event_year IS NOT NULL AND cake_source_item IS NOT NULL AND cake_event_year BETWEEN 2000 AND 9999 AND cake_source_item IN(12085,12086,12147,12148,12152,12153,12154,12155)));
CREATE FUNCTION game_cake_project_channel(p_world UUID,p_channel UUID,p_bundle BYTEA,p_map TEXT,
 p_content BYTEA,p_epoch BIGINT,p_year INTEGER,p_sources JSONB)
RETURNS BOOLEAN LANGUAGE plpgsql AS $$
DECLARE p game_cake_phases;
BEGIN
 PERFORM pg_advisory_xact_lock_shared(hashtextextended('oteryn:content-activation:'||p_world::text||':'||p_channel::text,0));
 IF octet_length(p_bundle)<>32 OR octet_length(p_content)<>32
  OR jsonb_typeof(p_sources)<>'array' OR jsonb_array_length(p_sources)>8192
  OR NOT game_world_reset_map_epoch_current(p_world,p_epoch::numeric)
  OR NOT EXISTS(SELECT 1 FROM game_content_activations a WHERE a.world_id=p_world AND a.channel_id=p_channel
   AND a.activation_sequence=(SELECT max(z.activation_sequence) FROM game_content_activations z WHERE z.world_id=p_world AND z.channel_id=p_channel)
   AND a.server_artifact_digest=p_content)
  OR NOT EXISTS(SELECT 1 FROM game_world_reset_active a JOIN game_world_reset_epochs e USING(world_id,reset_epoch)
   WHERE a.world_id=p_world AND a.reset_epoch=p_epoch AND e.target_digest=p_bundle)
 THEN RETURN FALSE; END IF;
 SELECT * INTO p FROM game_cake_phases WHERE world_id=p_world AND event_year=p_year FOR SHARE;
 IF NOT FOUND THEN RETURN TRUE; END IF;
 PERFORM pg_advisory_xact_lock(hashtextextended('oteryn:terrain-scope:'||p_world::text||':'||p_channel::text,0));
 IF EXISTS(SELECT 1 FROM jsonb_array_elements(p_sources) s WHERE
  length(s->>'placement')<>80 OR decode(s->>'placement','hex')<>p_bundle||decode(s->>'spatial','hex')
  OR (s->>'source_item')::integer NOT IN(12085,12086,12147,12148,12152,12153,12154,12155)
  OR ((s->>'source_item')::integer IN(12085,12086) AND (s->>'z')::integer<>7)
  OR ((s->>'source_item')::integer NOT IN(12085,12086) AND NOT
   ((s->>'x')::integer BETWEEN 33287 AND 33330 AND (s->>'y')::integer BETWEEN 32065 AND 32103 AND (s->>'z')::integer BETWEEN 4 AND 6)))
 THEN RETURN FALSE; END IF;
 IF (SELECT count(DISTINCT s->>'placement') FROM jsonb_array_elements(p_sources) s)<>jsonb_array_length(p_sources)
 THEN RETURN FALSE; END IF;
 -- Refuse an unrelated predecessor, including a stale reset/content generation.
 IF EXISTS(SELECT 1 FROM jsonb_array_elements(p_sources) s JOIN game_terrain_tool_transforms t
  ON t.world_id=p_world AND t.channel_id=p_channel AND t.reset_epoch=p_epoch AND t.placement=decode(s->>'placement','hex')
  WHERE t.map_revision<>p_map OR t.content_generation<>p_content
   OR (t.cake_event_year IS NOT NULL AND (t.cake_event_year<>p_year OR t.cake_source_item<>(s->>'source_item')::integer))
   OR (t.cake_event_year IS NULL AND t.state_item_id<>(s->>'source_item')::integer))
 THEN RETURN FALSE; END IF;
 WITH source AS(SELECT decode(s->>'placement','hex') placement,decode(s->>'spatial','hex') spatial,
  (s->>'source_item')::integer item,(s->>'z')::integer z FROM jsonb_array_elements(p_sources) s),
 desired AS(SELECT source.*,CASE
  WHEN item NOT IN(12085,12086) AND z<p.layer+3 THEN 0
  WHEN item IN(12085,12086) AND (p.window_opened_at IS NOT NULL OR p.stage=3)
   AND EXISTS(SELECT 1 FROM game_cake_bite_receipts r WHERE r.world_id=p_world AND r.event_year=p_year
    AND r.window_generation=p.window_generation AND substring(r.placement FROM 33 FOR 8)=source.spatial) THEN 0
  ELSE item END state FROM source)
 INSERT INTO game_terrain_tool_transforms(world_id,channel_id,placement,reset_epoch,map_revision,content_generation,
  state_item_id,object_revision,decay_states,decay_delays,decay_step,decay_at_unix_ms,cake_event_year,cake_source_item)
 SELECT p_world,p_channel,placement,p_epoch,p_map,p_content,state,1,'{}','{}',0,NULL,p_year,item FROM desired
 ON CONFLICT(world_id,channel_id,placement,reset_epoch) DO UPDATE SET
  state_item_id=EXCLUDED.state_item_id,
  object_revision=game_terrain_tool_transforms.object_revision+CASE WHEN game_terrain_tool_transforms.state_item_id<>EXCLUDED.state_item_id THEN 1 ELSE 0 END,
  cake_event_year=EXCLUDED.cake_event_year,cake_source_item=EXCLUDED.cake_source_item
 WHERE game_terrain_tool_transforms.state_item_id<>EXCLUDED.state_item_id OR game_terrain_tool_transforms.cake_event_year IS NULL;
 RETURN TRUE;
END $$;
REVOKE ALL ON FUNCTION game_cake_project_channel(UUID,UUID,BYTEA,TEXT,BYTEA,BIGINT,INTEGER,JSONB) FROM PUBLIC;
DO $$ BEGIN EXECUTE format('ALTER FUNCTION game_cake_project_channel(UUID,UUID,BYTEA,TEXT,BYTEA,BIGINT,INTEGER,JSONB) SET search_path=pg_catalog,%I,pg_temp',current_schema()); END $$;

-- Existing typed world/item owners hold the complete current fence before
-- invoking these closed participants, as for game_apply_terrain_tool_transform.
-- Direct phase table writes remain unavailable to runtime and control roles.
GRANT SELECT ON game_cake_phases,game_cake_world_progress,game_cake_bite_receipts TO oteryn_game_runtime;
DO $$ DECLARE n TEXT;BEGIN
 FOREACH n IN ARRAY ARRAY[
  'game_cake_enter_stage_two()',
  'game_cake_bite_prepare(UUID,INTEGER,UUID,NUMERIC,UUID,BYTEA,BIGINT,INTEGER,SMALLINT,BYTEA)',
  'game_cake_settle_window(UUID,INTEGER,NUMERIC)','game_cake_reset_continuation(UUID,NUMERIC)',
  'game_cake_errand_step(UUID,INTEGER,UUID,UUID,NUMERIC,NUMERIC,BYTEA,INTEGER)',
  'game_cake_reward_record(UUID,UUID,INTEGER,UUID,SMALLINT)',
  'game_cake_project_channel(UUID,UUID,BYTEA,TEXT,BYTEA,BIGINT,INTEGER,JSONB)'] LOOP
  EXECUTE format('ALTER FUNCTION %s SECURITY DEFINER',n);
  EXECUTE format('GRANT EXECUTE ON FUNCTION %s TO oteryn_game_runtime',n);
 END LOOP;
END $$;

-- A committed phase count is exactly its immutable world contributions, never
-- a caller's counter. Deferral permits the owner to insert the receipt and then
-- update its phase in either statement order within one physical transaction.
CREATE FUNCTION game_cake_phase_progress_proven() RETURNS TRIGGER LANGUAGE plpgsql AS $$
DECLARE p game_cake_phases;total BIGINT;
BEGIN
 SELECT * INTO p FROM game_cake_phases WHERE world_id=NEW.world_id AND event_year=NEW.event_year;
 SELECT count(*) INTO total FROM game_cake_bite_receipts WHERE world_id=p.world_id AND event_year=p.event_year AND window_generation=p.window_generation;
 IF p.layer_progress<>total THEN RAISE EXCEPTION 'Cake phase requires exact immutable bite receipts' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_cake_phase_progress_proven AFTER INSERT OR UPDATE ON game_cake_phases
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_cake_phase_progress_proven();
REVOKE ALL ON FUNCTION game_cake_phase_progress_proven() FROM PUBLIC;
DO $$ BEGIN EXECUTE format('ALTER FUNCTION game_cake_phase_progress_proven() SET search_path=pg_catalog,%I,pg_temp',current_schema()); END $$;

CREATE FUNCTION game_cake_lock_entitlement(p_world UUID,p_year INTEGER,p_character UUID,p_category SMALLINT)
RETURNS BOOLEAN LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE p game_cake_phases;e game_cake_errands;
BEGIN
 IF p_category NOT IN(1,2) THEN RETURN FALSE; END IF;
 SELECT * INTO p FROM game_cake_phases WHERE world_id=p_world AND event_year=p_year FOR UPDATE;
 IF NOT FOUND OR p.stage<>3 OR NOT p.brutus_present OR p.event_year IS DISTINCT FROM
  (SELECT max(event_year) FROM game_cake_world_progress WHERE world_id=p_world) THEN RETURN FALSE; END IF;
 SELECT * INTO e FROM game_cake_errands WHERE world_id=p_world AND event_year=p_year AND character_id=p_character FOR UPDATE;
 RETURN FOUND AND e.cities=31 AND CASE p_category WHEN 1 THEN NOT e.item_claimed ELSE NOT e.cupcake_claimed END;
END $$;
REVOKE ALL ON FUNCTION game_cake_lock_entitlement(UUID,INTEGER,UUID,SMALLINT) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_cake_lock_entitlement(UUID,INTEGER,UUID,SMALLINT) TO oteryn_game_runtime;
DO $$ BEGIN EXECUTE format('ALTER FUNCTION game_cake_lock_entitlement(UUID,INTEGER,UUID,SMALLINT) SET search_path=pg_catalog,%I,pg_temp',current_schema()); END $$;

-- Preserve the ordinary1024 terrain-transform budget. Source-qualified Cake
-- rows have their own8192 bound in the current Channel phase projector.
DO $migration$
DECLARE body TEXT;before TEXT;after TEXT;
BEGIN
 body:=pg_get_functiondef('game_apply_terrain_tool_transform(UUID,NUMERIC,UUID,UUID,BYTEA,UUID,UUID,BYTEA,TEXT,BYTEA,BIGINT,BIGINT,BIGINT,BIGINT,BIGINT[],BIGINT[])'::regprocedure);
 before:=$old$WHERE world_id=p_world_id AND channel_id=p_channel_id AND map_revision=p_map_revision
                AND reset_epoch=p_reset_epoch LIMIT 1024$old$;
 after:=$new$WHERE world_id=p_world_id AND channel_id=p_channel_id AND map_revision=p_map_revision
                AND reset_epoch=p_reset_epoch AND cake_event_year IS NULL LIMIT 1024$new$;
 IF strpos(body,before)=0 THEN RAISE EXCEPTION 'unexpected Cake terrain budget predecessor'; END IF;
 EXECUTE replace(body,before,after);
END $migration$;
-- These participants become callable only with their next coherent current-
-- Character movement / native NPC MINT owners; schema alone grants no authority.
REVOKE ALL ON FUNCTION game_cake_errand_step(UUID,INTEGER,UUID,UUID,NUMERIC,NUMERIC,BYTEA,INTEGER),
 game_cake_reward_record(UUID,UUID,INTEGER,UUID,SMALLINT),
 game_cake_lock_entitlement(UUID,INTEGER,UUID,SMALLINT) FROM oteryn_game_runtime;
