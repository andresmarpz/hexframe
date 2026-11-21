import * as Alchemy from "alchemy";
import * as Cloudflare from "@alchemy.run/cloudflare";
import * as Effect from "effect/Effect";
import Api from "../src/worker.ts";
import Probe from "./probe.ts";

export default Alchemy.Stack(
  "GreyscreenApiTest",
  { providers: Cloudflare.providers(), state: Alchemy.localState() },
  Effect.gen(function* () {
    const api = yield* Api;
    const probe = yield* Probe;
    return { url: api.url, probeUrl: probe.url };
  }),
);
