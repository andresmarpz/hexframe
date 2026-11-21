import * as Effect from "effect/Effect";
import * as HttpServerRequest from "effect/http/HttpServerRequest";
import * as HttpApiBuilder from "effect/http-api/HttpApiBuilder";
import { BadRequest, GreyscreenApi, TooManyRequests } from "./contract.ts";
import { hashClientAddress } from "./keys.ts";
import { rateLimit } from "./limits.ts";
import { feedbackCountInWindow, saveFeedback, type Store } from "./store.ts";

export const feedbackHandlers = (store: Store) =>
  HttpApiBuilder.group(GreyscreenApi, "feedback", (handlers) =>
    handlers.handle("submitFeedback", ({ payload }) =>
      Effect.gen(function* () {
        const request = yield* HttpServerRequest.HttpServerRequest;
        if (payload.message.trim() === "") {
          return yield* Effect.fail(new BadRequest({ error: "message must not be blank" }));
        }

        const now = new Date();
        const address = request.headers["cf-connecting-ip"] ?? "unknown";
        const ipHash = yield* Effect.promise(() => hashClientAddress(address));
        const recent = yield* feedbackCountInWindow(store, now, payload.client_id, ipHash);
        if (recent >= rateLimit.perHour) {
          return yield* Effect.fail(new TooManyRequests({ error: "too many feedback submissions, try again later" }));
        }

        const id = crypto.randomUUID();
        yield* saveFeedback(store, { id, now, ipHash, payload }).pipe(Effect.orDie);
        return { id };
      }),
    ),
  );

export const systemHandlers = (version: string) =>
  HttpApiBuilder.group(GreyscreenApi, "system", (handlers) =>
    handlers.handle("health", () => Effect.succeed({ ok: true as const, version })),
  );
