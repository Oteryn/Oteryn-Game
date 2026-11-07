-- Voice joins the same shared counters; its per-character eligibility belongs to
-- the encounter deadline, independently of the spawn cohort's wave generation.
ALTER TABLE game_world_change_counters DROP CONSTRAINT game_world_change_counters_change_key_check;
ALTER TABLE game_world_change_counters ADD CONSTRAINT game_world_change_counters_change_key_check
 CHECK(change_key IN ('deeplings','demon_wars','fire_from_the_earth','hive_born','overhunting','the_fire_feathered_serpent','the_mage_s_tower','their_masters_voice'));
ALTER TABLE game_world_change_personal_counters ADD COLUMN period_deadline_ms BIGINT NOT NULL DEFAULT 0 CHECK(period_deadline_ms>=0);
INSERT INTO game_world_change_counters(world_id,change_key,reset_epoch,state)
 SELECT world_id,'their_masters_voice',reset_epoch,
 jsonb_build_object('epoch',reset_epoch,'entered_epoch',reset_epoch,'stage',1,'phase_version',1,'credit_generation',1,'revision',1,'counts',jsonb_build_array(0,0,0),'condition',false,'deadline_ms',NULL)
 FROM game_world_reset_active
 ON CONFLICT DO NOTHING;
