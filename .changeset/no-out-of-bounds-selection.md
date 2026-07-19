---
"seamless-glance": patch
---

Make an out-of-bounds selection impossible. The Cost Savings view was the last list that clamped its own selection by hand, and the Findings and Cost Savings detail panes indexed the list directly by the selected row, which would panic if the selection ever outran a list that shrank. Cost Savings now renders through the shared list helper, so every list view clamps in one place, and both detail panes read the selected row fallibly.
