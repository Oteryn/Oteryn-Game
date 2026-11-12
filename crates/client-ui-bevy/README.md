# Bevy UI foundation

First shared native Bevy 0.20.1 presentation package. The comparison harness's
Options consumer uses the button, checkbox, text, frame and navigation helpers;
its shell and Task Board reuse the font/frame primitives. `apps/client` remains
the product composition root. This package does not integrate real sessions or
replace `client-ui` models, settings transactions, focus routing or action handlers.

Callers supply font and image handles, project `Checked` / `InteractionDisabled`,
and route upstream `Activate` / `ValueChange<bool>` events to their existing owner.
Install `GlobalUiPlugin` once to project button skins, caption colors, offsets and frame chrome.
The plugin runs in `PostUpdate`; input and application projection run in `Update`.
Chrome updates after native UI layout and appears in the following layout pass.
Each live frame owns exactly nine absolute, non-pickable, non-focusable cropped
image children, drawn behind body content. Their source and geometry update in
place on skin changes/resizes, and hierarchy despawn removes them with the owner.
There is no generated-texture cache or per-resize entity allocation. The parent
`ImageNode` is a transparent source holder; the child images draw the chrome.
All source holders and cropped images use `VisualBox::BorderBox`.
Attach `button_caption` and optional `ButtonIcon` entities directly to the button.
`ButtonIcon` assumes an absolutely positioned, vertically centered image with no
explicit top/bottom anchor. Use `Left` for the button's image and `Right` for a menu
arrow. The caller adds images, actions and focus scopes; the package opens no window.

Source-derived behavior was reconstructed from the supplied Global QML, rather
than transcribing the original implementation. No QML, fonts, images or captures
are distributed here. Relevant reference components:

| Source | Implemented contract |
| --- | --- |
| `TibiaStyle`, `TibiaTextBase` | Caller-provided bold face; 11px text/13px line, 8px small text/proportional line; hinting enabled; small text antialiased; dialog `#C0C0C0`, disabled `#707070` |
| `TibiaButton` | Native button; 20px height; centered small caption; 1px fixed-width cropped/tiled skin; pressed **or** checked selects down; centered caption shifts 1px from source 2px left/top padding; image shifts 1px; no hover texture or brightness multiplier |
| `TibiaCheckBox`, `TibiaMenuOptionCheckBox` | Native checkbox; 12px indicator, 4px caption gap; optional 22px framed row with 5px indicator inset |
| `TibiaFrame1PixelDown`, `TibiaFrame2PixelUpFilled` | Caller-selected images; fixed source-pixel insets via cropped native tiled children; repeating sides and center; unscaled corners |
| `OptionsMenuDialog`, `MenuWithSubMenu` | One 129x470 outer frame, 7px inset, 115px groups with 1px border, 113x20 rows, 10px group spacing; checked/expanded arrow selection |

Bevy's `TextFont` does not expose Qt's small-font `letterSpacing=0.1`; this is
unsupported. Qt rasterization and right elision are also not reproduced exactly.
Native Windows compilation and ECS tests are not GUI or pixel-parity evidence.
Bevy 0.20.1's proportional `Sliced` renderer scales borders below a pixel for a
98x40 source at 43x20; this package therefore uses cropped native `Tiled` nodes
instead. Tests drive native layout to verify edge widths, BorderBox coverage and
unchanged body placement.
Caller-owned controls with specialized bitmap skins (for example slider endcaps)
must not also carry ordinary `ButtonImages` or `Frame` components.

The dependency uses exact upstream Bevy with rendering, window creation, audio,
clipboard and default assets disabled. Bevy's UI graph still includes its upstream
sprite/image/text infrastructure; no fork or vendor patch is introduced. Asset
custody and redistribution remain the caller's responsibility. The exact MIT-0 license exceptions for
`encase`, `encase_derive` and `encase_derive_impl` version 0.12.1 are admitted in
`deny.toml` under explicit task authority. `dependency-custody.json` records the
verified registry archive checksums, SPDX text match and exact upstream source
revision (the derive archives declare SPDX MIT-0 but omit their own LICENSE file).
No global license allowlist was expanded.

Focused validation:

```sh
cargo fmt --all --check
cargo clippy --locked -p oteryn-client-ui-bevy --all-targets -- -D warnings
cargo test --locked -p oteryn-client-ui-bevy
cargo run --locked -p oteryn-architecture-check -- workspace .
```
