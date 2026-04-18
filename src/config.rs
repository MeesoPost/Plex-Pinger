use clap::Parser;

#[derive(Parser, Debug)]
#[clap(name = "plex-pinger", about = "Monitors Plex and qBittorrent, sends Pushover alerts on failure")]
pub struct Config {
    #[clap(long, default_value = "http://192.168.1.169:32400/identity", help = "Plex identity endpoint")]
    pub plex_url: String,

    #[clap(long, default_value = "http://192.168.1.169:8080", help = "qBittorrent WebUI base URL")]
    pub qbit_url: String,

    #[clap(long, env = "PUSHOVER_TOKEN", help = "Pushover application token")]
    pub pushover_token: String,

    #[clap(long, env = "PUSHOVER_USER", help = "Pushover user key")]
    pub pushover_user: String,

    #[clap(long, default_value = "60", help = "Check interval in seconds")]
    pub interval_seconds: u64,

    #[clap(long, default_value = "5", help = "HTTP request timeout in seconds")]
    pub timeout_seconds: u64,
}
