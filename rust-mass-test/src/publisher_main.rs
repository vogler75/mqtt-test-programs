mod config;
mod metrics;
mod producer;
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
#[command(name = "MQTT Test")]
#[command(about = "Fast MQTT test program with console GUI", long_about = None)]
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
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Load or create config
    // If no config file specified, try to load config.json if it exists
    let config_file = if args.config.is_some() {
        args.config.as_deref()
    } else {
        // Try to use config.json if it exists in current directory
        if std::path::Path::new("config.json").exists() {
            Some("config.json")
        } else {
            None
        }
    };

    let mut config = Config::load_or_default(config_file);
    config.broker_host = args.broker;
    config.broker_port = args.port;

    // Notify user if config was loaded
    if config_file.is_some() {
        eprintln!("✅ Loaded configuration from: {}", config_file.unwrap());
    }

    if args.auto_start {
        run_producers(&config).await?;
    } else {
        run_ui(&config).await?;
    }

    Ok(())
}

async fn run_ui(initial_config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Clear screen and hide cursor
    terminal.clear()?;
    terminal.hide_cursor()?;

    let mut ui_ctx = UIContext::new();
    ui_ctx.config = initial_config.clone();

    loop {
        // Configuration loop - can run multiple tests
        let mut should_exit = false;
        loop {
            // Draw configuration screen
            terminal.draw(|f| draw_config_screen(f, &ui_ctx))?;

            // Handle input
            if let Some(should_start) = ui::handle_ui_input(&mut ui_ctx).await {
                if should_start {
                    // Start producers - switch out of raw mode for metrics screen
                    disable_raw_mode()?;
                    crossterm::execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
                    terminal.show_cursor()?;
                    break;
                } else {
                    // Q was pressed - exit application
                    should_exit = true;
                    break;
                }
            }
        }

        if should_exit {
            // Cleanup terminal before exit
            disable_raw_mode()?;
            crossterm::execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
            terminal.show_cursor()?;
            eprintln!("\n👋 Goodbye!");
            return Ok(());
        }

        // Run producers
        run_producers_with_ui(&ui_ctx.config).await?;

        // Restore terminal for config screen
        eprintln!("\n📋 Returning to configuration screen...");
        tokio::time::sleep(std::time::Duration::from_millis(800)).await;

        // Reset UI state for next test
        ui_ctx.state = ui::UIState::ConfigInput;
        ui_ctx.field_index = 0;
        ui_ctx.input_buffer.clear();
        ui_ctx.in_edit_mode = false;

        // CRITICAL: Completely reset terminal state
        // Drop old terminal
        drop(terminal);

        // Disable raw mode if still active
        let _ = disable_raw_mode();
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // Enable raw mode
        enable_raw_mode()?;
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Create fresh stdout and enter alternate screen
        let mut stdout = io::stdout();

        // Clear any buffered data
        use std::io::Write;
        let _ = stdout.flush();

        // Enter alternate screen
        crossterm::execute!(stdout, EnterAlternateScreen)?;

        // Create completely new terminal
        let backend = CrosstermBackend::new(stdout);
        terminal = Terminal::new(backend)?;

        // Prepare screen
        terminal.clear()?;
        terminal.hide_cursor()?;

        // Wait for terminal to fully stabilize
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        // Drain any queued events
        loop {
            if !crossterm::event::poll(std::time::Duration::from_millis(100))? {
                break;
            }
            let _ = crossterm::event::read();
        }

        // Wait a bit more
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }
}

