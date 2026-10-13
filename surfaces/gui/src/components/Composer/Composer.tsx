import { useRef, useState } from "react";
import { totalTokens } from "../../usage";
import type { Props } from "./Props";
import { ComposerNotices } from "./ComposerNotices";
import { composerKeys } from "./composerKeys";
import { useComposerActions } from "./useComposerActions";
import { useDraftLifecycle } from "./useDraftLifecycle";
import { ComposerBar } from "./ComposerBar";
import { MentionPopup, SlashPopup } from "./ComposerPopups";
import { SendControls } from "./SendControls";
import { UsageBreakdown } from "./UsageBreakdown";
import { useAttachments } from "./useAttachments";
import { useComposerSettings } from "./useComposerSettings";
import { useDictation } from "./useDictation";
import { useMentions } from "./useMentions";
import { useSlashMenu } from "./useSlashMenu";

export function Composer(props: Props) {
  const [text, setText] = useState("");
  const fileInput = useRef<HTMLInputElement | null>(null);
  const textareaRef = useRef<HTMLTextAreaElement | null>(null);

  const {
    pendingSkill, setPendingSkill, prefixIntact, slashQuery, slashSkills, slashMatches, commandMatches,
    slashIndex, setSlashIndex, optionCount, choose, pickSkill, runCommand,
  } = useSlashMenu({
    text,
    setText,
    sessionId: props.sessionId,
    workspace: props.workspace,
    agent: props.agent,
    running: props.running,
    onSend: props.onSend,
    textareaRef,
  });
  const { mentionIndex, setMentionIndex, mentionMatches, mentionOpen, pickMention } = useMentions(text, setText, textareaRef, props.agent);

  const [dragging, setDragging] = useState(false);
  const { dictation, dictationBusy, dictationError, recordingTime, levels, voiceReady, toggleDictation } =
    useDictation(setText, textareaRef, props.onConfigureVoiceInput);
  const { attachments, setAttachments, attachNotice, setAttachNotice, addFiles } = useAttachments({
    text,
    sessionId: props.sessionId,
    running: props.running,
    onSend: props.onSend,
  });

  useDraftLifecycle({ text, setText, setAttachments, setPendingSkill, textareaRef, resetKey: props.resetKey, prefill: props.prefill });

  const { effort, setEffort, providers } = useComposerSettings();

  const { pickFiles, ATTACH_ACCEPT, addFolder, openSlashCommands } = useComposerActions(fileInput, textareaRef, setText);

  const usageBreakdown =
    props.usage && totalTokens(props.usage) > 0 ? <UsageBreakdown usage={props.usage} modelLabels={props.modelLabels} /> : undefined;

  const needsModel = props.modelReady === false;

  const { submit, onKey } = composerKeys({
    text, setText, attachments, setAttachments, pendingSkill, setPendingSkill, prefixIntact,
    slashQuery, slashIndex, setSlashIndex, optionCount, choose, runCommand,
    mentionOpen, mentionMatches, mentionIndex, setMentionIndex, pickMention,
    recording: !!dictation?.recording, dictationBusy, needsModel,
    onConnectModel: props.onConnectModel, onSend: props.onSend,
  });

  const onPaste = (e: React.ClipboardEvent) => {
    const imgs = Array.from(e.clipboardData.items)
      .filter((it) => it.kind === "file" && it.type.startsWith("image/"))
      .map((it) => it.getAsFile())
      .filter(Boolean) as File[];
    if (imgs.length) {
      e.preventDefault();
      addFiles(imgs);
    }
  };

  const modelsLoaded = !!(props.models && props.models.length);

  // The send button is accent only when there's something to send — subtle grey otherwise, so the
  // composer isn't carrying a constant blue dot.
  // A pinned /skill is sendable content on its own (tester catch 2026-07-26: the arrow
  // stayed grey after picking a skill, reading as "stuck").
  const hasContent = text.trim().length > 0 || attachments.length > 0 || !!pendingSkill;

  return (
    <div className="composer-wrap px-6 pb-5 pt-4">
      {props.approvalSlot}

      <ComposerNotices
        error={dictationError || props.voiceError}
        attachNotice={attachNotice}
        onDismissNotice={() => setAttachNotice(null)}
        attachments={attachments}
        onRemove={(i) => setAttachments((all) => all.filter((_, j) => j !== i))}
      />

      {/* The input and its controls, in one positioned block: a chip's menu anchors to THIS,
          so it opens above the whole composer instead of over the words you are writing. */}
      <div className="composer-stack relative max-w-3xl mx-auto">
      <div
        className={
          // `line-strong`, not `line`: this is the one input on the screen, and a
          // hairline meant for dividing lists left its edge almost invisible on the
          // dark ground. The dark theme adds glass and lift in styles.css, and
          // `is-working` runs a light along the edge while a turn is in progress.
          "composer relative rounded-xl2 border border-lineStrong bg-panel" +
          (dragging ? " dragging" : "") +
          (props.running ? " is-working" : "")
        }
        onDragOver={(e) => {
          e.preventDefault();
          setDragging(true);
        }}
        onDragLeave={() => setDragging(false)}
        onDrop={(e) => {
          e.preventDefault();
          setDragging(false);
          if (e.dataTransfer.files.length) addFiles(e.dataTransfer.files);
        }}
      >
        {/* "/" command list — a floating panel above the input; rows are the session's
            effective menu only (muted/disabled skills never appear). */}
        {slashQuery !== null && (
          <SlashPopup
            slashSkills={slashSkills}
            slashMatches={slashMatches}
            commandMatches={commandMatches}
            slashIndex={slashIndex}
            setSlashIndex={setSlashIndex}
            pickSkill={pickSkill}
            runCommand={runCommand}
          />
        )}
        {mentionOpen && (
          <MentionPopup
            mentionMatches={mentionMatches}
            mentionIndex={mentionIndex}
            setMentionIndex={setMentionIndex}
            pickMention={pickMention}
          />
        )}
        {/* Send stays IN the box, with the thing it sends. The chips moved out because
            they are things you set; this is the act itself, and it belongs at the end of
            what you typed. `items-end` so it sits by the last line as the textarea grows
            rather than drifting to the middle of a tall message. */}
        {/* ONE inset, on the row, instead of three. The textarea carried px-3.5/pt-3/pb-3,
            the send column carried pr-2.5/pb-2.5, and the two never agreed: text sat 15
            from the left and 13 from the top, Send sat 11 from the right and 7 from the
            top, and there was no gap at all between the last character and the button.
            The row owns the padding now, so every edge is the same number by
            construction rather than by three of them being kept in step. */}
        <div className="flex items-end gap-2 p-2">
          <textarea
            ref={textareaRef}
            // Tests address the composer by this, not by its placeholder: the placeholder is copy,
            // and 38 selectors went stale together the last time it changed.
            data-testid="composer-input"
            aria-label="Message"
            className="flex-1 min-w-0 block text-md"
            placeholder={
              props.running
                ? "Working… type to steer it"
                : props.placeholder || "Type / for commands, @ for my brain or another assistant"
            }
            value={text}
            onChange={(e) => setText(e.target.value)}
            onKeyDown={onKey}
            onPaste={onPaste}
            rows={1}
          />
          <SendControls
            running={props.running}
            hasContent={hasContent}
            connected={props.connected}
            blocked={!!dictation?.recording || !!dictationBusy}
            needsModel={needsModel}
            onSubmit={submit}
            onInterrupt={props.onInterrupt}
          />
        </div>
      </div>

      <ComposerBar
        props={props}
        fileInput={fileInput}
        onFiles={addFiles}
        onPickFiles={() => pickFiles(ATTACH_ACCEPT)}
        addFolder={addFolder}
        openSlashCommands={openSlashCommands}
        dictation={dictation}
        dictationBusy={dictationBusy}
        levels={levels}
        recordingTime={recordingTime}
        voiceReady={voiceReady}
        toggleDictation={toggleDictation}
        needsModel={needsModel}
        modelsLoaded={modelsLoaded}
        providers={providers}
        effort={effort}
        setEffort={setEffort}
        breakdown={usageBreakdown}
      />
      </div>
      <span className="sr-only" role="status" aria-live="polite">
        {dictation?.recording ? `Listening, ${recordingTime}` : dictationBusy || ""}
      </span>
    </div>
  );
}
