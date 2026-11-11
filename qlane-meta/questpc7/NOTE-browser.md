# Real browser access (owner's Chrome on the Windows PC)

- `/home/mole/oteryn-cache/obrowse URL` prints page title, final URL+status and
  the rendered text (add `--html` for HTML). Real Chrome, passes Cloudflare.
  Serialized with flock (one page at a time, up to ~60 s each). Use it for
  tibiawiki.com.br, tibia.com, reddit.com, youtube.com search pages.
- Exit 3 + `CHALLENGE_NOT_PASSED` = site still challenging; skip that source,
  use another (Fandom via wikiapi.py, tibiawiki.com.br via obrowse) and go on.
- YouTube subtitles: `/home/mole/.local/bin/yt-dlp --skip-download
  --write-auto-subs --sub-langs en --sub-format vtt -o '/tmp/yt/%(id)s' URL`.
- Never stop on missing web evidence: record the source gap and continue.
