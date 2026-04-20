# plex-pinger

Small Rust daemon that watches Plex, qBittorrent, and your NAS, and sends a Pushover notification when something goes down (or comes back up).

## Quick start

1. Copy `.env.example` to `.env` and fill in your [Pushover](https://pushover.net) credentials:

   ```
   PUSHOVER_TOKEN=...
   PUSHOVER_USER=...
   ```

2. Edit the URLs in `docker-compose.yml` for your network. Anything you don't want to monitor, leave empty (`--nas-url=`) or remove the line.

3. Start it:

   ```
   docker compose up -d --build
   ```

4. Tail the logs:

   ```
   docker compose logs -f plex-pinger
   ```

## Config

All options are CLI flags (see `docker-compose.yml`):

| Flag | Default | Purpose |
|---|---|---|
| `--plex-url` | _(empty)_ | Plex identity endpoint, e.g. `http://10.0.0.5:32400/identity` |
| `--qbit-url` | _(empty)_ | qBittorrent WebUI, e.g. `http://10.0.0.5:8080` |
| `--nas-url` | _(empty)_ | NAS, `http://...` or `tcp://host:port` (e.g. `tcp://10.0.0.10:445` for SMB) |
| `--interval-seconds` | `60` | How often to check |
| `--timeout-seconds` | `5` | HTTP/TCP timeout per check |
| `--failure-threshold` | `2` | Consecutive failures before alerting (avoids false alarms) |
| `--startup-grace-seconds` | `30` | Wait time at startup so services can come up after a reboot |

Pushover credentials come from env vars (`PUSHOVER_TOKEN`, `PUSHOVER_USER`).

## What you get

- 🔴 alert when a service goes down
- 🟢 alert when it's back online (with total downtime)
- Re-alerts after 1h, 6h, and 24h if something stays down
- Auto-restart on Docker or host reboot (`restart: unless-stopped`)

## Building locally without Docker

```
cargo build --release
./target/release/plex-pinger --help
```
