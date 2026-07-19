---
"seamless-glance": minor
---

Add an in-UI notification surface so failed actions no longer vanish. Errors were written with `eprintln!` to a stderr hidden behind the alternate screen, so the operator never saw them. Failed browser-open actions and unknown region, profile, or theme commands now show a transient toast that auto-dismisses after a few seconds.
