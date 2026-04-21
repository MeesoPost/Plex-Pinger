use std::fmt;
use std::net::{IpAddr, TcpStream, ToSocketAddrs};
use std::time::Duration;
use ureq::Agent;

/// Public-IP echo providers, tried in order. All IPv4-only so results compare
/// cleanly against a user-supplied IPv4 ISP address.
const IP_PROVIDERS: &[&str] = &[
    "https://api.ipify.org",
    "https://ipv4.icanhazip.com",
    "https://ipv4.seeip.org",
];

/// A fully-parsed, validated check. Constructing one proves the input was valid.
#[derive(Debug)]
pub enum Check {
    Http(String),
    Tcp(String),
    VpnLeak(IpAddr),
}

#[derive(Debug)]
pub enum CheckParseError {
    InvalidTcpAddress(String),
    UnsupportedScheme(String),
}

impl fmt::Display for CheckParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTcpAddress(s) => write!(f, "invalid tcp address (expected host:port): {}", s),
            Self::UnsupportedScheme(s) => write!(f, "unsupported URL scheme (expected http, https, or tcp): {}", s),
        }
    }
}

impl std::error::Error for CheckParseError {}

impl Check {
    /// Parse a URL string into a validated Check. VpnLeak is constructed
    /// separately (directly from an IpAddr clap gives us).
    pub fn parse_url(url: &str) -> Result<Self, CheckParseError> {
        match url.split_once("://") {
            Some(("http" | "https", _)) => Ok(Self::Http(url.to_string())),
            Some(("tcp", addr)) => {
                if !addr.contains(':') {
                    return Err(CheckParseError::InvalidTcpAddress(addr.to_string()));
                }
                Ok(Self::Tcp(addr.to_string()))
            }
            _ => Err(CheckParseError::UnsupportedScheme(url.to_string())),
        }
    }
}

/// Three-state result: Unknown means "we could not determine" and the caller
/// should not mutate state. Crucial for checks like VPN-leak where a network
/// blip must not be treated as a VPN outage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckResult {
    Up,
    Down,
    Unknown,
}

pub fn run(agent: &Agent, check: &Check, timeout: Duration) -> CheckResult {
    match check {
        Check::Http(url) => run_http(agent, url),
        Check::Tcp(addr) => run_tcp(addr, timeout),
        Check::VpnLeak(isp_ip) => run_vpn_leak(agent, *isp_ip),
    }
}

fn run_http(agent: &Agent, url: &str) -> CheckResult {
    match agent.get(url).call() {
        Ok(r) if r.status() == 200 => CheckResult::Up,
        _ => CheckResult::Down,
    }
}

fn run_tcp(addr: &str, timeout: Duration) -> CheckResult {
    let Some(sock_addr) = addr.to_socket_addrs().ok().and_then(|mut a| a.next()) else {
        return CheckResult::Down;
    };
    if TcpStream::connect_timeout(&sock_addr, timeout).is_ok() {
        CheckResult::Up
    } else {
        CheckResult::Down
    }
}

fn run_vpn_leak(agent: &Agent, isp_ip: IpAddr) -> CheckResult {
    match fetch_public_ip(agent) {
        Some(current) if current == isp_ip => CheckResult::Down,
        Some(_) => CheckResult::Up,
        None => CheckResult::Unknown,
    }
}

fn fetch_public_ip(agent: &Agent) -> Option<IpAddr> {
    for &url in IP_PROVIDERS {
        let Ok(response) = agent.get(url).call() else { continue };
        let Ok(body) = response.into_string() else { continue };
        if let Ok(ip) = body.trim().parse::<IpAddr>() {
            return Some(ip);
        }
    }
    None
}
