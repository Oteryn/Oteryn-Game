-- P08 perk shaping (mechanic 0495), PROFICIENCY-1C amendment of PROFICIENCY-1B. Forward only:
-- 0055 and 0060 are never edited.
--   * Rank model 0..10 (1C §4, `PROF1-RL-02`): MODIFY lands on rank 0 and ten steps reach 10.
--   * Orb BURN (1C §5): ORB_RANK burns one Lunar Ascension Orb (`oteryn:item.tibia.i53695`)
--     through the item-use record (cause 495, shape 1) whose TransactionId is the modification's
--     occurrence. 0060's `orb_cost = 0` pin is lifted; a deferred guard pairs the burn and its
--     single ORB_RANK line in one transaction, from either side.
--   * The draw seed's durable secret (SIM-DETERMINISM-01 §10): created once, never reseeded.
--   * Activation of `shaping-r1` for every shaped definition of
--     `rulesets/progression/weapon-proficiency/shaping.json`, through the §8 activation function,
--     so no manual control-plane step is needed. A later revision activates the same way.
-- Rollback: a successor stops routing the commands; lines, burns and activations are retained.

ALTER TABLE game_character_proficiency_modifications
    DROP CONSTRAINT game_character_proficiency_modifications_rank_check,
    ADD CONSTRAINT game_character_proficiency_modifications_rank_check
        CHECK (rank BETWEEN 0 AND 10);

ALTER TABLE game_character_proficiency_modification_lines
    DROP CONSTRAINT game_character_proficiency_modification_lines_rank_before_check,
    ADD CONSTRAINT game_character_proficiency_modification_lines_rank_before_check
        CHECK (rank_before BETWEEN 0 AND 10),
    DROP CONSTRAINT game_character_proficiency_modification_lines_rank_after_check,
    ADD CONSTRAINT game_character_proficiency_modification_lines_rank_after_check
        CHECK (rank_after BETWEEN 0 AND 10),
    -- 0055 §7.1 with MODIFY landing on rank 0; every other arm is unchanged.
    DROP CONSTRAINT game_character_proficiency_modification_line_direction,
    ADD CONSTRAINT game_character_proficiency_modification_line_direction CHECK (CASE operation
        WHEN 'MODIFY' THEN level_before IS NULL AND level_after IS NOT NULL AND rank_after = 0
            AND pending_offer_after IS NULL AND shaping_revision_after = bound_shaping_revision
        WHEN 'RANK_UP' THEN level_before IS NOT NULL AND pending_offer_before IS NULL
            AND pending_offer_after IS NULL AND rank_after = rank_before + 1
            AND (level_after, shaping_key_after, shaping_revision_after, entry_index_after)
                IS NOT DISTINCT FROM
                (level_before, shaping_key_before, shaping_revision_before, entry_index_before)
        WHEN 'ORB_RANK' THEN level_before IS NOT NULL AND pending_offer_before IS NULL
            AND pending_offer_after IS NULL AND rank_before < 10 AND rank_after = 10
            AND (level_after, shaping_key_after, shaping_revision_after, entry_index_after)
                IS NOT DISTINCT FROM
                (level_before, shaping_key_before, shaping_revision_before, entry_index_before)
        WHEN 'RESHAPE_OFFER' THEN level_before IS NOT NULL AND pending_offer_before IS NULL
            AND pending_offer_after IS NOT NULL
            AND (level_after, shaping_key_after, shaping_revision_after, entry_index_after,
                 rank_after) IS NOT DISTINCT FROM
                (level_before, shaping_key_before, shaping_revision_before, entry_index_before,
                 rank_before)
        WHEN 'RESHAPE_CHOOSE' THEN pending_offer_before IS NOT NULL AND pending_offer_after IS NULL
            AND entry_index_after = ANY(pending_offer_before)
            AND (level_after, shaping_key_after, shaping_revision_after, rank_after)
                IS NOT DISTINCT FROM
                (level_before, shaping_key_before, shaping_revision_before, rank_before)
        WHEN 'RESHAPE_DECLINE' THEN pending_offer_before IS NOT NULL AND pending_offer_after IS NULL
            AND (level_after, shaping_key_after, shaping_revision_after, entry_index_after,
                 rank_after) IS NOT DISTINCT FROM
                (level_before, shaping_key_before, shaping_revision_before, entry_index_before,
                 rank_before)
        WHEN 'CLEAR' THEN level_before IS NOT NULL AND level_after IS NULL
        WHEN 'MIGRATION_CLEAR' THEN level_before IS NOT NULL AND level_after IS NULL
        ELSE FALSE
    END),
    DROP CONSTRAINT game_character_proficiency_modification_line_no_orb_burn,
    -- With 0055's cost check (only ORB_RANK may burn), an orb is burnt exactly by ORB_RANK.
    ADD CONSTRAINT game_character_proficiency_modification_line_orb_rank_burns
        CHECK ((operation = 'ORB_RANK') = (orb_cost = 1));

