---
"seamless-glance": patch
---

Scope the cost cache to the profile and region it was fetched under. The cache was a single global `cost.json` with no profile or region in the key and nothing invalidating it on switch, so after switching to profile B the app could show profile A's cached spend. The cache file is now keyed by profile and region, and the stored scope is re-checked on load, so one profile's financial data is never shown under another.
