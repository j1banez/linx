# Linx

[![CI](https://github.com/j1banez/linx/actions/workflows/ci.yml/badge.svg?branch=master)](https://github.com/j1banez/linx/actions/workflows/ci.yml)
[![Docker Image](https://img.shields.io/badge/docker-ghcr.io%2Fj1banez%2Flinx-blue)](https://github.com/j1banez/linx/pkgs/container/linx)

Linx is a simple, lightweight, self-hosted URL shortener.

## ⚡ Quick start

### Run with Docker (recommended)

```sh
docker run -d \
  --name linx \
  -p 3000:3000 \
  -v linx_data:/data \
  -e LINX_URL="http://localhost:3000" \
  ghcr.io/j1banez/linx:latest
```

Then open http://localhost:3000 in your browser.

Linx stores its data in a local SQLite database located at /data/linx.db
(persisted using a Docker volume).

Alternatively, use docker compose:

```yaml
services:
  linx:
    image: ghcr.io/j1banez/linx:latest
    container_name: linx
    ports:
      - "3000:3000"
    environment:
      LINX_URL: "http://localhost:3000"
    volumes:
      - linx_data:/data
    restart: unless-stopped

volumes:
  linx_data:
```

### Run from source

Install Rust: https://rust-lang.org/tools/install

```sh
git clone https://github.com/j1banez/linx.git
cd linx
LINX_URL=http://localhost:3000 cargo run
```

Note: replace `http://localhost:3000` with your actual domain or IP address in production.

## ⚙️ Configuration

### Environment variables

| Variable        | Required | Default                         | Description |
|-----------------|----------|---------------------------------|-------------|
| `LINX_URL`      | no       | `http://127.0.0.1:3000`         | Public base URL used to generate short links. This should match how users access the service (domain, port, https, etc.). |
| `DATABASE_URL`  | no       | docker: `sqlite:///data/linx.db`, source: `sqlite://./linx.db` | SQLite database location. Use a volume to persist data when running in Docker. |
| `DATABASE_MAX_CONNECTIONS` | no | `10`                            | Max SQLite pool connections. |
| `HOST`          | no       | `0.0.0.0`                       | HTTP listen host/IP for the Linx server (e.g. `127.0.0.1`, `0.0.0.0`, `::`). |
| `PORT`          | no       | `3000`                          | HTTP listen port for the Linx server. |
| `CODE_LEN`      | no       | `6`                             | Default short code length (allowed range 4-32). |
| `REDIRECT_CACHE_CAPACITY`  | no | `10000`                    | Max number of redirect entries kept in the in-memory LRU cache. |
| `RUST_LOG`      | no       | `info`                          | Log level (e.g. `debug`, `info`, `warn`, `error`). |

## ✨ Features

- Shorten URLs, allow base62 custom codes
- Basic stats: click counter and last-access date
- Minimal web UI plus JSON API
- Zero config SQLite storage

## 🧠 Design

- Redirects are served from an in-memory LRU cache to avoid frequent DB reads.
- Stats updates are fire-and-forget and batched in memory to reduce SQLite write contention.
- SQLite runs in WAL mode for better concurrency on reads.

## 🧩 API

- `GET /api/health`
  - Returns `ok`.
- `POST /api/shorten`
  - Request body:
    ```json
    {"url":"https://example.com","code":"custom"}
    ```
  - `code` is optional; if omitted, a random one is generated.
  - Response:
    ```json
    {"short_url":"https://your.domain/AbC123","code":"AbC123"}
    ```
- `GET /api/{code}/stats`
  - Response:
    ```json
    {
      "code":"AbC123",
      "url":"https://example.com/",
      "clicks":12,
      "created_at":1700000000,
      "last_accessed_at":1700000100
    }
    ```

## 🔐 Authentication

Linx does **not** implement authentication.

Do it at the reverse proxy layer (Traefik / Nginx / Caddy / Apache), or via your SSO gateway (Authelia, Authentik, Keycloak, etc.).

### Recommended strategy

#### Public redirects, protected admin
Make the **redirect route public** (so anyone can use short links),
but protect everything else (UI + stats + API).

Typical policy:

- **Public** (no auth)
  - `GET /{code}` (redirect)
- **Protected** (auth required)
  - `GET /` (home UI)
  - `POST /` (create short link)
  - `GET /{code}/stats` (stats UI)
  - `GET /static/*` (static files)
  - `GET /api/*` (API)
  - `POST /api/*` (API)

##### Example with Traefik (docker labels) and basic auth

```yaml
labels:
  # Basic auth middleware
  # 
  # Example:
  # user: linx, password: linx
  # Generate with `htpasswd -nb linx 'linx'` and double each $
  - traefik.http.middlewares.linx-basic-auth.basicauth.users=linx:$$apr1$$AfVj3cVu$$91q1.8/CwJLjwkUBjWJJ1/

  # API
  - traefik.http.routers.linx-api.priority=1000
  - traefik.http.routers.linx-api.rule=Host(`your.domain`) && PathPrefix(`/api`)
  - traefik.http.routers.linx-api.service=linx-websecure
  - traefik.http.routers.linx-api.middlewares=linx-basic-auth
  # UI
  - traefik.http.routers.linx-ui.priority=900
  - traefik.http.routers.linx-ui.rule=Host(`your.domain`) && PathPrefix(`/`)
  - traefik.http.routers.linx-ui.service=linx-websecure
  - traefik.http.routers.linx-ui.middlewares=linx-basic-auth
  # Redirects: /{code} (This one is public, no basic auth middleware)
  - traefik.http.routers.linx-redirect.priority=1100
  - traefik.http.routers.linx-redirect.rule=Host(`your.domain`) && PathRegexp(`^/[A-Za-z0-9]+$`)
  - traefik.http.routers.linx-redirect.service=linx-websecure
```

## 📸 Screenshots

![Home](public/screenshot1.webp)

![Stats](public/screenshot2.webp)

## 🧪 Benchmarking

Use the built-in script for reproducible local benchmarks:

```sh
./scripts/bench.sh
```

It won't represent real-world performance but at least it allows to compare performance on the
same host when modifying the code.

Notes:
- Runs a release build and benchmarks the redirect route without following redirects (`-r 0`).
- Uses a mixed hot/cold workload (80/20) to avoid single-code cache bias.
- You need `oha` installed.

## ❓ FAQ

>Is Linx multi-user?

Not yet. It's designed for single-owner/self-hosted use.

>Why is the click counter not working?

Browsers cache redirections when using http code 301 or 308 so if the same client
clicks multiple time, the counter will only update the first time.
