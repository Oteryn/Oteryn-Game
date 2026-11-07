# Official client asset import (C28)

The original graphics inventory was produced from the owner's flat 15.30 `assets`
directory and its packaged archive, as described in
`imports/official/client-assets/15.30/README.md:12`. No executable generator was
retained. `import_assets.py` implements that documented method: sorted basenames,
byte sizes, per-file SHA-256, original archive SHA-256, compact sorted JSON and LF.
It can inventory graphics too. Existing graphics bytes and pins remain unchanged.

Sound uses a supplemental `sound-manifest.json` in the same import directory and
schema, with optional `sound_data` and `sound_cues` fields described by
`manifest.schema.json`. Keeping a separate inventory avoids replacing graphics
pins and accommodates the official client's separate flat sound directory.
The metadata file and every audio file are included in `files` and hashed.
`archive_sha256` hashes the supplied archive bytes, not a reconstructed archive.
The operator must supply the directory extracted from that archive.

## Source of cue indirection

The binary source MUST be the owner's original 15.30 `sounds*.dat` from the
sound directory (use its actual basename). No such file or audio is present in
this checkout or its mounted inputs. Consequently no real cue mapping or sound
manifest is committed. Metadata names are not inferred from cue numbers.

The compatibility schema is
[mehah/otclient src/protobuf/sounds.proto](https://github.com/mehah/otclient/blob/def57c1550d8c85379b147d20adfe52a5318db16/src/protobuf/sounds.proto),
lines 5–27 and 416–422. `Sounds.sound` (field 1) maps internal sound ID to
`Sound.filename` (field 2). `Sounds.numeric_sound_effect` (field 2) maps the
protocol cue ID to `simple_sound_effect.sound_id` (field 5 / nested field 1)
or `random_sound_effect.random_sound_id` (field 6 / repeated nested field 1).
The importer joins those two tables from the supplied binary. Random variants
are preserved in source order. Stream/music/ambience tables and pitch/volume
variation are outside this one-shot flow. Ambient loops use the separate optional
`ambient_cues` mapping: schema lines 29–33 join `AmbienceStream.looping_sound_id`
to `Sound.filename`. A zero server cue stops the loop. Delayed embellishments
remain outside this base-loop flow. Unsupported or ambiguous numeric cue
records fail import rather than yielding invented mappings.

## Required input and command

Provide the original owner-packaged sound archive and its extracted flat 15.30
sound directory, including the original `sounds*.dat` and all referenced `.ogg`
or `.wav` files. The directory must contain files only, without symlinks.

```sh
python3 tools/client-assets/import_assets.py \
  --source /path/to/original-client/sounds \
  --archive /path/to/owner-packaged-sounds.zip \
  --sound-data ACTUAL-sounds-metadata.dat \
  --manifest-out imports/official/client-assets/15.30/sound-manifest.json \
  --copy-to content/assets/sounds
sha256sum imports/official/client-assets/15.30/sound-manifest.json
```

The runtime requires three environment settings: `OTERYN_SOUND_DIR` points to
`content/assets/sounds`; `OTERYN_SOUND_MANIFEST` points to the resulting manifest;
`OTERYN_SOUND_MANIFEST_SHA256` is its separately trusted SHA-256 from the command
above. The manifest is bounded and pinned before parsing, the original metadata
is verified on opening, and each audio file is verified before decoding.
Unknown cues are silent; missing/tampered assets and device/decoder errors appear
in the native window title. Missing audio configuration does not prevent login.

F10 toggles mute, F11/F12 lower/raise master volume by five percentage points.
Controls ignore repeats and modified keys. Preferences live in the existing
`ReferencePreferences` state and its serde representation; this checkout has no
native preference-file store. The Windows shell retains audio settings across
session re-entry for the life of the process. Voices stop when leaving play.
Rodio 0.21.1 provides Vorbis/WAV decoding and native output; all four source groups
use master gain, with at most 32 simultaneous, 30-second one-shot voices. Pending
cues are capped at 128 and never replayed from snapshots.

```sh
python3 -m unittest discover -s tools/client-assets -p 'test_*.py'
```

Fixtures are synthetic protobuf records and placeholder bytes. They are never
published as official sound content or used to demonstrate audible playback.
