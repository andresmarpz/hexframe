import * as Cloudflare from "@alchemy.run/cloudflare";
import * as Effect from "effect/Effect";
import * as Layer from "effect/Layer";
import * as HttpServerRequest from "effect/http/HttpServerRequest";
import * as HttpServerResponse from "effect/http/HttpServerResponse";
import { Database, Storage } from "../src/resources.ts";

export default Cloudflare.Worker(
  "Probe",
  { main: import.meta.url },
  Effect.gen(function* () {
    const bucket = yield* Cloudflare.R2.ReadBucket(Storage);
    const db = yield* Cloudflare.D1.QueryDatabase(Database);

    return {
      fetch: Effect.gen(function* () {
        const request = yield* HttpServerRequest.HttpServerRequest;
        const url = new URL(request.url, "http://probe");

        if (url.pathname === "/objects") {
          const listing = yield* bucket.list({ prefix: url.searchParams.get("prefix") ?? "" });
          const objects = [];
          for (const entry of listing.objects) {
            const object = yield* bucket.get(entry.key);
            if (object) {
              objects.push({ key: entry.key, size: entry.size, body: yield* object.text() });
            }
          }
          return yield* HttpServerResponse.json(objects);
        }

        const table = url.pathname === "/rows/crash" ? "crash_reports" : "feedback";
        const rows = yield* db.prepare(`SELECT * FROM ${table} ORDER BY created_at`).all();
        return yield* HttpServerResponse.json(rows.results);
      }).pipe(Effect.orDie),
    };
  }).pipe(Effect.provide(Layer.mergeAll(Cloudflare.R2.ReadBucketBinding, Cloudflare.D1.QueryDatabaseBinding))),
);
