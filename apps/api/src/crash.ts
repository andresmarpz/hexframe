import * as Effect from "effect/Effect";
import * as HttpRouter from "effect/http/HttpRouter";
import * as HttpServerRequest from "effect/http/HttpServerRequest";
import * as HttpServerResponse from "effect/http/HttpServerResponse";
import { crashPrefix, hashClientAddress, isSafeVersion } from "./keys.ts";
import { maxCrashBodyBytes, maxCrashMetaBytes, maxMinidumpBytes, rateLimit } from "./limits.ts";
import { crashCountInWindow, saveCrashRow, type Store } from "./store.ts";

const reject = (status: number, error: string) => HttpServerResponse.jsonUnsafe({ error }, { status });

const clientAddress = (request: HttpServerRequest.HttpServerRequest) =>
  request.headers["cf-connecting-ip"] ?? "unknown";

const readMeta = async (value: string | { text(): Promise<string> } | null) => {
  if (value === null) return undefined;
  const text = typeof value === "string" ? value : await value.text();
  if (new TextEncoder().encode(text).length > maxCrashMetaBytes) return undefined;
  try {
    const parsed: unknown = JSON.parse(text);
    return typeof parsed === "object" && parsed !== null && !Array.isArray(parsed)
      ? (parsed as Record<string, unknown>)
      : undefined;
  } catch {
    return undefined;
  }
};

const parseUpload = (request: Request) =>
  Effect.promise(async () => {
    const form = await request.formData();
    const minidump = form.get("minidump");
    const meta = await readMeta(form.get("json"));
    return { minidump: typeof minidump === "string" || minidump === null ? undefined : minidump, meta };
  }).pipe(Effect.catchDefect(() => Effect.succeed({ minidump: undefined, meta: undefined })));

export const crashRoute = (store: Store) =>
  HttpRouter.add(
    "POST",
    "/v1/crash",
    Effect.gen(function* () {
      const request = yield* HttpServerRequest.HttpServerRequest;
      const declared = Number(request.headers["content-length"] ?? "0");
      if (!Number.isFinite(declared) || declared <= 0) return reject(400, "multipart body required");
      if (declared > maxCrashBodyBytes) return reject(413, "request body too large");
      if (!(request.headers["content-type"] ?? "").toLowerCase().startsWith("multipart/form-data")) {
        return reject(400, "multipart/form-data required");
      }

      const now = new Date();
      const ipHash = yield* Effect.promise(() => hashClientAddress(clientAddress(request)));
      if ((yield* crashCountInWindow(store, now, ipHash)) >= rateLimit.perHour) {
        return reject(429, "too many crash reports, try again later");
      }

      const { minidump, meta } = yield* parseUpload(request.source as Request);
      if (minidump === undefined || meta === undefined) {
        return reject(400, "parts `minidump` (file) and `json` (JSON object) are required");
      }
      if (minidump.size === 0 || minidump.size > maxMinidumpBytes) {
        return reject(minidump.size === 0 ? 400 : 413, "minidump size out of range");
      }

      const appVersion = typeof meta["app_version"] === "string" ? meta["app_version"] : request.headers["x-greyscreen-version"];
      if (appVersion === undefined || !isSafeVersion(appVersion)) {
        return reject(400, "app_version missing or invalid");
      }

      const id = crypto.randomUUID();
      const prefix = crashPrefix(appVersion, now, id);
      yield* store.bucket.put(`${prefix}minidump.dmp`, yield* Effect.promise(() => minidump.arrayBuffer()), {
        httpMetadata: { contentType: "application/octet-stream" },
      });
      yield* store.bucket.put(`${prefix}meta.json`, JSON.stringify(meta), {
        httpMetadata: { contentType: "application/json" },
      });
      yield* saveCrashRow(store, { id, now, ipHash, version: appVersion, minidumpBytes: minidump.size, prefix });
      return HttpServerResponse.jsonUnsafe({ id }, { status: 202 });
    }).pipe(Effect.orDie),
  );
