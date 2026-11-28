import type { Metadata } from "next";
import { NotFound } from "../components/notfound/NotFound";
import { notFound, site } from "../content/copy";

export const metadata: Metadata = {
  title: `${notFound.headline} — ${site.name}`,
  description: notFound.body,
  robots: { index: false },
  alternates: { canonical: null },
  openGraph: { url: null },
};

export default function NotFoundPage() {
  return <NotFound />;
}
