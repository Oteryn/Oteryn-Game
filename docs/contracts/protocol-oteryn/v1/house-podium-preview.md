# Podium draft artwork

Command 739 extends existing capability 150 without adding a capability. It is a read-only
lookup against the admitted active content generation for the current admitted actor, including outside a house. Artwork metadata is public.
It grants neither cosmetic ownership nor permission to modify a podium. Command 175 remains
the fenced ownership, house access and revision checked mutation.

## Request

Display kind (1 outfit, 2 mount, 3 creature), direction (0 north to 3 west), key byte length
(1–160), followed by exactly that many UTF-8 bytes. Maximum 163 bytes; no trailing bytes.

## Reply

Unknown artwork is one zero byte. Known artwork is eleven bytes: 1, object-appearance flag
(0 outfit, 1 object), direction, two nonzero u32 big-endian appearance IDs. Outfits return
male and female variants; mounts and creatures repeat their appearance. Object-look creatures
use their object appearance. The client draws only existing hash-pinned artwork and reports
missing assets explicitly. No official assets are copied or distributed by this mechanic.
