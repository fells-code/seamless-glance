---
"seamless-glance": patch
---

Stop hiding access-denied and unavailable errors behind empty tables. The SQS, VPC, Load Balancers, Target Groups, Security Groups, Lambda, and API Gateway list views turned any fetch error into an empty table indistinguishable from "no resources". Each list fetcher now carries a status, and the view shows a denied or unavailable message instead of a misleading empty success state.
