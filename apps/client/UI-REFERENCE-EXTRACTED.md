# Installed-original resource evidence — 2026-10-11

Reference: authorized Linux installation, Tibia **15.33.b21348**. This is an
independently written semantic inventory, not redistributed original source or
assets. Offline inspection did not modify the installation, apply preferences,
read account configuration or execute gameplay actions. Qt's resource APIs read
the packaged RCC and the ELF's embedded resource arrays; Qt decompression reads
the shader packages. Temporary tools and original captures remain private.

## Reusable evidence, not an implementation claim

| Material recovered | What it establishes | What it does not establish |
| --- | --- | --- |
| 396 readable QML files among 408 embedded files | Actual page composition, properties, slider bounds, dependencies and UI handlers | C++ implementations, factory defaults or working Oteryn consumers |
| Graphics RCC: 1,063 files including 1,040 PNGs and 14 JPGs | Cursor geometry, reference artwork/layout vocabulary | Rights to ship the artwork |
| Four decompressible QSB packages | Recoverable packaged shader representations for simple/world-map-highlight vertex/fragment stages | Recovery of the complete world-lighting/AA renderer |
| Sound protobuf catalogue | 844 sound records, 592 numeric effects, 91 ambience streams, nine ambience-object streams, 25 music templates; zero missing referenced filenames | An Oteryn mixer, authoritative events or permission to redistribute sound files |
| Spell resource: 198 records | Reference picker fields, costs, cooldowns, vocation/level restrictions and icon indices | Authoritative Oteryn learned spells or executable spell assignments |

The sound catalogue follows the existing upstream schema at OTClient revision
`53c3878a1c5119c78e148adb32def53fea3c53f2`, `src/protobuf/sounds.proto`.
Use that schema rather than inventing a format. The catalogue has pitch/volume
ranges and simple/random effect references. A sampled OGG was Vorbis, 48 kHz,
stereo; this is not a claim about every recording.

Packaged `tutorial/clientoptions.json` is tutorial configuration, **not factory
defaults**. The 23 × 23 default cursor and 256 × 32 walk/attack strips establish
geometry, not animation cadence. Original artwork/audio/source stays outside
the public repository; Oteryn implementations remain independently authored.

## Exact settings contracts recovered

Resource prefix: `:/qt/qml/qmlcomponents/qml/` in the installed executable.

| Resource | Confirmed contract |
| --- | --- |
| OptionsPageGraphics.qml | FPS integer **10–200**, step one; Unlimited disables and retains it. Renderer and AA models are runtime supplied. VSync depends on renderer limitations; Unlimited has a separate runtime capability predicate. Fullscreen sets the borderless-window property |
| OptionsPageEffects.qml | All seven sliders 0–100, step one. Light gates Ambient/Level Separator/combined Clouds & Indoor. Only Clouds & Indoor has an Off marker at zero; spell opacity is a separate group |
| OptionsPageSound.qml | Root requires initialized sound engine. Child volume group depends on runtime soundEnabled. Anthem additionally requires music > 0; food/move-item require item volume > 0. All volume sliders mark zero Off |
| OptionsPageBattleSounds.qml | Groups require runtime soundEnabled; spell/weapon filters additionally require their group volume > 0. Spell categories additionally require Spells. Children retain selected intent |
| OptionsPageUISounds.qml | UI filters require UI volume > 0 and runtime soundEnabled; message-category children additionally require Console Messages. Volume marks zero Off |
| OptionsPageControls.qml | Custom delay 0–1000 ms; default master disables it. Critical style/help below 50; warning below 250; neither at 250 or above. Shift/Ctrl/Alt rotation selections are separate |
| OptionsPageInterface.qml | Native/Animated/Big are independent controls. Unused expiry is gated by inventory OR container expiry |
| TibiaMenuSlider.qml / TibiaSlider.qml | Off marker supplements the minimum value. Arrow buttons and left/right keys step by one, Shift multiplies by 10, Ctrl by 100; wheel uses a tenth of the range per normal notch |
| TibiaAutorepeatIconButton.qml | Primary click executes on release with its modifiers. A 350 ms timer starts repetition; its first 60 ms interval follows that delay. Further 60 ms repeats carry no mouse modifiers; disable/release stops repetition |

The backend definition of `soundEnabled` was not recovered in this bounded pass;
do not silently equate it with a stored master-volume value. Device enumeration,
renderer capability lists and complete light/effect algorithms remain runtime
work. Source facts eliminate UI guesswork but do not enable absent consumers.

## Correction to historical FPS evidence

The earlier written **1–200** observation is superseded for version 15.33 by
Graphics lines 211–213 and the underlying slider's direct bounds aliases. No
factory preference or original Apply was needed. New Oteryn edits use 10–200;
existing 1–9 values remain valid, rendered without mutation and consumed by the
scheduler until explicitly edited. Unlimited still retains the prior value.

Private source digests (SHA-256, decompressed resource bytes):

- Graphics: `baebabc1add4912336c23dae0fa8286eb9917b94a60b4bea8c0ff7559c69bbf8`
- Controls: `e291d3822c1937c2f7692326c8dc3b594d1b188d1a46c4a08ed0722f50d8c87e`
- Sound: `270ae6c661b93d4b06640cc903bcca86f40b8d50b9d295a6fda6076943b5f37e`
- Battle Sounds: `6acc6c1c1abf68895a5a1f3247cbaf848bcda6b585ece22544f9b7b5904cf238`
- UI Sounds: `12a411dc8e0b88d7bebad2e9124199a8df31aca7b2d87a6931ef52bc3cce5088`
- Menu slider: `8a248cc70f61577e08ed5af862acfc51d14936dbccdc680ad96de15b58132294`
- Slider: `29565cd82ad5a6a1ecdec4cad0e03feef0596d5d5e614dc1681c91a9e9ba1e96`
- Repeating arrow: `217119765d86cddd906bf28cb36d8fdd77d139c1247152611b60c6a74743460b`

Current repairs cover the editable FPS minimum, delay warning boundaries, minimum
Off markers and volume-dependent anthem/item child groups. Audio rows remain
disabled without playback. The next source-backed batch adds arrow/key modifiers,
ordinary line-wheel steps, held-arrow timing and positive-volume Battle/UI filter
gates. It uses upstream egui sliders with a bounded Oteryn adapter, not copied QML
or a dependency fork. Pixel/trackpad wheel equivalence and full native interactive
comparison remain pending; these changes do not establish audio playback.
