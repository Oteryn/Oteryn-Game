# CipSoft client weapon to proficiency bindings (15.30)

Source observations of the 666 appearance objects in the owner-provided 15.30 client
file `appearances-2dfa943b...dat` (`content/assets/files/`, pinned by SHA-256 in
`manifest.json`, together with the proficiencies JSON used for validation) whose
flags carry a proficiency reference. Nothing here is an Oteryn Item identity.

Wire shape (inferred; the appearances protobuf has no shipped schema): top-level field 1
repeats one object (43,516 in the file); within an object, field 1 is the client object
id (unique), field 3 the flags message and field 4 the name. Flags field 61 is a message
holding exactly one varint (field 1), the proficiency id. It appears in exactly 666
objects, once each; the script rejects any other shape, duplicate object ids and any
proficiency id that has no definition in `../proficiencies/`. The field number 61 was
identified because its values are all valid proficiency ids; that is an inference.

Each record has `source_id` (client object id), `source_index` (position among the
top-level fields), `name` (field 4, `null` if absent; present for all 666), `proficiency_id`
and `appearance_object_sha256` (SHA-256 of the raw object record).

Observed: the 666 objects reference 430 distinct proficiency ids; 13 of the 443 definitions
are referenced by no object. Multiple objects sharing one proficiency is normal (up to 23).

Regenerate or verify with `python tools/content-census/stage_proficiencies.py [--check]`.
