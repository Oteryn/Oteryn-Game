-- Rathleton ballots reference the same atomic custody debit/contribution receipt.
CREATE TABLE game_world_change_ballots (
 world_id UUID NOT NULL,
 reset_epoch NUMERIC(20,0) NOT NULL,
 operation_id BYTEA NOT NULL CHECK(octet_length(operation_id)=16),
 election_period BIGINT NOT NULL,
 ballot TEXT NOT NULL CHECK(ballot IN ('shortcut/minotaur','shortcut/fungi','shortcut/catacomb',
  'dungeon/golems','dungeon/minotaurs','dungeon/wrathofevil','boss/bullwark','boss/lisa',
  'boss/gloothfairy','raid/minotaurs','raid/mechanical','raid/wildlife')),
 amount BIGINT NOT NULL CHECK(amount=1),
 PRIMARY KEY(world_id,reset_epoch,operation_id),
 FOREIGN KEY(world_id,reset_epoch,operation_id)
  REFERENCES game_world_change_debits(world_id,reset_epoch,operation_id)
  DEFERRABLE INITIALLY DEFERRED
);
CREATE TABLE game_world_change_ballot_tallies (
 world_id UUID NOT NULL,
 election_period BIGINT NOT NULL,
 ballot TEXT NOT NULL,
 tally NUMERIC(20,0) NOT NULL CHECK(tally BETWEEN 1 AND 18446744073709551615),
 PRIMARY KEY(world_id,election_period,ballot)
);
CREATE TABLE game_world_change_elections (
 world_id UUID NOT NULL,
 election_period BIGINT NOT NULL,
 reset_epoch NUMERIC(20,0) NOT NULL,
 winners JSONB NOT NULL CHECK(jsonb_typeof(winners)='object' AND octet_length(winners::text)<=4096),
 PRIMARY KEY(world_id,election_period),
 FOREIGN KEY(world_id,reset_epoch) REFERENCES game_world_change_days(world_id,reset_epoch)
);
CREATE INDEX game_world_change_elections_reset ON game_world_change_elections(world_id,reset_epoch);
CREATE FUNCTION game_world_change_ballot_admit() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE epoch_ms BIGINT; expected_period BIGINT;
BEGIN
 -- The active epoch, not UTC wall midnight, owns the election day. Sunday
 -- remains in the old period until Monday's actual server save activates.
 SELECT floor(extract(epoch FROM e.activated_at)*1000)::bigint INTO epoch_ms
  FROM game_world_reset_epochs e JOIN game_world_reset_active a USING(world_id,reset_epoch)
  WHERE e.world_id=NEW.world_id AND e.reset_epoch=NEW.reset_epoch;
 IF NOT FOUND THEN
  RAISE EXCEPTION 'ballot requires current active World epoch' USING ERRCODE='23514';
 END IF;
 expected_period:=floor((epoch_ms::numeric-946857600000)/1209600000)::bigint;
 IF NEW.election_period<>expected_period OR NOT EXISTS(
   SELECT 1 FROM game_world_change_debits d JOIN game_world_change_contributions c
    USING(world_id,reset_epoch,operation_id)
   JOIN game_character_quest_receipts r ON r.character_id=d.character_id
    AND r.cause_id=d.game_session_id AND r.cause_ordinal=d.cause_ordinal
    AND r.cause_kind='command' AND r.transition_key=d.transition_key
   WHERE (d.world_id,d.reset_epoch,d.operation_id)=(NEW.world_id,NEW.reset_epoch,NEW.operation_id)
    AND d.created_xact_id=pg_current_xact_id() AND c.created_xact_id=pg_current_xact_id()
    AND r.created_xact_id=pg_current_xact_id()
    AND c.resource='votes' AND c.amount=NEW.amount AND d.amount=NEW.amount
    AND d.points_track='oteryn:quest-progress/quest/u10_50/oramond_quest/voting_points'
    AND d.transition_key='oteryn:quest-transition/world-contribution/votes/'||NEW.ballot
    AND r.values_before[array_position(r.track_keys,'oteryn:quest-progress/quest/u10_50/oramond_quest/citizen_rank')]
      >=CASE split_part(NEW.ballot,'/',1) WHEN 'dungeon' THEN 1 WHEN 'boss' THEN 2 ELSE 0 END
    AND r.values_after[array_position(r.track_keys,'oteryn:quest-progress/quest/u10_50/oramond_quest/citizen_rank')]
      =r.values_before[array_position(r.track_keys,'oteryn:quest-progress/quest/u10_50/oramond_quest/citizen_rank')]
    AND r.values_after[array_position(r.track_keys,'oteryn:quest-progress/quest/u10_50/oramond_quest/votes_spent')]
      =r.values_before[array_position(r.track_keys,'oteryn:quest-progress/quest/u10_50/oramond_quest/votes_spent')]+1
 ) THEN RAISE EXCEPTION 'ballot lacks same atomic voting-right debit' USING ERRCODE='23514'; END IF;
 INSERT INTO game_world_change_ballot_tallies(world_id,election_period,ballot,tally)
  VALUES(NEW.world_id,NEW.election_period,NEW.ballot,NEW.amount)
  ON CONFLICT(world_id,election_period,ballot) DO UPDATE
   SET tally=game_world_change_ballot_tallies.tally+EXCLUDED.tally;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_world_change_ballot_admit AFTER INSERT ON game_world_change_ballots
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_world_change_ballot_admit();
CREATE TRIGGER game_world_change_ballot_immutable BEFORE UPDATE OR DELETE ON game_world_change_ballots
 FOR EACH ROW EXECUTE FUNCTION game_world_reset_history_immutable();
