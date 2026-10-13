import { fromSource } from "./FromSource";

// ---- Settings ▸ Boundaries: the real presets, two of them switched on ----
export const ON = ["no-acting-after-reading-outside", "no-secret-files", "unattended-only-reads"];
export const BOUNDARIES = {
  boundaries: {
    mode: "enforce",
    deny: fromSource.presets.filter((p) => ON.includes(p.id)).map((p) => ({ name: p.name, when: p.when, preset: p.id })),
    allow: ["true"],
  },
  problem: null,
  problems: [],
  presets: fromSource.presets,
  fields: fromSource.fields,
};
