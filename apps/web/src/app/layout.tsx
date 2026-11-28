import type { Metadata, Viewport } from "next";
import { Analytics } from "@vercel/analytics/next";
import { IBM_Plex_Mono, Inter } from "next/font/google";
import type { ReactNode } from "react";
import { Footer } from "../components/site/Footer";
import { Nav } from "../components/site/Nav";
import { site } from "../content/copy";
import { siteUrl } from "../lib/site-url";
import "../styles/global.css";

const sans = Inter({ subsets: ["latin"], axes: ["opsz"], variable: "--font-sans", display: "swap" });
const mono = IBM_Plex_Mono({ subsets: ["latin"], weight: ["400", "500"], variable: "--font-mono", display: "swap" });

const imageAlt = "Greyscreen: automatic League of Legends game recorder and review timeline.";

export const metadata: Metadata = {
  metadataBase: siteUrl,
  title: site.title,
  description: site.description,
  alternates: { canonical: "./" },
  icons: {
    icon: [
      { url: "/favicon.svg", type: "image/svg+xml" },
      { url: "/favicon.ico", sizes: "32x32" },
    ],
    apple: "/apple-touch-icon.png",
  },
  openGraph: {
    type: "website",
    siteName: site.name,
    title: site.ogTitle,
    description: site.ogDescription,
    url: "./",
    images: [{ url: "/og.png", width: 1200, height: 630, type: "image/png", alt: imageAlt }],
  },
  twitter: {
    card: "summary_large_image",
    title: site.ogTitle,
    description: site.ogDescription,
    images: [{ url: "/og.png", alt: imageAlt }],
  },
};

export const viewport: Viewport = {
  colorScheme: "light",
  themeColor: "#ffffff",
};

export default function RootLayout({ children }: { children: ReactNode }) {
  return (
    <html lang="en" className={`${sans.variable} ${mono.variable}`}>
      <body>
        <a className="skip-link" href="#main">
          Skip to content
        </a>
        <Nav />
        <main id="main">{children}</main>
        <Footer />
        <Analytics />
      </body>
    </html>
  );
}