CREATE TRIGGER game_world_change_election_immutable BEFORE UPDATE OR DELETE ON game_world_change_elections
 FOR EACH ROW EXECUTE FUNCTION game_world_reset_history_immutable();
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_world_change_ballot_admit() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON game_world_change_ballots,game_world_change_ballot_tallies,game_world_change_elections FROM PUBLIC;
REVOKE ALL ON FUNCTION game_world_change_ballot_admit() FROM PUBLIC;
GRANT SELECT,INSERT ON game_world_change_ballots,game_world_change_elections TO oteryn_game_runtime;
GRANT SELECT ON game_world_change_ballots,game_world_change_ballot_tallies,game_world_change_elections TO oteryn_game_control;
GRANT SELECT ON game_world_change_ballot_tallies TO oteryn_game_runtime;

-- A voting-right debit cannot be committed without its exact ballot identity.
CREATE FUNCTION game_world_change_ballot_complete() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.points_track='oteryn:quest-progress/quest/u10_50/oramond_quest/voting_points'
   AND NOT EXISTS(SELECT 1 FROM game_world_change_ballots b
    WHERE (b.world_id,b.reset_epoch,b.operation_id)=(NEW.world_id,NEW.reset_epoch,NEW.operation_id)
      AND b.amount=NEW.amount
      AND NEW.transition_key='oteryn:quest-transition/world-contribution/votes/'||b.ballot)
 THEN RAISE EXCEPTION 'voting-right debit requires exact ballot receipt' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_world_change_ballot_complete AFTER INSERT ON game_world_change_debits
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_world_change_ballot_complete();
DO $$ DECLARE n TEXT; BEGIN
 EXECUTE format('ALTER FUNCTION game_world_change_ballot_complete() SET search_path=pg_catalog,%I,pg_temp',current_schema());
 FOREACH n IN ARRAY ARRAY['game_world_change_ballots','game_world_change_ballot_tallies','game_world_change_elections'] LOOP
  EXECUTE format('CREATE TRIGGER %I BEFORE TRUNCATE ON %I FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate()',n||'_no_truncate',n);
 END LOOP;
END $$;
REVOKE ALL ON FUNCTION game_world_change_ballot_complete() FROM PUBLIC;
