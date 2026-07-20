---
"seamless-glance": patch
---

Rename the misspelled API Gateway model module and render function to match the rest of the codebase, and stop the account overview from reporting a hardcoded row count. That count was never read, because the account overview free-scrolls a fixed layout and clamps its own offset while rendering, so it was a number that could only drift from the rows actually drawn.
