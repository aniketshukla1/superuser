import type { RecentChannel } from "../../api";
import { ChannelPicker } from "../SubscriptionsChip";
import { SelectMenu } from "../SelectMenu";
import { DAYS, type QuickTemplate } from "./TEMPLATES";

const label = "block text-sm text-muted mt-3 mb-1";
const input = "w-full px-3 py-2 rounded-lg border border-line bg-panel text-md outline-none focus:border-accent";

/** The template's settings: repository, channel, when, where it delivers, and consent. */
export function RecipeFields({
  picked,
  repo,
  setRepo,
  channel,
  setChannel,
  recent,
  onPickName,
  day,
  setDay,
  time,
  setTime,
  deliver,
  setDeliver,
  consent,
  setConsent,
  channelLabel,
  channelWorkspace,
}: {
  picked: QuickTemplate;
  repo: string;
  setRepo: (r: string) => void;
  channel: string;
  setChannel: (c: string) => void;
  recent: RecentChannel[];
  onPickName: (address: string, name: string, workspace?: string) => void;
  day: string;
  setDay: (d: string) => void;
  time: string;
  setTime: (t: string) => void;
  deliver: "app" | "slack";
  setDeliver: (d: "app" | "slack") => void;
  consent: boolean;
  setConsent: (c: boolean) => void;
  channelLabel: string;
  channelWorkspace?: string;
}) {
  return (
    <div className={picked.conns.length ? "bg-paper rounded-xl px-4 py-3.5 mt-3" : ""} data-testid="ob-recipe">
      {picked.needsRepo && (
        <>
          <label className={label}>Repository</label>
          <input
            className={input}
            placeholder="owner/repo"
            value={repo}
            onChange={(e) => setRepo(e.target.value)}
            data-testid="ob-repo"
          />
        </>
      )}
      {picked.needsChannel && (
        <>
          <label className={label}>Post to channel</label>
          <div data-testid="ob-channel">
            <ChannelPicker
              value={channel}
              onChange={setChannel}
              recent={recent}
              onPickName={onPickName}
            />
          </div>
          <p className="text-xs text-warnInk mt-1">
            The bot must be a member of the channel — invite @Superuser in Slack if it isn't.
          </p>
        </>
      )}
      <label className={label}>When</label>
      <div className="flex gap-2">
        <div className="flex-1 min-w-0">
          <SelectMenu
            ariaLabel="Day"
            value={day}
            options={Object.entries(DAYS).map(([k, v]) => ({ value: k, label: v.label }))}
            onChange={setDay}
          />
        </div>
        <input
          className="w-28 px-3 py-2 rounded-lg border border-line bg-panel text-md outline-none focus:border-accent"
          type="time"
          aria-label="Time"
          value={time}
          onChange={(e) => setTime(e.target.value)}
        />
      </div>
      {picked.deliver && (
        <>
          <label className={label}>Deliver to</label>
          <SelectMenu
            ariaLabel="Deliver to"
            value={deliver}
            options={[
              { value: "app", label: "In the app" },
              { value: "slack", label: "Slack DM (connect Slack later)" },
            ]}
            onChange={(v) => setDeliver(v as "app" | "slack")}
          />
        </>
      )}
      {picked.consent ? (
        <label className="flex items-start gap-2.5 mt-3.5 text-sm text-muted select-none">
          <input
            type="checkbox"
            className="mt-0.5"
            checked={consent}
            onChange={(e) => setConsent(e.target.checked)}
            data-testid="ob-consent"
          />
          <span>
            Allow this automation to post its digest to{" "}
            <b className="text-ink" title={channel || undefined}>
              {channelLabel || "the channel"}
              {channelWorkspace ? ` (${channelWorkspace})` : ""}
            </b>{" "}
            without asking each time. Anything else still asks first.
          </span>
        </label>
      ) : picked.conns.length > 0 ? (
        <p className="text-sm text-muted mt-3">
          This automation only <b className="text-ink">reads</b> on schedule — reading
          never needs approval.
        </p>
      ) : null}
    </div>
  );
}
