# litehook

Fetch public telegram channel and send a webhook on new posts. Litehook will fetch the channel page every `POLL_INTERVAL` seconds and send a webhook to `WEBHOOK_URL` if there are new posts. Posts are stored in a SQLite database in `data/litehook.db`. Also has support for SOCKS5 proxy.

## Installation

### Requirements

- [Rust](https://rust-lang.org/tools/install/) version 1.93.0 or higher
- [Cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html) version 1.93.0 or higher

> [!TIP]
> You can install Rust and Cargo by using [rustup](https://rustup.rs/).

### Build

1. Build the binary with:

    ```bash
    cargo build --release
    ```

2. And to start the server run:

    ```bash
    cargo run --release
    ```

### Running in Docker

1. Build the docker image with:

    ```bash
    docker compose build
    ```

2. Run the docker container with:

    ```bash
    docker compose up -d
    ```

## Configuration

Environment variables used by litehook, for example in your `.env` file.

| Environment Variable | Description |
| --- | --- |
| POLL_INTERVAL | Poll interval in seconds. Default is `600` |
| CHANNELS | Telegram channel IDs or URLs to monitor, separated by "," without spaces (e.g. `channel_id` or <https://t.me/s/channel_id>) |
| WEBHOOK_URL | URL to API endpoint that will receive the webhook |
| WEBHOOK_SECRET | Webhook secret in `x-secret` header |
| PROXY_LIST_URL | URL to SOCKS5 proxy list, you can try using [IPLocate proxy list](https://github.com/iplocate/free-proxy-list). But be aware that proxy can be slow and timeout. |
| DB_PATH | Path to SQLite database file, default is `data/litehook.db` |

> [!TIP]
> Set `POLL_INTERVAL` to something reasonable if you don't want to get rate limited by telegram, for example if the channel is posting rarely set to something like 300-600 seconds, otherwise set to a lower value.

## Webhook Documentation

Webhook will be sent to `WEBHOOK_URL` with `POST` method, the server must return a `2xx` HTTP status code, otherwise the webhook will be retried 4 additional times with a 1 second interval.
And will include a `x-secret` header with the webhook secret from `WEBHOOK_SECRET` environment variable that you should verify on server before trusting the payload.

Example of the webhook payload:

```json
{
    "channel": { 
        "id": "str",
        "name": "str",
        "image": URL of channel image,
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
            "media": [list of media URLs],
            "reactions": [
                {
                    "emoji": "♥",
                    "count": "35"
                }
            ],
            "views": "13.4K",
            "date": ISO 8601 date
        },
        ...
    ]
}
```
