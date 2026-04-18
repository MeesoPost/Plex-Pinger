mod config;
mod checker;
mod notifier;

use clap::Parser;
use config::Config;
use std::collections::HashMap;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tokio::time::{interval, Duration};

fn format_duration(seconds: u64) -> String {
    if seconds < 60 {
        format!("{} seconden", seconds)
    } else if seconds < 3600 {
        let m = seconds / 60;
        let s = seconds % 60;
        if s == 0 {
            format!("{} minuten", m)
        } else {
            format!("{} min {} sec", m, s)
        }
    } else {
        let h = seconds / 3600;
        let m = (seconds % 3600) / 60;
        if m == 0 {
            format!("{} uur", h)
        } else {
            format!("{} uur {} min", h, m)
        }
    }
}

fn current_time_str() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let h = (secs % 86400) / 3600;
    let m = (secs % 3600) / 60;
    // Offset for Europe/Amsterdam (UTC+1 winter, UTC+2 zomer) — simpel: UTC+1
    let h_local = (h + 1) % 24;
    format!("{:02}:{:02}", h_local, m)
}

#[tokio::main]
async fn main() {
    let config = Config::parse();

    println!(
        "plex-pinger started — checking every {}s",
        config.interval_seconds
    );
    println!("  Plex:  {}", config.plex_url);
    println!("  qBit:  {}", config.qbit_url);

    let services: Vec<(&str, &str)> = vec![
        ("Plex", &config.plex_url),
        ("qBittorrent", &config.qbit_url),
    ];

    // true = healthy, false = down
    let mut state: HashMap<String, bool> = services
        .iter()
        .map(|(name, _)| (name.to_string(), true))
        .collect();

    // tijdstip waarop service down ging
    let mut down_since: HashMap<String, Instant> = HashMap::new();

    let mut ticker = interval(Duration::from_secs(config.interval_seconds));

    loop {
        ticker.tick().await;

        for (name, url) in &services {
            let healthy = if *name == "Plex" {
                checker::check_plex(url, config.timeout_seconds)
            } else {
                checker::check_qbit(url, config.timeout_seconds)
            };

            let was_healthy = *state.get(*name).unwrap_or(&true);

            if was_healthy && !healthy {
                let time = current_time_str();
                down_since.insert(name.to_string(), Instant::now());
                let message = format!("🔴 {} is down (sinds {})", name, time);
                eprintln!("{}", message);
                if let Err(e) = notifier::send_pushover(
                    &config.pushover_token,
                    &config.pushover_user,
                    &message,
                ) {
                    eprintln!("Failed to send Pushover alert: {}", e);
                }
            } else if !was_healthy && healthy {
                let duration_str = down_since
                    .get(*name)
                    .map(|t| format_duration(t.elapsed().as_secs()))
                    .unwrap_or_else(|| "onbekend".to_string());
                let message = format!("🟢 {} is weer online (was {} down)", name, duration_str);
                println!("{}", message);
                if let Err(e) = notifier::send_pushover(
                    &config.pushover_token,
                    &config.pushover_user,
                    &message,
                ) {
                    eprintln!("Failed to send Pushover recovery: {}", e);
                }
                down_since.remove(*name);
            }

            state.insert(name.to_string(), healthy);
        }
    }
}
