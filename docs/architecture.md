# Architecture

## Overview

Seamless Glance is a single-binary Rust TUI application. Its control flow is straightforward:

1. parse CLI flags
2. ensure a license exists and validate it
3. initialize terminal state and AWS clients
4. load cached cost data
5. enter the event loop
6. refresh overview and active-view data on demand
7. render the current screen with ratatui

The codebase is intentionally organized around one central `App` state object and a set of service-specific fetch modules.

Today that architecture is better at inventory and summaries than triage. A likely next evolution is to add a dedicated findings layer that consumes the existing AWS fetchers and turns raw resource data into prioritized operator actions.

That findings layer now exists in an initial form and is intentionally lightweight: it derives a small set of triage findings from already-fetched overview and security-group data, then routes operators into the relevant service views.

## Module Ownership

### `src/main.rs`

Owns:

- CLI entrypoint
- help and version output
- license gating before TUI startup
- crossterm alternate-screen lifecycle
- keyboard event handling

### `src/app/mod.rs`

Owns:

- `App` state
- active view selection
- region selection and persistence
- refresh orchestration
- cached cost loading
- resource describe/open/SSH actions
- overlay state

This is the highest-leverage file in the app. Many user-visible workflows converge here.

### `src/aws/`

Owns service integration and data fetching. Common patterns:

- build or reuse AWS SDK clients
- fetch raw AWS data
- map AWS responses into local model structs
- handle access denied or unavailable states where the view expects status feedback

This layer should remain primarily factual. As the product shifts toward triage, avoid burying prioritization logic deep inside fetchers unless it is tightly coupled to the AWS response itself.

Representative files:

- [`src/aws/account.rs`](/Users/brandoncorbett/git/seamless-glance/src/aws/account.rs): account overview fan-out
- [`src/aws/clients.rs`](/Users/brandoncorbett/git/seamless-glance/src/aws/clients.rs): shared SDK client bundle
- [`src/aws/cost.rs`](/Users/brandoncorbett/git/seamless-glance/src/aws/cost.rs): cost explorer and budget queries
- [`src/aws/ec2.rs`](/Users/brandoncorbett/git/seamless-glance/src/aws/ec2.rs): EC2 inventory and global aggregation

### `src/models/`

Owns app-facing types used by both fetchers and UI views:

- resource rows
- summary cards
- finding rows and finding routing metadata
- service status types
- traits such as resource description and console linking behavior

### `src/ui/`

Owns presentation concerns:

- screen layout
- headers and footers
- shared navigation metadata and command palette rendering
- theme definitions and theme switching presentation
- individual views
- help and overlay rendering
- terminal suspend/resume helpers used for shell execution

### `src/resources/`

Owns helpers shared across domains:

- multi-region aggregation
- SSH command context construction

This area is a good home for future shared action helpers such as CLI command generation or resource-specific operator pivots.

### `src/cache/`

Currently used for cost caching. Cost data is loaded outside the main refresh loop so the app can feel responsive while still surfacing recent billing information.

### `src/license/`

Owns:

- license path resolution
- trial license creation
- paid license verification
- status output

License validation happens before the TUI is allowed to start.

## Runtime Data Flow

### Startup

At startup, `main`:

- handles `--help`, `--version`, and `--license-status`
- ensures a license file exists
- validates the license
- loads config from `~/.seamless-glance/config.json`
- loads persisted theme preference from `~/.seamless-glance/config.json`
- fetches enabled AWS regions
- creates `AwsClients` for the selected region
- constructs `App`
- preloads cost data
- triggers the first refresh

### Refresh Flow

`App::trigger_refresh` marks refresh intent, and `App::refresh_active` performs the work:

1. refresh account overview first
2. refresh the active view's service data
3. clear loading state
4. update `last_refresh`
5. reset selection and scroll state

This design keeps the header accurate even when a service view is active.
It also allows the TUI to render a loading state between view transitions instead of blocking silently on inline fetches.

### View Entry Flow

`App::on_view_enter` now schedules a refresh and resets selection state when the user changes screens, rather than fetching inline. The actual AWS work happens in `refresh_active`, which keeps the event loop free to draw an intermediate loading overlay.

### Navigation Flow

Navigation is currently shared across three layers:

1. direct key shortcuts such as `f` and the transitional `1` through `9`
2. a slash command palette with grouped service metadata and aliases
3. `Tab` / `Shift+Tab` cycling through major views

The command palette, help overlay, and header/footer cues should prefer shared navigation metadata rather than hand-maintained duplicated strings.

### Action Flow

User actions typically route through `App` helpers:

- `trigger_describe`
- `trigger_cli`
- `trigger_open`
- `trigger_ssh`
- `open_selected_finding`

Those helpers pull the selected row, derive a description request, CLI command, or console URL, and open an overlay or external browser action as needed.

For region-aware resources, action handlers should prefer the row's own region over the currently selected UI region. This matters most in global aggregation views, where a generic fallback region can otherwise produce misleading console links or failed describe calls.

### Region Model

The app maintains a list of enabled AWS regions plus a synthetic `global` slot.

Important nuance:

- the global slot is a UI concept, not an AWS SDK region
- only some service fetchers currently aggregate across all enabled regions
- EC2, Lambda, and RDS implement global aggregation today

When expanding global behavior, keep the user-facing label honest and update the docs.

This is especially important for a triage tool because incorrect region context breaks trust in both console and CLI pivots.

### Error And Status Handling

The codebase uses a mix of strategies:

- some views convert permission problems into `ServiceStatus`
- some fetchers log errors and return empty collections
- account overview fans out with concurrent calls and composes summary status

This is workable, but it means contributors should be careful not to turn AWS failures into misleading empty-state success UIs.

## Extension Pattern

To add a new service view cleanly:

1. create or extend the model types
2. add a fetcher under `src/aws/`
3. extend `AwsClients` if a new SDK client is needed
4. add app state fields and refresh hooks in `src/app/mod.rs`
5. add a `src/ui/views/*.rs` renderer
6. wire navigation and command palette support
7. update docs

To add a future triage or waste feature cleanly:

1. keep resource collection in `src/aws/`
2. derive findings from collected facts in a dedicated layer rather than directly in view code
3. attach operator actions to each finding
4. make severity, category, and next step obvious in the UI

The initial findings implementation follows that pattern with:

- a `Finding` model in `src/models/finding.rs`
- derived findings stored in `App`
- a dedicated `Findings` view that routes into related service screens

## Current Gaps

- automated tests are minimal
- global aggregation support is incomplete across services
- service coverage is still missing several high-value AWS domains like EBS, Elastic IPs, CloudWatch Logs, and S3
- the findings backlog is still much smaller than the intended triage and waste catalog
- refresh and error handling conventions are not fully uniform across all fetchers

Those are good places to tighten over time, but any changes should preserve the current operator-friendly speed and simplicity.
