---
"seamless-glance": patch
---

Stop rendering Cost Explorer failures as $0 spend. A denied or throttled billing account previously showed a zeroed cost overview and a flat chart, indistinguishable from a genuinely cheap account. The cost overview and cost savings views now show an unavailable state instead, and a failed fetch is no longer written to the cost cache (which would otherwise persist the misleading $0 for the cache lifetime).
