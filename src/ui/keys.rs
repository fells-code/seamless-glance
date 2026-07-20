//! One registry for the keys the app binds.
//!
//! The real key handler, the footer hints, and the help screen all read this
//! table, so an advertised binding cannot drift from the one that actually runs.
//! Adding a key means adding one entry here plus one arm in the dispatcher.

use crate::app::ActiveView;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum KeyAction {
    Findings,
    CommandPalette,
    Help,
    Refresh,
    Quit,
    CycleTheme,
    SwitchProfile,
    GlobalRegion,
    ToggleWrap,
    Describe,
    Cli,
    OpenConsole,
    Ssh,
    Filter,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum KeyGroup {
    Navigation,
    ResourceAction,
}

/// Where a binding is meaningful. The footer only advertises a key where it
/// actually does something, so the hints match real behavior.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum KeyScope {
    /// Useful on every view.
    Everywhere,
    /// Views listing a selectable AWS resource.
    ResourceViews,
    /// SSH targets an EC2 instance.
    Ec2Only,
    /// Views that render a wrapped detail pane.
    WrappableViews,
    /// Views that render a scrollable row list, so narrowing it means
    /// something. Account overview paints a fixed layout instead.
    ListViews,
}

impl KeyScope {
    pub fn applies_to(self, view: ActiveView, supports_wrap: bool) -> bool {
        match self {
            KeyScope::Everywhere => true,
            KeyScope::ResourceViews => !matches!(
                view,
                ActiveView::Findings
                    | ActiveView::CostSavings
                    | ActiveView::AccountOverview
                    | ActiveView::CostOverview
            ),
            KeyScope::Ec2Only => view == ActiveView::Ec2,
            KeyScope::WrappableViews => supports_wrap,
            KeyScope::ListViews => view != ActiveView::AccountOverview,
        }
    }
}

pub struct KeyBinding {
    pub key: char,
    pub action: KeyAction,
    /// Short label for the footer, for example "Describe".
    pub label: &'static str,
    /// Longer description for the help screen.
    pub help: &'static str,
    pub group: KeyGroup,
    pub scope: KeyScope,
}

pub const KEY_BINDINGS: &[KeyBinding] = &[
    KeyBinding {
        key: 'f',
        action: KeyAction::Findings,
        label: "Findings",
        help: "Findings home",
        group: KeyGroup::Navigation,
        scope: KeyScope::Everywhere,
    },
    KeyBinding {
        key: '/',
        action: KeyAction::CommandPalette,
        label: "Jump",
        help: "Open the command palette",
        group: KeyGroup::Navigation,
        scope: KeyScope::Everywhere,
    },
    KeyBinding {
        key: 'm',
        action: KeyAction::Filter,
        label: "Filter rows",
        help: "Narrow the current view to rows matching a query",
        group: KeyGroup::Navigation,
        scope: KeyScope::ListViews,
    },
    KeyBinding {
        key: 'g',
        action: KeyAction::GlobalRegion,
        label: "Global",
        help: "Jump to the synthetic global region slot",
        group: KeyGroup::Navigation,
        scope: KeyScope::Everywhere,
    },
    KeyBinding {
        key: 'p',
        action: KeyAction::SwitchProfile,
        label: "Profile",
        help: "Switch AWS profile",
        group: KeyGroup::Navigation,
        scope: KeyScope::Everywhere,
    },
    KeyBinding {
        key: 't',
        action: KeyAction::CycleTheme,
        label: "Theme",
        help: "Cycle through the Seamless themes",
        group: KeyGroup::Navigation,
        scope: KeyScope::Everywhere,
    },
    KeyBinding {
        key: 'w',
        action: KeyAction::ToggleWrap,
        label: "Wrapped detail",
        help: "Toggle wrapped detail mode on supported views",
        group: KeyGroup::Navigation,
        scope: KeyScope::WrappableViews,
    },
    KeyBinding {
        key: 'r',
        action: KeyAction::Refresh,
        label: "Refresh",
        help: "Refresh the active view",
        group: KeyGroup::Navigation,
        scope: KeyScope::Everywhere,
    },
    KeyBinding {
        key: '?',
        action: KeyAction::Help,
        label: "Help",
        help: "Open help",
        group: KeyGroup::Navigation,
        scope: KeyScope::Everywhere,
    },
    KeyBinding {
        key: 'q',
        action: KeyAction::Quit,
        label: "Quit",
        help: "Quit",
        group: KeyGroup::Navigation,
        scope: KeyScope::Everywhere,
    },
    KeyBinding {
        key: 'd',
        action: KeyAction::Describe,
        label: "Describe",
        help: "Describe the selected resource",
        group: KeyGroup::ResourceAction,
        scope: KeyScope::ResourceViews,
    },
    KeyBinding {
        key: 'c',
        action: KeyAction::Cli,
        label: "CLI",
        help: "Show the AWS CLI command for the selected resource",
        group: KeyGroup::ResourceAction,
        scope: KeyScope::ResourceViews,
    },
    KeyBinding {
        key: 'o',
        action: KeyAction::OpenConsole,
        label: "Console",
        help: "Open the selected resource in the AWS console",
        group: KeyGroup::ResourceAction,
        scope: KeyScope::ResourceViews,
    },
    KeyBinding {
        key: 's',
        action: KeyAction::Ssh,
        label: "SSH",
        help: "Prepare an SSH command for the selected EC2 instance",
        group: KeyGroup::ResourceAction,
        scope: KeyScope::Ec2Only,
    },
];

