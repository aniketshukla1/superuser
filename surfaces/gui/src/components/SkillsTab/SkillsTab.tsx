import { useState } from "react";
import { useEffect } from "react";
import { deleteSkill, listSkills, previewSkillLink, installSkillLink, updateSkill, type SkillRow, type SkillLinkPreview } from "../../api";
import { CARD } from "../../ui";
import { AddSkillMenu } from "./AddSkillMenu";
import { LinkInstallCard } from "./LinkInstallCard";
import { SkillNotice, type Notice } from "./SkillNotice";
import { SkillRowItem } from "./SkillRowItem";

export function SkillsTab({
  onCreateSkill,
}: {
  // The doorway (SKILLS-SPEC §5.2): starts a new conversation with the description
  // prefilled in the composer — the worker builds the skill and proposes it via save_skill.
  onCreateSkill?: (description: string) => void;
}) {
  const [rows, setRows] = useState<SkillRow[]>([]);
  // Provenance filter. "Shipped" are the deliverable templates that come with the
  // app; "Yours" are the ones under data_dir/skills. Counts are over everything, so
  // a tab tells you what exists rather than what survived a filter.
  const [skillTab, setSkillTab] = useState<"all" | "yours" | "shipped">("all");
  // Installing from a link is the same two-step shape as importing a file: fetch and
  // show the actual instructions, then install exactly what was on screen.
  const [linkSource, setLinkSource] = useState<string | null>(null);
  const [linkPreview, setLinkPreview] = useState<SkillLinkPreview | null>(null);
  const [linkBusy, setLinkBusy] = useState(false);
  const [armedDelete, setArmedDelete] = useState<string | null>(null);
  const [error, setError] = useState("");
  // The state-change callout (SKILLS-SPEC §4.1 #2): name-first so the user knows WHICH
  // skill, and visually distinct so it can't be skimmed past (tester ask 2026-07-27).
  const [notice, setNotice] = useState<Notice | null>(null);

  // Confirmation copy (SKILLS-SPEC §4.1 #2): name-first, outcome + remedy only, in words a
  // person already owns — now / everywhere / off / start a new one. Never mechanism ("the
  // model will be told…") or engineering timing ("from the next message") — owner-driver
  // review rounds, 2026-07-27. The engine countermands disabled-but-loaded skills silently;
  // the copy promises only the guaranteed part.
  const CONFIRMATION = "— your assistants can now use it in every conversation.";
  const OFF_NOTE =
    "turned off everywhere. If a conversation already used it, start a new one for a completely clean slate.";
  const DELETE_NOTE =
    "removed. If a conversation already used it, start a new one for a completely clean slate.";

  const refresh = () => listSkills().then(setRows);
  useEffect(() => {
    refresh();
  }, []);

  const fail = (res: { ok?: boolean; error?: string }) => {
    setNotice(null);
    if (res.ok === false) {
      setError(res.error || "Something went wrong.");
      return true;
    }
    setError("");
    return false;
  };

  const fetchLink = async () => {
    const source = (linkSource ?? "").trim();
    if (!source) return;
    setLinkBusy(true);
    const res = await previewSkillLink(source);
    setLinkBusy(false);
    if (fail(res)) return;
    if (res.preview) setLinkPreview(res.preview);
  };

  const confirmLink = async () => {
    if (!linkPreview) return;
    setLinkBusy(true);
    const res = await installSkillLink(linkPreview);
    setLinkBusy(false);
    if (fail(res)) return;
    setNotice({ name: linkPreview.name, text: CONFIRMATION, tone: "ok" });
    setLinkPreview(null);
    setLinkSource(null);
    refresh();
  };

  const remove = async (row: SkillRow) => {
    if (armedDelete !== row.name) {
      setArmedDelete(row.name);
      return;
    }
    setArmedDelete(null);
    const res = await deleteSkill(row.name);
    if (fail(res)) return;
    setNotice({ name: row.name, text: DELETE_NOTE, tone: "warn" });
    refresh();
  };

  return (
    <section>
      <div className="flex items-start justify-between gap-3 mb-4">
        <div>
          <h2 className="text-lg font-semibold">Skills</h2>
          <p className="text-sm text-muted mt-1 leading-prose">
            Reusable instructions your assistants can follow in every conversation. Off here means
            off everywhere.
          </p>
        </div>
        <AddSkillMenu
          onLink={() => {
            setLinkPreview(null);
            setLinkSource("");
          }}
          onCreateSkill={onCreateSkill}
        />
      </div>

      {error ? (
        <div className="text-sm text-red-500 mb-3" role="alert">
          {error}
        </div>
      ) : null}
      {notice ? <SkillNotice notice={notice} onDismiss={() => setNotice(null)} /> : null}

      {linkSource !== null ? (
        <LinkInstallCard
          linkSource={linkSource}
          setLinkSource={setLinkSource}
          linkPreview={linkPreview}
          setLinkPreview={setLinkPreview}
          linkBusy={linkBusy}
          fetchLink={fetchLink}
          confirmLink={confirmLink}
        />
      ) : null}

      <div className="flex items-center gap-1 mb-3" role="tablist">
        {([
          { id: "all", label: "All" },
          { id: "yours", label: "Yours" },
          { id: "shipped", label: "Shipped" },
        ] as const).map((t) => {
          const n =
            t.id === "all"
              ? rows.length
              : rows.filter((r) => (t.id === "shipped" ? r.source === "builtin" : r.source !== "builtin")).length;
          return (
            <button
              key={t.id}
              role="tab"
              aria-selected={skillTab === t.id}
              data-testid={`skills-tab-${t.id}`}
              onClick={() => setSkillTab(t.id)}
              className={
                "px-2.5 py-1 rounded-lg text-sm " +
                (skillTab === t.id ? "bg-accentSoft text-accent" : "text-muted hover:text-ink")
              }
            >
              {t.label} <span className="text-faint">{n}</span>
            </button>
          );
        })}
      </div>

      <div className={`${CARD} divide-y divide-line`}>
        {rows.length === 0 ? (
          <div className="p-5 text-md text-muted">
            No skills yet — <b>Add skill</b> teaches your assistants their first one, like
            “prepare my Monday status report”.
          </div>
        ) : null}
        {rows
          .filter((r) =>
            skillTab === "all" ? true : skillTab === "shipped" ? r.source === "builtin" : r.source !== "builtin",
          )
          .map((row) => (
          <SkillRowItem
            key={row.name}
            row={row}
            armed={armedDelete === row.name}
            onRemove={() => remove(row)}
            onDisarm={() => setArmedDelete(null)}
            onToggle={(on) =>
              updateSkill(row.name, { enabled: on }).then((res) => {
                if (!fail(res))
                  setNotice({
                    name: row.name,
                    text: on ? CONFIRMATION : OFF_NOTE,
                    tone: on ? "ok" : "warn",
                  });
                refresh();
              })
            }
          />
        ))}
      </div>

    </section>
  );
}
