# ![Litehook Thumbnail](https://10ku.net/litehook/thumbnail.png)

## Litehook

Litehook is a self-hosted social media monitoring tool and webhook server. Supports public channels or private channels and even DMs if you use a self-bot. It has support for SOCKS proxies, Docker deployment, and includes a lightweight web dashboard.

## Overview

![Dashboard Screenshot](https://10ku.net/litehook/demo/dashboard-v3.0.png)

| Platform | Scraper Support | Self-Bot Support |
| -------- | --------------- | ---------------- |
| Telegram | ✅              | ✅               |

## Quick start

### Docker (recommended)

```bash
git clone https://github.com/KITFC-dev/litehook.git
cd litehook
docker compose up -d
```

### Build from Source

Download from [releases](https://github.com/KITFC-dev/litehook/releases/latest) or clone the repository and compile the binary with:

```bash
git clone https://github.com/KITFC-dev/litehook.git
cd litehook
cargo run
```

> [!NOTE]
> After that use the dashboard at <http://localhost:4101/> to configure the sources.

## How it works

Litehook works by scraping public telegram channels at a set interval, which doesn't require any authorization, or authenticate with user account to get all DMs and private channels. It saves posts to the database and sends webhook if the post is new. You can see the [Webhook Documentation](#webhook-documentation) below. You can also setup [Environment Variables](#environment-variables) for litehook.

## Build

### Requirements

- [Rust](https://rust-lang.org/tools/install/) version 1.93.0 or higher
- [Cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html) version 1.93.0 or higher

> [!TIP]
> You can install Rust and Cargo by using [rustup](https://rustup.rs/).

### Steps to build

1. Build the binary with:

   ```bash
   cargo build --release
   ```

2. And to start the server run:

   ```bash
   cargo run --release
   ```

## Environment Variables

Environment variables used by litehook, for example in your `.env` file in the same directory as the litehook binary.

| Environment Variable | Description                                                 |
| -------------------- | ----------------------------------------------------------- |
| PORT                 | Port for web interface, default is `4101`                   |
| WEBHOOK_SECRET       | Webhook secret in `x-secret` header                         |
| PROXY_LIST_URL       | URL to SOCKS5 proxy list                                    |
| DB_PATH              | Path to SQLite database file, default is `data/litehook.db` |

> [!TIP]
> You can try using [IPLocate proxy list](https://github.com/iplocate/free-proxy-list).
> Be aware that proxy can be slow and timeout the HTTP request.

## Webhook Documentation

Webhook will be sent to webhook url with `POST` method, the server must return a `2xx` HTTP status code, otherwise the webhook will be retried 4 additional times with a 1 second interval. If all retries fail, the data is still stored in the database and webhook will be dropped.
Webhook request will include a `x-secret` header with the webhook secret from `WEBHOOK_SECRET` environment variable that **you should verify on server before trusting the payload**.

Example of the webhook payload:

```json
{
  "channel": {
    "id": "str",
    "name": "str",
    "image": "https://...",
    "counters": {
      "subscribers": "1.2M",
      "photos": "392",
      "videos": "104",
      "links": "39"
    },
    "description": "str"
  },
  "new_posts": [
    {
      "id": "channel_id/post_id",
      "author": "str",
      "text": "str",
      "media": ["https://...", "https://..."],
      "reactions": [
        {
          "emoji": "♥",
          "count": "35"
        }
      ],
      "views": "13.4K",
      "date": "2026-03-04T12:00:00Z"
    }
  ]
}
```
