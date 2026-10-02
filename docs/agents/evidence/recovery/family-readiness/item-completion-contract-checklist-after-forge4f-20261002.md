Kontrakt ukończenia Item: audyt po Forge4f

Źródła: native `4f690c1e8a8966a4d074dde94d5a884dbb34fb94`, overlay taxonomy `88edf552b323cef8cb68cef5b10949fbf0c85424`. To dwa osobne opublikowane drzewa, nie sprawdzony wspólny kandydat.

Istniejący walidator określa poprawność struktury authoring i zadeklarowanych zależności. Nie określa globalnej gotowości rozgrywki: wynik CLI ma runtime_qualified=false. Nie istnieje uniwersalna lista wszystkich wymaganych Known dla każdej rodziny. Pełny licznik ukończonych Item pozostaje NULL / NOT_ESTABLISHED.

Pięć wymaganych pól głównych: identity, display_name, family_profile, delivery_task_eligible, taxonomy. Taxonomy wymaga item_class i primary. Pozostałe capability groups są warunkowe. common_capabilities z katalogu wywołuje ostrzeżenia; expected_capabilities nie jest listą wymaganych pól.

22 profile i 23 szablony / zero brakujących profili dotyczą taxonomy88ed. Native4f ma ten sam katalog22, ale13 szablonów /10 niepokrytych profili. Wszystkie23 są przykładami z delivery_task_eligible=false; rune charges=1/stack=100 i inne literalne wartości szablonów nie są dowodami źródłowymi. Przed wspólną walidacją trzeba zachować jednocześnie nowe family/itemclass+imbuement maxima z taxonomy oraz poprawkę partial leech z native.

| Rodzina | Common: tylko ostrzeżenia | Optional: tylko po potwierdzeniu applicability | Item class w taxonomy88ed | Szablony overlay/native4f |
|---|---|---|---|---|
| equipment_armor | physical, equipment, protection, trade | modifiers, imbuement, forge | equipment | 1/1 |
| equipment_offhand | physical, equipment, trade | protection, modifiers, charges, temporal, imbuement, forge | equipment | 1/1 |
| container_equipment | physical, equipment, container, trade | — | container | 1/0 |
| weapon_melee | physical, equipment, weapon, trade | modifiers, imbuement, forge, proficiency | weapon | 1/1 |
| weapon_distance | physical, weapon, trade | equipment, stack, modifiers, imbuement, forge | weapon, ammunition | 2/2 |
| weapon_magic | physical, equipment, weapon, use, requirements, trade | modifiers | weapon | 1/1 |
| rune | physical, stack, charges, use, requirements, trade | — | rune | 1/1 |
| document | physical, readable, presentation, trade | — | document | 1/1 |
| container | physical, container, trade | — | container | 1/1 |
| decoration | physical, presentation, trade | use, light, lifecycle | decoration | 1/1 |
| event_collectible | physical, presentation, trade | — | collectible | 1/0 |
| progression_material | physical, trade | stack, use | material | 1/0 |
| transformation_item | physical, lifecycle, use | temporal, charges, trade | transformation | 1/0 |
| quest_item | physical | use, readable, container, fluid, light, lifecycle, presentation | quest_item | 1/0 |
| material_valuable | physical, trade | stack | material | 1/1 |
| trash | physical | use | trash | 1/0 |
| key | physical, use | — | key | 1/0 |
| light_source | physical, light | use, temporal, lifecycle, presentation | light_source | 1/0 |
| tool | physical, use, trade | charges, temporal, lifecycle | tool | 1/0 |
| food | physical, consumable, use, trade | stack | food, consumable | 1/1 |
| fluid | physical, fluid, use, trade | requirements | fluid, container, fluid_container | 1/1 |
| plant | physical, trade | stack, consumable, use | plant | 1/0 |

Warunkowe wymagania mają dokładne granice: authored equipment wybiera compact slot+hands lub niepuste patterns z pattern_id+slot+hands; charges wymaga dodatniego count; forge wymaga classification+max_tier; imbuement wymaga slot_count, nie domyślnego pełnego zestawu rodzin. Partial leech wymaga resource oraz co najmniej chance albo amount, bez wypełniania nieobecnej wartości zerem. Pełna lista zagnieżdżonych constraints i źródłowe SHA są w JSON.

Proponowana skończona karta kontroli dla każdego Item:

G1_IDENTITY_AND_DOMAIN — Unique exact current numeric binding and target identity; preserve own-ID-first and all-present source agreement; separate portable Item versus Terrain/WorldObject/instance state. Qualified family/navigation metadata alone is not native admission. Autor: EXISTING_SOURCE_QUALIFICATION_PLUS_SCHEMA.

