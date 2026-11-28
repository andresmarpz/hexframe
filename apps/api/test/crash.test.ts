import { expect } from "@effect/vitest";
import * as Effect from "effect/Effect";
import { freshAddress, readJson, send, test, useStack } from "./harness.ts";

const stack = useStack();

const upload = (url: string, parts: Record<string, string | Blob>, headers: Record<string, string> = {}) => {
  const form = new FormData();
  for (const [name, value] of Object.entries(parts)) {
    if (typeof value === "string") form.set(name, value);
    else form.set(name, value, `${name}.bin`);
  }
  return send(`${url}/v1/crash`, { method: "POST", headers: { "cf-connecting-ip": freshAddress(), ...headers }, body: form });
};

test(
  "stores the minidump and metadata under crashes/<version>/<date>/<id>/ and records a row",
  Effect.gen(function* () {
    const { url, probeUrl } = yield* stack;
    const dump = new Blob([new Uint8Array([77, 68, 77, 80, 1, 2, 3])]);
    const meta = { app_version: "0.1.8", os: "Windows 11" };
    const response = yield* upload(url, { minidump: dump, json: JSON.stringify(meta) });
    expect(response.status).toBe(202);
    const { id } = (yield* readJson(response)) as { id: string };

    const objects = (yield* readJson(yield* send(`${probeUrl}/objects?prefix=crashes/0.1.8/`))) as Array<{
      key: string;
      size: number;
      body: string;
    }>;
    const mine = objects.filter((object) => object.key.includes(`/${id}/`));
    expect(mine.map((object) => object.key.split("/").pop()).sort()).toEqual(["meta.json", "minidump.dmp"]);
    expect(mine.find((object) => object.key.endsWith("minidump.dmp"))?.size).toBe(7);
    expect(JSON.parse(mine.find((object) => object.key.endsWith("meta.json"))?.body ?? "")).toEqual(meta);

    const rows = (yield* readJson(yield* send(`${probeUrl}/rows/crash`))) as Array<Record<string, unknown>>;
    expect(rows.find((row) => row["id"] === id)).toMatchObject({ app_version: "0.1.8", minidump_bytes: 7 });
  }),
);

test(
  "takes the version from x-greyscreen-version when the JSON has none",
  Effect.gen(function* () {
    const { url } = yield* stack;
    const response = yield* upload(
      url,
      { minidump: new Blob([new Uint8Array([1])]), json: JSON.stringify({ os: "Windows 11" }) },
      { "x-greyscreen-version": "0.1.8" },
    );
    expect(response.status).toBe(202);
  }),
);

test(
  "rejects a missing minidump with 400",
  Effect.gen(function* () {
    const { url } = yield* stack;
    const response = yield* upload(url, { json: JSON.stringify({ app_version: "0.1.8" }) });
    expect(response.status).toBe(400);
  }),
);

test(
  "rejects an unsafe app_version with 400",
  Effect.gen(function* () {
    const { url } = yield* stack;
    const response = yield* upload(url, {
      minidump: new Blob([new Uint8Array([1])]),
      json: JSON.stringify({ app_version: "../../etc" }),
    });
    expect(response.status).toBe(400);
  }),
);

test(
  "rejects a non-multipart body with 400",
  Effect.gen(function* () {
    const { url } = yield* stack;
    const response = yield* send(`${url}/v1/crash`, {
      method: "POST",
      headers: { "content-type": "application/json", "cf-connecting-ip": freshAddress() },
      body: "{}",
    });
    expect(response.status).toBe(400);
  }),
);

test(
  "rejects an oversized declared body with 413",
  Effect.gen(function* () {
    const { url } = yield* stack;
    const response = yield* upload(url, {
      minidump: new Blob([new Uint8Array(21 * 1024 * 1024)]),
      json: JSON.stringify({ app_version: "0.1.8" }),
    });
    expect(response.status).toBe(413);
  }),
);

test(
  "limits crash uploads to 10 per hour per IP",
  Effect.gen(function* () {
    const { url } = yield* stack;
    const address = freshAddress();
    for (let sent = 0; sent < 10; sent += 1) {
      const response = yield* upload(
        url,
        { minidump: new Blob([new Uint8Array([1])]), json: JSON.stringify({ app_version: "0.1.8" }) },
        { "cf-connecting-ip": address },
      );
      expect(response.status).toBe(202);
    }
    const blocked = yield* upload(
      url,
      { minidump: new Blob([new Uint8Array([1])]), json: JSON.stringify({ app_version: "0.1.8" }) },
      { "cf-connecting-ip": address },
    );
    expect(blocked.status).toBe(429);
  }),
);
