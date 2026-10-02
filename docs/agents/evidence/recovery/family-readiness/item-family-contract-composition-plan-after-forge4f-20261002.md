Plan integracji istniejącego kontraktu rodzin do faktycznego native successor

Taxonomy88ed/PR1523 zawiera22profiles23templates; oryginalne template/family constraints opublikowano w PR1437:340a6 oraz7aa9351. Native4f nadal ma13templates/12coveredprofiles oraz taxonomy164rows;10templates nie są „nieutworzone”, lecz nie są w złożonym native head.

Minimalny wykonany w nowym draft carry:10 exact template blobs + addytywny family allOf w obecnym build_formal_schema/item.schema + właściwy formal verifier/report. PROFILE_ITEM_CLASS jest już identyczny; engine28644 i sourcecatalog825 muszą pozostać byte-identical. Zachować partial leech anyOf i native tests. Opublikowany taxonomy schema wymaga obu leech pól, więc skopiowanie go w całości byłoby regresją.

Merge verifier: profile-set coverage zamiast „13”; family/itemclass mismatch fixture; istniejący engine-type container fixture musi zmieniać również family_profile=container. Wszystkie późniejsze checks i context workflow zachować. Nie importować starego generated report. JSON wymienia exact source SHA/path/commit dla10templates, pełne oryginalne owned paths i fragment allOf.

Przejrzane current promotion packets nie pinują Item schema/generator/formal verifier. Przy pozostawieniu decoder/catalog/compiler/source witnesses nie trzeba ich resealować ani zmieniać Rust packet pins. Trzeba wykonać wszystkie packet checks na realnym nowym successor; przyszłe Weapon/Default sprawdzić po faktycznej publikacji.

Pełna integracja rodzin/world to dodatkowa konkretna praca: native taxonomy164 vs published12252 oraz relations203 vs729. Nie wolno wstawić starego overlay na aktualne owner316/source203/relations278, Known nazwy i69 native shards bez closed sourceguard/current target/owner comparison oraz qualified composition receipt. Minimalne template/schema carry samo nie zamyka tej pracy.
