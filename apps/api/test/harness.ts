import * as Cloudflare from "@alchemy.run/cloudflare";
import * as Test from "@alchemy.run/test/Vitest";
import * as Alchemy from "alchemy";
import * as Effect from "effect/Effect";
import Stack from "./stack.ts";

export const { test, beforeAll, afterAll, deploy, destroy } = Test.make({
  providers: Cloudflare.providers(),
  state: Alchemy.localState(),
  dev: true,
});

const required = (value: string | undefined, name: string) => {
  if (value === undefined) throw new Error(`stack output ${name} is missing`);
  return value;
};

export const useStack = () => {
  const stack = beforeAll(deploy(Stack));
  afterAll(destroy(Stack));
  return Effect.map(stack, (outputs) => ({
    url: required(outputs.url, "url"),
    probeUrl: required(outputs.probeUrl, "probeUrl"),
  }));
};

export const send = (url: string, init?: RequestInit) => Effect.promise(() => fetch(url, init));

export const readJson = (response: Response) => Effect.promise(() => response.json() as Promise<unknown>);

export const postJson = (url: string, body: unknown, headers: Record<string, string> = {}) =>
  send(url, {
    method: "POST",
    headers: { "content-type": "application/json", "x-greyscreen-version": "0.1.8", ...headers },
    body: JSON.stringify(body),
  });

let counter = 0;
export const freshAddress = () => `203.0.113.${(counter += 1)}`;

export const validFeedback = (overrides: Record<string, unknown> = {}) => ({
  kind: "bug",
  message: "The overlay flickers after champion select.",
  email: "player@example.com",
  app_version: "0.1.8",
  os: "Windows 11 23H2",
  log_tail: "line one\nline two\n",
  client_id: crypto.randomUUID(),
  ...overrides,
});
