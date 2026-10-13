import type { ReactNode, RefObject } from "react";
import { isTauri, type DictationStatus } from "../../tauri";
import { AttachMenu, ContextRing, EffortChip, MicControl, ModelMenu, ModeMenu } from "../ComposerControls";
import { Icon, ICON } from "../Icon";
import { BYPASS_OPTION, PERMISSION_OPTIONS, type Props } from "./Props";

/** OUTSIDE the box, on the page's own ground. The controls used to sit inside the border,
 * which made the input read as a panel of settings with a text field in it. The box holds
 * the one thing you type into; what you can do with it belongs under it. `min-w-0` so a long
 * model name truncates rather than pushing the right-hand group onto a second row. */
export function ComposerBar({
  props,
  fileInput,
  onFiles,
  onPickFiles,
  addFolder,
  openSlashCommands,
  dictation,
  dictationBusy,
  levels,
  recordingTime,
  voiceReady,
  toggleDictation,
  needsModel,
  modelsLoaded,
  providers,
  effort,
  setEffort,
  breakdown,
}: {
  props: Props;
  fileInput: RefObject<HTMLInputElement>;
  onFiles: (files: FileList) => void;
  onPickFiles: () => void;
  addFolder: () => void;
  openSlashCommands: () => void;
  dictation: DictationStatus | null;
  dictationBusy: string | null;
  levels: number[];
  recordingTime: string;
  voiceReady: boolean;
  toggleDictation: () => Promise<void>;
  needsModel: boolean;
  modelsLoaded: boolean;
  providers?: string[];
  effort: { value: string; levels: string[] } | null;
  setEffort: (update: (e: { value: string; levels: string[] } | null) => { value: string; levels: string[] } | null) => void;
  breakdown?: ReactNode;
}) {
  return (
    <div className="composer-bar min-w-0 flex items-center gap-0.5">
        <input
          ref={fileInput}
          type="file"
          multiple
          style={{ display: "none" }}
          onChange={(e) => {
            if (e.target.files) onFiles(e.target.files);
            e.target.value = "";
          }}
        />

        {/* Listening replaces the quiet controls with a LIVE waveform (mic RMS, polled
            ~10Hz, scrolling left) + elapsed time (§37). The mic itself stays: it is how
            you stop. */}
        {dictation?.recording ? (
          <>
            <div className="voice-wave-row flex-1 flex items-center gap-2 ml-1" aria-hidden="true">
              <span className="voice-wave-line" />
              <span className="voice-wave-bars">
                {Array.from({ length: 14 }, (_, index) => {
                  const level = levels[levels.length - 14 + index] ?? 0;
                  return <i key={index} style={{ height: Math.round(4 + level * 24) }} />;
                })}
              </span>
              <span className="text-sm text-muted tabular-nums">{recordingTime}</span>
            </div>
            <MicControl
              recording
              busy={dictationBusy}
              ready={voiceReady}
              onToggle={() => void toggleDictation()}
            />
          </>
        ) : (
          <>
            {/* Left: how it should work, what to give it, and how to talk to it. */}
            {props.workspace !== undefined && (
              <ModeMenu
                mode={props.mode}
                options={PERMISSION_OPTIONS}
                bypass={BYPASS_OPTION}
                onModeChange={props.onModeChange}
                unattended={props.unattended}
                onUnattendedChange={props.onUnattendedChange}
              />
            )}
            <AttachMenu
              onFiles={onPickFiles}
              onFolder={isTauri() ? addFolder : undefined}
              onSlashCommands={openSlashCommands}
              onConnectors={props.onOpenConnectors}
            />
            {isTauri() && (
              <MicControl
                recording={false}
                busy={dictationBusy}
                ready={voiceReady}
                onToggle={() => void toggleDictation()}
              />
            )}
            {/* Hands-free. Distinct from the mic on purpose: dictation is a button that
                types, this is a conversation that listens. */}
            {isTauri() && props.onToggleHandsFree && (
              <button
                type="button"
                className={"composer-chip is-icon" + (props.handsFree ? " is-open" : "") + (!voiceReady && !props.handsFree ? " is-dim" : "")}
                onClick={() => props.onToggleHandsFree?.()}
                data-testid="handsfree-toggle"
                title={
                  props.handsFree
                    ? `Hands-free: ${props.voiceState ?? "listening"} — click to stop`
                    : voiceReady
                      ? "Talk to it — mic stays open, it replies out loud"
                      : "Configure Voice Input in Settings"
                }
                aria-pressed={!!props.handsFree}
                aria-label={props.handsFree ? "Stop hands-free conversation" : "Start hands-free conversation"}
              >
                <Icon name={props.handsFree ? "stop" : "sparkle"} size={ICON.md} />
              </button>
            )}
          </>
        )}

        {dictationBusy === "Transcribing…" && <span className="text-xs text-accent">Transcribing…</span>}

        <span className="ml-auto" />

        {/* Right: which model, how hard it should think, and what it has spent. */}
        {!dictation?.recording && (needsModel ? (
          <button
            className="pill model-warn chip"
            onClick={() => props.onConnectModel?.()}
            title="Connect a model"
            aria-label="No model connected — connect a model"
          >
            <span className="pill-label">No model</span>
            <span className="model-warn-ico" aria-hidden>⚠</span>
          </button>
        ) : modelsLoaded ? (
          <ModelMenu
            model={props.model}
            models={props.models || []}
            labels={props.modelLabels}
            providers={providers}
            onChange={props.onModelChange}
          />
        ) : (
          <button
            className="pill chip text-faint cursor-default"
            disabled
            data-testid="models-loading"
            title="Fetching the model list from the server"
          >
            <span className="pill-label">Loading models…</span>
          </button>
        ))}

        {!dictation?.recording && effort && (
          <EffortChip
            effort={effort.value}
            levels={effort.levels}
            onChange={(value) => setEffort((e) => (e ? { ...e, value } : e))}
          />
        )}

        {!dictation?.recording && (
          <ContextRing
            contextUsed={props.usage?.context ?? 0}
            contextWindow={props.contextWindow}
            breakdown={breakdown}
          />
        )}

    </div>
  );
}
