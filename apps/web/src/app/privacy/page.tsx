import type { Metadata } from "next";
import { Privacy } from "../../components/privacy/Privacy";
import { site } from "../../content/copy";
import { privacy } from "../../content/privacy";

export const metadata: Metadata = {
  title: `${privacy.title} — ${site.name}`,
  description: privacy.description,
};

export default function PrivacyPage() {
  return <Privacy />;
}