async fn run_producers_with_ui(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let config = Arc::new(config.clone());
    let metrics = Arc::new(Mutex::new(GlobalMetrics::new(config.num_producers)));

    eprintln!("\n📊 Starting {} producers...", config.num_producers);

    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
    let (pause_tx, pause_rx) = tokio::sync::watch::channel(false);

    let mut handles: Vec<JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>>> =
        Vec::new();

    for producer_id in 0..config.num_producers {
        let config_clone = config.clone();
        let client_metrics = metrics.lock().unwrap().clients[producer_id].clone();
        let shutdown_rx_clone = shutdown_rx.clone();
        let pause_rx_clone = pause_rx.clone();

        let handle: JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>> =
            tokio::spawn(async move {
            crate::producer::run_producer(producer_id, config_clone, Arc::new(client_metrics), shutdown_rx_clone, pause_rx_clone)
                .await
        });

        handles.push(handle);
    }

    // Small delay to let producers connect
    tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
    eprintln!("✅ All producers connected!");

    // Setup UI terminal for metrics display
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let start_time = Instant::now();
    let mut is_paused = false;

    loop {
        // Draw metrics
        let metrics_guard = metrics.lock().unwrap();
        terminal.draw(|f| {
            let _ui_ctx = crate::ui::UIContext::new();
            // Draw normal metrics screen
            draw_metrics_screen(f, &metrics_guard, start_time.elapsed());

            // Draw pause status if paused
            if is_paused {
                use ratatui::widgets::{Paragraph, Block, Borders};
                use ratatui::style::{Style, Color, Modifier};
                use ratatui::layout::{Alignment, Rect};

                let area = f.area();
                let pause_area = Rect {
                    x: area.x + area.width / 2 - 15,
                    y: area.y + 3,
                    width: 30,
                    height: 3,
                };

                let pause_widget = Paragraph::new("⏸ PAUSED - Press P to resume")
                    .block(Block::default().borders(Borders::ALL).title("Status"))
                    .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                    .alignment(Alignment::Center);
                f.render_widget(pause_widget, pause_area);
            }
        })?;
        drop(metrics_guard);

        // Check for key input
        if crossterm::event::poll(std::time::Duration::from_millis(100))? {
            if let crossterm::event::Event::Key(key) = crossterm::event::read()? {
                match key.code {
                    crossterm::event::KeyCode::Char('q') | crossterm::event::KeyCode::Char('Q') => {
                        eprintln!("\n🛑 Stopping producers...");
                        let _ = shutdown_tx.send(true); // Send shutdown signal
                        break;
                    }
                    crossterm::event::KeyCode::Char('p') | crossterm::event::KeyCode::Char('P') => {
                        is_paused = !is_paused;
                        let _ = pause_tx.send(is_paused);
                    }
                    crossterm::event::KeyCode::Char('c') | crossterm::event::KeyCode::Char('C') => {
                        eprintln!("\n🔄 Clearing metrics...");
                        metrics.lock().unwrap().reset();
                        eprintln!("✅ Metrics cleared");
                    }
                    _ => {}
                }
            }
        }

        // Check if any producer crashed
        let mut any_crashed = false;
        for handle in &handles {
            if handle.is_finished() {
                any_crashed = true;
                break;
            }
        }
        if any_crashed {
            eprintln!("\n⚠️  A producer crashed, stopping test...");
            let _ = shutdown_tx.send(true); // Send shutdown signal
            break;
        }
    }

    // Cleanup
    disable_raw_mode()?;
    crossterm::execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    eprintln!("📊 Stopping all producers...");

    // Abort all tasks (as a fallback, if graceful shutdown fails)
    for handle in handles {
        handle.abort();
    }

    // Wait a bit for tasks to finish
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    eprintln!("✅ Test completed!");
    eprintln!("Total messages published: {}", metrics.lock().unwrap().get_total_published());

    Ok(())
}

async fn run_producers(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let config = Arc::new(config.clone());
    let metrics = Arc::new(Mutex::new(GlobalMetrics::new(config.num_producers)));

    println!("Starting {} producers...", config.num_producers);

    let (_shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
    let (_pause_tx, pause_rx) = tokio::sync::watch::channel(false);

    let mut handles: Vec<JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>>> =
        Vec::new();

    for producer_id in 0..config.num_producers {
        let config_clone = config.clone();
        let client_metrics = metrics.lock().unwrap().clients[producer_id].clone();
        let shutdown_rx_clone = shutdown_rx.clone();
        let pause_rx_clone = pause_rx.clone();

        let handle: JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>> =
            tokio::spawn(async move {
            crate::producer::run_producer(producer_id, config_clone, Arc::new(client_metrics), shutdown_rx_clone, pause_rx_clone)
                .await
        });

        handles.push(handle);
    }

    // Wait for all tasks to finish (they should exit gracefully on shutdown signal)
    for handle in handles {
        let _ = handle.await;
    }

    Ok(())
}
