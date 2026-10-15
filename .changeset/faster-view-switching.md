---
"seamless-glance": patch
---

Make startup and view switching faster. Switching views no longer refetches the whole account overview (about ten service fetches) when it is still fresh, and when the overview is needed it now loads alongside the view's data instead of ahead of it. The header keeps showing the account during a refresh instead of blanking. Startup reads cost data from the local cache only, so the first frame no longer waits on Cost Explorer, and the overview's Lambda count no longer makes one tag lookup per function. Refreshing Findings no longer fetches every service twice: the account overview is now built from the inventories Findings already loads, with only the caller identity and ECS fetched on top.
