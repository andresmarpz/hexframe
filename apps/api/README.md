# Greyscreen API

Business-logic backend for the Greyscreen desktop app. It runs on Cloudflare Workers with R2 and D1. Today it takes feedback and, experimentally, crash uploads. Auth and telemetry ingest come later.

Stack: TypeScript 7.0.2 (the native compiler, package `typescript`), Effect 4.1.5 (`HttpApi` and `Schema`), Alchemy 2.0.0-beta.87 (infrastructure as Effect code). Alchemy v2 is a beta, so pin its version and read its changelog before you bump it.

## Endpoints

### `GET /health`

Returns `{"ok":true,"version":"<api package version>"}`.

### `POST /v1/feedback`

JSON body, with the header `x-greyscreen-version` (the server also accepts the old `x-greyscreen-version`, which installed apps still send):

```json
{
  "kind": "bug | idea | other",
  "message": "1 to 5000 characters",
  "email": "string or null",
  "app_version": "string",
  "os": "string",
  "log_tail": "string or null, at most 200 KB",
  "client_id": "uuid"
}
```

- `202 {"id"}`: stored. The full JSON goes to R2 at `feedback/<yyyy>/<mm>/<dd>/<id>.json`. A row goes to the D1 table `feedback` (id, created_at, kind, app_version, os, client_id, has_email, has_logs, 200 character preview, status `new`, and a SHA-256 of the sender address).
- `400 {"error"}`: the body failed validation.
- `413 {"error"}`: the body is larger than 512 KB.
- `429 {"error"}`: more than 10 submissions in the last hour from the same `client_id` or the same IP address. The limit counts rows in D1.

### `POST /v1/crash` (experimental)

Not part of the desktop contract yet: the app does not call it. It can change without notice.

`multipart/form-data` with two parts: `minidump` (a file, at most 20 MB) and `json` (a JSON object, at most 64 KB). `app_version` comes from the JSON, or from the `x-greyscreen-version` header (or the old `x-greyscreen-version`) when the JSON has none. The whole body is capped at 25 MB.

- `202 {"id"}`: stored in R2 as `crashes/<version>/<yyyy>/<mm>/<dd>/<id>/minidump.dmp` and `meta.json`, and indexed in the D1 table `crash_reports`.
- `400`, `413`, `429`: same error shape. The limit is 10 uploads per hour per IP address.

There is no authentication yet. Anyone who finds the URL can send reports, so the rate limits are the only protection.

## Local development

No Cloudflare account is needed. Alchemy runs the Worker in workerd and emulates R2 and D1 on your machine.

```sh
bun install
just api-dev          # or: cd apps/api && bun run dev
curl <local url that Alchemy prints>/health
```

`local.env` holds placeholder Cloudflare credentials. Alchemy asks for credentials even when everything is local, and no call leaves the machine. Local state lives in `.alchemy/`.

## Tests

```sh
just api-test         # tests only
just api-check        # typecheck, then tests
```

The tests deploy the real Worker to the local emulators through the Alchemy test harness. A second "probe" Worker, defined in `test/`, reads R2 and D1 back so the tests can check what was stored. The first run takes about 20 seconds.

## Deploy

Deploying needs a Cloudflare account. Run these once:

```sh
cd apps/api
bun run login                      # Cloudflare sign-in through Alchemy (browser OAuth or an API token)
bun run deploy --stage prod        # shows a plan, asks to approve, then creates the Worker, R2 bucket and D1 database
```

Alchemy applies the SQL files in `migrations/` on every deploy. It keeps its own state in an R2 bucket in your account. Use `--stage dev` for a separate cloud copy. Stages never share resources.

The deploy prints `url: https://<name>.<account>.workers.dev`. The desktop app talks to `https://api.greyscreen.app` unless the `GREYSCREEN_API_URL` environment variable names another base, for example this URL.

## Layout

- `alchemy.run.ts`: the stack. Local runs use local state, deploys use Cloudflare state.
- `src/contract.ts`: request, response and error schemas, and the `HttpApi` definition.
- `src/worker.ts`: the Worker. It binds R2 and D1 and serves the API.
- `src/handlers.ts`, `src/crash.ts`, `src/store.ts`: endpoint logic and storage.
- `migrations/`: D1 schema.
