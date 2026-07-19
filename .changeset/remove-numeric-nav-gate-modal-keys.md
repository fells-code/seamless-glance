---
"seamless-glance": minor
---

Remove numeric view-switching and stop navigation keys from firing while a modal is open. The digit keys `1`-`9`,`0` switched views via an arbitrary, non-mnemonic mapping, fired inconsistently while overlays were open, and blocked using digits for in-view input. Views are now reached through the command palette (`/name` or an alias) plus `f` (Findings) and `Tab` / `Shift+Tab`. All navigation and resource-action keys are gated behind a single "modal open" predicate, so region switching, view switching, describe, open, and SSH no longer mutate state while the command palette, help, or an overlay is up. Digits `1` and `2` still pick an SSH command when the key-selection overlay is open.
