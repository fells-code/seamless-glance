---
"seamless-glance": patch
---

Stop keys from acting on views where the footer does not offer them. Each action previously guarded its own scope, so a key whose handler forgot to check would fire anywhere: pressing the row-filter key on the account overview opened a filter prompt for a view that has no rows to filter and never advertises the key. The dispatcher now refuses any key outside the scope the footer builds its hints from, so what is advertised and what runs cannot disagree.
