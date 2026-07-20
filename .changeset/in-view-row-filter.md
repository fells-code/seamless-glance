---
"seamless-glance": patch
---

Add an in-view row filter. Press `m` on any list view to narrow it to rows matching a query, with a live match count and the selection following as you type. Enter keeps the filter, Esc clears it, and switching views clears it too, so a filter never silently hides rows in a view it was not typed in. Filtering is case-insensitive substring matching over the fields each view shows. Describe, open, CLI, and Enter all act on the row that is actually highlighted, and the wrapped detail panes on findings, cost savings, and cost overview follow the filtered rows and report their position within them.
