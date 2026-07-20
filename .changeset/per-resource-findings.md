---
"seamless-glance": patch
---

Findings now name the individual resource they are about instead of collapsing into one counted line per rule. A row reads "Instance web-1 averaged 2.1% CPU over the last 14 days" rather than "4 running EC2 instance(s) averaged below 5.0% CPU: web-1, web-2, api-3 (+1 more)", and the findings table gains a RESOURCE column. Each finding carries a stable identity that survives a refresh, so a repeated resource is reported once. Adds a LOW severity for pure hygiene findings, and sorts categories by urgency so incidents come ahead of waste and hygiene, which alphabetical ordering had reversed. EC2, RDS, and Lambda findings are now labelled with the resource's own region rather than the selected view's, which matters when the global region is selected.
