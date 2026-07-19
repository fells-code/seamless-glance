---
"seamless-glance": minor
---

Add in-app AWS profile switching. Press `p` or run `profile` in the command palette to open a picker sourced from `~/.aws/config` and `~/.aws/credentials`, start on a profile with `--profile <name>`, or jump to one with `profile <name>`. The active profile shows in the header and is persisted between launches, and a selected profile is preserved across region changes.
