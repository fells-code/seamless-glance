---
"seamless-glance": minor
---

Keep the dashboard responsive during a refresh and show real per-service progress. The AWS refresh was awaited inline in the event loop, so the UI could not redraw or accept input while it ran (a frozen dashboard) and the per-service progress phases never rendered. Refresh now runs on a background task that streams results back over a channel: the loop keeps drawing and handling input, the loading overlay advances through each service as it loads, and you can switch views or regions mid-refresh (which supersedes the in-flight fetch).
