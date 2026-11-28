import type { Metadata } from "next";
import { CodeSigning } from "../../components/code-signing/CodeSigning";
import { codeSigning } from "../../content/code-signing";
import { site } from "../../content/copy";

export const metadata: Metadata = {
  title: `${codeSigning.title} — ${site.name}`,
  description: codeSigning.description,
};

export default function CodeSigningPage() {
  return <CodeSigning />;
}
