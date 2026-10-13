import { Composer } from "../components/Composer";
import { contextFromLabel } from "../modelName";
import { PlanCard } from "../components/PlanCard";
import { DirectoryRequestCard } from "../components/DirectoryRequestCard";
import { ApprovalCard } from "../components/ApprovalCard";
import { AskCard } from "../components/AskCard";
import { FormCard } from "../components/FormCard";
import { ComputerHelpCard, SecretCard } from "../components/ComputerCards";
import { resolveHelp, resolveSecret } from "../browserAsks";
import { InboxItemCard } from "../components/InboxItemCard";
import type { AppModel } from "./model/useAppModel";

/** The composer, with the live inline cards above it. */
export function ChatComposer({ m }: { m: AppModel }) {
  const { mode, model, models, modelLabels, running, connected, modelReady, openModelSetup,
      openSettings, send, handsFree, voiceState, voiceError, toggleHandsFree, interrupt,
      changeMode, changeModel, setSurface, sessionId, agent, needsWorkspace, workspace,
      unattended, toggleUnattended, composerPrefill, usage, modelContextWindows, pendingPlan,
      respondPlan, pendingDirReq, respondDirectory, pendingApproval, approve, runContext,
      pendingQuestion, answerQuestion, pendingForm, answerForm, pendingHelp, items, nameOf,
      setItems, setLiveView, pendingSecret, sessionInbox, resolveSessionInbox } = m;
  return (
    <Composer
              mode={mode}
              model={model}
              models={models}
              modelLabels={modelLabels}
              running={running}
              connected={connected}
              modelReady={modelReady}
              onConnectModel={openModelSetup}
              onConfigureVoiceInput={() => openSettings("voice")}
              onSend={send}
              handsFree={handsFree}
              voiceState={voiceState}
              voiceError={voiceError}
              onToggleHandsFree={() => void toggleHandsFree().catch(() => {})}
              onInterrupt={interrupt}
              onModeChange={changeMode}
              onModelChange={changeModel}
              onOpenConnectors={() => setSurface("integrations")}
              sessionId={sessionId}
              agent={agent}
              workspace={needsWorkspace(agent) ? workspace || "" : undefined}
              unattended={unattended}
              onUnattendedChange={toggleUnattended}
              prefill={composerPrefill}
              resetKey={sessionId}
              usage={usage}
              contextWindow={modelContextWindows[model] ?? contextFromLabel(modelLabels[model])}
              // One line for every persona. The three variants said the same thing three
              // ways and none of them said the useful part: that "/" opens the command
              // list. Drag-and-drop needs no instruction — you either try it or you do
              // not — and it was taking the half of the line that could teach something.
              placeholder="Type / for commands, @ for my brain or another assistant"
              approvalSlot={
                // Live inline cards are for ATTENDED sessions only; when Unattended the prompt is
                // parked in the Inbox and surfaced via the answer-in-context card below.
                !unattended && pendingPlan?.kind === "planreq" ? (
                  <PlanCard item={pendingPlan} onRespond={respondPlan} />
                ) : !unattended && pendingDirReq?.kind === "dirreq" ? (
                  <DirectoryRequestCard item={pendingDirReq} onRespond={respondDirectory} />
                ) : !unattended && pendingApproval?.kind === "approval" ? (
                  <ApprovalCard item={pendingApproval} onApprove={approve} runTask={runContext} compact />
                ) : !unattended && pendingQuestion?.kind === "question" ? (
                  <AskCard questions={pendingQuestion.questions} onDone={answerQuestion} compact />
                ) : !unattended && pendingForm?.kind === "form" ? (
                  <FormCard form={pendingForm.form} onDone={answerForm} compact />
                ) : pendingHelp?.kind === "computer_help" ? (
                  <ComputerHelpCard
                    key={`help-${items.indexOf(pendingHelp)}`}
                    persona={pendingHelp.persona}
                    name={nameOf(pendingHelp.persona)}
                    reason={pendingHelp.reason}
                    url={pendingHelp.url}
                    onTaken={(p) => {
                      setItems((prev) => resolveHelp(prev, "taken"));
                      setLiveView(p);
                    }}
                    onDeclined={() => setItems((prev) => resolveHelp(prev, "declined"))}
                    onEnded={() => setItems((prev) => resolveHelp(prev, "ended"))}
                    compact
                  />
                ) : pendingSecret?.kind === "computer_secret" ? (
                  <SecretCard
                    key={`secret-${items.indexOf(pendingSecret)}`}
                    persona={pendingSecret.persona}
                    name={nameOf(pendingSecret.persona)}
                    label={pendingSecret.label}
                    onDone={(r) => setItems((prev) => resolveSecret(prev, r))}
                    onEnded={() => setItems((prev) => resolveSecret(prev, "ended"))}
                    compact
                  />
                ) : sessionInbox[0] ? (
                  // Unattended session blocked on an Inbox item — answer it in context.
                  <InboxItemCard item={sessionInbox[0]} onResolve={resolveSessionInbox} compact />
                ) : undefined
              }
            />
  );
}
