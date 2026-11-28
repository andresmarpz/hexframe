import type { Metadata } from "next";
import { Faq } from "../../components/faq/Faq";
import { site } from "../../content/copy";
import { faqPage } from "../../content/faq";

export const metadata: Metadata = {
  title: `${faqPage.title} — ${site.name}`,
  description: faqPage.description,
};

export default function FaqPage() {
  return <Faq />;
}
