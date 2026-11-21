const pad = (value: number) => String(value).padStart(2, "0");

export const datePath = (at: Date) =>
  `${at.getUTCFullYear()}/${pad(at.getUTCMonth() + 1)}/${pad(at.getUTCDate())}`;

export const feedbackKey = (at: Date, id: string) => `feedback/${datePath(at)}/${id}.json`;

export const crashPrefix = (version: string, at: Date, id: string) =>
  `crashes/${version}/${datePath(at)}/${id}/`;

export const messagePreview = (message: string) => Array.from(message).slice(0, 200).join("");

const safeVersion = /^[A-Za-z0-9][A-Za-z0-9._+-]{0,63}$/;

export const isSafeVersion = (value: string) => safeVersion.test(value);

export const hashClientAddress = async (address: string) => {
  const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(`greyscreen:${address}`));
  return Array.from(new Uint8Array(digest), (byte) => byte.toString(16).padStart(2, "0")).join("");
};
