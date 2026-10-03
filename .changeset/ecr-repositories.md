---
"seamless-glance": minor
---

Add an ECR view (`ecr`, or `registry`/`images`/`repositories` in the command palette) listing each repository's image count, untagged images, stored size, latest push and recorded pull, and lifecycle policy. Three new findings come with it: repositories holding 20 or more untagged images (waste), repositories with no push in 180 days and no pull in 90 days (waste), and repositories with images but no lifecycle policy (hygiene). ECR repositories also join the missing `Owner` tag finding. Describe, console, and CLI pivots land on the selected repository. The view is region-scoped and needs `ecr:DescribeRepositories`, `ecr:DescribeImages`, `ecr:GetLifecyclePolicy`, and `ecr:ListTagsForResource`.