/// Keys whose behavior depends on the surface they are pressed on (overlays,
/// help, lists), so they are handled contextually rather than dispatched from
/// the table. Listed here so the help screen still describes them in one place.
pub const CONTEXTUAL_KEYS: &[(&str, &str)] = &[
    ("Enter", "Open the related view, or confirm an overlay"),
    (
        "Esc",
        "Close an overlay, help, or the command palette, or clear a row filter",
    ),
    ("Tab / Shift+Tab", "Cycle through major views"),
    ("↑ / ↓", "Move the selection or scroll an overlay"),
    (
        "PgUp / PgDn",
        "Jump-scroll lists, overlays, help, or wrapped detail",
    ),
    ("Home / End", "Jump to the top or bottom"),
    ("← / →", "Change AWS region"),
    ("v", "Toggle Describe between structured and JSON"),
];

pub fn binding_for(key: char) -> Option<&'static KeyBinding> {
    KEY_BINDINGS.iter().find(|binding| binding.key == key)
}

/// Bindings worth advertising on `view`, in registry order.
pub fn bindings_for_view(
    view: ActiveView,
    supports_wrap: bool,
) -> impl Iterator<Item = &'static KeyBinding> {
    KEY_BINDINGS
        .iter()
        .filter(move |binding| binding.scope.applies_to(view, supports_wrap))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_key_is_bound_once() {
        for binding in KEY_BINDINGS {
            let matches = KEY_BINDINGS
                .iter()
                .filter(|other| other.key == binding.key)
                .count();
            assert_eq!(matches, 1, "key `{}` is bound more than once", binding.key);
        }
    }

    #[test]
    fn no_binding_collides_with_a_contextual_key() {
        for (label, _) in CONTEXTUAL_KEYS {
            if label.chars().count() == 1 {
                let key = label.chars().next().unwrap();
                assert!(
                    binding_for(key).is_none(),
                    "`{key}` is both dispatched and contextual"
                );
            }
        }
    }

    #[test]
    fn every_action_has_a_key_bound_to_it() {
        // The dispatcher matches exhaustively on KeyAction, so the compiler
        // already proves every action is handled. This catches the other
        // direction: an action that exists but no key reaches.
        for action in [
            KeyAction::Findings,
            KeyAction::CommandPalette,
            KeyAction::Help,
            KeyAction::Refresh,
            KeyAction::Quit,
            KeyAction::CycleTheme,
            KeyAction::SwitchProfile,
            KeyAction::GlobalRegion,
            KeyAction::ToggleWrap,
            KeyAction::Describe,
            KeyAction::Cli,
            KeyAction::OpenConsole,
            KeyAction::Ssh,
        ] {
            assert!(
                KEY_BINDINGS.iter().any(|binding| binding.action == action),
                "an action exists that no key reaches"
            );
        }
    }

    #[test]
    fn ssh_is_only_advertised_on_ec2() {
        assert!(bindings_for_view(ActiveView::Ec2, false).any(|b| b.key == 's'));
        assert!(!bindings_for_view(ActiveView::Lambda, false).any(|b| b.key == 's'));
    }

    #[test]
    fn resource_actions_are_not_advertised_on_findings_or_overviews() {
        for view in [
            ActiveView::Findings,
            ActiveView::CostSavings,
            ActiveView::AccountOverview,
            ActiveView::CostOverview,
        ] {
            let advertised: Vec<char> = bindings_for_view(view, false).map(|b| b.key).collect();
            for key in ['d', 'c', 'o', 's'] {
                assert!(
                    !advertised.contains(&key),
                    "`{key}` should not be advertised on a view where it does nothing"
                );
            }
        }
    }

    #[test]
    fn wrap_is_only_advertised_where_wrapping_exists() {
        assert!(bindings_for_view(ActiveView::Findings, true).any(|b| b.key == 'w'));
        assert!(!bindings_for_view(ActiveView::Findings, false).any(|b| b.key == 'w'));
    }
}