ALTER TABLE game_item_use_records
 DROP CONSTRAINT game_item_use_records_cause_kind_check,
 ADD CONSTRAINT game_item_use_records_cause_kind_check CHECK(cause_kind IN (1,2,3,328,494,495)),
 DROP CONSTRAINT game_item_use_cause_shape,
 ADD CONSTRAINT game_item_use_cause_shape
 CHECK((cause_kind IN (1,328,494,495) AND shape=1) OR cause_kind=2 OR (cause_kind=3 AND shape IN (3,4)));

CREATE FUNCTION game_proficiency_orb_burn_consistent() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
DECLARE v_id UUID;
BEGIN
 IF TG_TABLE_NAME='game_item_use_records' THEN v_id:=NEW.transaction_id;
 ELSE v_id:=NEW.proficiency_occurrence_id; END IF;
 IF NOT EXISTS (
   SELECT 1 FROM public.game_item_use_records u
   JOIN public.game_character_proficiency_modification_lines l
     ON l.proficiency_occurrence_id=u.transaction_id AND l.character_id=u.character_id
   WHERE u.transaction_id=v_id AND u.cause_kind=495 AND u.shape=1
    AND u.source_definition_production_key='oteryn:item.tibia.i53695'
    AND u.created_xact_id=pg_current_xact_id()
    AND l.operation='ORB_RANK' AND l.orb_cost=1 AND l.orbs_spent=1)
  OR (SELECT count(*) FROM public.game_character_proficiency_modification_lines
       WHERE proficiency_occurrence_id=v_id)<>1
  OR (SELECT count(*) FROM public.game_item_use_records WHERE transaction_id=v_id)<>1
  OR EXISTS (SELECT 1 FROM public.game_character_proficiency_receipt_lines
              WHERE proficiency_occurrence_id=v_id AND cause<>'perk_modification') THEN
   RAISE EXCEPTION 'orb burn and ORB_RANK line differ' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_proficiency_orb_burn_line AFTER INSERT ON game_item_use_records
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW WHEN(NEW.cause_kind=495)
 EXECUTE FUNCTION game_proficiency_orb_burn_consistent();
CREATE CONSTRAINT TRIGGER game_proficiency_orb_line_burn
 AFTER INSERT ON game_character_proficiency_modification_lines
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW WHEN(NEW.operation='ORB_RANK')
 EXECUTE FUNCTION game_proficiency_orb_burn_consistent();
REVOKE ALL ON FUNCTION game_proficiency_orb_burn_consistent() FROM PUBLIC;

-- SIM-DETERMINISM-01 §10: the draw seed's server secret, created once and never reseeded.
CREATE TABLE game_proficiency_shaping_decision_root (
    singleton BOOLEAN PRIMARY KEY CHECK (singleton),
    secret BYTEA NOT NULL CHECK (octet_length(secret) = 32)
);
INSERT INTO game_proficiency_shaping_decision_root
SELECT TRUE, sha256(uuid_send(gen_random_uuid()) || uuid_send(gen_random_uuid()))
ON CONFLICT DO NOTHING;
CREATE TRIGGER game_proficiency_shaping_decision_root_immutable BEFORE UPDATE OR DELETE
    ON game_proficiency_shaping_decision_root FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
CREATE TRIGGER game_proficiency_shaping_decision_root_no_truncate BEFORE TRUNCATE
    ON game_proficiency_shaping_decision_root EXECUTE FUNCTION game_character_reject_truncate();
