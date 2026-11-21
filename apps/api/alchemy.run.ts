import * as Alchemy from "alchemy";
import * as Cloudflare from "@alchemy.run/cloudflare";
import * as Effect from "effect/Effect";
import Worker from "./src/worker.ts";

const local = process.env.ALCHEMY_DEV === "true" || process.env.ALCHEMY_DEV === "1";

export default Alchemy.Stack(
  "GreyscreenApi",
  {
    providers: Cloudflare.providers(),
    state: local ? Alchemy.localState() : Cloudflare.state(),
  },
  Effect.gen(function* () {
    const worker = yield* Worker;
    return { url: worker.url };
  }),
);
