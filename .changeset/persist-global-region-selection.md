---
"seamless-glance": patch
---

Restore the global region slot on restart. The selection was saved as the `global` sentinel, but startup only matched saved regions against real region names, so `global` matched nothing and silently fell back to the first region. Startup now recognizes the `global` sentinel and restores the global slot, so operators who work globally keep the setting across launches.
