---
"seamless-glance": patch
---

Stop silently wiping saved preferences when the config file cannot be read. A malformed `config.json` was swallowed and treated as absent, so the next region, profile, or theme change overwrote it with defaults and the original was gone. An unreadable config is now moved aside to a timestamped file, reported in the UI with the path it was kept at, and preference writes are refused entirely if it could not be preserved. An unrecognized theme now costs only the theme rather than being discarded along with the region and profile, and failures to write the config surface instead of being ignored. Adds a schema version so future changes can migrate rather than guess.
