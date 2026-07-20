---
"seamless-glance": patch
---

Fix Describe and Console doing nothing on the SQS view. The footer advertised `d` and `o` on every service view, but SQS was only wired up for `c`, so the other two keys fell through to a catch-all and silently no-opped. Describe, open, and CLI now resolve the selected row through one shared registry that every view is registered in, so a view cannot support one of these actions and quietly miss the others.
