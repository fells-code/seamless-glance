---
"seamless-glance": patch
---

Mark values the table had to cut and give every list view a way to read them in full. Long values such as secret paths, ARNs, and joined signal lists were silently clipped at the column edge with no indication anything was missing; they now end in an ellipsis, and `w` expands the selected row to show every column's full value. Wrap mode previously worked on only three of sixteen views, so on the rest a truncated value was unreadable anywhere. A view emptied by a filter now says so and offers a way to clear it, instead of reporting that the region has no such resources.
