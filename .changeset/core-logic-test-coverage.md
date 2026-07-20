---
"seamless-glance": patch
---

Fix cost data failing to load on the first of each month. Cost Explorer rejects a time period whose start and end are the same day, which is exactly what the month-to-date window collapsed to on the first, so every cost view reported the service as unavailable for that day. The window now covers the current day instead of collapsing. Found by adding test coverage for the cost time-bucket logic, which previously had none.
