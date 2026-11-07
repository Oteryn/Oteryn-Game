-- Appearance-definition provenance is distinct from the gameplay Root content pin.
-- Existing immutable history retains its old meaning through COALESCE; all eight
-- gameplay chain pins and the original receipt immutability guards stay intact.
ALTER TABLE game_character_appearance_receipts ADD COLUMN cosmetics_revision TEXT
 CHECK(cosmetics_revision IS NULL OR octet_length(cosmetics_revision) BETWEEN 1 AND 128);
CREATE OR REPLACE FUNCTION game_appearance_selection_consistency() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE v_character UUID;
BEGIN
 v_character:=NEW.character_id;
 IF EXISTS (
   WITH ordered AS (
     SELECT a.*, lag(outfit_after) OVER w AS previous_outfit,
       lag(colours_after) OVER w AS previous_colours,
       lag(addons_after) OVER w AS previous_addons,
       lag(mount_after) OVER w AS previous_mount,
       lag(COALESCE(cosmetics_revision,content_revision)) OVER w AS previous_content,
       lag(committed_character_revision) OVER w AS previous_revision,
       row_number() OVER (ORDER BY committed_character_revision DESC) AS latest
     FROM game_character_appearance_receipts a WHERE character_id=v_character
     WINDOW w AS (ORDER BY committed_character_revision)
   )
   SELECT 1 FROM ordered a WHERE
     (outfit_before,colours_before,addons_before,mount_before,content_before,selection_revision_before)
       IS DISTINCT FROM
     (previous_outfit,previous_colours,previous_addons,previous_mount,previous_content,previous_revision)
     OR (latest=1 AND NOT EXISTS (
       SELECT 1 FROM game_character_appearance p WHERE p.character_id=v_character
         AND (p.outfit_key,p.colours,p.addons,p.mount_key,p.content_revision,
              p.committed_character_revision,p.last_appearance_occurrence_id)
           IS NOT DISTINCT FROM
             (a.outfit_after,a.colours_after,a.addons_after,a.mount_after,COALESCE(a.cosmetics_revision,a.content_revision),
              a.committed_character_revision,a.appearance_occurrence_id)))
   UNION ALL
   SELECT 1 FROM game_character_appearance p WHERE p.character_id=v_character
     AND NOT EXISTS(SELECT 1 FROM ordered)
 ) THEN
   RAISE EXCEPTION 'appearance projection differs from receipt chain' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END;
$$;