REVOKE ALL ON game_proficiency_shaping_decision_root FROM PUBLIC;
GRANT SELECT ON game_proficiency_shaping_decision_root TO oteryn_game_runtime;

-- PROFICIENCY-1C §2: `shaping-r1` is the active revision of every shaped definition. The key
-- list equals shaping.json's `definition_vocations` (checked by a durability unit test).
SELECT game_proficiency_shaping_activate(shaping_key, 'shaping-r1', NULL)
  FROM unnest(ARRAY[
    'oteryn:proficiency-shaping.tibia.p10', 'oteryn:proficiency-shaping.tibia.p100',
    'oteryn:proficiency-shaping.tibia.p101', 'oteryn:proficiency-shaping.tibia.p102',
    'oteryn:proficiency-shaping.tibia.p103', 'oteryn:proficiency-shaping.tibia.p104',
    'oteryn:proficiency-shaping.tibia.p105', 'oteryn:proficiency-shaping.tibia.p106',
    'oteryn:proficiency-shaping.tibia.p107', 'oteryn:proficiency-shaping.tibia.p108',
    'oteryn:proficiency-shaping.tibia.p109', 'oteryn:proficiency-shaping.tibia.p11',
    'oteryn:proficiency-shaping.tibia.p110', 'oteryn:proficiency-shaping.tibia.p111',
    'oteryn:proficiency-shaping.tibia.p112', 'oteryn:proficiency-shaping.tibia.p113',
    'oteryn:proficiency-shaping.tibia.p114', 'oteryn:proficiency-shaping.tibia.p115',
    'oteryn:proficiency-shaping.tibia.p117', 'oteryn:proficiency-shaping.tibia.p118',
    'oteryn:proficiency-shaping.tibia.p119', 'oteryn:proficiency-shaping.tibia.p12',
    'oteryn:proficiency-shaping.tibia.p120', 'oteryn:proficiency-shaping.tibia.p121',
    'oteryn:proficiency-shaping.tibia.p122', 'oteryn:proficiency-shaping.tibia.p123',
    'oteryn:proficiency-shaping.tibia.p124', 'oteryn:proficiency-shaping.tibia.p125',
    'oteryn:proficiency-shaping.tibia.p126', 'oteryn:proficiency-shaping.tibia.p127',
    'oteryn:proficiency-shaping.tibia.p128', 'oteryn:proficiency-shaping.tibia.p129',
    'oteryn:proficiency-shaping.tibia.p13', 'oteryn:proficiency-shaping.tibia.p130',
    'oteryn:proficiency-shaping.tibia.p131', 'oteryn:proficiency-shaping.tibia.p132',
    'oteryn:proficiency-shaping.tibia.p133', 'oteryn:proficiency-shaping.tibia.p134',
    'oteryn:proficiency-shaping.tibia.p135', 'oteryn:proficiency-shaping.tibia.p136',
    'oteryn:proficiency-shaping.tibia.p137', 'oteryn:proficiency-shaping.tibia.p138',
    'oteryn:proficiency-shaping.tibia.p139', 'oteryn:proficiency-shaping.tibia.p14',
    'oteryn:proficiency-shaping.tibia.p140', 'oteryn:proficiency-shaping.tibia.p141',
    'oteryn:proficiency-shaping.tibia.p142', 'oteryn:proficiency-shaping.tibia.p143',
    'oteryn:proficiency-shaping.tibia.p144', 'oteryn:proficiency-shaping.tibia.p145',
    'oteryn:proficiency-shaping.tibia.p146', 'oteryn:proficiency-shaping.tibia.p147',
    'oteryn:proficiency-shaping.tibia.p148', 'oteryn:proficiency-shaping.tibia.p149',
    'oteryn:proficiency-shaping.tibia.p15', 'oteryn:proficiency-shaping.tibia.p150',
    'oteryn:proficiency-shaping.tibia.p151', 'oteryn:proficiency-shaping.tibia.p152',
    'oteryn:proficiency-shaping.tibia.p153', 'oteryn:proficiency-shaping.tibia.p154',
    'oteryn:proficiency-shaping.tibia.p155', 'oteryn:proficiency-shaping.tibia.p156',
    'oteryn:proficiency-shaping.tibia.p157', 'oteryn:proficiency-shaping.tibia.p158',
    'oteryn:proficiency-shaping.tibia.p159', 'oteryn:proficiency-shaping.tibia.p16',
    'oteryn:proficiency-shaping.tibia.p161', 'oteryn:proficiency-shaping.tibia.p163',
    'oteryn:proficiency-shaping.tibia.p164', 'oteryn:proficiency-shaping.tibia.p165',
    'oteryn:proficiency-shaping.tibia.p166', 'oteryn:proficiency-shaping.tibia.p17',
    'oteryn:proficiency-shaping.tibia.p178', 'oteryn:proficiency-shaping.tibia.p179',
    'oteryn:proficiency-shaping.tibia.p18', 'oteryn:proficiency-shaping.tibia.p180',
    'oteryn:proficiency-shaping.tibia.p181', 'oteryn:proficiency-shaping.tibia.p19',
    'oteryn:proficiency-shaping.tibia.p193', 'oteryn:proficiency-shaping.tibia.p194',
    'oteryn:proficiency-shaping.tibia.p195', 'oteryn:proficiency-shaping.tibia.p196',
    'oteryn:proficiency-shaping.tibia.p20', 'oteryn:proficiency-shaping.tibia.p207',
    'oteryn:proficiency-shaping.tibia.p208', 'oteryn:proficiency-shaping.tibia.p209',
    'oteryn:proficiency-shaping.tibia.p21', 'oteryn:proficiency-shaping.tibia.p210',
    'oteryn:proficiency-shaping.tibia.p211', 'oteryn:proficiency-shaping.tibia.p212',
    'oteryn:proficiency-shaping.tibia.p213', 'oteryn:proficiency-shaping.tibia.p214',
    'oteryn:proficiency-shaping.tibia.p215', 'oteryn:proficiency-shaping.tibia.p216',
    'oteryn:proficiency-shaping.tibia.p218', 'oteryn:proficiency-shaping.tibia.p219',
    'oteryn:proficiency-shaping.tibia.p22', 'oteryn:proficiency-shaping.tibia.p223',
    'oteryn:proficiency-shaping.tibia.p224', 'oteryn:proficiency-shaping.tibia.p225',
    'oteryn:proficiency-shaping.tibia.p226', 'oteryn:proficiency-shaping.tibia.p227',
    'oteryn:proficiency-shaping.tibia.p228', 'oteryn:proficiency-shaping.tibia.p229',
    'oteryn:proficiency-shaping.tibia.p23', 'oteryn:proficiency-shaping.tibia.p231',
    'oteryn:proficiency-shaping.tibia.p232', 'oteryn:proficiency-shaping.tibia.p233',
    'oteryn:proficiency-shaping.tibia.p234', 'oteryn:proficiency-shaping.tibia.p235',
    'oteryn:proficiency-shaping.tibia.p236', 'oteryn:proficiency-shaping.tibia.p237',
    'oteryn:proficiency-shaping.tibia.p238', 'oteryn:proficiency-shaping.tibia.p239',
    'oteryn:proficiency-shaping.tibia.p24', 'oteryn:proficiency-shaping.tibia.p240',
    'oteryn:proficiency-shaping.tibia.p241', 'oteryn:proficiency-shaping.tibia.p242',
    'oteryn:proficiency-shaping.tibia.p243', 'oteryn:proficiency-shaping.tibia.p244',
    'oteryn:proficiency-shaping.tibia.p245', 'oteryn:proficiency-shaping.tibia.p246',
    'oteryn:proficiency-shaping.tibia.p247', 'oteryn:proficiency-shaping.tibia.p248',
    'oteryn:proficiency-shaping.tibia.p249', 'oteryn:proficiency-shaping.tibia.p25',
    'oteryn:proficiency-shaping.tibia.p250', 'oteryn:proficiency-shaping.tibia.p251',
    'oteryn:proficiency-shaping.tibia.p252', 'oteryn:proficiency-shaping.tibia.p253',
    'oteryn:proficiency-shaping.tibia.p254', 'oteryn:proficiency-shaping.tibia.p255',
    'oteryn:proficiency-shaping.tibia.p256', 'oteryn:proficiency-shaping.tibia.p257',
    'oteryn:proficiency-shaping.tibia.p258', 'oteryn:proficiency-shaping.tibia.p259',
    'oteryn:proficiency-shaping.tibia.p26', 'oteryn:proficiency-shaping.tibia.p260',
    'oteryn:proficiency-shaping.tibia.p261', 'oteryn:proficiency-shaping.tibia.p262',
    'oteryn:proficiency-shaping.tibia.p263', 'oteryn:proficiency-shaping.tibia.p264',
    'oteryn:proficiency-shaping.tibia.p265', 'oteryn:proficiency-shaping.tibia.p266',
    'oteryn:proficiency-shaping.tibia.p268', 'oteryn:proficiency-shaping.tibia.p269',
    'oteryn:proficiency-shaping.tibia.p27', 'oteryn:proficiency-shaping.tibia.p270',
    'oteryn:proficiency-shaping.tibia.p271', 'oteryn:proficiency-shaping.tibia.p272',
    'oteryn:proficiency-shaping.tibia.p273', 'oteryn:proficiency-shaping.tibia.p274',
    'oteryn:proficiency-shaping.tibia.p275', 'oteryn:proficiency-shaping.tibia.p28',
    'oteryn:proficiency-shaping.tibia.p283', 'oteryn:proficiency-shaping.tibia.p284',
    'oteryn:proficiency-shaping.tibia.p285', 'oteryn:proficiency-shaping.tibia.p286',
    'oteryn:proficiency-shaping.tibia.p287', 'oteryn:proficiency-shaping.tibia.p288',
    'oteryn:proficiency-shaping.tibia.p289', 'oteryn:proficiency-shaping.tibia.p29',
    'oteryn:proficiency-shaping.tibia.p290', 'oteryn:proficiency-shaping.tibia.p291',
    'oteryn:proficiency-shaping.tibia.p292', 'oteryn:proficiency-shaping.tibia.p293',
    'oteryn:proficiency-shaping.tibia.p294', 'oteryn:proficiency-shaping.tibia.p295',
    'oteryn:proficiency-shaping.tibia.p296', 'oteryn:proficiency-shaping.tibia.p297',
    'oteryn:proficiency-shaping.tibia.p298', 'oteryn:proficiency-shaping.tibia.p299',
    'oteryn:proficiency-shaping.tibia.p30', 'oteryn:proficiency-shaping.tibia.p300',
    'oteryn:proficiency-shaping.tibia.p301', 'oteryn:proficiency-shaping.tibia.p302',
    'oteryn:proficiency-shaping.tibia.p303', 'oteryn:proficiency-shaping.tibia.p304',
    'oteryn:proficiency-shaping.tibia.p305', 'oteryn:proficiency-shaping.tibia.p307',
    'oteryn:proficiency-shaping.tibia.p308', 'oteryn:proficiency-shaping.tibia.p309',
    'oteryn:proficiency-shaping.tibia.p31', 'oteryn:proficiency-shaping.tibia.p310',
    'oteryn:proficiency-shaping.tibia.p311', 'oteryn:proficiency-shaping.tibia.p312',
    'oteryn:proficiency-shaping.tibia.p313', 'oteryn:proficiency-shaping.tibia.p315',
    'oteryn:proficiency-shaping.tibia.p316', 'oteryn:proficiency-shaping.tibia.p317',
    'oteryn:proficiency-shaping.tibia.p318', 'oteryn:proficiency-shaping.tibia.p319',
    'oteryn:proficiency-shaping.tibia.p32', 'oteryn:proficiency-shaping.tibia.p320',
    'oteryn:proficiency-shaping.tibia.p321', 'oteryn:proficiency-shaping.tibia.p322',
    'oteryn:proficiency-shaping.tibia.p323', 'oteryn:proficiency-shaping.tibia.p324',
    'oteryn:proficiency-shaping.tibia.p325', 'oteryn:proficiency-shaping.tibia.p326',
    'oteryn:proficiency-shaping.tibia.p327', 'oteryn:proficiency-shaping.tibia.p328',
    'oteryn:proficiency-shaping.tibia.p329', 'oteryn:proficiency-shaping.tibia.p33',
    'oteryn:proficiency-shaping.tibia.p330', 'oteryn:proficiency-shaping.tibia.p331',
    'oteryn:proficiency-shaping.tibia.p332', 'oteryn:proficiency-shaping.tibia.p333',
    'oteryn:proficiency-shaping.tibia.p334', 'oteryn:proficiency-shaping.tibia.p335',
    'oteryn:proficiency-shaping.tibia.p336', 'oteryn:proficiency-shaping.tibia.p337',
    'oteryn:proficiency-shaping.tibia.p338', 'oteryn:proficiency-shaping.tibia.p339',
    'oteryn:proficiency-shaping.tibia.p34', 'oteryn:proficiency-shaping.tibia.p340',
    'oteryn:proficiency-shaping.tibia.p341', 'oteryn:proficiency-shaping.tibia.p342',
    'oteryn:proficiency-shaping.tibia.p343', 'oteryn:proficiency-shaping.tibia.p344',
    'oteryn:proficiency-shaping.tibia.p345', 'oteryn:proficiency-shaping.tibia.p346',
    'oteryn:proficiency-shaping.tibia.p347', 'oteryn:proficiency-shaping.tibia.p348',
    'oteryn:proficiency-shaping.tibia.p349', 'oteryn:proficiency-shaping.tibia.p35',
    'oteryn:proficiency-shaping.tibia.p350', 'oteryn:proficiency-shaping.tibia.p351',
    'oteryn:proficiency-shaping.tibia.p352', 'oteryn:proficiency-shaping.tibia.p353',
    'oteryn:proficiency-shaping.tibia.p354', 'oteryn:proficiency-shaping.tibia.p355',
    'oteryn:proficiency-shaping.tibia.p356', 'oteryn:proficiency-shaping.tibia.p357',
    'oteryn:proficiency-shaping.tibia.p358', 'oteryn:proficiency-shaping.tibia.p359',
    'oteryn:proficiency-shaping.tibia.p36', 'oteryn:proficiency-shaping.tibia.p360',
    'oteryn:proficiency-shaping.tibia.p361', 'oteryn:proficiency-shaping.tibia.p362',
    'oteryn:proficiency-shaping.tibia.p363', 'oteryn:proficiency-shaping.tibia.p364',
    'oteryn:proficiency-shaping.tibia.p365', 'oteryn:proficiency-shaping.tibia.p366',
    'oteryn:proficiency-shaping.tibia.p367', 'oteryn:proficiency-shaping.tibia.p368',
    'oteryn:proficiency-shaping.tibia.p369', 'oteryn:proficiency-shaping.tibia.p37',
    'oteryn:proficiency-shaping.tibia.p370', 'oteryn:proficiency-shaping.tibia.p372',
    'oteryn:proficiency-shaping.tibia.p373', 'oteryn:proficiency-shaping.tibia.p374',
    'oteryn:proficiency-shaping.tibia.p375', 'oteryn:proficiency-shaping.tibia.p376',
    'oteryn:proficiency-shaping.tibia.p377', 'oteryn:proficiency-shaping.tibia.p378',
    'oteryn:proficiency-shaping.tibia.p379', 'oteryn:proficiency-shaping.tibia.p38',
    'oteryn:proficiency-shaping.tibia.p380', 'oteryn:proficiency-shaping.tibia.p381',
    'oteryn:proficiency-shaping.tibia.p382', 'oteryn:proficiency-shaping.tibia.p383',
    'oteryn:proficiency-shaping.tibia.p384', 'oteryn:proficiency-shaping.tibia.p385',
    'oteryn:proficiency-shaping.tibia.p386', 'oteryn:proficiency-shaping.tibia.p387',
    'oteryn:proficiency-shaping.tibia.p388', 'oteryn:proficiency-shaping.tibia.p389',
    'oteryn:proficiency-shaping.tibia.p39', 'oteryn:proficiency-shaping.tibia.p391',
    'oteryn:proficiency-shaping.tibia.p392', 'oteryn:proficiency-shaping.tibia.p393',
    'oteryn:proficiency-shaping.tibia.p397', 'oteryn:proficiency-shaping.tibia.p398',
    'oteryn:proficiency-shaping.tibia.p399', 'oteryn:proficiency-shaping.tibia.p40',
    'oteryn:proficiency-shaping.tibia.p400', 'oteryn:proficiency-shaping.tibia.p401',
    'oteryn:proficiency-shaping.tibia.p402', 'oteryn:proficiency-shaping.tibia.p403',
    'oteryn:proficiency-shaping.tibia.p404', 'oteryn:proficiency-shaping.tibia.p405',
    'oteryn:proficiency-shaping.tibia.p406', 'oteryn:proficiency-shaping.tibia.p407',
    'oteryn:proficiency-shaping.tibia.p408', 'oteryn:proficiency-shaping.tibia.p409',
    'oteryn:proficiency-shaping.tibia.p41', 'oteryn:proficiency-shaping.tibia.p410',
    'oteryn:proficiency-shaping.tibia.p411', 'oteryn:proficiency-shaping.tibia.p412',
    'oteryn:proficiency-shaping.tibia.p413', 'oteryn:proficiency-shaping.tibia.p414',
    'oteryn:proficiency-shaping.tibia.p415', 'oteryn:proficiency-shaping.tibia.p416',
    'oteryn:proficiency-shaping.tibia.p417', 'oteryn:proficiency-shaping.tibia.p418',
    'oteryn:proficiency-shaping.tibia.p419', 'oteryn:proficiency-shaping.tibia.p42',
    'oteryn:proficiency-shaping.tibia.p420', 'oteryn:proficiency-shaping.tibia.p421',
    'oteryn:proficiency-shaping.tibia.p422', 'oteryn:proficiency-shaping.tibia.p423',
    'oteryn:proficiency-shaping.tibia.p424', 'oteryn:proficiency-shaping.tibia.p425',
    'oteryn:proficiency-shaping.tibia.p426', 'oteryn:proficiency-shaping.tibia.p427',
    'oteryn:proficiency-shaping.tibia.p428', 'oteryn:proficiency-shaping.tibia.p429',
    'oteryn:proficiency-shaping.tibia.p43', 'oteryn:proficiency-shaping.tibia.p430',
    'oteryn:proficiency-shaping.tibia.p431', 'oteryn:proficiency-shaping.tibia.p432',
    'oteryn:proficiency-shaping.tibia.p433', 'oteryn:proficiency-shaping.tibia.p434',
    'oteryn:proficiency-shaping.tibia.p435', 'oteryn:proficiency-shaping.tibia.p436',
    'oteryn:proficiency-shaping.tibia.p437', 'oteryn:proficiency-shaping.tibia.p438',
    'oteryn:proficiency-shaping.tibia.p439', 'oteryn:proficiency-shaping.tibia.p44',
    'oteryn:proficiency-shaping.tibia.p440', 'oteryn:proficiency-shaping.tibia.p441',
    'oteryn:proficiency-shaping.tibia.p442', 'oteryn:proficiency-shaping.tibia.p443',
    'oteryn:proficiency-shaping.tibia.p444', 'oteryn:proficiency-shaping.tibia.p445',
    'oteryn:proficiency-shaping.tibia.p446', 'oteryn:proficiency-shaping.tibia.p447',
    'oteryn:proficiency-shaping.tibia.p448', 'oteryn:proficiency-shaping.tibia.p449',
    'oteryn:proficiency-shaping.tibia.p45', 'oteryn:proficiency-shaping.tibia.p450',
    'oteryn:proficiency-shaping.tibia.p451', 'oteryn:proficiency-shaping.tibia.p452',
    'oteryn:proficiency-shaping.tibia.p453', 'oteryn:proficiency-shaping.tibia.p457',
    'oteryn:proficiency-shaping.tibia.p458', 'oteryn:proficiency-shaping.tibia.p459',
    'oteryn:proficiency-shaping.tibia.p46', 'oteryn:proficiency-shaping.tibia.p460',
    'oteryn:proficiency-shaping.tibia.p461', 'oteryn:proficiency-shaping.tibia.p462',
    'oteryn:proficiency-shaping.tibia.p463', 'oteryn:proficiency-shaping.tibia.p47',
    'oteryn:proficiency-shaping.tibia.p474', 'oteryn:proficiency-shaping.tibia.p475',
    'oteryn:proficiency-shaping.tibia.p476', 'oteryn:proficiency-shaping.tibia.p477',
    'oteryn:proficiency-shaping.tibia.p478', 'oteryn:proficiency-shaping.tibia.p479',
    'oteryn:proficiency-shaping.tibia.p48', 'oteryn:proficiency-shaping.tibia.p480',
    'oteryn:proficiency-shaping.tibia.p481', 'oteryn:proficiency-shaping.tibia.p482',
    'oteryn:proficiency-shaping.tibia.p483', 'oteryn:proficiency-shaping.tibia.p484',
    'oteryn:proficiency-shaping.tibia.p485', 'oteryn:proficiency-shaping.tibia.p486',
    'oteryn:proficiency-shaping.tibia.p487', 'oteryn:proficiency-shaping.tibia.p488',
    'oteryn:proficiency-shaping.tibia.p489', 'oteryn:proficiency-shaping.tibia.p49',
    'oteryn:proficiency-shaping.tibia.p490', 'oteryn:proficiency-shaping.tibia.p491',
    'oteryn:proficiency-shaping.tibia.p492', 'oteryn:proficiency-shaping.tibia.p493',
    'oteryn:proficiency-shaping.tibia.p494', 'oteryn:proficiency-shaping.tibia.p495',
    'oteryn:proficiency-shaping.tibia.p496', 'oteryn:proficiency-shaping.tibia.p50',
    'oteryn:proficiency-shaping.tibia.p51', 'oteryn:proficiency-shaping.tibia.p52',
    'oteryn:proficiency-shaping.tibia.p53', 'oteryn:proficiency-shaping.tibia.p54',
    'oteryn:proficiency-shaping.tibia.p55', 'oteryn:proficiency-shaping.tibia.p56',
    'oteryn:proficiency-shaping.tibia.p57', 'oteryn:proficiency-shaping.tibia.p58',
    'oteryn:proficiency-shaping.tibia.p59', 'oteryn:proficiency-shaping.tibia.p6',
    'oteryn:proficiency-shaping.tibia.p60', 'oteryn:proficiency-shaping.tibia.p61',
    'oteryn:proficiency-shaping.tibia.p62', 'oteryn:proficiency-shaping.tibia.p63',
    'oteryn:proficiency-shaping.tibia.p64', 'oteryn:proficiency-shaping.tibia.p65',
    'oteryn:proficiency-shaping.tibia.p66', 'oteryn:proficiency-shaping.tibia.p67',
    'oteryn:proficiency-shaping.tibia.p68', 'oteryn:proficiency-shaping.tibia.p69',
    'oteryn:proficiency-shaping.tibia.p70', 'oteryn:proficiency-shaping.tibia.p71',
    'oteryn:proficiency-shaping.tibia.p72', 'oteryn:proficiency-shaping.tibia.p73',
    'oteryn:proficiency-shaping.tibia.p74', 'oteryn:proficiency-shaping.tibia.p75',
    'oteryn:proficiency-shaping.tibia.p76', 'oteryn:proficiency-shaping.tibia.p77',
    'oteryn:proficiency-shaping.tibia.p78', 'oteryn:proficiency-shaping.tibia.p79',
    'oteryn:proficiency-shaping.tibia.p8', 'oteryn:proficiency-shaping.tibia.p80',
    'oteryn:proficiency-shaping.tibia.p81', 'oteryn:proficiency-shaping.tibia.p82',
    'oteryn:proficiency-shaping.tibia.p83', 'oteryn:proficiency-shaping.tibia.p84',
    'oteryn:proficiency-shaping.tibia.p85', 'oteryn:proficiency-shaping.tibia.p86',
    'oteryn:proficiency-shaping.tibia.p87', 'oteryn:proficiency-shaping.tibia.p88',
    'oteryn:proficiency-shaping.tibia.p89', 'oteryn:proficiency-shaping.tibia.p9',
    'oteryn:proficiency-shaping.tibia.p90', 'oteryn:proficiency-shaping.tibia.p91',
    'oteryn:proficiency-shaping.tibia.p92', 'oteryn:proficiency-shaping.tibia.p93',
    'oteryn:proficiency-shaping.tibia.p94', 'oteryn:proficiency-shaping.tibia.p95',
    'oteryn:proficiency-shaping.tibia.p96', 'oteryn:proficiency-shaping.tibia.p97',
    'oteryn:proficiency-shaping.tibia.p98', 'oteryn:proficiency-shaping.tibia.p99'
  ]::TEXT[]) AS shaping_key;
