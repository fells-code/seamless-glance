mod app;
mod aws;
mod cache;
mod config;
mod license;
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
    license::{
        ensure_license::ensure_license_present, status::print_license_status,
        verify::validate_license,
    },
    ui::{
        footer::FooterMode,
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
  --help       Show this help message
  --version    Show version information

INSTALL:
  brew install fells-code/seamless/seamless-glance
  curl -fsSL https://seamlessglance.com/install.sh | bash

LICENSE:
  Place your license at:
    ~/.seamless-glance/license.json
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
                eprintln!("Unknown region: {}", args);
            }
        }
        "theme" => {
            if args.is_empty() {
                return;
            }

            if let Some(theme_name) = ThemeName::from_str(&args) {
                app.set_theme_name(theme_name);
            } else {
                eprintln!("Unknown theme: {}", args);
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

async fn activate_view(app: &mut App, view: ActiveView) {
    app.active_view = view;
    app.on_view_enter().await;
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 {
        match args[1].as_str() {
            "--help" | "-h" => {
                print_help();
                return Ok(());
            }
            "--version" | "-v" => {
                println!("Seamless Glance v{}", VERSION);
                return Ok(());
            }
            "--license-status" => {
                print_license_status();
                return Ok(());
            }
            _ => {
                eprintln!("Unknown option: {}", args[1]);
                eprintln!("Run `seamless-glance --help` for usage.");
                std::process::exit(1);
            }
        }
    }

    let license = ensure_license_present().map_err(anyhow::Error::msg)?;

    if let Err(e) = validate_license(&license) {
        eprintln!();
        eprintln!("Seamless Glance — License");
        eprintln!("-------------------------");
        eprintln!("{}", e);
        eprintln!();
        eprintln!("To continue, purchase a license at:");
        eprintln!("  https://seamlessglance.com");
        eprintln!();
        eprintln!("Then place the license file at:");
        eprintln!("  ~/.seamless-glance/license.json");
        eprintln!();
        std::process::exit(1);
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
    let cfg = config::load_config();
    let mut terminal = Terminal::new(backend)?;

    let region_names = aws::regions::fetch_enabled_regions().await;
    let regions: Vec<Region> = region_names.into_iter().map(Region::new).collect();

    let mut current_region_index = 0;

    if let Some(ref region_str) = cfg.region {
        if let Some(idx) = regions.iter().position(|r| r.as_ref() == region_str) {
            current_region_index = idx;
        }
    }

    let sdk_config = aws_config::defaults(aws_config::BehaviorVersion::v2026_01_12())
        .region(regions[current_region_index].clone())
        .load()
        .await;

    let aws = AwsClients::new(&sdk_config);
    let mut app = App::new(aws);
    app.license = Some(license);
    app.regions = regions;
    app.current_region_index = current_region_index;
    if let Some(theme_name) = cfg.theme.as_deref().and_then(ThemeName::from_str) {
        app.theme_name = theme_name;
        app.theme = crate::ui::theme::Theme::from_name(theme_name);
    }

    app.load_cost_data().await;
    app.trigger_refresh();

    loop {
        terminal.draw(|f| ui::draw(f, &mut app))?;

        if app.should_quit {
            break;
        }

        if app.is_refreshing {
            app.refresh_active().await;
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
            KeyCode::Char('/') => {
                app.command_mode = true;
                app.command_input.clear();
                app.footer_mode = FooterMode::Command;
            }
            KeyCode::Char('?') => {
                app.show_help = true;
                app.footer_mode = FooterMode::Help;
                app.scroll_offset = 0;
            }
            KeyCode::Char(c) if app.command_mode => {
                app.command_input.push(c);
            }
            KeyCode::Char('r') => {
                if !app.command_mode {
                    app.trigger_refresh();
                    continue;
                }
            }
            KeyCode::Enter => {
                if let Some(OverlayState::ConfirmCommand(state)) = &app.overlay {
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
            KeyCode::Backspace if app.command_mode => {
                app.command_input.pop();
            }
            KeyCode::Tab => {
                if !app.command_mode && !app.show_help && app.overlay.is_none() {
                    let current_view = app.active_view;
                    activate_view(&mut app, next_command(current_view).view).await;
                }
            }
            KeyCode::BackTab => {
                if !app.command_mode && !app.show_help && app.overlay.is_none() {
                    let current_view = app.active_view;
                    activate_view(&mut app, previous_command(current_view).view).await;
                }
            }
            KeyCode::Left => {
                app.previous_region().await;
            }
            KeyCode::Right => {
                app.next_region().await;
            }
            KeyCode::Down => {
                if app.show_help {
                    app.scroll_offset = app.scroll_offset.saturating_add(1);
                } else if let Some(overlay) = &mut app.overlay {
                    overlay.scroll_down();
                } else {
                    app.selected_row = app.selected_row.saturating_add(1);
                }
            }

            KeyCode::Up => {
                if app.show_help {
                    app.scroll_offset = app.scroll_offset.saturating_sub(1);
                } else if let Some(overlay) = &mut app.overlay {
                    overlay.scroll_up();
                } else {
                    app.selected_row = app.selected_row.saturating_sub(1);
                }
            }
            KeyCode::Char('o') => {
                app.trigger_open();
            }
            KeyCode::Char('q') => {
                app.persist_region_selection();
                app.should_quit = true;
            }
            KeyCode::Char('d') => {
                if app.overlay.is_none() {
                    app.trigger_describe().await;
                }
            }
            KeyCode::Char('f') => {
                if !app.command_mode && !app.show_help && app.overlay.is_none() {
                    activate_view(&mut app, ActiveView::Findings).await;
                }
            }
            KeyCode::Char('c') => {
                if !app.command_mode && !app.show_help && app.overlay.is_none() {
                    app.trigger_cli();
                }
            }
            KeyCode::Char('g') => {
                if !app.command_mode && !app.show_help && app.overlay.is_none() {
                    app.set_global_region();
                    app.persist_region_selection();
                    app.trigger_refresh();
                }
            }
            KeyCode::Char('s') => {
                app.trigger_ssh();
            }
            KeyCode::Char('t') => {
                if !app.command_mode && !app.show_help && app.overlay.is_none() {
                    app.cycle_theme();
                }
            }
            KeyCode::Char('1') => {
                if let Some(OverlayState::SelectSshKey(state)) = &app.overlay {
                    let cmd = format!("ssh {}@{}", state.context.user, state.context.host);

                    app.overlay = Some(OverlayState::ConfirmCommand(ConfirmCommandState {
                        title: format!("SSH into {}", state.context.instance_name),
                        command: cmd,
                        scroll: 0,
                    }));
                    continue;
                } else {
                    activate_view(&mut app, ActiveView::AccountOverview).await;
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
                } else {
                    activate_view(&mut app, ActiveView::CostOverview).await;
                }
            }
            KeyCode::Char('3') => {
                activate_view(&mut app, ActiveView::Vpc).await;
            }
            KeyCode::Char('4') => {
                activate_view(&mut app, ActiveView::Ec2).await;
            }
            KeyCode::Char('5') => {
                activate_view(&mut app, ActiveView::CloudWatch).await;
            }
            KeyCode::Char('6') => {
                activate_view(&mut app, ActiveView::Lambda).await;
            }
            KeyCode::Char('7') => {
                activate_view(&mut app, ActiveView::Secrets).await;
            }
            KeyCode::Char('8') => {
                activate_view(&mut app, ActiveView::Ecs).await;
            }

            KeyCode::Char('9') => {
                activate_view(&mut app, ActiveView::Apigateway).await;
            }
            _ => {}
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, Show)?;
    terminal.show_cursor()?;
    Ok(())
}
