import { useRef, useState } from "react";
import { motion, useReducedMotion } from "framer-motion";
import { JOBS } from "../data";
import { onScroll, rise, stagger, still } from "../motion";

// The new assistants, led by the film of them working: the real app over invented demo data,
// so what plays is what the app draws. Muted and looping until someone asks for sound; under
// reduced motion it waits on its poster with controls instead of playing by itself.
const media = (file: string) => `${import.meta.env.BASE_URL}${file}`;

export function Jobs() {
  const reduced = useReducedMotion() ?? false;
  const v = reduced ? still : rise;
  const film = useRef<HTMLVideoElement>(null);
  const [sound, setSound] = useState(false);

  const toggleSound = () => {
    const el = film.current;
    if (!el) return;
    el.muted = sound;
    if (!sound) {
      el.currentTime = 0;
      void el.play();
    }
    setSound(!sound);
  };

  return (
    <section id="jobs" className="jobs">
      <div className="wrap">
        <motion.div {...onScroll} variants={v} className="section-head">
          <p className="eyebrow">New · assistants with a job</p>
          <h2>Hand over the whole job.</h2>
          <p className="lead">
            Each one owns a job end to end, and the part that has to be right is done by code, not by
            the model: the series that moved, the people already in your ATS, the site it may open.
          </p>
        </motion.div>

        <motion.div {...onScroll} variants={v} className="film">
          <video
            ref={film}
            src={media("assistants.mp4")}
            poster={media("assistants.jpg")}
            muted
            loop
            playsInline
            autoPlay={!reduced}
            controls={reduced}
            preload="metadata"
            aria-label="Product Performance finds the series that moved after a release; Talent Scout leaves out everyone already in the ATS; the Inbox holds what waits for a yes"
          />
          {!reduced && (
            <button className="film-sound" onClick={toggleSound} aria-pressed={sound}>
              {sound ? "Sound off" : "▶ Watch with sound"}
            </button>
          )}
        </motion.div>
        <p className="film-note">The real app, over invented demo data. 24 seconds.</p>

        <motion.ul className="jobs-grid" {...onScroll} variants={stagger(0.06)}>
          {JOBS.map((j) => (
            <motion.li key={j.id} variants={v}>
              <span className="roster-id mono">{j.id}</span>
              <h3>{j.name}</h3>
              <p className="job-what">{j.job}</p>
              <p className="job-line">
                <span>What holds the line</span>
                {j.line}
              </p>
            </motion.li>
          ))}
        </motion.ul>
      </div>
    </section>
  );
}
