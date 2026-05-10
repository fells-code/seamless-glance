# Navigation Strategy

## Purpose

This document describes the preferred navigation direction for Seamless Glance as the app grows beyond a small fixed set of service views.

The current keyboard model works for a compact service list, but the long-term product direction requires a more scalable navigation system.

## Current State

Today the app uses a mix of:

- one-key view shortcuts such as `f`, `1` through `9`
- a slash command palette
- in-view actions such as `d`, `c`, and `o`

This is effective for a small number of screens, but it will not scale well as the app adds more AWS services and more triage-specific views.

## Design Goals

Navigation should optimize for:

1. fast access to the most important information
2. predictable movement between findings and service detail
3. discoverability for new services
4. minimal cognitive load
5. keyboard-first operation

## Recommended Direction

### Findings First

The app should increasingly treat Findings as the operator home screen.

Reasoning:

- operators usually start from “what needs attention”
- findings provide a natural bridge into service detail
- this reduces the need to memorize a growing list of service shortcuts

Long-term expectation:

- Findings becomes the default mental hub
- service views become drill-down destinations

### Command Palette First

The slash command palette should become the primary way to jump directly to a service or feature.

Desired palette evolution:

- fuzzy matching
- service aliases
- grouped results
- recent destinations
- future action commands, not just view jumps

Examples:

- `/findings`
- `/ec2`
- `/security-groups`
- `/volumes`
- `/logs`

### Numeric Shortcuts Become Transitional

The current numeric shortcuts should be treated as a legacy convenience, not the long-term navigation model.

Why:

- numbers do not scale as services grow
- numbers are hard to discover without help text
- changing the mapping later can be disruptive

Recommended strategy:

- keep current numeric shortcuts for now
- avoid expanding the numeric set much further
- shift new service discoverability to the command palette
- eventually reduce dependence on numeric destination bindings

## Suggested Navigation Layers

### Layer 1: Global Navigation

Always available:

- `f` Findings
- `/` command palette
- `?` help
- `r` refresh
- `q` quit

### Layer 2: Context Navigation

Context-sensitive:

- `Enter` open related service from Findings
- `d` describe selected item
- `c` AWS CLI handoff
- `o` AWS console handoff

### Layer 3: Service Browsing

For direct browsing when the operator is not starting from a finding:

- command palette service jump
- optional future grouped service browser

## Proposed Future Enhancements

### Grouped Service Browser

Instead of flat numeric jumps, consider a grouped browser by domain:

- Compute
- Networking
- Data
- Messaging
- Security
- Observability
- Storage

This would scale better than one-key service mappings and teach the shape of the AWS account more naturally.

### Recent And Frequent Destinations

Useful future behavior:

- recently opened service views
- recently opened findings routes
- operator-pinned favorite services

### Filtered Findings Navigation

Future findings navigation should likely include:

- category filters
- severity filters
- region filters
- service filters

This reduces the need for direct raw-service navigation in many workflows.

## Recommended Near-Term Navigation Plan

1. Keep `f` as the Findings shortcut.
2. Keep existing numeric shortcuts as transitional compatibility.
3. Add new services primarily to the command palette rather than to numeric slots.
4. Avoid growing the one-key destination map aggressively.
5. Evolve the help text and README to emphasize Findings and the command palette over numeric memorization.
6. Consider a future grouped service launcher once service count grows meaningfully.

## UX Principle

The operator should rarely need to remember a long keyboard map.

Good navigation should make the next move obvious:

- start with Findings
- jump to detail with `Enter`
- pivot with `d`, `c`, or `o`
- jump elsewhere with `/`
