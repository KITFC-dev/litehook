# litehook

![Litehook Thumbnail](https://10ku.net/litehook/thumbnail.png)

Litehook is an async and lightweight tool that can monitor multiple public Telegram channels and send webhooks on new posts.

## Features

Litehook works by scraping public telegram channels at a set interval, which doesn't require any authorization. It saves posts to the database and sends webhook if the post is new. You can see the [Webhook Documentation](#webhook-documentation) below. You can also setup [Environment Variables](#environment-variables) for litehook.

### Dashboard

This is where you can manage and configure your listeners, you can create new, delete or edit listeners in here.

![Litehook Thumbnail](https://10ku.net/litehook/demo/dashboard-v2.0.png)

Dashboard features web UI with a [catppuccin](https://catppuccin.com/) pallete.

![Litehook Thumbnail](https://10ku.net/litehook/demo/create-listener-v2.0.png)

## Usage

### Running and Configuring litehook

1. Download the latest release from [Releases](https://github.com/KITFC-dev/litehook/releases/latest)
2. Before running the binary, you can setup [environment variables](#environment-variables)
3. Run the binary, and open <http://localhost:4101/> in your browser. Now you can add some listeners from the dashboard

### Running in Docker

1. Build the docker image with:

    ```bash
    docker compose build
    ```

2. Run the docker container with:

    ```bash
    docker compose up -d
    ```

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

Environment variables used by litehook, for example in your `.env` file in the same directory as the litehook binary

| Environment Variable | Description |
| --- | --- |
| PORT | Port for web interface, default is `4101` |
| POLL_INTERVAL | Poll interval in seconds that will be used as a default. Default is `600` |
| CHANNELS | IDs of channels to monitor separated by "," as an alternative to using web dashboard |
| WEBHOOK_URL | URL to API endpoint that will receive the webhook, will be used as a default. |
| WEBHOOK_SECRET | Webhook secret in `x-secret` header |
| PROXY_LIST_URL | URL to SOCKS5 proxy list, you can try using [IPLocate proxy list](https://github.com/iplocate/free-proxy-list). But be aware that proxy can be slow and timeout |
| DB_PATH | Path to SQLite database file, default is `data/litehook.db` |

> [!TIP]
> Set `POLL_INTERVAL` to something reasonable if you don't want to get rate limited by telegram, for example if the channel is posting rarely set to something like 300-600 seconds, otherwise set to a lower value

## Webhook Documentation

Webhook will be sent to `WEBHOOK_URL` with `POST` method, the server must return a `2xx` HTTP status code. 
The webhook will be retried 4 additional times with a 1 second interval. If all retries fail, the data is still stored in the database and webhook will be dropped.
It will include a `x-secret` header with the webhook secret from `WEBHOOK_SECRET` environment variable that you should verify on server before trusting the payload

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
        },
        ...
    ]
}
```
