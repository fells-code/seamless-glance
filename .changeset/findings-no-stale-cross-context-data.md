---
"seamless-glance": patch
---

Stop findings from showing a prior region or profile's data. Refreshing a single-service view fetched only that service but still rebuilt findings from the other services' leftover inventories, then stamped them with the current region label, so findings could display old-account or old-region data under the wrong region. The app now tracks the account context its inventory was fetched under and clears stale data when the profile or region changes, so findings are only built from current-context data and labeled with the region they were computed in.
