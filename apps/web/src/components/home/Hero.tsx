import type { Downloads } from "../../lib/release";

interface Props {
  downloads: Downloads | null;
  stars: number | null;
}

export function Hero(_props: Props) {
  return (
    <section>
      <p>Hero</p>
    </section>
  );
}
