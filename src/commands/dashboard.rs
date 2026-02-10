use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Line},
    widgets::{Block, Borders, List, ListItem, Sparkline},
    Terminal,
};
use std::{io, time::Duration};
use tokio::sync::mpsc;
use crate::clash::api::{ClashClient, Traffic};
use futures_util::StreamExt;

#[derive(Debug)]
enum AppEvent {
    Input(Event),
    Tick,
    Traffic(Traffic),
    Log(String),
    Proxies(Vec<(String, String, String)>), // Name, Now, Delay
}

struct App {
    logs: Vec<String>,
    traffic_up: Vec<u64>,
    traffic_down: Vec<u64>,
    proxies: Vec<(String, String, String)>,
    should_quit: bool,
}

impl App {
    fn new() -> Self {
        Self {
            logs: Vec::new(),
            traffic_up: vec![0; 300],
            traffic_down: vec![0; 300],
            proxies: Vec::new(),
            should_quit: false,
        }
    }

    fn on_traffic(&mut self, traffic: Traffic) {
        self.traffic_up.push(traffic.up);
        if self.traffic_up.len() > 300 {
            self.traffic_up.remove(0);
        }
        self.traffic_down.push(traffic.down);
        if self.traffic_down.len() > 300 {
            self.traffic_down.remove(0);
        }
    }

    fn on_log(&mut self, log: String) {
        self.logs.push(log);
        if self.logs.len() > 50 {
            self.logs.remove(0);
        }
    }
}

pub async fn run() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let app = App::new();
    let res = run_app(&mut terminal, app).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}

async fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> Result<()> {
    let (tx, mut rx) = mpsc::channel(100);
    let tick_rate = Duration::from_millis(250);

    // Event loop for inputs
    let tx_input = tx.clone();
    tokio::spawn(async move {
        let mut reader = crossterm::event::EventStream::new();
        loop {
            let delay = tokio::time::sleep(tick_rate);
            tokio::select! {
                _ = delay => {
                    if tx_input.send(AppEvent::Tick).await.is_err() {
                        break;
                    }
                }
                maybe_event = reader.next() => {
                    match maybe_event {
                        Some(Ok(evt)) => {
                            if tx_input.send(AppEvent::Input(evt)).await.is_err() {
                                break;
                            }
                        }
                        Some(Err(_)) => break,
                        None => break,
                    }
                }
            }
        }
    });

    // Traffic stream
    let client = ClashClient::new();
    let tx_traffic = tx.clone();
    let client_clone = ClashClient::new();
    tokio::spawn(async move {
        if let Ok(mut response) = client_clone.stream_traffic().await {
            while let Some(chunk) = response.chunk().await.unwrap_or(None) {
                let s = String::from_utf8_lossy(&chunk);
                for line in s.lines() {
                     if let Ok(t) = serde_json::from_str::<Traffic>(line) {
                         if tx_traffic.send(AppEvent::Traffic(t)).await.is_err() {
                             return;
                         }
                     }
                }
            }
        }
    });

    // Log stream
    let tx_log = tx.clone();
    let client_clone2 = ClashClient::new();
    tokio::spawn(async move {
         if let Ok(mut response) = client_clone2.stream_logs().await {
            while let Some(chunk) = response.chunk().await.unwrap_or(None) {
                let s = String::from_utf8_lossy(&chunk);
                for line in s.lines() {
                     // Parse JSON log: {"type":"info","payload":"..."}
                     if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
                         if let Some(payload) = v.get("payload").and_then(|p| p.as_str()) {
                             if tx_log.send(AppEvent::Log(payload.to_string())).await.is_err() {
                                 return;
                             }
                         }
                     }
                }
            }
        }
    });

    // Proxies poller (every 2s)
    let tx_proxies = tx.clone();
    let client_clone3 = ClashClient::new();
    tokio::spawn(async move {
        loop {
            if let Ok(proxies) = client_clone3.get_proxies().await {
                let mut data = Vec::new();
                for (name, item) in proxies {
                    if item.proxy_type == "Selector" || item.proxy_type == "URLTest" {
                        let now = item.now.unwrap_or_default();
                        let delay = if let Some(history) = item.history {
                            history.last().map(|h| h.delay.to_string()).unwrap_or("0".to_string())
                        } else {
                            "0".to_string()
                        };
                         data.push((name, now, delay));
                    }
                }
                // Sort
                 data.sort_by(|a, b| {
                     if a.0 == "GLOBAL" { std::cmp::Ordering::Less }
                     else if b.0 == "GLOBAL" { std::cmp::Ordering::Greater }
                     else { a.0.cmp(&b.0) }
                 });

                if tx_proxies.send(AppEvent::Proxies(data)).await.is_err() {
                    break;
                }
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    });

    loop {
        terminal.draw(|f| ui(f, &app))?;

        match rx.recv().await {
            Some(AppEvent::Input(event)) => {
                 if let Event::Key(key) = event {
                    if key.kind == KeyEventKind::Press {
                        if key.code == KeyCode::Char('q') || key.code == KeyCode::Esc {
                            app.should_quit = true;
                        }
                    }
                }
            }
            Some(AppEvent::Tick) => {}
            Some(AppEvent::Traffic(t)) => app.on_traffic(t),
            Some(AppEvent::Log(l)) => app.on_log(l),
            Some(AppEvent::Proxies(p)) => app.proxies = p,
            None => break,
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}

fn ui(f: &mut ratatui::Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(66),
            Constraint::Percentage(34),
        ])
        .split(f.size());
    
    // Top section: Logs (Left 2/3) and Proxies (Right 1/3)
    let top_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(66),
            Constraint::Percentage(34),
        ])
        .split(chunks[0]);

    // Logs (Show newest at bottom, so iterate normally)
    // We only have space for N lines.
    // Let's just take the last N lines.
    let log_height = top_chunks[0].height as usize;
    let skip = if app.logs.len() > log_height { app.logs.len() - log_height + 2 } else { 0 };
    
    let logs: Vec<ListItem> = app.logs.iter()
        .skip(skip)
        .map(|l| ListItem::new(Line::from(Span::raw(l))))
        .collect();
    
    let logs_widget = List::new(logs)
        .block(Block::default().borders(Borders::ALL).title("日志 (Real-time Logs)"));
    f.render_widget(logs_widget, top_chunks[0]);

    // Proxies
    let proxies: Vec<ListItem> = app.proxies.iter()
        .map(|(name, now, delay)| {
            let delay_val = delay.parse::<u64>().unwrap_or(0);
            let style = if delay_val > 0 && delay_val < 500 { Style::default().fg(Color::Green) } 
                       else if delay_val == 0 { Style::default().fg(Color::Red) }
                       else { Style::default().fg(Color::Yellow) };
            
            ListItem::new(Line::from(vec![
                Span::styled(format!("{}: ", name), Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format!("{} ", now)),
                Span::styled(format!("({}ms)", delay), style),
            ]))
        })
        .collect();

    let proxies_widget = List::new(proxies)
        .block(Block::default().borders(Borders::ALL).title("节点状态 (Proxies)"));
    f.render_widget(proxies_widget, top_chunks[1]);

    // Bottom: Traffic
    let bottom_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(chunks[1]);

    let sparkline_up = Sparkline::default()
        .block(Block::default().borders(Borders::ALL).title(format!("上传速度 (Upload): {}/s", format_speed(*app.traffic_up.last().unwrap_or(&0)))))
        .data(&app.traffic_up)
        .style(Style::default().fg(Color::Green));
    f.render_widget(sparkline_up, bottom_chunks[0]);

    let sparkline_down = Sparkline::default()
        .block(Block::default().borders(Borders::ALL).title(format!("下载速度 (Download): {}/s", format_speed(*app.traffic_down.last().unwrap_or(&0)))))
        .data(&app.traffic_down)
        .style(Style::default().fg(Color::Green));
    f.render_widget(sparkline_down, bottom_chunks[1]);
}

fn format_speed(speed: u64) -> String {
    if speed < 1024 {
        format!("{} B", speed)
    } else if speed < 1024 * 1024 {
        format!("{:.1} KB", speed as f64 / 1024.0)
    } else {
        format!("{:.1} MB", speed as f64 / 1024.0 / 1024.0)
    }
}
