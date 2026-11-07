-- S04a: a stopped presence listener refuses a new VIP write, but that outcome
-- must survive ingress-cache loss. Extend the existing VIP writer and its
-- existing per-operation definer wrappers; no new write grant or receipt owner.
--
-- p_cap=0, p_groups=0 is an internal denial-only selector, never a playable
-- allowance. Existing 20/100 allowance paths keep every rule and fence. The
-- canonical request, command key and SHA-256 binding stay byte-for-byte the
-- same. Recovery and retained replay still precede live authority checks.
DO $migration$
DECLARE
    definition TEXT;
    validation_before TEXT := 'IF op NOT BETWEEN 1 AND 12 OR p_cap NOT IN(20,100) OR p_groups NOT IN(0,5) OR p_command<1 THEN';
    validation_after TEXT := 'IF op NOT BETWEEN 1 AND 12 OR p_cap NOT IN(0,20,100) OR p_groups NOT IN(0,5) OR p_command<1 OR (p_cap=0 AND (op=12 OR p_groups<>0)) THEN';
    rate_before TEXT := 'IF list_row.rate_tokens=0 THEN result_code:=8; seconds:=greatest(1,3-(now_s-list_row.rate_at));';
    rate_after TEXT := E'-- Unavailable is receipted through the common tail, without attempting\n  -- the requested mutation or spending a bucket token. Its result is final.\n  IF p_cap=0 THEN result_code:=9;\n  ELSIF list_row.rate_tokens=0 THEN result_code:=8; seconds:=greatest(1,3-(now_s-list_row.rate_at));';
BEGIN
    -- Preserve the deployed owner's full definition, including any preceding
    -- recovery/guard hardening, owner, ACL and pinned search_path. Refuse a
    -- different baseline instead of silently replacing an unexpected body.
    SELECT pg_get_functiondef(
        'game_vip_execute(jsonb,jsonb,numeric,jsonb,integer,integer)'::regprocedure
    ) INTO definition;
    IF (length(definition)-length(replace(definition,validation_before,''))) <> length(validation_before)
       OR (length(definition)-length(replace(definition,rate_before,''))) <> length(rate_before) THEN
        RAISE EXCEPTION 'unexpected VIP writer definition for availability receipt'
            USING ERRCODE='23514';
    END IF;
    definition := replace(definition,validation_before,validation_after);
    definition := replace(definition,rate_before,rate_after);
    EXECUTE definition;
END;
$migration$;
