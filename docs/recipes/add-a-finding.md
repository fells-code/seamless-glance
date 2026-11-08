# Add a finding rule

A finding rule looks at data Glance has already fetched and reports resources
that need attention. Rules are pure functions, so you can write and test one
without any AWS access.

Reference pull request: #78 (a small rule change with tests). For a rule added
alongside a new service, see the `ecr_*` rules from #91.

## 1. Decide what the rule is

Answer these in your PR description:

- **Category:** `Incident` (broken now), `Waste` (costs money for little
  value), or `Hygiene` (risky or sloppy, will hurt later).
- **Severity:** `High` for incidents and real exposure, `Medium` for most
  waste and hygiene.
- **Next step:** one sentence telling the operator what to do.

## 2. Add a predicate to the model

Put the yes/no logic on the model in `src/models/<service>.rs`, not in the
rule. For example, `SqsQueueInfo::has_backlog_incident` in
`src/models/sqs.rs`. Add a unit test for it in the same file's `tests`
module, covering one case that should fire and one that should not.

If the rule uses a threshold, make it an associated constant (see
`SqsQueueInfo::HIGH_VISIBLE_THRESHOLD`) so it has a name and one home.

## 3. Write the rule

In `src/app/findings.rs`:

1. Add a `const` describing the rule, next to the other rules for the same
   service:

   ```rust
   const SQS_DLQ_HAS_MESSAGES: Rule = Rule {
       id: "sqs_dlq_has_messages",
       severity: FindingSeverity::High,
       category: FindingCategory::Incident,
       service: "SQS",
       route: FindingRoute::Sqs,
   };
   ```

   The `id` is permanent. Once released, never rename it, because saved
   acknowledgements are keyed by it.

2. Add a function that filters the data in `FindingContext` and returns one
   `Finding` per offending resource, using `Rule::resource`. Copy the shape of
   `ecr_no_lifecycle_policy`.

3. Add the function to the `FINDING_RULES` list.

If the data you need is not in `FindingContext` yet, the issue will say so and
tell you which field to add.

## 4. Test the rule

In the `tests` module at the bottom of `src/app/findings.rs`, add at least:

- a test where the rule fires on a resource that should be reported
- a test where it stays quiet on a healthy resource

Use the existing builder helpers (`sqs_queue`, `lambda`, `ctx`, and so on) and
add a new one if your model has none yet.

## 5. Document it

Add the rule to the "Current Findings" list in `docs/findings-roadmap.md`, and
remove it from the "Near-Term" or later sections if it was listed there. Then
add a changeset.
