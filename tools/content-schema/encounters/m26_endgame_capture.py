"""Bounded source facts for M26; these helpers never qualify runtime hooks."""
import hashlib
import re


def duration_ratio(expression):
    match = re.fullmatch(
        r'''ParseDuration\(\s*(["'])([0-9]+)([smhd])\1\s*\)\s*/\s*ParseDuration\(\s*(["'])1s\4\s*\)''',
        expression.strip(),
    )
    if match is None:
        return None
    value = int(match[2]) * {'s': 1, 'm': 60, 'h': 3600, 'd': 86400}[match[3]]
    if value > 2**32 - 1:
        raise ValueError('source duration outside supported bounds')
    return value


def last_policy(config, name, default, number):
    matches = list(re.finditer(r'\b' + re.escape(name) + r'\s*=\s*([^,\n}]+)', config))
    return number(matches[-1][1]) if matches else default


def empty_monsters(value):
    return value is None or re.fullmatch(r'\{\s*\}', value) is not None


SOUL_PATH = 'data-global/lib/quests/soul_war.lua'
SOUL_SHA = '89c0024914b0593b9b7d2bfb535a24025a552f54b158319697cdf56a66375e75'
SOUL_CONFIGURATIONS = ('goshnarsSpite', 'goshnarsMalice', 'goshnarsGreed',
                       'goshnarsHatred', 'goshnarsCruelty', 'goshnarsMegalomania')


def soul_configurations(raw, table):
    if hashlib.sha256(raw).hexdigest() != SOUL_SHA:
        raise ValueError('M26_SOUL_SOURCE_CHANGED')
    text = re.sub(r'--\[\[.*?\]\]', '', raw.decode(), flags=re.S)
    text = re.sub(r'--[^\n]*', '', text)
    levers = table(text, r'\blevers')
    if levers is None:
        raise ValueError('M26_SOUL_LEVERS_MISSING')
    result = []
    for name in SOUL_CONFIGURATIONS:
        config = table(levers, r'\b' + name)
        constructor = r'local\s+' + name + r'Lever\s*=\s*BossLever\(SoulWarQuest\.levers\.' + name + r'\)'
        registration = name + r'Lever:position\(SoulWarQuest\.levers\.' + name + r'Position\)'
        if config is None or len(re.findall(constructor, text)) != 1 or len(re.findall(registration, text)) != 1:
            raise ValueError('M26_SOUL_LEVER_BINDING_MISSING:' + name)
        result.append((name, config))
    if len(re.findall(r'\bBossLever\s*\(', text)) != len(result):
        raise ValueError('M26_SOUL_UNCAPTURED_LEVER')
    return result


ENDGAME_PATHS = (SOUL_PATH,) + tuple(
    'data-global/scripts/quests/rotten_blood_quest/rotten_' + boss + '.lua'
    for boss in ('bakragore', 'bakragore1', 'chagorz', 'ichgahal', 'murcion', 'vemiath')
)

SPITE_MONSTER_PATH = 'data-global/monster/quests/soul_war/goshnars_spite.lua'
SPITE_MONSTER_BLOB = '1daedf2a19178f90bc7f6ea6041f8f3fd1bacef6'
SPITE_MECHANICS_PATH = 'data-global/scripts/quests/soul_war/soul_war_mechanics.lua'
SPITE_MECHANICS_BLOB = '51d42e7e970a2f24fc8438a833aacac2c061dfc2'


def spite_hooks(root, table):
    path = root / SPITE_MONSTER_PATH
    if not path.is_file():
        return [], False
    raw = path.read_bytes()
    blob = hashlib.sha1(b'blob '+str(len(raw)).encode()+b'\0'+raw).hexdigest()
    if blob != SPITE_MONSTER_BLOB:
        return [], False
    path = root / SPITE_MECHANICS_PATH
    if not path.is_file():
        return [], False
    mechanics = path.read_bytes()
    if hashlib.sha1(b'blob '+str(len(mechanics)).encode()+b'\0'+mechanics).hexdigest() != SPITE_MECHANICS_BLOB:
        return [], False
    if (b'CreatureEvent("SoulWarBossesDeath")' not in mechanics
            or b'function bossesDeath.onDeath(' not in mechanics
            or b'bossesDeath:register()' not in mechanics):
        return [], False
    text = re.sub(r'--[^\n]*', '', raw.decode())
    events = re.findall(r'"([^"\n]+)"', table(text, r'\bmonster\.events') or '')
    if events != ['SoulWarBossesDeath'] or re.search(r'\bmType\.on\w+\s*=',text):
        return [], False
    return events, True


ROTTEN_SOURCE_SHA256 = {'data-global/scripts/quests/rotten_blood_quest/rotten_bakragore.lua': '821b49caf5e1c55f8a0e9a83b5f1158e5f34d3b3876681c5d9adfb6eedd96189', 'data-global/scripts/quests/rotten_blood_quest/rotten_bakragore1.lua': 'f6c8fc46dfde8d43bec59b0a2355871afbd19e6cb745f3096456d40c2bdc59a1', 'data-global/scripts/quests/rotten_blood_quest/rotten_chagorz.lua': 'd3f632880a7e871e0303994905ef3e814b22d6f797e51b732d4ce7f7c49050c2', 'data-global/scripts/quests/rotten_blood_quest/rotten_ichgahal.lua': '0822f2c430b389aa74bc3109db975e2bb96cd69385e6fc8fe26991b9b69f9b7c', 'data-global/scripts/quests/rotten_blood_quest/rotten_murcion.lua': '48f572ec9c418340bb07490334a216e81f4361627495e5c221d2a2d45b71d727', 'data-global/scripts/quests/rotten_blood_quest/rotten_vemiath.lua': '9b163af79214effbb5e9d2bd3d54290c266b988b33a89d809a70563e24c0c6c5'}

def verify_source_file(path, raw):
    expected = SOUL_SHA if path == SOUL_PATH else ROTTEN_SOURCE_SHA256.get(path)
    if expected is None or hashlib.sha256(raw).hexdigest() != expected:
        raise ValueError('M26_SOURCE_SUBSTITUTION:' + path)
