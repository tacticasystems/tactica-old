# configuration

Tactica reads configuration from environment variables when the
`tactica serve` command starts. There is no configuration file or
command-line flag for these values at present.

## environment variable naming

Environment variables use the `TACTICA_` prefix. A double underscore (`__`)
separates nested configuration sections from fields:

```text
TACTICA_<SECTION>__<FIELD>
```

For example, `api.bind_addr` is set with `TACTICA_API__BIND_ADDR`.
Environment variable names are case-insensitive, but the uppercase names shown
below are recommended.

The top-level sections are:

- `api`
- `auth`
- `database`
- `telemetry`

If an environment value cannot be deserialised, or a required value is missing,
the process exits while loading configuration.

> **Important:** API defaults are applied per field. You can set any individual
> `TACTICA_API__...` variable and the remaining API options retain their
> defaults.

## api

| option                          | type                     | default                   | environment variable                     | description                                                                                 |
| ------------------------------- | ------------------------ | ------------------------- | ---------------------------------------- | ------------------------------------------------------------------------------------------- |
| `api.bind_addr`                 | socket address           | `0.0.0.0:8080`            | `TACTICA_API__BIND_ADDR`                 | Local address and port on which the HTTP listener binds.                                    |
| `api.public_base_url`           | URL                      | `http://localhost:8080`   | `TACTICA_API__PUBLIC_BASE_URL`           | Public URL for the running API.                                                             |
| `api.request_timeout`           | duration                 | 30 seconds                | `TACTICA_API__REQUEST_TIMEOUT`           | Request timeout value.                                                                      |
| `api.graceful_shutdown_timeout` | duration                 | 10 seconds                | `TACTICA_API__GRACEFUL_SHUTDOWN_TIMEOUT` | Graceful shutdown timeout value.                                                            |
| `api.max_request_body_size`     | unsigned integer (bytes) | 1 MiB (1048576 bytes)     | `TACTICA_API__MAX_REQUEST_BODY_SIZE`     | Maximum request body size.                                                                  |
| `api.cors_allowed_origins`      | list of URLs             | `[http://localhost:5173]` | `TACTICA_API__CORS_ALLOWED_ORIGINS`      | Origins allowed by CORS.                                                                    |
| `api.csrf_cookie_domain`        | optional domain          | unset                     | `TACTICA_API__CSRF_COOKIE_DOMAIN`        | Parent cookie domain readable by the web app, such as `.tactica.systems`.                   |
| `api.proxy_mode`                | `Direct` or `Trusted`    | `Direct`                  | `TACTICA_API__PROXY_MODE`                | Whether requests are treated as coming directly from the client or through a trusted proxy. |

`request_timeout` and `graceful_shutdown_timeout` use
`std::time::Duration`. Figment's environment provider expects the duration's
`secs` and `nanos` fields rather than a human-readable value such as `30s`:

```sh
TACTICA_API__REQUEST_TIMEOUT='{secs=30,nanos=0}'
TACTICA_API__GRACEFUL_SHUTDOWN_TIMEOUT='{secs=10,nanos=0}'
```

`proxy_mode` is an externally tagged enum. Use `Direct` for the default mode,
or provide a positive hop count for `Trusted`:

```sh
TACTICA_API__PROXY_MODE=Direct
TACTICA_API__PROXY_MODE='{Trusted={hops=1}}'
```

`Trusted.hops` must be at least `1`. Lists use Figment's TOML-like array
syntax; quote the value in a shell command:

```sh
TACTICA_API__CORS_ALLOWED_ORIGINS='["https://a.example","https://b.example"]'
```

> **Current implementation note:** `bind_addr` is used by the server listener.
> The remaining API values are loaded into `ApiConfig`, but the current router
> does not yet apply `public_base_url`, the timeout values, the body-size limit,
> CORS, or proxy handling.

### complete api example

To override every API option explicitly:

```sh
export TACTICA_API__BIND_ADDR='0.0.0.0:8080'
export TACTICA_API__PUBLIC_BASE_URL='https://api.example.com'
export TACTICA_API__REQUEST_TIMEOUT='{secs=30,nanos=0}'
export TACTICA_API__GRACEFUL_SHUTDOWN_TIMEOUT='{secs=10,nanos=0}'
export TACTICA_API__MAX_REQUEST_BODY_SIZE='1048576'
export TACTICA_API__CORS_ALLOWED_ORIGINS='["https://app.example.com"]'
export TACTICA_API__CSRF_COOKIE_DOMAIN='.tactica.systems'
export TACTICA_API__PROXY_MODE='Direct'
```

The code currently sets `max_request_body_size` to 1048576 bytes (1 MiB).

## authentication

Authentication durations are configurable with Figment's `{secs=...,nanos=...}` duration syntax.

| option                      | default  | environment variable                 | description                                     |
| --------------------------- | -------- | ------------------------------------ | ----------------------------------------------- |
| `auth.idle_timeout`         | 30 days  | `TACTICA_AUTH__IDLE_TIMEOUT`         | Inactivity window before a Session expires.     |
| `auth.absolute_timeout`     | 180 days | `TACTICA_AUTH__ABSOLUTE_TIMEOUT`     | Maximum Session lifetime.                       |
| `auth.refresh_interval`     | 1 hour   | `TACTICA_AUTH__REFRESH_INTERVAL`     | Minimum interval between sliding-expiry writes. |
| `auth.verification_timeout` | 24 hours | `TACTICA_AUTH__VERIFICATION_TIMEOUT` | Email verification challenge lifetime.          |

## telemetry

| option                | type                  | default | environment variable           | description                                                          |
| --------------------- | --------------------- | ------- | ------------------------------ | -------------------------------------------------------------------- |
| `telemetry.log_level` | tracing filter string | `info`  | `TACTICA_TELEMETRY__LOG_LEVEL` | Fallback filter used when `RUST_LOG` is not set or cannot be parsed. |

`RUST_LOG` takes precedence at startup because telemetry first tries
`EnvFilter::try_from_default_env()`. For example:

```sh
TACTICA_TELEMETRY__LOG_LEVEL=debug tactica serve
RUST_LOG='tactica=debug,tower_http=info' tactica serve
```

The filter syntax is provided by `tracing-subscriber`; values such as `info`,
`debug`, and target-specific filters are supported.
