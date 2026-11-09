# T004/O002: authenticated operator bulletin

ServerAccepted optional field 21 carries UTF-8 JSON, at most 256 KiB, with the
same news/events document consumed by the packaged client bulletin. Fields 11–20
remain reserved. Older peers skip field 21. It is presentation data and changes
no capability, session identity, runtime scope, gameplay state or admission rule.

The Game node explicitly configures `OTERYN_BULLETIN_FILE` to a local operator
publication. No default path, HTTP service, commercial integration or provider
credential is created. The file is read again after each successful fresh
admission, so operator replacements appear on the next sign-in without a server
or client release. Publish by writing a temporary file and renaming it over the
configured file. The source permits only one in-flight read, reads at most
256 KiB + 1, and waits at most 200 ms. Missing, oversized, malformed or unavailable
publication data omits the optional field and never prevents admission.

The root object contains exactly `news` and `events`, each an array of at most
64 entries. News entries contain exactly nonempty `title` and `body` strings,
each at most 4096 UTF-8 bytes and without NUL. Events have the same strings plus
integer `year` (1–9999), `month` and `day`, forming a valid Gregorian date.
Example:

```json
{"news":[{"title":"Operator notice","body":"A new publication"}],"events":[{"year":2027,"month":1,"day":12,"title":"Gathering","body":"Meet at the town square"}]}
```

No bulletin is read or emitted for a refused admission. Session preserves a
bounded valid UTF-8 document after checking the ordinary acceptance; malformed
optional metadata is ignored. The presentation parser validates the complete
news/events document before replacing the previous feed. News and month/year
browsing use the existing login controls. Ctrl+Alt+Shift+L also opens those
controls while playing; connected forms cannot submit another login. Publication
refresh occurs on sign-in, not through an additional gameplay command.