G2_FAMILY_AND_ENVELOPE — identity, display_name, family_profile, delivery_task_eligible, taxonomy present; taxonomy item_class and primary present. Validate family/item_class using taxonomy88ed constraint only after real composition; native4f schema alone does not impose it. Autor: EXISTING_SCHEMA.

G3_FIELD_INVENTORY — Pinned source identities and real capture timestamps; every source field occurrence has exactly one disposition, and dispositions equal complete field_inventory. No unsupported_source_field, unresolved_semantics or conflict may pass import-readiness. Missing source field remains UNKNOWN; absence is not FALSE/N/A. Autor: EXISTING_VALIDATE_WITH_MANIFEST; REQUIRED_FOR_PROPOSED_IMPORT_COMPLETION_GATE.

G4_INTRINSIC_TYPED_PROJECTION — Destination exists with admitted unit/type/exact value and accepted owner (native semantics or ItemAuthoring), all applicable guards and known/conflict-state preservation. Rational percentage points, relative p/100, milliseconds and source units stay distinct. OTS hypotheses remain labeled hypotheses; engine agreement does not become official truth. Autor: EXISTING_CATALOG_ROUTES_AND_PACKET_GUARDS.

G5_CONDITIONAL_CAPABILITY_VALIDITY — Apply exact nested required/oneOf/anyOf constraints and semantic coherence; missing common capabilities produce warning, not failure. Equipment slot/hands/reservation, damage minimum<=maximum, resource/vector uniqueness, imbuement exclusion, lifecycle decay consistency. No generic all-optional-known rule. Autor: EXISTING_SCHEMA_AND_VALIDATE.

G6_EXTERNAL_RELATION_CLOSURE — Resolve every exact used definition/asset/proficiency crosswalk to its owning domain; no duplicate or unused declarations. Quest/drop/trade/actor/effect/formula/FX/audio relations retain source owning disposition; identity of a sound/effect alone does not establish its activation/consumer binding. Autor: EXISTING_VALIDATE_AND_SOURCE_DISPOSITIONS.

G7_PRESENTATION_AND_RESOURCE_STATUS — appearance_binding requires actual pinned presentation payload and valid frame geometry/sprite counts. Sprite-atlas absence is a warning in structural validator, a real-example blocker, and must remain explicit in any renderability claim. Do not infer zero/missing FX from no source binding. Autor: EXISTING_VALIDATE; FULL_RENDERABILITY_SEPARATE.

G8_EVERY_AUTHORED_LEAF_EVIDENCE — field_evidence and non_source_defaults exactly partition authored leaves (validator-specific exceptions preserved). Defaults/normalizations are labeled, not source-proven. Existing validate_real_example is a closed real-example contract, not evidence that all34031 rows were checked. Autor: EXISTING_REAL_EXAMPLE_RULE; PROPOSED_GENERALIZATION_OUTSIDE_6_FIXTURES.

G9_PUBLISHED_COMPOSITION — Exact final published parent/source/packet SHA, actual typed owner equality, deterministic native/tree/migration comparison, preservation of unrelated fields, approved dependencies. Overlay taxonomy/world/native reports are not proof that their branches were integrated. Autor: EXISTING_PACKET_GUARDS; PROPOSED_GLOBAL_AGGREGATE_GATE.

G10_RUNTIME_READINESS — Existing Item authoring CLI always reports runtime_qualified=false. Rune ability/formula, modifier target/evaluation/priority, interaction/effects, sprites/resources and actor rules need their actual accepted owning contracts; do not substitute source metadata completion for runtime readiness. Autor: NOT_ESTABLISHED_AS_GLOBAL_PER_ITEM_CONTRACT.

Cztery osobne wyniki: categorized; complete source field coverage dla zadeklarowanych źródeł; complete typed import z zamkniętymi zadeklarowanymi zależnościami; osobno runtime-qualified. UNKNOWN bez odpowiedniego źródła nie jest automatycznym błędem. Jawny source-fact bez typed projection jest konkretnym import gap. Relacja do Quest/Ability/Effect/FX/audio należy do właściwego właściciela i wymaga dowodu/bindingu, jeżeli źródło ją deklaruje.

Manifest source inventory jest opcjonalnym argumentem istniejącego CLI; wymaganie go dla pełnego globalnego ukończenia jest wyraźnie oznaczoną propozycją agregacji, nie dotychczasową obowiązkową walidacją całej puli. Podobnie zamknięte real-example leaf proofs nie zostały wykonane dla34031 Item. Brak autorytatywnego globalnego kontraktu runtime nie blokuje dalszego źródłowego uzupełniania; blokuje jedynie twierdzenie „wszystkie gotowe/zero”.
