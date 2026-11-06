// The site's facts, in one file. Each list is copied from the app itself rather than
// written for the page, so the page cannot promise something the build doesn't have:
// the assistants from src-tauri/src/core/personas_builtin, the connectors from
// src-tauri/src/integrations/descriptors.rs, the scores from docs/harness/HARNESS-SCOREBOARD.md.

/** The built-in assistants, with their own taglines. `id` is also the API's model name. */
export const ASSISTANTS = [
  { id: "chief", name: "Chief of Staff", tagline: "Reads what came in since yesterday and tells you what matters" },
  { id: "assistant", name: "Main Assistant", tagline: "Get things done — research, drafts, and analysis" },
  { id: "email", name: "Email Assistant", tagline: "Reads your email and drafts the replies you approve" },
  { id: "perf", name: "Product Performance", tagline: "Investigates a slowdown in your metrics — the hotspot, the evidence, and links" },
  { id: "repro", name: "Bug Reproduction", tagline: "Reproduces a bug report on staging, with exact steps and a test case" },
  { id: "talent", name: "Talent Scout", tagline: "Finds candidates who meet the must-haves — none already in your ATS — with outreach drafts" },
  { id: "coder", name: "Coder", tagline: "Work in a codebase — edit, run, and ship" },
  { id: "analyst", name: "Data Analyst", tagline: "Answer the question that was asked, and show the numbers that complicate it" },
  { id: "finance", name: "Finance Analyst", tagline: "Say where the money actually went, and when it runs out" },
  { id: "expenses", name: "Expense Manager", tagline: "Logs receipts from your email and builds summaries that add up" },
  { id: "ops", name: "Ops Assistant", tagline: "Incidents and runbooks — logs, metrics, infrastructure" },
  { id: "security", name: "Security Assistant", tagline: "Security problems found and fixed — scan, triage, PR" },
  { id: "dep-audit", name: "Dependency Audit Assistant", tagline: "Risky dependencies — audit, smallest upgrades, PRs" },
  { id: "cloud-posture", name: "Cloud Posture Assistant", tagline: "Review Terraform and cloud config — read-only, evidence first" },
  { id: "qa", name: "QA Analyst", tagline: "Find what breaks before someone else does, and say what you didn't check" },
  { id: "pm", name: "Product Manager", tagline: "Specify it so it can be built and tested without a meeting" },
  { id: "lead", name: "Team Lead", tagline: "Break work down, hand it out, check it came back right" },
  { id: "writer", name: "Technical Writer", tagline: "Documentation written from the code, and followed before it ships" },
  { id: "support", name: "Customer Support", tagline: "Solve the problem behind the ticket, and answer even when the answer is no" },
  { id: "hr", name: "HR Partner", tagline: "Describe the job, run the same process for everyone, and write it down" },
] as const;

/** The six assistants that each own a whole job, and what in the code holds each one to it. */
export const JOBS = [
  { id: "email", name: "Email Assistant", job: "Answers mail from senders you allow.", line: "Every reply is a draft in your Inbox. Nothing is sent without your yes." },
  { id: "expenses", name: "Expense Manager", job: "Logs receipts from email and summarises the week.", line: "The totals are computed, and each one lists the receipts it adds up." },
  { id: "chief", name: "Chief of Staff", job: "What came in since yesterday, mapped to your priorities.", line: "It can't send a message or move a meeting. It has nothing that could." },
  { id: "repro", name: "Bug Reproduction", job: "Reproduces a bug report with steps, screenshots and a test case.", line: "Its browser opens your staging site and nothing else, so production data never reaches it." },
  { id: "perf", name: "Product Performance", job: "Finds where a slowdown is, from your Prometheus.", line: "Every series is compared before and after the change in code. Only a clear shift is called a hotspot." },
  { id: "talent", name: "Talent Scout", job: "Finds candidates who meet the must-haves, and drafts outreach.", line: "Your ATS export is checked in code, so nobody already in it makes the list. Nobody is contacted." },
] as const;

/** The 37 connectors that ship, by area. Plus any MCP server. */
export const APPS: { area: string; apps: string[] }[] = [
  { area: "Messaging", apps: ["Slack", "Telegram", "Discord", "WhatsApp"] },
  { area: "Email and calendar", apps: ["Email (IMAP)", "Gmail", "Outlook", "Google Calendar"] },
  { area: "Files and docs", apps: ["Google Drive", "Dropbox", "Box", "Notion", "Confluence", "Docusign"] },
  { area: "Engineering", apps: ["GitHub", "GitLab", "Jira", "Linear", "Asana", "ClickUp"] },
  { area: "Operations", apps: ["PagerDuty", "Prometheus", "Datadog"] },
  { area: "Sales and support", apps: ["HubSpot", "Salesforce", "Attio", "Close", "Apollo.io", "Hunter", "Zendesk"] },
  { area: "Finance", apps: ["Stripe", "QuickBooks"] },
  { area: "Design", apps: ["Figma", "Canva"] },
  { area: "Analytics", apps: ["PostHog", "Mixpanel", "Amplitude"] },
];

/** The product tour: real screens from the app, on invented demo data. */
export const TOUR = [
  { file: "chat", label: "Answers", window: "Checkout latency since the 2.41 release", caption: "Ask an assistant. It shows its work: the steps, a chart with its source, and the facts kept apart from the guesses.", alt: "An assistant's answer: a latency chart with its source, then the facts and the hypotheses." },
  { file: "inbox", label: "Inbox", window: "Inbox", caption: "Anything that would leave your computer waits here for your yes, showing exactly what would be sent and where.", alt: "The Inbox, with a Slack message and an email waiting for approval." },
  { file: "boundaries", label: "Boundaries", window: "Settings · Boundaries", caption: "Rules you can read, checked before every action. Deny wins, and each preset says what it costs.", alt: "Boundaries: rules written as plain expressions, each with what it costs." },
  { file: "network", label: "Network", window: "Privacy · Network", caption: "Everything that left this machine, who received it, and what it was for.", alt: "The Network page: each host contacted, what for, and how much was sent." },
  { file: "network-local-only", label: "Local-only", window: "Privacy · Network", caption: "Turn on local-only mode and every outbound request is refused at the one point they all pass through, and written down.", alt: "Local-only mode on: requests refused and recorded." },
  { file: "brain", label: "Memory", window: "Brain · Memories", caption: "What it knows about you is markdown files on your disk, each with where it came from.", alt: "The Brain: memories grouped by source, each one a markdown file." },
  { file: "connector-prometheus", label: "Connectors", window: "Connectors · Prometheus", caption: "Connect your own tools, and see exactly what an assistant can do with each of them.", alt: "A connector's page listing the tools assistants use on it." },
  { file: "automations", label: "Routines", window: "Automations", caption: "Routines run on a schedule while you're away, and report back with the evidence.", alt: "Scheduled routines with their next run." },
] as const;

/** 20 agent tasks, three runs each, through Ollama on one Mac. */
export const SCORES = [
  { model: "gemma4:12b", passed: 54 },
  { model: "qwen2.5:7b-instruct", passed: 44 },
  { model: "llama3.2 (3B)", passed: 29 },
];
export const SCORE_OF = 60;
