import { useEffect, useState } from "react";
import { chipName, menuName } from "../../modelName";
import { ChipButton, Menu, MenuHeading, MenuRow, useNumberKeys } from "./MenuRow";
import { AttachRow } from "./AttachMenu";

/* ------------------------------------------------------------------- model */

/** The model list. Four get a number and a row; the rest live behind "More models",
 *  because a menu you have to scroll is a menu that has stopped being a menu. */
export const HEAD_MODELS = 4;

export function ModelMenu({
  model,
  models,
  labels,
  providers,
  defaultModel,
  onChange,
  onMore,
}: {
  model: string;
  models: string[];
  labels?: Record<string, string>;
  /** Which leading `x:` counts as a provider. From the server — see modelName.ts. */
  providers?: string[];
  defaultModel?: string;
  onChange: (m: string) => void;
  onMore?: () => void;
}) {
  const [open, setOpen] = useState(false);
  const [all, setAll] = useState(false);
  // The one in use is always in the list, even when the server has stopped offering it.
  const ordered = Array.from(new Set([model, ...models])).filter(Boolean);
  const shown = all ? ordered : ordered.slice(0, HEAD_MODELS);
  const choose = (i: number) => {
    const m = shown[i];
    if (!m) return;
    onChange(m);
    setOpen(false);
  };
  useNumberKeys(shown.length, choose, open);
  useEffect(() => {
    if (!open) setAll(false);
  }, [open]);
  return (
    <div className="composer-slot">
      {/* The chip gets the short name and the tooltip gets the whole truth. The full
          label is "<name> · <vendor>", which is right in the menu and absurd on a
          control this narrow — it was pushing the bar onto a second row. */}
      <ChipButton
        label={chipName(model, labels, providers)}
        open={open}
        onClick={() => setOpen((v) => !v)}
        title={`Model: ${menuName(model, labels)}`}
        ariaLabel="Model"
        testid="model-chip"
      />
      {open && (
        <Menu onClose={() => setOpen(false)} label="Models" align="right" width={296} testid="model-menu">
          <MenuHeading>Models</MenuHeading>
          {shown.map((m, i) => (
            <MenuRow
              key={m}
              name={menuName(m, labels)}
              badge={m === defaultModel ? "Default" : undefined}
              checked={m === model}
              index={i + 1}
              onClick={() => choose(i)}
              testid={`model-${m}`}
            />
          ))}
          {ordered.length > shown.length && (
            <AttachRow
              icon="sliders"
              label={`More models`}
              chevron
              onClick={() => (onMore ? onMore() : setAll(true))}
              testid="model-more"
            />
          )}
        </Menu>
      )}
    </div>
  );
}
