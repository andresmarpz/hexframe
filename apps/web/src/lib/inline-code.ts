const TOKENS = ["greyscreen-cli record --seconds 10", "greyscreen-cli doctor", "greyscreen-cli.exe", ".exe"];

const escapeHtml = (text: string) => text.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
const pattern = new RegExp(TOKENS.map((t) => t.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")).join("|"), "g");

export function withCode(text: string): string {
  return escapeHtml(text).replace(pattern, (token) => `<code>${token}</code>`);
}
