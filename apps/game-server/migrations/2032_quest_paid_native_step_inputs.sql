-- M01c: automatic native STEP inputs retain the carried custody, quantity,
-- mutation and same-transaction receipt guards installed by 0268 and QENGINE2.
CREATE FUNCTION game_quest_callback_paid_step_matches(
    consumption game_quest_callback_consumptions,
    receipt game_character_quest_receipts
) RETURNS BOOLEAN LANGUAGE SQL IMMUTABLE SET search_path=pg_catalog,public AS $body$
    SELECT receipt.cause_kind='step'
        AND receipt.quest_key=callback->>'quest'
        AND consumption.transition_key=callback->>'key'
        AND callback->>'trigger'='step_in'
        AND callback->'requires_world_admission'='true'::jsonb
        AND jsonb_typeof(callback#>'{selector,native}')='object'
        AND callback#>'{item_on_target,on_step}'='true'::jsonb
        AND callback#>'{item_on_target,count}'=to_jsonb(consumption.quantity)
        AND (callback#>>'{item_on_target,item}'=consumption.before_item->>'definition_production_key'
            OR (callback#>'{item_on_target,alternatives}') ? (consumption.before_item->>'definition_production_key'))
        AND coalesce(callback#>'{item_on_target,reusable}','false'::jsonb)='false'::jsonb
        AND coalesce(callback#>'{item_on_target,replacement}','null'::jsonb)='null'::jsonb
        AND coalesce(callback#>'{item_on_target,target_replacement}','null'::jsonb)='null'::jsonb
        AND consumption.replacement IS NULL
    FROM (SELECT convert_from(consumption.proof,'UTF8')::jsonb->'callback' AS callback) proof
$body$;

DO $migration$
DECLARE body TEXT; original TEXT:='r.cause_kind=''use''';
BEGIN
    body:=pg_get_functiondef('game_quest_callback_complete()'::regprocedure);
    IF (length(body)-length(replace(body,original,'')))/length(original)<>1 THEN
        RAISE EXCEPTION 'unexpected paid native STEP completion guard';
    END IF;
    EXECUTE replace(body,original,
        '(r.cause_kind=''use'' OR coalesce(game_quest_callback_paid_step_matches(NEW,r),false))');
END $migration$;
REVOKE ALL ON FUNCTION game_quest_callback_paid_step_matches(
    game_quest_callback_consumptions,game_character_quest_receipts) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_quest_callback_paid_step_matches(
    game_quest_callback_consumptions,game_character_quest_receipts) TO oteryn_game_runtime;
