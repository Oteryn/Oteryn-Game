#!/usr/bin/env python3
"""Evidence tool (NPC authoring D16): render Tibia outfits from the client assets and fit colours to TibiaWiki images.

It produced the WIKI_IMAGE_FIT colours in promotion_candidates.py; it is not run in CI and needs numpy and Pillow.
Usage: wiki_outfit_fit.py arbitrate <look_type> <addons> <gif> <head> <body> <legs> <feet> | fit | score | render.

Reuses the appearances protobuf decoder from tools/game-atlas-appearances/export.py (read-only import).
"""
import sys, json, lzma, functools, importlib.util
from pathlib import Path
import numpy as np
from PIL import Image

REPO = Path(__file__).resolve().parents[3]
ASSETS = REPO / 'content/assets/files'
_spec = importlib.util.spec_from_file_location('atlas_appearances', REPO / 'tools/game-atlas-appearances/export.py')
AX = importlib.util.module_from_spec(_spec); sys.modules['atlas_appearances'] = AX; _spec.loader.exec_module(AX)

TEMPLATE = {'head': (255, 255, 0), 'body': (255, 0, 0), 'legs': (0, 255, 0), 'feet': (0, 0, 255)}
REGIONS = ('head', 'body', 'legs', 'feet')
MISS_PENALTY = 255.0  # render-opaque pixel landing on a transparent wiki pixel


