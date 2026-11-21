# Account security revocation stream fixtures

Source: Platform `a85b5da`, owner-approved P1-a contract packet (2026-11-13).
`request.json`, `page.json` and `empty.json` preserve the supplied bytes, without
terminal newlines. The social `platform_streams` golden test checks the production
request encoder and closed page decoder against these files.
