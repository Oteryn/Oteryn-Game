"""Reproduce M11's source-pinned Stricken Soul programme (offline)."""
import argparse
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
OUTPUT = ROOT / "content/dialogues/quest-rules/quest-batch3-engnpc2-dream-courts.json"
QUEST = "oteryn:quest.the_dream_courts_quest"
TRACK = "oteryn:quest-progress/quest/u12_00/the_dream_courts/haunted_house/questline"
SOURCE = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/stricken_soul.lua:"
TEXT = {'greet_new': 'This place is... haunted... heed my warning... there are... ghooooooosts here...! '
              'Why are you giving me that... look? I am certain, there aaaaaaare ghosts here - '
              "I've seen them! Do you believe me?",
 'greet_done': 'Gree... tings.',
 'ask_name': 'Excellent... I hope they will haaaaaaunt my house no longer. What was your... '
             'naaaaaame again, tell me?',
 'ask_help': 'Yeeeees... you need to help meeeeeee. I want those ghosts gone... this is my home '
             'and I need it to teach my students. Will you take care of the... ghosts?',
 'accepted': [' Ah yeeeeees, |PLAYERNAME|. I will remember you. Now, lessons are every day in the '
              'morning and once a week in the evening... ...',
              "Oh, you're not here for this, are you? So about the ghoooosts, yes. You seeeee, "
              'there are 3 secret passages here. ...',
              'Thiiiiis is no ordinary house... it is a nexus, a gateway to a once hidden '
              'cathedral. Sheltering a small and peaceful society of scholars and monks. Secluded '
              'from every distraction. ...',
              'I was one of them and ordered to hold contact to the outside woooorld. But then, '
              'something... happened. ...',
              'Outsiders managed to sneak in, infiltrate and influence the society... for the '
              'worse. Who knows for what ends. They chaaaaanged... ...',
              'Shortly after, contact was lost... the nexus broken and sealed, ghosts appeared... '
              'eeeeeeeverywhere. ...',
              'Find the three passages... one is right here in the cellars, one in the jungles of '
              'Tiquanda and one in the deserts of Darama. ...',
              'Restore their connection and open this nexus to access the buried cathedral and '
              'find the cause to this... eliminate all remainders there if you must, '
              '|PLAYERNAME|.'],
 'fallback': "Sorry, I didn't understand."}


def document():
    def rule(key, line, reply=(), effects=(), **selectors):
        return dict(key="oteryn:quest-npc/the_dream_courts/stricken_soul/" + key,
                    quest=QUEST, npc="oteryn:npc.stricken_soul", trigger="say",
                    guards=[], reply=list(reply), effects=list(effects), sources=[SOURCE + line],
                    **selectors)
    def storage(op):
        return [dict(kind="storage", track=TRACK, op=op, value=1)]
    rules = []
    for key, op, line in [("greet_new", "lt", "52-53"), ("greet_done", "ge", "54-55")]:
        r = rule(key, line, [TEXT[key]])
        r.update(trigger="greet", guards=storage(op))
        rules.append(r)
    rules.append(rule("ask_name", "72-74", [TEXT["ask_name"]],
                      [dict(kind="set_topic", value=3)], keywords=["yes"], topics=[2]))
    r = rule("ask_help", "75-77", [TEXT["ask_help"]],
             [dict(kind="set_topic", value=2)], keywords=["yes"])
    r["guards"] = storage("lt")
    rules.append(r)
    rules.append(rule("yes_noop", "71-78", keywords=["yes"]))
    rules.append(rule("name_grant", "79-93", TEXT["accepted"],
                      [dict(kind="set", track=TRACK, value=1), dict(kind="set_topic", value=0)],
                      match_player_name=True, topics=[3], exclude_keywords=["yes"]))
    rules.append(rule("name_noop", "79-94", match_player_name=True, exclude_keywords=["yes"]))
    rules.append(rule("fallback", "95-98", [TEXT["fallback"]],
                      [dict(kind="set_topic", value=0)], any_message=True))
    return dict(schema=1, quest=QUEST, source_files=[dict(
                path="data-global/npc/stricken_soul.lua",
                revision="201189f871d23597b82caf88fd611454a2ddefb8",
                sha256="506efb1fc93172d5a9fef91360686e1cb080d6830897d44e5cb8dfd86ea8b39c")], tracks=[dict(key=TRACK, quest=QUEST, initial=-1,
                min=-1, max=2147483647, sources=[SOURCE + "52-93"])], rules=rules)


def render():
    return json.dumps(document(), indent=2, ensure_ascii=False) + "\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    expected = render()
    if args.check:
        if OUTPUT.read_text() != expected:
            raise SystemExit("Stale M11 Stricken Soul programme: run engnpc2_player_name.py")
    else:
        OUTPUT.write_text(expected)


if __name__ == "__main__":
    main()
