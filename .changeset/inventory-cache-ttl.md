---
"seamless-glance": minor
---

Serve fresh-enough inventory from memory on navigation instead of refetching every view switch. Switching views previously re-hit AWS every time (Findings refetched around eleven services on each entry). Each fetched service inventory is now tracked with a short TTL keyed to the current profile and region; a view switch within that window serves the cached data with no AWS call, while a manual refresh (`r`) or a profile/region switch always refetches. The header now shows the data's relative age (for example "Last updated 14:32:10 UTC (2m ago)") so staleness is explicit.
