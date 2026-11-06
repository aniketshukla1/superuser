import { resolveHelp, resolveSecret } from "../browserAsks";
import { ApprovalCard } from "../components/ApprovalCard";
import { AskCard } from "../components/AskCard";
import { Composer } from "../components/Composer";
import { ComputerHelpCard, SecretCard } from "../components/ComputerCards";
import { DirectoryRequestCard } from "../components/DirectoryRequestCard";
import { FormCard } from "../components/FormCard";
import { InboxItemCard } from "../components/InboxItemCard";
import { PlanCard } from "../components/PlanCard";
import { contextFromLabel } from "../modelName";
import type { AppModel } from "./model/useAppModel";

/** The card that sits above the composer, if any: the first request still waiting.
 *
 *  Plans, folders, approvals, questions and forms appear here only in an ATTENDED session; an
 *  Unattended one has parked them in the Inbox, and they come back through the
 *  answer-in-context card at the end. The browser's two requests show either way. */
function liveCard(m: AppModel) {
  const { items, nameOf, pendingApproval, pendingDirReq, pendingForm, pendingHelp, pendingPlan, pendingQuestion, pendingSecret, setItems, unattended } = m;
  if (!unattended) {
    if (pendingPlan) return <PlanCard item={pendingPlan} onRespond={m.respondPlan} />;
    if (pendingDirReq) return <DirectoryRequestCard item={pendingDirReq} onRespond={m.respondDirectory} />;
    if (pendingApproval) return <ApprovalCard item={pendingApproval} onApprove={m.approve} runTask={m.runContext} compact />;
    if (pendingQuestion) return <AskCard questions={pendingQuestion.questions} onDone={m.answerQuestion} compact />;
    if (pendingForm) return <FormCard form={pendingForm.form} onDone={m.answerForm} compact />;
  }
  if (pendingHelp) {
    const settle = (how: "taken" | "declined" | "ended") => setItems((before) => resolveHelp(before, how));
    return (
      <ComputerHelpCard
        key={`help-${items.indexOf(pendingHelp)}`}
        persona={pendingHelp.persona}
        name={nameOf(pendingHelp.persona)}
        reason={pendingHelp.reason}
        url={pendingHelp.url}
        onTaken={(persona) => {
          settle("taken");
          m.setLiveView(persona);
        }}
        onDeclined={() => settle("declined")}
        onEnded={() => settle("ended")}
        compact
      />
    );
  }
  if (pendingSecret) {
    return (
      <SecretCard
        key={`secret-${items.indexOf(pendingSecret)}`}
        persona={pendingSecret.persona}
        name={nameOf(pendingSecret.persona)}
        label={pendingSecret.label}
        onDone={(how) => setItems((before) => resolveSecret(before, how))}
        onEnded={() => setItems((before) => resolveSecret(before, "ended"))}
        compact
      />
    );
  }
  // An Unattended session waiting on an Inbox item: answer it right here.
  const parked = m.sessionInbox[0];
  return parked ? <InboxItemCard item={parked} onResolve={m.resolveSessionInbox} compact /> : undefined;
}

/** The composer, with the live request cards above it. */
export function ChatComposer({ m }: { m: AppModel }) {
  const { agent, model, modelLabels, sessionId } = m;
  return (
    <Composer
      mode={m.mode}
      model={model}
      models={m.models}
      modelLabels={modelLabels}
      running={m.running}
      connected={m.connected}
      modelReady={m.modelReady}
      onConnectModel={m.openModelSetup}
      onConfigureVoiceInput={() => m.openSettings("voice")}
      onSend={m.send}
      handsFree={m.handsFree}
      voiceState={m.voiceState}
      voiceError={m.voiceError}
      onToggleHandsFree={() => void m.toggleHandsFree().catch(() => {})}
      onInterrupt={m.interrupt}
      onModeChange={m.changeMode}
      onModelChange={m.changeModel}
      onOpenConnectors={() => m.setSurface("integrations")}
      sessionId={sessionId}
      agent={agent}
      workspace={m.needsWorkspace(agent) ? m.workspace || "" : undefined}
      unattended={m.unattended}
      onUnattendedChange={m.toggleUnattended}
      prefill={m.composerPrefill}
      resetKey={sessionId}
      usage={m.usage}
      contextWindow={m.modelContextWindows[model] ?? contextFromLabel(modelLabels[model])}
      // One line for every assistant. The three versions said the same thing three ways and
      // none said the useful part, that "/" opens the command list; drag-and-drop needs no
      // instructions and was taking the half of the line that could teach something.
      placeholder="Type / for commands, @ for my brain or another assistant"
      approvalSlot={liveCard(m)}
    />
  );
}
