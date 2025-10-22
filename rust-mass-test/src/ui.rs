use crate::config::Config;
use crate::metrics::GlobalMetrics;
use crossterm::event::{self, Event, KeyCode};
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Frame;
use std::time::Duration;

pub enum UIState {
    ConfigInput,
    Running,
}

pub struct UIContext {
    pub state: UIState,
    pub config: Config,
    pub field_index: usize,
    pub input_buffer: String,
    pub in_edit_mode: bool,
}

impl UIContext {
    pub fn new() -> Self {
        UIContext {
            state: UIState::ConfigInput,
            config: Config::default(),
            field_index: 0,
            input_buffer: String::new(),
            in_edit_mode: false,
        }
    }

    pub fn next_field(&mut self) {
        self.field_index = (self.field_index + 1) % 10; // 10 fields total
        self.input_buffer.clear();
        self.in_edit_mode = false;
    }

    pub fn prev_field(&mut self) {
        if self.field_index == 0 {
            self.field_index = 9;
        } else {
            self.field_index -= 1;
        }
        self.input_buffer.clear();
        self.in_edit_mode = false;
    }

    pub fn update_field(&mut self) {
        if self.input_buffer.is_empty() {
            return;
        }

        match self.field_index {
            0 => self.config.broker_host = self.input_buffer.clone(),
            1 => {
                if let Ok(port) = self.input_buffer.parse::<u16>() {
                    self.config.broker_port = port;
                }
            }
            2 => {
                if let Ok(n) = self.input_buffer.parse::<usize>() {
                    self.config.num_producers = n;
                }
            }
            3 => {
                if let Ok(n) = self.input_buffer.parse::<usize>() {
                    self.config.num_topics = n;
                }
            }
            4 => {
                if let Ok(n) = self.input_buffer.parse::<usize>() {
                    self.config.topics_per_node = n;
                }
            }
            5 => {
                if let Ok(n) = self.input_buffer.parse::<usize>() {
                    self.config.max_depth = n.max(1).min(10);
                }
            }
            6 => {
                if let Ok(n) = self.input_buffer.parse::<u64>() {
                    self.config.sleep_ms = n;
                }
            }
            7 => {
                if let Ok(q) = self.input_buffer.parse::<i32>() {
                    self.config.qos = q.max(0).min(2);
                }
            }
            8 => {
                self.config.retained = self.input_buffer.to_lowercase() == "true"
                    || self.input_buffer == "1";
            }
            9 => self.config.topic_prefix = self.input_buffer.clone(),
            _ => {}
        }

        self.input_buffer.clear();
    }
}

