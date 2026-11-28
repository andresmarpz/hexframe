const PRODUCTION_URL = "https://greyscreen.app";

function resolve(): URL {
  try {
    return new URL(process.env.SITE_URL || PRODUCTION_URL);
  } catch {
    return new URL(PRODUCTION_URL);
  }
}

export const siteUrl = resolve();
