import * as Cloudflare from "@alchemy.run/cloudflare";

export const Storage = Cloudflare.R2.Bucket("Storage");

export const Database = Cloudflare.D1.Database("Database", { migrations: "./migrations" });