def palette(color):
    """Tibia outfit colour (0..132) -> RGB, OTClient HSI formula."""
    H, SI = 19, 7
    if color >= H * SI:
        color = 0
    if color % H:
        l1 = color % H / 18.0
        l2, l3 = [(0.25, 1.0), (0.25, 0.75), (0.5, 0.75), (0.667, 0.75), (1.0, 1.0), (1.0, 0.75), (1.0, 0.5)][color // H]
    else:
        l1, l2, l3 = 0.0, 0.0, 1 - color / H / SI
    if l3 == 0:
        return (0, 0, 0)
    if l2 == 0:
        v = int(l3 * 255); return (v, v, v)
    if l1 < 1/6: r = l3; b = l3 * (1 - l2); g = b + (l3 - b) * 6 * l1
    elif l1 < 2/6: g = l3; b = l3 * (1 - l2); r = g - (l3 - b) * (6 * l1 - 1)
    elif l1 < 3/6: g = l3; r = l3 * (1 - l2); b = r + (l3 - r) * (6 * l1 - 2)
    elif l1 < 4/6: b = l3; r = l3 * (1 - l2); g = b - (l3 - r) * (6 * l1 - 3)
    elif l1 < 5/6: b = l3; g = l3 * (1 - l2); r = g + (l3 - g) * (6 * l1 - 4)
    else: r = l3; g = l3 * (1 - l2); b = r - (l3 - g) * (6 * l1 - 5)
    return (int(r * 255), int(g * 255), int(b * 255))

PAL = np.array([palette(i) for i in range(133)], dtype=np.float32)  # (133,3)

# ---------------- assets ----------------

@functools.lru_cache(None)
def outfits():
    cat = json.loads((ASSETS / 'catalog-content.json').read_text())
    app = next(e['file'] for e in cat if e['type'] == 'appearances')
    return {a.appearance_id: a for a in AX.decode_category((ASSETS / app).read_bytes(), 'outfit')}

@functools.lru_cache(None)
def sprite_ranges():
    cat = json.loads((ASSETS / 'catalog-content.json').read_text())
    return sorted((e['firstspriteid'], e['lastspriteid'], e['spritetype'], e['file']) for e in cat if e['type'] == 'sprite')

@functools.lru_cache(64)
def sheet(file):
    """CipSoft sprite sheet: zero padding, 70 0A FA 80 24, 7-bit varint size, LZMA1 props+dict, 8-byte size, raw stream -> 384x384 BMP."""
    d = (ASSETS / file).read_bytes()
    i = 0
    while d[i] == 0: i += 1
    assert d[i:i+5] == b'\x70\x0a\xfa\x80\x24', file
    i += 5
    while d[i] & 0x80: i += 1
    i += 1
    props, dict_size = d[i], int.from_bytes(d[i+1:i+5], 'little')
    lc, rem = props % 9, props // 9; lp, pb = rem % 5, rem // 5
    raw = d[i+13:]
    bmp = lzma.LZMADecompressor(lzma.FORMAT_RAW, filters=[{'id': lzma.FILTER_LZMA1, 'dict_size': dict_size, 'lc': lc, 'lp': lp, 'pb': pb}]).decompress(raw)
    off = int.from_bytes(bmp[10:14], 'little'); w = int.from_bytes(bmp[18:22], 'little', signed=True)
    h = int.from_bytes(bmp[22:26], 'little', signed=True); bpp = int.from_bytes(bmp[28:30], 'little')
    assert bpp == 32, bpp
    px = np.frombuffer(bmp[off:off + w * abs(h) * 4], np.uint8).reshape(abs(h), w, 4)
    if h > 0: px = px[::-1]
    rgba = px[..., [2, 1, 0, 3]].copy()  # BGRA -> RGBA
    magenta = (rgba[..., 0] == 255) & (rgba[..., 1] == 0) & (rgba[..., 2] == 255)
    rgba[magenta, 3] = 0
    return rgba

def sprite(sid):
    for first, last, st, f in sprite_ranges():
        if first <= sid <= last:
            w, h = [(32, 32), (32, 64), (64, 32), (64, 64)][st]
            k = sid - first; cols = 384 // w
            s = sheet(f); y, x = (k // cols) * h, (k % cols) * w
            return s[y:y+h, x:x+w]
    raise KeyError(sid)

def idle_group(look_type):
    a = outfits()[look_type]
    return next((g for g in a.frame_groups if g.fixed_frame_group == 0), a.frame_groups[0])

def layers_for(look_type, addons, phase=0):
    """[(base RGBA, template RGBA|None)] for pattern_y 0 and each worn addon; south, no mount, phase 0."""
    g = idle_group(look_type)
    out = []
    for y in [0] + [k for k in (1, 2) if addons & k and k < g.pattern_height]:
        x = 2 if g.pattern_width > 2 else 0
        idx = lambda layer: (((phase * g.pattern_depth + 0) * g.pattern_height + y) * g.pattern_width + x) * g.layers + layer
        base = sprite(g.sprite_ids[idx(0)]).astype(np.float32)
        tmpl = sprite(g.sprite_ids[idx(1)]) if g.layers >= 2 else None
        out.append((base, tmpl))
    return out

def _masks(tmpl):
    if tmpl is None: return {}
    return {r: (tmpl[..., 3] > 0) & np.all(tmpl[..., :3] == c, axis=-1) for r, c in TEMPLATE.items()}

def render(look_type, head, body, legs, feet, addons, phase=0):
    cols = dict(zip(REGIONS, (head, body, legs, feet)))
    canvas = None
    for base, tmpl in layers_for(look_type, addons, phase):
        img = base.copy()
        for r, m in _masks(tmpl).items():
            img[m, :3] = img[m, :3] * PAL[cols[r]] / 255.0
        if canvas is None:
            canvas = np.zeros_like(img)
        op = img[..., 3] > 0
        canvas[op] = img[op]
    return Image.fromarray(np.clip(canvas, 0, 255).astype(np.uint8), 'RGBA')

def region_map(look_type, addons, phase=0):
    """Per-pixel owner after compositing: -1 transparent, 0..3 region, 4 uncoloured; plus base RGB."""
    lay = layers_for(look_type, addons, phase)
    h, w = lay[0][0].shape[:2]
    reg = np.full((h, w), -1, np.int8); base = np.zeros((h, w, 3), np.float32)
    for b, t in lay:
        op = b[..., 3] > 0
        reg[op] = 4; base[op] = b[op, :3]
        for i, (r, m) in enumerate(_masks(t).items()):
            reg[m & op] = i
    return reg, base

# ---------------- wiki images & alignment ----------------

@functools.lru_cache(64)
def wiki_frames(path):
    im = Image.open(path); frames = []
    for i in range(getattr(im, 'n_frames', 1)):
        im.seek(i); frames.append(np.asarray(im.convert('RGBA'), dtype=np.float32))
    # de-duplicate identical frames
    uniq = []
    for f in frames:
        if not any(np.array_equal(f, u) for u in uniq): uniq.append(f)
    return uniq

MIN_COVERAGE = 0.75  # fraction of render-opaque pixels that must fall inside the wiki image bounds

def _pixel_dist(pred_rgb, win):
    """Per-pixel distance; NaN where the pixel falls outside the wiki image (cropped by the wiki, not scored)."""
    d = np.sqrt(((pred_rgb - win[..., :3]) ** 2).sum(-1))
    d = np.where(win[..., 3] > 0, d, MISS_PENALTY)
    return np.where(win[..., 3] < 0, np.nan, d)

PAD = 40

def _padded(f):
    H, Wd = f.shape[:2]
    P = np.full((H + 2 * PAD, Wd + 2 * PAD, 4), -1.0, np.float32); P[PAD:PAD+H, PAD:PAD+Wd] = f
    return P

def _best_offset(r, f):
    """Best (mean distance, dy, dx) of render array r (h,w,4) over one wiki frame f; vectorised over dx."""
    op = r[..., 3] > 0; ys, xs = np.nonzero(op); pred = r[ys, xs, :3]; h, w = r.shape[:2]
    P = _padded(f); Hp, Wp = P.shape[:2]; best = (1e9, 0, 0)
    for dy in range(Hp - h + 1):
        win = P[dy + ys[None, :], xs[None, :] + np.arange(Wp - w + 1)[:, None]]   # (nx, n, 4)
        d = np.sqrt(((pred[None] - win[..., :3]) ** 2).sum(-1))
        d = np.where(win[..., 3] > 0, d, MISS_PENALTY)
        inside = win[..., 3] >= 0
        m = np.where(inside, d, 0).sum(1) / np.maximum(inside.sum(1), 1)
        m = np.where(inside.mean(1) >= MIN_COVERAGE, m, 1e9)
        k = int(m.argmin())
        if m[k] < best[0]: best = (float(m[k]), dy - PAD, k - PAD)
    return best

def score(image, wiki_gif, return_align=False):
    """Mean RGB distance over render-opaque pixels at the best (frame, dy, dx)."""
    r = np.asarray(image, dtype=np.float32)
    best = min((_best_offset(r, f) + (fi,) for fi, f in enumerate(wiki_frames(str(wiki_gif)))), key=lambda t: t[0])
    return (best[0], (best[3], best[1], best[2])) if return_align else best[0]

def _window(f, dy, dx, h, w):
    return _padded(f)[dy + PAD:dy + PAD + h, dx + PAD:dx + PAD + w]

def _fit_at(reg, base, win):
    """Per-region argmin over the 133 palette colours at a fixed window; returns cols, per-region dist, overall mean."""
    cols, per_region = [0, 0, 0, 0], {}
    for i in range(4):
        m = (reg == i) & (win[..., 3] >= 0)
        if not m.any(): per_region[REGIONS[i]] = None; continue
        b, wv = base[m], win[m]
        d = np.sqrt(((b[None] * PAL[:, None, :] / 255.0 - wv[None, :, :3]) ** 2).sum(-1))
        d = np.where(wv[None, :, 3] > 0, d, MISS_PENALTY).mean(1)
        cols[i] = int(d.argmin()); per_region[REGIONS[i]] = round(float(d.min()), 2)
    return cols, per_region

def fit(look_type, addons, wiki_gif):
    """For each wiki frame: align on the uncoloured (non-template) pixels, then optimise each template region
    independently over all 133 palette indices at that alignment. Keep the frame whose fitted render scores best,
    then report the global best-alignment score of the fitted colours."""
    reg, base = region_map(look_type, addons); h, w = reg.shape
    fixed = reg == 4
    fixed_img = np.zeros((h, w, 4), np.float32); fixed_img[fixed, :3] = base[fixed]; fixed_img[fixed, 3] = 255
    best = None
    for fi, f in enumerate(wiki_frames(str(wiki_gif))):
        _, dy, dx = _best_offset(fixed_img, f)
        cols, per_region = _fit_at(reg, base, _window(f, dy, dx, h, w))
        s = _best_offset(np.asarray(render(look_type, *cols, addons), np.float32), f)
        if best is None or s[0] < best[0]:
            best = (s[0], cols, per_region, (fi, s[1], s[2]))
    s, cols, per_region, align = best
    return {'head': cols[0], 'body': cols[1], 'legs': cols[2], 'feet': cols[3], 'score': round(s, 2),
            'align': {'frame': align[0], 'dy': align[1], 'dx': align[2]}, 'region_dist': per_region,
            'region_px': {REGIONS[i]: int((reg == i).sum()) for i in range(4)}}

TOL_REL, TOL_ABS, MIN_PX = 0.10, 1.0, 30  # a source colour within 10% (+1) of the best fits "as well"; <30 px never overrides

def region_costs(look_type, addons, gif):
    f = fit(look_type, addons, gif)
    reg, base = region_map(look_type, addons); h, w = reg.shape
    frame = wiki_frames(str(gif))[f['align']['frame']]
    fixed = reg == 4
    fixed_img = np.zeros((h, w, 4), np.float32); fixed_img[fixed, :3] = base[fixed]; fixed_img[fixed, 3] = 255
    _, dy, dx = _best_offset(fixed_img, frame)
    win = _window(frame, dy, dx, h, w)
    costs = {}
    for i in range(4):
        m = (reg == i) & (win[..., 3] >= 0)
        if not m.any():
            costs[REGIONS[i]] = None; continue
        b, wv = base[m], win[m]
        d = np.sqrt(((b[None] * PAL[:, None, :] / 255.0 - wv[None, :, :3]) ** 2).sum(-1))
        costs[REGIONS[i]] = np.where(wv[None, :, 3] > 0, d, MISS_PENALTY).mean(1)
    return costs, f

def arbitrate(look_type, addons, gif, source):
    costs, f = region_costs(look_type, addons, gif)
    out, why = {}, {}
    for r in ('head', 'body', 'legs', 'feet'):
        c = costs[r]
        if c is None or f['region_px'][r] < MIN_PX:
            out[r], why[r] = source[r], 'few_pixels'; continue
        best = int(c.argmin())
        if c[source[r]] <= c[best] * (1 + TOL_REL) + TOL_ABS:
            out[r], why[r] = source[r], 'source'
        else:
            out[r], why[r] = best, 'wiki'
    # the score of the colours actually returned, not of the fully fitted ones
    return out, why, round(score(render(look_type, out['head'], out['body'], out['legs'], out['feet'], addons), gif), 2)

def main(argv):
    if len(argv) >= 8 and argv[0] == 'arbitrate':  # D16: arbitrate <look_type> <addons> <gif> <head> <body> <legs> <feet>
        out, why, score_ = arbitrate(int(argv[1]), int(argv[2]), argv[3],
                                     dict(zip(('head', 'body', 'legs', 'feet'), map(int, argv[4:8]))))
        print(json.dumps({'colours': out, 'why': why, 'fit_score': score_})); return 0
    elif len(argv) >= 4 and argv[0] == 'fit':
        print(json.dumps({'look_type': int(argv[1]), 'addons': int(argv[2]), **fit(int(argv[1]), int(argv[2]), argv[3])}))
    elif len(argv) >= 8 and argv[0] == 'score':
        lt, h, b, l, f, ad = map(int, argv[1:7])
        s, al = score(render(lt, h, b, l, f, ad), argv[7], True)
        print(json.dumps({'look_type': lt, 'head': h, 'body': b, 'legs': l, 'feet': f, 'addons': ad, 'score': round(s, 2),
                          'align': {'frame': al[0], 'dy': al[1], 'dx': al[2]}}))
    elif len(argv) >= 8 and argv[0] == 'render':
        render(*map(int, argv[1:7])).save(argv[7]); print(json.dumps({'saved': argv[7]}))
    else:
        print('usage: outfit.py fit <look_type> <addons> <gif> | score <lt> <h> <b> <l> <f> <addons> <gif> | render <lt> <h> <b> <l> <f> <addons> <out.png>', file=sys.stderr); return 2
    return 0

if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
