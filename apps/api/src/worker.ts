import * as Cloudflare from "@alchemy.run/cloudflare";
import * as Effect from "effect/Effect";
import * as Layer from "effect/Layer";
import * as Path from "effect/Path";
import * as ByteSize from "effect/ByteSize";
import * as Etag from "effect/http/Etag";
import * as HttpPlatform from "effect/http/HttpPlatform";
import * as HttpIncomingMessage from "effect/http/HttpIncomingMessage";
import * as HttpRouter from "effect/http/HttpRouter";
import * as HttpApiBuilder from "effect/http-api/HttpApiBuilder";
import pkg from "../package.json" with { type: "json" };
import { GreyscreenApi } from "./contract.ts";
import { crashRoute } from "./crash.ts";
import { JsonErrors } from "./errors.ts";
import { feedbackHandlers, systemHandlers } from "./handlers.ts";
import { maxFeedbackBodyKib } from "./limits.ts";
import { Database, Storage } from "./resources.ts";

const HttpPlatformStub = Layer.succeed(HttpPlatform.HttpPlatform, {
  platform: "web",
  compression: {
    algorithms: new Set<HttpPlatform.CompressionAlgorithm>(),
    compressResponse: (response) => Effect.succeed(response),
  },
  fileResponse: () => Effect.die("HttpPlatform.fileResponse not supported"),
  fileWebResponse: () => Effect.die("HttpPlatform.fileWebResponse not supported"),
});

export default Cloudflare.Worker(
  "Api",
  { main: import.meta.url },
  Effect.gen(function* () {
    const bucket = yield* Cloudflare.R2.ReadWriteBucket(Storage);
    const db = yield* Cloudflare.D1.QueryDatabase(Database);
    const store = { bucket, db };

    const app = Layer.mergeAll(
      HttpApiBuilder.layer(GreyscreenApi).pipe(
        Layer.provide(feedbackHandlers(store)),
        Layer.provide(systemHandlers(pkg.version)),
      ),
      crashRoute(store),
      JsonErrors,
    ).pipe(Layer.provide([Etag.layer, HttpPlatformStub, Path.layer]));

    const handle = yield* HttpRouter.toHttpEffect(app);
    return {
      fetch: handle.pipe(
        Effect.provideService(HttpIncomingMessage.MaxBodySize, ByteSize.kibibytes(maxFeedbackBodyKib)),
      ),
    };
  }).pipe(
    Effect.provide(Layer.mergeAll(Cloudflare.R2.ReadWriteBucketBinding, Cloudflare.D1.QueryDatabaseBinding)),
  ),
);
