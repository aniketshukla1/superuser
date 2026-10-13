export const ICON_FOR: Record<string, "diamond" | "chat" | "code"> = {
  cowork: "diamond",
  chat: "chat",
  code: "code",
};

export const KIND_TABS: { key: string; label: string }[] = [
  { key: "all", label: "All" },
  { key: "approval", label: "Approvals" },
  { key: "question", label: "Questions" },
];

export const CHIP = (active: boolean) =>
  "text-xs px-2.5 py-1 rounded-full border " +
  (active
    ? "border-accent text-accent bg-accentSoft"
    : "border-line text-muted hover:border-lineStrong");

export type InboxTab = "pending" | "configure";
