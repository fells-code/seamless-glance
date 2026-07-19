---
"seamless-glance": patch
---

Make region discovery failures visible instead of silently collapsing to a single region. If `ec2:DescribeRegions` fails or is denied, the app previously fell back to just `us-east-1` and logged an invisible error, so every global view was quietly incomplete. It now falls back to a documented multi-region list and raises a visible warning that global views may be incomplete.
