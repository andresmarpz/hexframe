import type { Downloads } from "../../lib/release";

interface Props {
  downloads: Downloads | null;
  stars: number | null;
}

export function Download(_props: Props) {
  return (
    <section>
      <p>Download</p>
    </section>
  );
}
