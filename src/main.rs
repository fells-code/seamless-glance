mod app;
mod aws;
mod cache;
mod config;
mod models;
mod resources;
mod ui;

use app::App;
use aws_config::Region;
use crossterm::{
    cursor::{Hide, Show},
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

use ratatui::{backend::CrosstermBackend, Terminal};
use std::{env, io::stdout};

use crate::{
    app::ActiveView,
    aws::clients::AwsClients,
    config::VERSION,
    ui::{
        footer::FooterMode,
        keys::{self, KeyAction},
        overlay::overlays::{ConfirmCommandState, OverlayState},
        theme::ThemeName,
        views::command::{self, next_command, previous_command},
    },
};

fn print_help() {
    println!(
        "\
Seamless Glance — AWS visibility in your terminal

USAGE:
  seamless-glance [OPTIONS]

OPTIONS:
  --help              Show this help message
  --version           Show version information
  --profile <name>    Start with a specific AWS profile (overrides config)

INSTALL:
  brew install fells-code/seamless/seamless-glance
"
    );
}

async fn handle_command(app: &mut App) {
    let (cmd, args) = command::parse_command(&app.command_input);
    let cmd = cmd.to_ascii_lowercase();
    let args = args.to_string();

    if cmd.is_empty() {
        return;
    }

    match cmd.as_str() {
        "region" | "rg" => {
            if args.is_empty() {
                return;
            }

            if app.set_region_by_name(&args).await {
                app.persist_region_selection();
                app.trigger_refresh();
            } else {
                app.notify_warning(format!("Unknown region: {}", args));
            }
        }
        "profile" | "pf" => {
            if args.is_empty() {
                app.open_profile_picker();
            } else if !app.set_profile_by_name(&args).await {
                app.notify_warning(format!("Unknown profile: {}", args));
            }
        }
        "theme" => {
            if args.is_empty() {
                return;
            }

            if let Some(theme_name) = ThemeName::from_str(&args) {
                app.set_theme_name(theme_name);
            } else {
                app.notify_warning(format!("Unknown theme: {}", args));
            }
        }
        _ => {
            if let Some(command) = command::matching_commands(&cmd).first() {
                app.active_view = command.view;
                app.on_view_enter().await;
            }
        }
    }
}

/// Run the action a bound key maps to. Every key in `keys::KEY_BINDINGS`
/// resolves here, so adding a binding is one registry entry plus one arm.
async fn run_key_action(app: &mut App, action: KeyAction) {
    use KeyAction as A;

    // The palette, help, refresh, and quit stay reachable with a modal open;
    // everything else requires a normal view (the guard from #23).
    let modal_safe = matches!(action, A::CommandPalette | A::Help | A::Refresh | A::Quit);
    if !modal_safe && app.modal_open() {
        return;
    }

    match action {
        A::CommandPalette => {
            app.command_mode = true;
            app.command_input.clear();
            app.footer_mode = FooterMode::Command;
        }
        A::Help => {
            app.show_help = true;
            app.footer_mode = FooterMode::Help;
            app.scroll_offset = 0;
        }
        A::Filter => app.open_filter(),
        A::Refresh => app.trigger_refresh(),
        A::Quit => {
            app.persist_region_selection();
            app.should_quit = true;
        }
        A::Findings => activate_view(app, ActiveView::Findings).await,
        A::CycleTheme => app.cycle_theme(),
        A::SwitchProfile => app.open_profile_picker(),
        A::GlobalRegion => {
            app.set_global_region();
            app.persist_region_selection();
            app.trigger_refresh();
        }
        A::ToggleWrap => app.toggle_wrap_mode(),
        A::Describe => app.trigger_describe().await,
        A::Cli => app.trigger_cli(),
        A::OpenConsole => app.trigger_open(),
        A::Ssh => app.trigger_ssh(),
    }
}

async fn activate_view(app: &mut App, view: ActiveView) {
    app.active_view = view;
    app.on_view_enter().await;
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();
    let mut cli_profile: Option<String> = None;

    let mut i = 1;
    while i < args.len() {
        let arg = args[i].as_str();
        match arg {
            "--help" | "-h" => {
                print_help();
                return Ok(());
            }
            "--version" | "-v" => {
                println!("Seamless Glance v{}", VERSION);
                return Ok(());
            }
            "--profile" | "-p" => {
                i += 1;
                let Some(value) = args.get(i) else {
                    eprintln!("--profile requires a profile name");
                    std::process::exit(1);
                };
                cli_profile = Some(value.clone());
            }
            _ if arg.starts_with("--profile=") => {
                cli_profile = Some(arg.trim_start_matches("--profile=").to_string());
            }
            _ => {
                eprintln!("Unknown option: {}", arg);
                eprintln!("Run `seamless-glance --help` for usage.");
                std::process::exit(1);
            }
        }
        i += 1;
    }

    enable_raw_mode()?;
    let mut stdout = stdout();

    // Enter TUI-safe environment
    execute!(stdout, EnterAlternateScreen, Hide)?;

    // Clear any previous terminal contents
    execute!(
        stdout,
        crossterm::terminal::Clear(crossterm::terminal::ClearType::All)
    )?;

    let backend = CrosstermBackend::new(stdout);
    let loaded_config = config::load_config();
    let config_warning = loaded_config.warning();
    let cfg = loaded_config.config();
    let mut terminal = Terminal::new(backend)?;

    let profile = cli_profile.or_else(|| cfg.profile.clone());
    let profiles = aws::profiles::discover_profiles();

    let (region_names, region_discovery_warning) =
        match aws::regions::fetch_enabled_regions(profile.as_deref()).await {
            Ok(regions) if !regions.is_empty() => (regions, None),
            Ok(_) => (
                aws::regions::fallback_regions(),
                Some("Region discovery returned no regions; using a static fallback list. Global views may be incomplete.".to_string()),
            ),
            Err(err) => (
                aws::regions::fallback_regions(),
                Some(format!(
                    "Region discovery failed; using a static fallback list. Global views may be incomplete. {err}"
                )),
            ),
        };
    let regions: Vec<Region> = region_names.into_iter().map(Region::new).collect();

    let mut current_region_index = 0;

    if let Some(ref region_str) = cfg.region {
        if region_str.eq_ignore_ascii_case("global") {
            // The global slot sits one past the last real region.
            current_region_index = regions.len();
        } else if let Some(idx) = regions.iter().position(|r| r.as_ref() == region_str) {
            current_region_index = idx;
        }
    }

    // The initial client bundle needs a real region even when the global slot is
    // selected; global fans out per region separately.
    let client_region_index = current_region_index.min(regions.len().saturating_sub(1));
    let sdk_config =
        aws::clients::build_sdk_config(regions[client_region_index].clone(), profile.as_deref())
            .await;

    let aws = AwsClients::new(&sdk_config);
    let mut app = App::new(aws);
    app.regions = regions;
    app.current_region_index = current_region_index;
    app.profiles = profiles;
    app.current_profile = profile;
    if let Some(theme_name) = cfg.theme.as_deref().and_then(ThemeName::from_str) {
        app.theme_name = theme_name;
        app.theme = crate::ui::theme::Theme::from_name(theme_name);
    }

    app.load_cost_data().await;
    app.trigger_refresh();

    // Raise warnings last so their display window starts at the first draw
    // rather than being consumed by startup fetches. A config problem outranks
    // a region-discovery one: it is the only warning that names a file the
    // operator may want to recover.
    if let Some(warning) = region_discovery_warning {
        app.notify_warning(warning);
    }

    if let Some(warning) = config_warning {
        app.notify_warning(warning);
    }

    const PAGE_SCROLL_LINES: usize = 10;

    loop {
        app.clear_expired_notification();
        app.drain_refresh_updates();

        terminal.draw(|f| ui::draw(f, &mut app))?;

        if app.should_quit {
            break;
        }

        if !event::poll(std::time::Duration::from_millis(100))? {
            continue;
        }

        let key_event = event::read()?;
        let key = if let Event::Key(k) = key_event {
            k
        } else {
            continue;
        };

        match key.code {
            KeyCode::Char('v') if !app.command_mode && !app.filter_mode => {
                if let Some(overlay) = &mut app.overlay {
                    if overlay.toggle_describe_mode() {
                        app.footer_mode = FooterMode::Overlay;
                        continue;
                    }
                }
            }
            KeyCode::Char(c) if app.command_mode => {
                app.command_input.push(c);
            }
            // Ahead of the digit and binding arms below, so a query can contain
            // any character without firing the key it is bound to.
            KeyCode::Char(c) if app.filter_mode => {
                app.push_filter_char(c);
            }
            KeyCode::Enter if app.filter_mode => {
                app.commit_filter();
            }
            KeyCode::Enter => {
                if let Some(OverlayState::SelectProfile(_)) = &app.overlay {
                    app.commit_profile_selection().await;
                    continue;
                } else if let Some(OverlayState::ConfirmCommand(state)) = &app.overlay {
                    let _ = crate::ui::terminal::suspend_tui();

                    let _ = std::process::Command::new("sh")
                        .arg("-c")
                        .arg(&state.command)
                        .status();

                    let _ = crate::ui::terminal::resume_tui();

                    app.overlay = None;
                    app.footer_mode = FooterMode::Normal;
                    continue;
                } else if app.command_mode {
                    handle_command(&mut app).await;
                    app.command_mode = false;
                    app.footer_mode = FooterMode::Normal;
                    app.command_input.clear();
                } else if app.active_view == ActiveView::Findings
                    && !app.show_help
                    && app.overlay.is_none()
                {
                    app.open_selected_finding().await;
                } else if app.active_view == ActiveView::CostSavings
                    && !app.show_help
                    && app.overlay.is_none()
                {
                    app.open_selected_cost_savings_opportunity().await;
                }
            }
            KeyCode::Esc if app.overlay.is_some() => {
                app.overlay = None;
                app.footer_mode = FooterMode::Normal;
                continue;
            }
            KeyCode::Esc if app.show_help => {
                app.show_help = false;
                app.command_mode = false;
                app.footer_mode = FooterMode::Normal;
                app.command_input.clear();
                app.scroll_offset = 0;
            }
            KeyCode::Esc if app.command_mode => {
                app.command_mode = false;
                app.footer_mode = FooterMode::Normal;
                app.command_input.clear();
            }
            // Esc clears the filter rather than only leaving the prompt, so a
            // narrowed view is never left behind with no visible way out.
            KeyCode::Esc if app.filter_mode || app.filter_is_active() => {
                app.clear_filter();
            }
            KeyCode::Backspace if app.command_mode => {
                app.command_input.pop();
            }
            KeyCode::Backspace if app.filter_mode => {
                app.pop_filter_char();
            }
            KeyCode::Tab => {
                if !app.modal_open() {
                    let current_view = app.active_view;
                    activate_view(&mut app, next_command(current_view).view).await;
                }
            }
            KeyCode::BackTab => {
                if !app.modal_open() {
                    let current_view = app.active_view;
                    activate_view(&mut app, previous_command(current_view).view).await;
                }
            }
            KeyCode::Left => {
                if !app.modal_open() {
                    app.previous_region().await;
                }
            }
            KeyCode::Right => {
                if !app.modal_open() {
                    app.next_region().await;
                }
            }
            KeyCode::Down => {
                if app.show_help {
                    app.scroll_offset = app.scroll_offset.saturating_add(1);
                } else if let Some(overlay) = &mut app.overlay {
                    overlay.scroll_down();
                } else {
                    app.scroll_active_view_down(1);
                }
            }

            KeyCode::Up => {
                if app.show_help {
                    app.scroll_offset = app.scroll_offset.saturating_sub(1);
                } else if let Some(overlay) = &mut app.overlay {
                    overlay.scroll_up();
                } else {
                    app.scroll_active_view_up(1);
                }
            }
            KeyCode::PageDown => {
                if app.show_help {
                    app.scroll_offset = app.scroll_offset.saturating_add(PAGE_SCROLL_LINES as u16);
                } else if let Some(overlay) = &mut app.overlay {
                    overlay.page_down(PAGE_SCROLL_LINES as u16);
                } else if app.wrap_mode_active() {
                    app.scroll_wrapped_detail_down(PAGE_SCROLL_LINES);
                } else {
                    app.scroll_active_view_down(PAGE_SCROLL_LINES);
                }
            }
            KeyCode::PageUp => {
                if app.show_help {
                    app.scroll_offset = app.scroll_offset.saturating_sub(PAGE_SCROLL_LINES as u16);
                } else if let Some(overlay) = &mut app.overlay {
                    overlay.page_up(PAGE_SCROLL_LINES as u16);
                } else if app.wrap_mode_active() {
                    app.scroll_wrapped_detail_up(PAGE_SCROLL_LINES);
                } else {
                    app.scroll_active_view_up(PAGE_SCROLL_LINES);
                }
            }
            KeyCode::Home => {
                if app.show_help {
                    app.scroll_offset = 0;
                } else if let Some(overlay) = &mut app.overlay {
                    overlay.scroll_to_top();
                } else if app.wrap_mode_active() {
                    app.scroll_wrapped_detail_to_top();
                } else {
                    app.scroll_active_view_to_top();
                }
            }
            KeyCode::End => {
                if app.show_help {
                    app.scroll_offset = u16::MAX;
                } else if let Some(overlay) = &mut app.overlay {
                    overlay.scroll_to_bottom();
                } else if app.wrap_mode_active() {
                    app.scroll_wrapped_detail_to_bottom();
                } else {
                    app.scroll_active_view_to_bottom();
                }
            }
            // Digits 1 and 2 pick an SSH command when the key-selection overlay is
            // open. Numeric view-switching was removed in favor of the command
            // palette; digits do nothing otherwise.
            KeyCode::Char('1') => {
                if let Some(OverlayState::SelectSshKey(state)) = &app.overlay {
                    let cmd = format!("ssh {}@{}", state.context.user, state.context.host);

                    app.overlay = Some(OverlayState::ConfirmCommand(ConfirmCommandState {
                        title: format!("SSH into {}", state.context.instance_name),
                        command: cmd,
                        scroll: 0,
                    }));
                    continue;
                }
            }
            KeyCode::Char('2') => {
                if let Some(OverlayState::SelectSshKey(state)) = &app.overlay {
                    let cmd = format!(
                        "ssh -i ~/{}.pem {}@{}",
                        state.context.key_name.as_deref().unwrap_or("key"),
                        state.context.user,
                        state.context.host
                    );

                    app.overlay = Some(OverlayState::ConfirmCommand(ConfirmCommandState {
                        title: "SSH with private key".into(),
                        command: cmd,
                        scroll: 0,
                    }));
                    continue;
                }
            }
            // Anything else the registry binds runs through one dispatcher, so
            // the keys advertised in the footer and help cannot drift from the
            // keys that actually do something.
            KeyCode::Char(c) => {
                if let Some(binding) = keys::binding_for(c) {
                    run_key_action(&mut app, binding.action).await;
                }
            }
            _ => {}
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, Show)?;
    terminal.show_cursor()?;
    Ok(())
}
