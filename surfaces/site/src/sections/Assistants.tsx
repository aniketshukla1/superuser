import { motion, useReducedMotion } from "framer-motion";
import { APPS, ASSISTANTS } from "../data";
import { onScroll, rise, stagger, still } from "../motion";

// Who does the work, and where. A roster rather than a grid of cards: twenty boxed
// tiles would be a wall, while a list with hairlines between entries reads like what
// it is, a team. The mono id over each name is real: it is how the local API names
// that assistant (`"model": "chief"`), which is the kind of detail only this product has.
export function Assistants() {
  const reduced = useReducedMotion() ?? false;
  const v = reduced ? still : rise;

  return (
    <section id="assistants">
      <div className="wrap">
        <motion.div {...onScroll} variants={v} className="section-head">
          <p className="eyebrow">Who does the work</p>
          <h2>Twenty assistants, each one different.</h2>
          <p className="lead">
            Each has its own instructions, tools, skills and model. Pick one per conversation,
            or describe a new one and create it in the app.
          </p>
        </motion.div>

        <motion.ul className="roster" {...onScroll} variants={stagger(0.025)}>
          {ASSISTANTS.map((a) => (
            <motion.li key={a.id} variants={v}>
              <span className="roster-id mono">{a.id}</span>
              <span className="roster-name">{a.name}</span>
              <span className="roster-line">{a.tagline}</span>
            </motion.li>
          ))}
        </motion.ul>

        <motion.div {...onScroll} variants={v} className="apps">
          <h3>They work in your apps, not in screenshots of them.</h3>
          <p className="apps-lead">
            37 connectors with real accounts, plus any MCP server you run. Your keys stay
            encrypted on your Mac.
          </p>
          <dl className="apps-list">
            {APPS.map((g) => (
              <div key={g.area} className="apps-row">
                <dt>{g.area}</dt>
                <dd>
                  {g.apps.map((app) => (
                    <span key={app} className="chip">
                      {app}
                    </span>
                  ))}
                </dd>
              </div>
            ))}
            <div className="apps-row">
              <dt>Anything else</dt>
              <dd>
                <span className="chip chip-accent">Any MCP server</span>
              </dd>
            </div>
          </dl>
        </motion.div>
      </div>
    </section>
  );
}
