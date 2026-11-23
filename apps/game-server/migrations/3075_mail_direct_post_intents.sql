-- Retain the original Direct Post handles alongside the immutable delivery.
-- Item moves through a mailbox keep their existing resolved-intent receipt.
CREATE TABLE game_mail_direct_post_intents (
    mail_id UUID PRIMARY KEY REFERENCES game_mail_operations(mail_id),
    wire_intent BYTEA NOT NULL CHECK (
        octet_length(wire_intent)=17
        AND substring(wire_intent FROM 1 FOR 1)=decode('01','hex')
        AND substring(wire_intent FROM 2 FOR 8)<>int8send(0)
        AND substring(wire_intent FROM 10 FOR 8)<>int8send(0)
    )
);

CREATE FUNCTION game_mail_direct_post_intent_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM game_mail_operations
                   WHERE mail_id=NEW.mail_id AND created_xact_id=pg_current_xact_id()) THEN
        RAISE EXCEPTION 'Direct Post intent requires a delivery from the same transaction'
            USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END
$$;
CREATE TRIGGER game_mail_direct_post_intent_guard
    BEFORE INSERT ON game_mail_direct_post_intents FOR EACH ROW
    EXECUTE FUNCTION game_mail_direct_post_intent_guard();

DO $migration$
BEGIN
    EXECUTE format('ALTER FUNCTION game_mail_direct_post_intent_guard() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;
REVOKE ALL ON FUNCTION game_mail_direct_post_intent_guard() FROM PUBLIC;
REVOKE ALL ON game_mail_direct_post_intents FROM PUBLIC;
GRANT SELECT, INSERT ON game_mail_direct_post_intents TO oteryn_game_runtime;
GRANT SELECT ON game_mail_direct_post_intents TO oteryn_game_control;
