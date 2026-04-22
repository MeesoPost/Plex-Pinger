mod checker;
mod config;
mod notifier;

use checker::{Check, CheckResult};
use clap::Parser;
use config::Config;
use std::thread;
use std::time::{Duration, Instant};
use ureq::{Agent, AgentBuilder};

/// Re-alert when a service has been continuously down for these durations.
const REALERT_AFTER: &[Duration] = &[
    Duration::from_secs(60 * 60),
    Duration::from_secs(6 * 60 * 60),
    Duration::from_secs(24 * 60 * 60),
];

struct Service {
    name: &'static str,
    check: Check,
    healthy: bool,
    consecutive_failures: u32,
    down_since: Option<Instant>,
    realerts_sent: usize,
}

impl Service {
    fn new(name: &'static str, check: Check) -> Self {
        Self {
            name,
            check,
            healthy: true,
            consecutive_failures: 0,
            down_since: None,
            realerts_sent: 0,
        }
    }

    /// Returns a recovery message if the service just transitioned to healthy.
    fn record_success(&mut self) -> Option<String> {
        self.consecutive_failures = 0;
        if self.healthy {
            return None;
        }
        self.healthy = true;
        self.realerts_sent = 0;
        let duration = self
            .down_since
            .take()
            .map(|t| format_duration(t.elapsed()))
            .unwrap_or_else(|| "unknown".to_string());
        Some(format!(
            "🟢 {} is back online (was down for {})",
            self.name, duration
        ))
    }

    /// Returns an alert message if the failure trips the threshold for the
    /// first time, or if a re-alert milestone is crossed.
    fn record_failure(&mut self, threshold: u32) -> Option<String> {
        self.consecutive_failures = self.consecutive_failures.saturating_add(1);

        if self.healthy && self.consecutive_failures >= threshold {
            self.healthy = false;
            self.down_since = Some(Instant::now());
            return Some(format!(
                "🔴 {} is down (after {} failed checks)",
                self.name, self.consecutive_failures
            ));
        }

        if !self.healthy {
            if let Some(down_since) = self.down_since {
                let elapsed = down_since.elapsed();
                if let Some(&next) = REALERT_AFTER.get(self.realerts_sent) {
                    if elapsed >= next {
                        self.realerts_sent += 1;
                        return Some(format!(
                            "🔴 {} is still down — offline for {}",
                            self.name,
                            format_duration(elapsed)
                        ));
                    }
                }
            }
        }

        None
    }
}

/// Human-readable duration showing the two largest non-zero units.
fn format_duration(d: Duration) -> String {
    let total = d.as_secs();
    let days = total / 86_400;
    let hours = (total % 86_400) / 3600;
    let minutes = (total % 3600) / 60;
    let secs = total % 60;

    let parts: Vec<String> = [
        (days, "day", "days"),
        (hours, "hour", "hours"),
        (minutes, "minute", "minutes"),
        (secs, "second", "seconds"),
    ]
    .iter()
    .filter(|(n, _, _)| *n > 0)
    .take(2)
    .map(|(n, sing, plur)| {
        let word = if *n == 1 { sing } else { plur };
        format!("{} {}", n, word)
    })
    .collect();

    if parts.is_empty() {
        "0 seconds".to_string()
    } else {
        parts.join(" ")
    }
}

fn main() {
    let config = Config::parse();

    if config.pushover_token.trim().is_empty() || config.pushover_user.trim().is_empty() {
        eprintln!("Pushover token or user is empty — refusing to start without a notifier.");
        eprintln!("Set --pushover-token and --pushover-user.");
        std::process::exit(1);
    }

    let agent: Agent = AgentBuilder::new()
        .timeout(Duration::from_secs(config.timeout_seconds))
        .build();

    println!(
        "OKO started — checking every {}s (alert after {} consecutive failures)",
        config.interval_seconds, config.failure_threshold
    );

    let mut services: Vec<Service> = [
        ("Plex", &config.plex_url),
        ("qBittorrent", &config.qbit_url),
        ("NAS", &config.nas_url),
    ]
    .into_iter()
    .filter_map(|(name, url)| {
        let url = url.trim();
        if url.is_empty() {
            println!("  {}: (disabled)", name);
            return None;
        }
        match Check::parse_url(url) {
            Ok(check) => {
                println!("  {}: {}", name, url);
                Some(Service::new(name, check))
            }
            Err(e) => {
                eprintln!("  {}: {}", name, e);
                std::process::exit(1);
            }
        }
    })
    .collect();

    if let Some(isp_ip) = config.isp_ip {
        println!("  VPN leak check: alert if current public IP == {}", isp_ip);
        services.push(Service::new("VPN", Check::VpnLeak(isp_ip)));
    }

    if services.is_empty() {
        eprintln!("No services configured — pass at least one URL or --isp-ip. Exiting.");
        std::process::exit(1);
    }

    let interval = Duration::from_secs(config.interval_seconds);
    let check_timeout = Duration::from_secs(config.timeout_seconds);

    if let Some(isp_ip) = config.isp_ip {
        match checker::run(&agent, &Check::VpnLeak(isp_ip), check_timeout) {
            CheckResult::Down => {
                eprintln!(
                    "WARNING: VPN appears OFF at startup — current public IP matches --isp-ip ({}).",
                    isp_ip
                );
                eprintln!("         If ProtonVPN is actually connected, your --isp-ip value is wrong.");
            }
            CheckResult::Up => println!("VPN OK at startup — public IP differs from --isp-ip"),
            CheckResult::Unknown => {
                eprintln!("note: could not verify VPN state at startup (no IP provider reachable).");
            }
        }
    }

    if config.startup_grace_seconds > 0 {
        println!(
            "waiting {}s for services to settle before first check",
            config.startup_grace_seconds
        );
        thread::sleep(Duration::from_secs(config.startup_grace_seconds));
    }

    loop {
        let cycle_start = Instant::now();

        for svc in &mut services {
            let result = checker::run(&agent, &svc.check, check_timeout);

            let message = match result {
                CheckResult::Up => svc.record_success(),
                CheckResult::Down => svc.record_failure(config.failure_threshold),
                CheckResult::Unknown => None,
            };

            if let Some(msg) = message {
                if result == CheckResult::Up {
                    println!("{}", msg);
                    notify(&agent, &config, &msg, "recovery");
                } else {
                    eprintln!("{}", msg);
                    notify(&agent, &config, &msg, "alert");
                }
            }
        }

        let elapsed = cycle_start.elapsed();
        if elapsed < interval {
            thread::sleep(interval - elapsed);
        } else {
            eprintln!(
                "warning: check cycle took {} (longer than interval of {}s)",
                format_duration(elapsed),
                interval.as_secs()
            );
        }
    }
}

fn notify(agent: &Agent, config: &Config, message: &str, kind: &str) {
    if let Err(e) = notifier::send_pushover(
        agent,
        &config.pushover_token,
        &config.pushover_user,
        message,
    ) {
        eprintln!("Failed to send Pushover {}: {}", kind, e);
    }
}
