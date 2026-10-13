import { FolderGate } from "../components/FolderGate";
import { gateCancelTarget, fallbackAgent } from "../gateCancel";
import { GENERAL_ASSISTANT } from "./appHelpers";
import type { AppModel } from "./model/useAppModel";

/** The folder gate for assistants that need a workspace. */
export function AppFolderGate({ m }: { m: AppModel }) {
  const { showGate, surface, gatesWorkspace, agent, gateCreate, chooseWorkspace, setShowGate,
      setGateCreate, gateReturn, workspace, selectSession, switchAgent, personas } = m;
  return (
    <>{showGate && surface === "session" && gatesWorkspace(agent) && (
        <FolderGate
          create={gateCreate}
          onChoose={chooseWorkspace}
          // Always cancellable. Previously this was only offered when a workspace
          // already existed, which trapped anyone who opened a workbench by mistake:
          // Coder demands a folder, and with no way out the app was simply stuck.
          //
          // Cancel UNDOES the navigation that opened it — see `gateCancel.ts`, where
          // the decision and its one trap live. The × and Esc land here too, so the
          // three ways out of this dialog agree.
          onCancel={() => {
            setShowGate(false);
            setGateCreate(false);
            const back = gateReturn.current;
            gateReturn.current = null;
            const to = gateCancelTarget(back, workspace, gatesWorkspace);
            if (to.kind === "resume") selectSession(to.sessionId, to.workspace, to.agent, to.branch);
            else if (to.kind === "fallback") switchAgent(fallbackAgent(personas, GENERAL_ASSISTANT));
          }}
        />
      )}</>
  );
}
