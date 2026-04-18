mod config;
mod checker;
mod notifier;

use clap::Parser;
use config::Config;
use std::collections::HashMap;
use tokio::time::{interval, Duration};

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
                let message = format!("🔴 {} is down — {} niet bereikbaar", name, url);
                eprintln!("{}", message);
                if let Err(e) = notifier::send_pushover(
                    &config.pushover_token,
                    &config.pushover_user,
                    &message,
                ) {
                    eprintln!("Failed to send Pushover alert: {}", e);
                }
            } else if !was_healthy && healthy {
                let message = format!("🟢 {} is weer online", name);
                println!("{}", message);
                if let Err(e) = notifier::send_pushover(
                    &config.pushover_token,
                    &config.pushover_user,
                    &message,
                ) {
                    eprintln!("Failed to send Pushover recovery: {}", e);
                }
            }

            state.insert(name.to_string(), healthy);
        }
    }
}
