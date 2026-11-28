import type { Downloads } from "../../lib/release";

interface Props {
  downloads: Downloads | null;
}

export function Respawn(_props: Props) {
  return (
    <section>
      <p>Respawn</p>
    </section>
  );
}
