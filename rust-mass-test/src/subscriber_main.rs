mod config;
mod metrics;
mod subscriber;
mod topic;
mod ui;

use crate::config::Config;
use crate::metrics::GlobalMetrics;
use crate::ui::{draw_config_screen, draw_metrics_screen, UIContext};
use clap::Parser;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::task::JoinHandle;

#[derive(Parser, Debug)]
#[command(name = "MQTT Subscribe")]
#[command(about = "Fast MQTT subscribe program with console GUI", long_about = None)]
struct Args {
    /// MQTT broker host
    #[arg(long, default_value = "localhost")]
    broker: String,

    /// MQTT broker port
    #[arg(long, default_value = "1883")]
    port: u16,

    /// Configuration file to load (JSON)
    #[arg(long)]
    config: Option<String>,

    /// Auto-start without UI (use config file)
    #[arg(long)]
    auto_start: bool,

    /// Percentage of topics to subscribe to (0-100)
    #[arg(long, default_value = "100")]
    subscribe_percentage: u8,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Load or create config
    let config_file = if args.config.is_some() {
        args.config.as_deref()
    } else {
        if std::path::Path::new("config.json").exists() {
            Some("config.json")
        } else {
            None
        }
    };

    let mut config = Config::load_or_default(config_file);
    config.broker_host = args.broker;
    config.broker_port = args.port;
    config.subscribe_percentage = args.subscribe_percentage;

    if config_file.is_some() {
        eprintln!("✅ Loaded configuration from: {}", config_file.unwrap());
    }

    if args.auto_start {
        run_subscribers(&config).await?;
    } else {
        run_ui(&config).await?;
    }

    Ok(())
}

async fn run_ui(initial_config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    terminal.clear()?;
    terminal.hide_cursor()?;

    let mut ui_ctx = UIContext::new();
    ui_ctx.config = initial_config.clone();

    loop {
        let mut should_exit = false;
        loop {
            terminal.draw(|f| draw_config_screen(f, &ui_ctx))?;

            if let Some(should_start) = ui::handle_ui_input(&mut ui_ctx).await {
                if should_start {
                    disable_raw_mode()?;
                    crossterm::execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
                    terminal.show_cursor()?;
                    break;
                } else {
                    should_exit = true;
                    break;
                }
            }
        }

        if should_exit {
            disable_raw_mode()?;
            crossterm::execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
            terminal.show_cursor()?;
            eprintln!("\n👋 Goodbye!");
            return Ok(())
        }

        run_subscribers_with_ui(&ui_ctx.config).await?;

        eprintln!("\n📋 Returning to configuration screen...");
        tokio::time::sleep(std::time::Duration::from_millis(800)).await;

        ui_ctx.state = ui::UIState::ConfigInput;
        ui_ctx.field_index = 0;
        ui_ctx.input_buffer.clear();
        ui_ctx.in_edit_mode = false;

        drop(terminal);

        let _ = disable_raw_mode();
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        enable_raw_mode()?;
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let mut stdout = io::stdout();
        use std::io::Write;
        let _ = stdout.flush();

        crossterm::execute!(stdout, EnterAlternateScreen)?;

        let backend = CrosstermBackend::new(stdout);
        terminal = Terminal::new(backend)?;

        terminal.clear()?;
        terminal.hide_cursor()?;

        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        loop {
            if !crossterm::event::poll(std::time::Duration::from_millis(100))? {
                break;
            }
            let _ = crossterm::event::read();
        }

        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }
}

async fn run_subscribers_with_ui(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let config = Arc::new(config.clone());
    let metrics = Arc::new(Mutex::new(GlobalMetrics::new(config.num_producers)));

    eprintln!("\n📊 Starting {} subscribers...", config.num_producers);

    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

    let mut handles: Vec<JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>>> =
        Vec::new();

    for subscriber_id in 0..config.num_producers {
        let config_clone = config.clone();
        let client_metrics = metrics.lock().unwrap().clients[subscriber_id].clone();
        let shutdown_rx_clone = shutdown_rx.clone();

        let handle: JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>> =
            tokio::spawn(async move {
                crate::subscriber::run(config_clone, Arc::new(client_metrics), shutdown_rx_clone)
                    .await
            });

        handles.push(handle);
    }

    tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
    eprintln!("✅ All subscribers connected!");

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let start_time = Instant::now();

    loop {
        terminal.draw(|f| draw_metrics_screen(f, &metrics.lock().unwrap(), start_time.elapsed()))?;

        if crossterm::event::poll(std::time::Duration::from_millis(100))? {
            if let crossterm::event::Event::Key(key) = crossterm::event::read()? {
                match key.code {
                    crossterm::event::KeyCode::Char('q') | crossterm::event::KeyCode::Char('Q') => {
                        eprintln!("\n🛑 Stopping subscribers...");
                        let _ = shutdown_tx.send(true); // Send shutdown signal
                        break;
                    }
                    _ => {}
                }
            }
        }

        let mut any_crashed = false;
        for handle in &handles {
            if handle.is_finished() {
                any_crashed = true;
                break;
            }
        }
        if any_crashed {
            eprintln!("\n⚠️  A subscriber crashed, stopping test...");
            let _ = shutdown_tx.send(true); // Send shutdown signal
            break;
        }
    }

    disable_raw_mode()?;
    crossterm::execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    eprintln!("📊 Stopping all subscribers...");

    // Abort all tasks (as a fallback, if graceful shutdown fails)
    for handle in handles {
        handle.abort();
    }

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    eprintln!("✅ Test completed!");
    let final_metrics = metrics.lock().unwrap();
    eprintln!("Total messages received: {}", final_metrics.get_total_received());
    eprintln!("Average throughput: {:.2} msg/s", final_metrics.get_total_received_vps());

    Ok(())
}

async fn run_subscribers(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let config = Arc::new(config.clone());
    let metrics = Arc::new(Mutex::new(GlobalMetrics::new(config.num_producers)));

    let (_shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

    let mut handles: Vec<JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>>> =
        Vec::new();

    for subscriber_id in 0..config.num_producers {
        let config_clone = config.clone();
        let client_metrics = metrics.lock().unwrap().clients[subscriber_id].clone();
        let shutdown_rx_clone = shutdown_rx.clone();

        let handle: JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>> =
            tokio::spawn(async move {
                crate::subscriber::run(config_clone, Arc::new(client_metrics), shutdown_rx_clone)
                    .await
            });

        handles.push(handle);
    }

    // Wait for all tasks to finish (they should exit gracefully on shutdown signal)
    for handle in handles {
        let _ = handle.await;
    }

    eprintln!("✅ Test completed!");
    let final_metrics = metrics.lock().unwrap();
    eprintln!("Total messages received: {}", final_metrics.get_total_received());
    eprintln!("Average throughput: {:.2} msg/s", final_metrics.get_total_received_vps());

    Ok(())
}