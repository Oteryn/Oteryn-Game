-- GUILDHALL-1: retain the qualified catalogue class without installing a
-- surrogate personal owner. Hall acquisition/custody remains fail-closed until
-- the existing Guildhall lifecycle owner lands.
ALTER TABLE game_house_properties DROP CONSTRAINT game_house_properties_kind_check;
ALTER TABLE game_house_properties
 ADD CONSTRAINT game_house_properties_kind_check
  CHECK(kind IN ('private_house','shop','guildhall')),
 ADD CONSTRAINT game_house_guildhall_lifecycle_absent CHECK(
  kind<>'guildhall' OR (state=1 AND owner_character_id IS NULL AND owner_account_id IS NULL
   AND paid_until IS NULL AND acquisition_receipt_ref IS NULL));

COMMENT ON CONSTRAINT game_house_guildhall_lifecycle_absent ON game_house_properties IS
 'GUILDHALL-1 classification only: delegates leader eligibility, Guild custody, ACL, escrow, rent and release to their accepted owners';
