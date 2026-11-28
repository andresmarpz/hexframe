import type { ChangelogEntry } from "../../lib/changelog";
import type { Downloads } from "../../lib/release";

interface Props {
  entries: ChangelogEntry[];
  downloads: Downloads | null;
}

export function Changelog(_props: Props) {
  return (
    <section>
      <p>Changelog</p>
    </section>
  );
}
