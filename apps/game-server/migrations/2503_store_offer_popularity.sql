-- T130 / owner decision B 2026-11-09: public ranking of accepted Game receipts.
-- This summary is not a balance, debit, order, or purchased-coin authority.
LOCK TABLE public.game_store_purchase_history IN SHARE ROW EXCLUSIVE MODE;
CREATE TABLE IF NOT EXISTS public.game_store_offer_popularity (
    offer_key TEXT PRIMARY KEY CHECK (octet_length(offer_key) BETWEEN 1 AND 128),
    purchases BIGINT NOT NULL CHECK (purchases >= 0)
);
INSERT INTO public.game_store_offer_popularity(offer_key,purchases)
    SELECT offer_key,count(*) FROM public.game_store_purchase_history GROUP BY offer_key
    ON CONFLICT (offer_key) DO UPDATE SET purchases=EXCLUDED.purchases;
CREATE OR REPLACE FUNCTION public.game_store_count_accepted_purchase() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
BEGIN
    INSERT INTO public.game_store_offer_popularity(offer_key,purchases) VALUES(NEW.offer_key,1)
        ON CONFLICT (offer_key) DO UPDATE
        SET purchases=public.game_store_offer_popularity.purchases+1;
    RETURN NEW;
END $$;
DROP TRIGGER IF EXISTS game_store_count_accepted_purchase ON public.game_store_purchase_history;
CREATE TRIGGER game_store_count_accepted_purchase
    AFTER INSERT ON public.game_store_purchase_history
    FOR EACH ROW EXECUTE FUNCTION public.game_store_count_accepted_purchase();
REVOKE ALL ON public.game_store_offer_popularity FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
GRANT SELECT ON public.game_store_offer_popularity TO oteryn_game_runtime,oteryn_game_control;
REVOKE ALL ON FUNCTION public.game_store_count_accepted_purchase() FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
