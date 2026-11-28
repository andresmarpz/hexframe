import { Clips } from "../components/home/Clips";
import { Features } from "../components/home/Features";
import { Hero } from "../components/home/Hero";
import { HeroStage } from "../components/home/HeroStage";
import { Numbers } from "../components/home/Numbers";
import { Respawn } from "../components/home/Respawn";
import { Safe } from "../components/home/Safe";
import { Timeline } from "../components/home/Timeline";
import { getDownloads, getStars } from "../lib/github-build";

export default async function HomePage() {
  const [downloads, stars] = await Promise.all([getDownloads(), getStars()]);
  return (
    <>
      <Hero downloads={downloads} stars={stars} />
      <HeroStage />
      <Features />
      <Timeline />
      <Clips />
      <Safe />
      <Numbers />
      <Respawn downloads={downloads} />
    </>
  );
}
