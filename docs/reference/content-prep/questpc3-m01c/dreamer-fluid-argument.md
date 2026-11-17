# Dreamer's Challenge: preserve the fluid argument

Pinned donor: Crystal `201189f871d23597b82caf88fd611454a2ddefb8`.
The existing `container_native_source.json` entry for UID 6197 records child
`[2874, 1]`. Its second argument must not become a stack quantity or be inferred
from the chest's display name.

Checked upstream implementation:

- [container_functions.cpp:156–183](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/src/lua/functions/items/container_functions.cpp#L156): `Container:addItem` reads the third Lua argument as count/subtype, clamps it only for stackable items, and passes it to `Item::CreateItem`. File SHA-256: `70bc1c33d6b5f612400ec52cd218c8ab6d1190605ddf24f1454fd790c66d35fd`.
- [item.cpp:105–110](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/src/items/item.cpp#L105): ordinary items pass that argument into the Item constructor.
- [item.cpp:322–328](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/src/items/item.cpp#L322): fluid containers interpret the argument as fluid subtype. File SHA-256: `e27e622bf4fd9a94d7fffb3a818fd0c5f911e6fbf4b92844320154ded326950b`.

The native descriptor therefore needs separate `count: 1` and `fluid_subtype: 1`.
The chest binding must preserve this modifier through to the existing complete-plan
reward MINT. Missing Item qualification and claim holds remain binding; this
note does not qualify the source claim or mark Dreamer's Challenge playable.