pub fn draw_config_screen(f: &mut Frame, ui: &UIContext) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(15),
            Constraint::Length(3),
        ])
        .split(f.area());

    // Header
    let header = Paragraph::new("MQTT Test Program Configuration")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
    f.render_widget(header, chunks[0]);

    // Config fields
    let broker_host_str = ui.config.broker_host.clone();
    let broker_port_str = ui.config.broker_port.to_string();
    let num_producers_str = ui.config.num_producers.to_string();
    let num_topics_str = ui.config.num_topics.to_string();
    let topics_per_node_str = ui.config.topics_per_node.to_string();
    let max_depth_str = ui.config.max_depth.to_string();
    let sleep_ms_str = ui.config.sleep_ms.to_string();
    let qos_str = ui.config.qos.to_string();
    let retained_str = ui.config.retained.to_string();
    let topic_prefix_str = ui.config.topic_prefix.clone();

    let fields: Vec<(&str, String)> = vec![
        ("Broker Host", broker_host_str),
        ("Broker Port", broker_port_str),
        ("Producers", num_producers_str),
        ("Num Topics", num_topics_str),
        ("Topics per Node", topics_per_node_str),
        ("Max Depth", max_depth_str),
        ("Sleep (ms)", sleep_ms_str),
        ("QoS", qos_str),
        ("Retained", retained_str),
        ("Topic Prefix", topic_prefix_str),
    ];

    let mut items = Vec::new();
    for (idx, (label, value)) in fields.iter().enumerate() {
        let is_selected = ui.field_index == idx;
        let is_editing = ui.in_edit_mode && is_selected;

        let style = if is_selected {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        // Show input buffer if editing, otherwise show current value
        let display_value = if is_editing {
            format!("> {}", ui.input_buffer)
        } else {
            value.clone()
        };

        let line = Line::from(vec![
            Span::styled(format!("{:<20}", label), style),
            Span::styled(display_value, style),
        ]);
        items.push(ListItem::new(line));
    }

    let list = List::new(items).block(Block::default().borders(Borders::ALL).title(" Config "));
    f.render_widget(list, chunks[1]);

    // Footer
    let footer_text = vec![
        Span::raw("↑/↓: Navigate | "),
        Span::styled("ENTER", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(": Edit/Confirm | "),
        Span::styled("S", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(": Save | "),
        Span::styled("SPACE", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(": Start | "),
        Span::styled("Q", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(": Quit"),
    ];
    let footer = Paragraph::new(Line::from(footer_text))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Green));
    f.render_widget(footer, chunks[2]);
}

pub fn draw_confirmation_screen(f: &mut Frame, config: &Config) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(f.area());

    // Header
    let header = Paragraph::new("Confirm Configuration")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
    f.render_widget(header, chunks[0]);

    // Config summary
    let summary = format!(
        "Broker: {}:{}\nProducers: {}\nTopics per Producer: {} (depth: {})\nSleep: {}ms\nQoS: {} | Retained: {}\n\nReady to start?",
        config.broker_host,
        config.broker_port,
        config.num_producers,
        config.num_topics,
        config.max_depth,
        config.sleep_ms,
        config.qos,
        config.retained,
    );

    let summary_widget = Paragraph::new(summary)
        .block(Block::default().borders(Borders::ALL).title(" Summary "))
        .style(Style::default().fg(Color::White));
    f.render_widget(summary_widget, chunks[1]);

    // Footer
    let footer_text = vec![
        Span::styled("ENTER", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(": Start | "),
        Span::styled("ESC", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(": Back | "),
        Span::styled("Q", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(": Quit"),
    ];
    let footer = Paragraph::new(Line::from(footer_text))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Gray));
    f.render_widget(footer, chunks[2]);
}

pub fn draw_metrics_screen(
    f: &mut Frame,
    metrics: &GlobalMetrics,
    uptime: Duration,
) {
    let total_area = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Length(8), Constraint::Min(10)])
        .split(total_area);

    // Global metrics
    let total_vps = metrics.get_total_vps();
    let total_published = metrics.get_total_published();
    let uptime_secs = uptime.as_secs();
    let uptime_str = format!(
        "{}:{:02}:{:02}",
        uptime_secs / 3600,
        (uptime_secs % 3600) / 60,
        uptime_secs % 60
    );

    let global_info = format!(
        "Global Metrics\n\
         ═════════════════════════════════════════════════════════════\n\
         Total Published: {}  |  Global v/s: {:.2}\n\
         Uptime: {}  |  Active Producers: {}\n\
         ═════════════════════════════════════════════════════════════\n\
         Press Q to STOP the test",
        total_published, total_vps, uptime_str, metrics.producers.len(),
    );

    let global_widget = Paragraph::new(global_info)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD));
    f.render_widget(global_widget, chunks[0]);

    // Per-producer metrics
    let per_producer_metrics: Vec<String> = metrics
        .producers
        .iter()
        .map(|p| {
            format!(
                "Producer {:3}: Total={:8}  v/s={:7.2}  Counter={:8}",
                p.id + 1,
                p.get_total(),
                p.calculate_vps(),
                p.get_counter()
            )
        })
        .collect();

    let producer_lines: Vec<ListItem> = per_producer_metrics
        .iter()
        .map(|line| ListItem::new(line.clone()))
        .collect();

    let producer_list = List::new(producer_lines)
        .block(Block::default().borders(Borders::ALL).title(" Per-Producer Metrics "))
        .style(Style::default().fg(Color::Yellow));
    f.render_widget(producer_list, chunks[1]);
}

pub async fn handle_ui_input(ui: &mut UIContext) -> Option<bool> {
    if event::poll(Duration::from_millis(50)).ok()? {
        if let Event::Key(key) = event::read().ok()? {
            match key.code {
                KeyCode::Char('q') | KeyCode::Char('Q') => return Some(false),
                KeyCode::Char(c @ 's') | KeyCode::Char(c @ 'S') => {
                    // Save configuration - only if not currently typing
                    if matches!(ui.state, UIState::ConfigInput) && ui.input_buffer.is_empty() {
                        match ui.config.save("config.json") {
                            Ok(_) => {
                                eprintln!("\n✅ Configuration saved to config.json");
                            }
                            Err(e) => {
                                eprintln!("\n❌ Failed to save configuration: {}", e);
                            }
                        }
                    } else if matches!(ui.state, UIState::ConfigInput) {
                        // If we're typing, add 's' or 'S' to the buffer
                        ui.input_buffer.push(c);
                    }
                }
                KeyCode::Up => {
                    if !ui.in_edit_mode {
                        ui.prev_field();
                    }
                }
                KeyCode::Down => {
                    if !ui.in_edit_mode {
                        ui.next_field();
                    }
                }
                KeyCode::Enter => {
                    if matches!(ui.state, UIState::ConfigInput) {
                        if ui.in_edit_mode {
                            // Confirm the value and exit edit mode
                            ui.update_field();
                            ui.in_edit_mode = false;
                        } else {
                            // Enter edit mode
                            ui.in_edit_mode = true;
                            ui.input_buffer.clear();
                        }
                    }
                }
                KeyCode::Char(c) => {
                    if matches!(ui.state, UIState::ConfigInput) {
                        if c == ' ' && !ui.in_edit_mode {
                            // SPACE starts the test immediately (only when not editing)
                            ui.state = UIState::Running;
                            return Some(true);
                        } else if ui.in_edit_mode {
                            // Add character to buffer only when in edit mode
                            ui.input_buffer.push(c);
                        }
                    }
                }
                KeyCode::Backspace => {
                    if matches!(ui.state, UIState::ConfigInput) && ui.in_edit_mode {
                        ui.input_buffer.pop();
                    }
                }
                _ => {}
            }
        }
    }
    None
}
