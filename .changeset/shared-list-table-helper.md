---
"seamless-glance": patch
---

Fix Security Groups not scrolling, and make selection behave consistently across every list view. The Security Groups view rendered every row unwindowed with no scroll handling, so selecting past the bottom of the screen moved an invisible cursor. All list views now render through one shared table renderer that owns scrolling, selection clamping, and the empty state, so scrolling works everywhere and the selected row, headers, and empty states look the same in every view.
