# Tibia manual notes: forum

- Source: <https://www.tibia.com/gameguides/?subtopic=manual&section=forum>
- Capture: owner, 2026-09-28 (tibia.com blocks cloud containers and CI runners)
- Clean-text SHA-256: `3636861616b3c92761b03d94ec81409c8a28839aa2fb4035e5eefb0bac617bbb`
- Full private text: Jira `KAN-33`, attachment `tibia-manual-2026-09-28.txt`
- Evidence class: `CIPSOFT_OFFICIAL` / `PRIMARY_OFFICIAL`
- Domain: Platform / web, not Game runtime; kept for completeness
- These are Oteryn-written notes, not the manual text. Cite as `tibia.com manual §forum <heading>, capture 2026-09-28`.

## 9.1 How to Use the Forum: Board Rights and Posting

**Forum structure** [platform]: Boards (sections) subdivided into threads (discussion topics). Threads marked as New, Hot (16+ posts), or Closed [platform]

**Access control** [platform]: Read unrestricted; write requires login and minimum character level (Level 2 if paid account history, Level 21 if f2p) [platform]

**Post composition** [platform]: Selectable character, required subject (for new threads), optional icon, up to 4096 characters, signature/URL/smiley options [platform]

**Post options** [platform]: Automatic URL parsing, smiley rendering, signature display toggleable per post [platform]

## 9.1 Replace Code (Forum Markup)

Forum text supports HTML-like bracket codes:

**Links** [platform]:
- `[url]URL[/url]` or `[url="URL"]label[/url]` (security-restricted on public boards) [platform]
- `[tibia=https://...]text[/tibia]` (tibia.com internal links) [platform]
- `[thread=XXX]title[/thread]`, `[post=XXX]title[/post]` (forum self-references) [platform]
- `[news=XXX]title[/news]` (news archive links via news ID) [platform]
- `[email]address@example.com[/email]` (disabled on public boards) [platform]

**Formatting** [platform]:
- `[b]bold[/b]`, `[i]italic[/i]`, `[u]underline[/u]` [platform]
- `[list]...[*]items[/list]` (unordered), `[list=1]...[/list=1]` (ordered numbers), `[list=a]...[/list=a]` (letters) [platform]
- `[code]...`[/code]` (monospace) [platform]

**Special references** [platform]:
- `[player]character[/player]` (character info link) [platform]
- `[guild]name[/guild]` (guild info link) [platform]
- `[img]URL[/img]` (disabled on public/guild boards) [platform]

## 9.1 Forum Rules of Conduct

Public forum subject to Tibia Rules; offensive statements, spam, off-topic/advertising, rule-incitement are violations [platform]. Banished players cannot start threads or reply [platform]

## 9.2 The Boards

**World Boards** [platform]: Per-world discussion; accessible even without character on world [platform]

**Trade Boards** [platform]: Per-world trading convenience [platform]

**Community Boards** [platform]: Off-topic (real-life, non-game topics), role-playing fiction, fantasy writing, meetup coordination [platform]

**Gameplay Board** [platform]: Quest help, achievement discussion, cross-world gameplay topics [platform]

**Role Playing Board** [platform]: Creative fantasy writing and storytelling [platform]

**Real Life Board** [platform]: Non-Tibia topics [platform]

**Convention Board** [platform]: Real-life meetup organization [platform]

**Proposals Board** [platform]: Feature suggestions; feedback welcome but not guaranteed response [platform]

**Auditorium Board** [platform]: CipSoft-moderated feedback on game topics, polls, news commentary [platform]

**Events Board** [platform]: Event announcements, signups, organization discussion [platform]

**Council Board** [platform]: Reserved for CipSoft + selected players (new feature discussion) [platform]

**Support Boards** [platform]:
- Payment Support (orders, payments, billing) [platform]
- Technical Support (client bugs, connection issues) [platform]
- Help (game-related questions) [platform]

**Guild Boards** [platform]: Private guild discussion if guild leader creates; auto-close after 30 days inactivity; deletion irreversible [platform]

**CM Post Archive** [platform]: Searchable community manager posts [platform]

---

## Open questions for Oteryn

- Should Oteryn support per-guild private boards, or use in-game guild chat only?
- Will Oteryn forum use same authentication as game accounts, or separate registration?
- Should Oteryn enforce minimum-level posting requirements, or allow free-to-play posting immediately?
- How should Oteryn handle cross-language/international forums (single board vs. per-language)?
- Should forum moderation be community-driven (player moderators) or CipSoft-only?
