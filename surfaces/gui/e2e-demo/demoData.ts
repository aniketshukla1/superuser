// Demo data for the README's screenshots. Everything a person could mistake for somebody's
// real account is invented here: the company (Northwind), the people, the numbers. What is
// NOT invented is the product's own content, which is read from the source so a screenshot
// cannot show a feature the build does not have: the assistants and the boundary presets
// come from `from-source.json` (regenerate it with scripts/readme-assets.py --extract).

export { MODEL, MODEL_LABELS } from "./demoData/FromSource";
export { PERSONAS } from "./demoData/assistants";
export { HERO, SESSIONS } from "./demoData/conversations";
export { HERO_MESSAGES } from "./demoData/heroConversation";
export { INBOX } from "./demoData/inbox";
export { EGRESS, EGRESS_LOCAL_ONLY } from "./demoData/privacyNetwork";
export { BOUNDARIES } from "./demoData/settingsBoundaries";
export { SETTINGS_SECTIONS } from "./demoData/newerSettingsSections";
export { CONNECTED, PROMETHEUS_TOOLS, SYNC_TOOLS } from "./demoData/connectors";
export { AUTOMATIONS } from "./demoData/automations";
export { MEMORIES } from "./demoData/brainMemories";
