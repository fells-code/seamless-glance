---
"seamless-glance": patch
---

Stop the footer and help screen from advertising keys that do nothing. Key handling, the footer hints, and the help screen were three separate hand-maintained lists that could disagree, so views such as Account Overview and Cost Overview offered Describe, CLI, and Console hints even though those keys do nothing there. All three now read one key registry, so the footer only shows keys that are actually bound and meaningful on the current view, and help always matches the real bindings.
