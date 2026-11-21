import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    include: ["test/**/*.test.ts"],
    testTimeout: 60_000,
    hookTimeout: 180_000,
    env: {
      ALCHEMY_DEV: "true",
      CLOUDFLARE_API_TOKEN: "local-dev-placeholder",
      CLOUDFLARE_ACCOUNT_ID: "00000000000000000000000000000000",
    },
    fileParallelism: false,
  },
});
