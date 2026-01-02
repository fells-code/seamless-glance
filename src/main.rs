mod app;
mod aws;
mod config;
mod license;
mod models;
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
use std::io::stdout;

use crate::{
    app::ActiveView,
    ui::{footer::FooterMode, views::command::COMMANDS},
};

use crate::license::{load::load_license, verify::verify_license};

fn check_license_or_exit() {
    match load_license().and_then(|l| verify_license(&l)) {
        Ok(_) => {}
        Err(err) => {
            eprintln!();
            eprintln!("Seamless Glance — License Error");
            eprintln!("--------------------------------");
            eprintln!("{}", err);
            eprintln!();
            eprintln!("Place a valid license at:");
            eprintln!("  ~/.seamless-glance/license.json");
            eprintln!();
            std::process::exit(1);
        }
    }
}

async fn handle_command(app: &mut App) {
    if let Some(cmd) = COMMANDS.iter().find(|c| c.name == app.command_input) {
        app.active_view = cmd.view.clone();
        app.on_view_enter().await;
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    check_license_or_exit();
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
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let region_names = aws::regions::fetch_enabled_regions().await;

    app.regions = region_names.into_iter().map(Region::new).collect();

    let cfg = config::load_config();

    if let Some(region) = cfg.region {
        if let Some(idx) = app.regions.iter().position(|r| r.as_ref() == region) {
            app.current_region_index = idx;
        }
    }

    // fetch data from AWS
    app.services = aws::service::fetch_services().await;
    app.budget = aws::cost::fetch_budget().await;
    app.monthly_costs = aws::cost::fetch_last_6_month_costs().await;
    app.service_costs = aws::cost::fetch_service_cost_breakdown().await;
    app.ecs_clusters = aws::ecs::fetch_ecs_clusters().await;

    loop {
        terminal.draw(|f| ui::draw(f, &app))?;

        if app.should_quit {
            break;
        }

        if app.should_auto_refresh() {
            app.trigger_auto_refresh();
        }

        if app.is_refreshing {
            let region = app.current_region().clone();

            app.is_refreshing = false;

            let overview = aws::account::fetch_account_overview(&region).await;

            app.account_overview = Some(overview);
            app.last_refresh = Some(chrono::Utc::now());
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
            }
            KeyCode::Esc if app.show_help => {
                app.show_help = false;
                app.command_mode = false;
                app.footer_mode = FooterMode::Normal;
                app.command_input.clear();
            }
            KeyCode::Esc if app.command_mode => {
                app.command_mode = false;
                app.footer_mode = FooterMode::Normal;
            }
            KeyCode::Enter => {
                handle_command(&mut app).await;
                app.command_mode = false;
                app.footer_mode = FooterMode::Normal;
                app.command_input.clear();
            }
            KeyCode::Char(c) if app.command_mode => {
                app.command_input.push(c);
            }
            KeyCode::Backspace if app.command_mode => {
                app.command_input.pop();
            }
            KeyCode::Left => {
                app.previous_region().await;
            }
            KeyCode::Right => {
                app.next_region().await;
            }
            KeyCode::Char('q') => {
                config::save_config(&config::GlanceConfig {
                    region: Some(app.current_region().as_ref().to_string()),
                    profile: None,
                });
                app.should_quit = true;
            }
            KeyCode::Char('1') => {
                app.active_view = ActiveView::AccountOverview;
                app.on_view_enter().await;
            }
            KeyCode::Char('2') => {
                app.active_view = ActiveView::CostOverview;
                app.on_view_enter().await;
            }
            KeyCode::Char('3') => {
                app.active_view = ActiveView::Ecs;
                app.on_view_enter().await;
            }
            KeyCode::Char('4') => {
                app.active_view = ActiveView::Lambda;
                app.on_view_enter().await;
            }
            KeyCode::Char('5') => {
                app.active_view = ActiveView::Apigateway;
                app.on_view_enter().await;
            }
            KeyCode::Char('6') => {
                app.active_view = ActiveView::Sqs;
                app.on_view_enter().await;
            }
            KeyCode::Char('7') => {
                app.active_view = ActiveView::Vpc;
                app.on_view_enter().await;
            }
            _ => {}
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, Show)?;
    terminal.show_cursor()?;
    Ok(())
}
