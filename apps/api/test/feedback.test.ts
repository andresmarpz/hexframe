import { expect } from "@effect/vitest";
import * as Effect from "effect/Effect";
import { freshAddress, postJson, readJson, send, test, useStack, validFeedback } from "./harness.ts";

const stack = useStack();

const probe = (path: string) =>
  Effect.gen(function* () {
    const { probeUrl } = yield* stack;
    return yield* readJson(yield* send(`${probeUrl}${path}`));
  });

test(
  "health reports ok and the package version",
  Effect.gen(function* () {
    const { url } = yield* stack;
    const response = yield* send(`${url}/health`);
    expect(response.status).toBe(200);
    expect(yield* readJson(response)).toEqual({ ok: true, version: "0.1.0" });
  }),
);

test(
  "valid feedback is accepted and stored in R2 and D1",
  Effect.gen(function* () {
    const { url } = yield* stack;
    const body = validFeedback({ message: "x".repeat(300) });
    const response = yield* postJson(`${url}/v1/feedback`, body, { "cf-connecting-ip": freshAddress() });
    expect(response.status).toBe(202);
    const { id } = (yield* readJson(response)) as { id: string };
    expect(id).toMatch(/^[0-9a-f-]{36}$/);

    const objects = (yield* probe("/objects?prefix=feedback/")) as Array<{ key: string; body: string }>;
    const stored = objects.find((object) => object.key.endsWith(`/${id}.json`));
    expect(stored).toBeDefined();
    expect(stored?.key).toMatch(/^feedback\/\d{4}\/\d{2}\/\d{2}\/[0-9a-f-]{36}\.json$/);
    expect(JSON.parse(stored?.body ?? "")).toEqual(body);

    const rows = (yield* probe("/rows/feedback")) as Array<Record<string, unknown>>;
    const row = rows.find((candidate) => candidate["id"] === id);
    expect(row).toMatchObject({
      kind: "bug",
      app_version: "0.1.8",
      os: "Windows 11 23H2",
      client_id: body.client_id,
      has_email: 1,
      has_logs: 1,
      status: "new",
    });
    expect(row?.["preview"]).toBe("x".repeat(200));
    expect(typeof row?.["created_at"]).toBe("number");
  }),
);

test(
  "null email and log_tail are accepted and flagged as absent",
  Effect.gen(function* () {
    const { url } = yield* stack;
    const body = validFeedback({ kind: "idea", email: null, log_tail: null });
    const response = yield* postJson(`${url}/v1/feedback`, body, { "cf-connecting-ip": freshAddress() });
    expect(response.status).toBe(202);
    const { id } = (yield* readJson(response)) as { id: string };
    const rows = (yield* probe("/rows/feedback")) as Array<Record<string, unknown>>;
    expect(rows.find((row) => row["id"] === id)).toMatchObject({ kind: "idea", has_email: 0, has_logs: 0 });
  }),
);

const invalidBodies: Array<[string, Record<string, unknown>]> = [
  ["unknown kind", validFeedback({ kind: "praise" })],
  ["empty message", validFeedback({ message: "" })],
  ["message over 5000 characters", validFeedback({ message: "a".repeat(5001) })],
  ["client_id that is not a uuid", validFeedback({ client_id: "not-a-uuid" })],
  ["log_tail over 200 KB", validFeedback({ log_tail: "a".repeat(200 * 1024 + 1) })],
  ["missing app_version", (({ app_version: _drop, ...rest }) => rest)(validFeedback())],
];

for (const [name, body] of invalidBodies) {
  test(
    `rejects ${name} with 400 and an error message`,
    Effect.gen(function* () {
      const { url } = yield* stack;
      const response = yield* postJson(`${url}/v1/feedback`, body, { "cf-connecting-ip": freshAddress() });
      expect(response.status).toBe(400);
      const payload = (yield* readJson(response)) as { error: unknown };
      expect(typeof payload.error).toBe("string");
    }),
  );
}

test(
  "rejects a body that is not JSON with 400",
  Effect.gen(function* () {
    const { url } = yield* stack;
    const response = yield* send(`${url}/v1/feedback`, {
      method: "POST",
      headers: { "content-type": "application/json", "cf-connecting-ip": freshAddress() },
      body: "{nope",
    });
    expect(response.status).toBe(400);
    expect(typeof ((yield* readJson(response)) as { error: unknown }).error).toBe("string");
  }),
);

test(
  "rejects a body over 512 KB with 413 and an error message",
  Effect.gen(function* () {
    const { url } = yield* stack;
    const response = yield* postJson(`${url}/v1/feedback`, validFeedback({ message: "a".repeat(700_000) }), {
      "cf-connecting-ip": freshAddress(),
    });
    expect(response.status).toBe(413);
    expect(typeof ((yield* readJson(response)) as { error: unknown }).error).toBe("string");
  }),
);

test(
  "accepts exactly 10 per hour per client_id and answers the 11th with 429",
  Effect.gen(function* () {
    const { url } = yield* stack;
    const clientId = crypto.randomUUID();
    for (let sent = 0; sent < 10; sent += 1) {
      const response = yield* postJson(`${url}/v1/feedback`, validFeedback({ client_id: clientId }), {
        "cf-connecting-ip": freshAddress(),
      });
      expect(response.status).toBe(202);
    }
    const blocked = yield* postJson(`${url}/v1/feedback`, validFeedback({ client_id: clientId }), {
      "cf-connecting-ip": freshAddress(),
    });
    expect(blocked.status).toBe(429);
    expect(typeof ((yield* readJson(blocked)) as { error: unknown }).error).toBe("string");

    const other = yield* postJson(`${url}/v1/feedback`, validFeedback(), { "cf-connecting-ip": freshAddress() });
    expect(other.status).toBe(202);
  }),
);

test(
  "limits by IP address across different client_ids",
  Effect.gen(function* () {
    const { url } = yield* stack;
    const address = freshAddress();
    for (let sent = 0; sent < 10; sent += 1) {
      const response = yield* postJson(`${url}/v1/feedback`, validFeedback(), { "cf-connecting-ip": address });
      expect(response.status).toBe(202);
    }
    const blocked = yield* postJson(`${url}/v1/feedback`, validFeedback(), { "cf-connecting-ip": address });
    expect(blocked.status).toBe(429);
  }),
);
