import type * as Cloudflare from "@alchemy.run/cloudflare";
import * as Effect from "effect/Effect";
import type { FeedbackPayload } from "./contract.ts";
import { feedbackKey, messagePreview } from "./keys.ts";
import { rateLimit } from "./limits.ts";

export interface Store {
  readonly bucket: Cloudflare.R2.ReadWriteBucketClient;
  readonly db: Cloudflare.D1.QueryDatabaseClient;
}

const sinceWindow = (now: Date) => now.getTime() - rateLimit.windowMs;

export const feedbackCountInWindow = (store: Store, now: Date, clientId: string, ipHash: string) =>
  store.db
    .prepare(
      `SELECT COUNT(*) AS total FROM feedback
       WHERE created_at > ?1 AND (client_id = ?2 OR ip_hash = ?3)`,
    )
    .bind(sinceWindow(now), clientId, ipHash)
    .first<{ total: number }>()
    .pipe(Effect.map((row) => row?.total ?? 0));

export const crashCountInWindow = (store: Store, now: Date, ipHash: string) =>
  store.db
    .prepare("SELECT COUNT(*) AS total FROM crash_reports WHERE created_at > ?1 AND ip_hash = ?2")
    .bind(sinceWindow(now), ipHash)
    .first<{ total: number }>()
    .pipe(Effect.map((row) => row?.total ?? 0));

export const saveFeedback = (
  store: Store,
  input: { id: string; now: Date; ipHash: string; payload: FeedbackPayload },
) =>
  Effect.gen(function* () {
    const { id, now, ipHash, payload } = input;
    yield* store.bucket.put(feedbackKey(now, id), JSON.stringify(payload), {
      httpMetadata: { contentType: "application/json" },
    });
    yield* store.db
      .prepare(
        `INSERT INTO feedback
           (id, created_at, kind, app_version, os, client_id, ip_hash, has_email, has_logs, preview)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)`,
      )
      .bind(
        id,
        now.getTime(),
        payload.kind,
        payload.app_version,
        payload.os,
        payload.client_id,
        ipHash,
        payload.email !== null && payload.email.trim() !== "" ? 1 : 0,
        payload.log_tail !== null && payload.log_tail !== "" ? 1 : 0,
        messagePreview(payload.message),
      )
      .run();
  });

export const saveCrashRow = (
  store: Store,
  input: { id: string; now: Date; ipHash: string; version: string; minidumpBytes: number; prefix: string },
) =>
  store.db
    .prepare(
      `INSERT INTO crash_reports (id, created_at, app_version, ip_hash, minidump_bytes, object_prefix)
       VALUES (?1, ?2, ?3, ?4, ?5, ?6)`,
    )
    .bind(input.id, input.now.getTime(), input.version, input.ipHash, input.minidumpBytes, input.prefix)
    .run();
