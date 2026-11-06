import { Hero } from "./sections/Hero";
import { Tour } from "./sections/Tour";
import { LocalOnly } from "./sections/LocalOnly";
import { Memory } from "./sections/Memory";
import { AppPreview } from "./sections/AppPreview";
import { Assistants } from "./sections/Assistants";
import { Jobs } from "./sections/Jobs";
import { Features } from "./sections/Features";
import { LocalModels } from "./sections/LocalModels";
import { GetStarted } from "./sections/GetStarted";
import { Licence } from "./sections/Licence";
import { Contact } from "./sections/Contact";
import { Faq } from "./sections/Faq";
import { Mark } from "./Mark";
import { REPO_URL, repoFile } from "./config";

export function App() {
  return (
    <>
      <a className="skip" href="#main">
        Skip to content
      </a>

      <header className="nav">
        <div className="wrap nav-in">
          <a href="#top" className="brand">
            <Mark size={15} />
            <span>superuser</span>
          </a>
          <nav className="nav-links" aria-label="Sections">
            <a className="hide-sm" href="#stays">
              Privacy
            </a>
            <a className="hide-sm" href="#assistants">
              Assistants
            </a>
            <a className="hide-sm" href="#licence">
              Licence
            </a>
            <a className="hide-xs" href="#contact">
              Business
            </a>
            <a href="#start">Get started</a>
            <a className="btn btn-ghost btn-sm" href={REPO_URL}>
              GitHub
            </a>
          </nav>
        </div>
      </header>

      <main id="main">
        {/* The first five are the argument, and each earns the next: it stays here ->
            so it can know you -> so the work comes back right. Reordering them turns a
            chain back into a list of features. What follows is reference: who does the
            work, how well it runs locally, how to start, the terms, and who to ask. */}
        <Hero />
        <Tour />
        <LocalOnly />
        <Memory />
        <AppPreview />
        <Jobs />
        <Assistants />
        <Features />
        <LocalModels />
        <GetStarted />
        <Licence />
        <Contact />
        <Faq />
      </main>

      <footer>
        <div className="wrap foot-in">
          <span className="foot-brand">
            <Mark size={13} />
            Superuser
          </span>
          <nav className="foot-links" aria-label="Project">
            <a href={REPO_URL}>GitHub</a>
            <a href={repoFile("LICENSE")}>Licence</a>
            <a href={repoFile("THIRD_PARTY_NOTICES.md")}>Third-party notices</a>
            <a href={repoFile("CONTRIBUTING.md")}>Contributing</a>
          </nav>
          <span className="spacer foot-note">No cookies, no analytics and no trackers on this site.</span>
        </div>
      </footer>
    </>
  );
}
