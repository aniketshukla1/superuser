import { siAsana, siBox, siClickup, siConfluence, siDatadog, siDiscord, siDropbox, siFigma, siGithub, siGitlab, siGmail, siGooglecalendar, siGoogledrive, siHubspot, siJira, siLinear, siMixpanel, siNotion, siPagerduty, siPosthog, siPrometheus, siQuickbooks, siStripe, siTelegram, siWhatsapp, siZendesk } from "simple-icons";
import * as glyph from "./marks";
import { brand, pathLogo, type ConnectorRegistryEntry } from "./marks";
import * as retired from "./retiredMarks";

export type { ConnectorRegistryEntry, LogoComponent } from "./marks";

/** What an unknown or empty logo id gets. */
export const FALLBACK: ConnectorRegistryEntry = { label: "Connector", logo: glyph.PlugLogo };

const drawn = (label: string, logo: ConnectorRegistryEntry["logo"]): ConnectorRegistryEntry => ({ label, logo });

export const CONNECTORS: Record<string, ConnectorRegistryEntry> = {
  // Marks from the icon package.
  asana: brand(siAsana),
  box: brand(siBox),
  clickup: brand(siClickup),
  confluence: brand(siConfluence),
  datadog: brand(siDatadog),
  discord: brand(siDiscord),
  dropbox: brand(siDropbox),
  figma: brand(siFigma),
  github: brand(siGithub),
  gitlab: brand(siGitlab),
  gmail: brand(siGmail),
  google_calendar: brand(siGooglecalendar),
  google_drive: brand(siGoogledrive),
  hubspot: brand(siHubspot),
  jira: brand(siJira),
  linear: brand(siLinear),
  mixpanel: brand(siMixpanel),
  notion: brand(siNotion),
  pagerduty: brand(siPagerduty),
  posthog: brand(siPosthog),
  prometheus: brand(siPrometheus),
  quickbooks: brand(siQuickbooks),
  stripe: brand(siStripe),
  telegram: brand(siTelegram),
  whatsapp: brand(siWhatsapp),
  zendesk: brand(siZendesk),
  // Marks the package has since dropped.
  canva: drawn("Canva", pathLogo(retired.CANVA)),
  outlook: drawn("Outlook", pathLogo(retired.OUTLOOK)),
  salesforce: drawn("Salesforce", pathLogo(retired.SALESFORCE)),
  slack: drawn("Slack", pathLogo(retired.SLACK)),
  // Line glyphs for brands without a one-colour mark, in their real colour.
  amplitude: drawn("Amplitude", glyph.AmplitudeLogo),
  apollo: drawn("Apollo.io", glyph.ApolloLogo),
  attio: drawn("Attio", glyph.AttioLogo),
  clay: drawn("Clay", glyph.ClayLogo),
  close: drawn("Close", glyph.CloseLogo),
  descript: drawn("Descript", glyph.DescriptLogo),
  docusign: drawn("Docusign", glyph.DocusignLogo),
  hunter: drawn("Hunter", glyph.HunterLogo),
  monday: drawn("monday.com", glyph.MondayLogo),
  // Not brands.
  browser: drawn("Browser", glyph.BrowserLogo),
  email: drawn("Email", glyph.EmailLogo),
  mcp: drawn("MCP", glyph.McpLogo),
};
