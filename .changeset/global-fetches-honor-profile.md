---
"seamless-glance": patch
---

Honor the active AWS profile in global and cross-region operations. Global (multi-region) EC2, RDS, and Lambda fetches, and cross-region describes, previously rebuilt SDK config off the default credential chain instead of the selected profile, so those views could silently read from the wrong account. They now route through the shared profile-aware config builder.
