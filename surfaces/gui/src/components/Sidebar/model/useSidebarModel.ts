import type { Props } from "../Props";
import { useAccountMenu } from "./useAccountMenu";
import { useAutomationsList } from "./useAutomationsList";
import { useRowMenu } from "./useRowMenu";
import { usePersonaLayout } from "./usePersonaLayout";
import { useProjects } from "./useProjects";
import { useSessionLists } from "./useSessionLists";
import { useProjectLists } from "./useProjectLists";
import { useSessionNav } from "./useSessionNav";

/** Everything Sidebar renders from: its state, what it derives, and its handlers. */
export function useSidebarModel(props: Props) {
  const m0 = { props };
  const m1 = { ...m0, ...useAccountMenu() };
  const m2 = { ...m1, ...useAutomationsList() };
  const m3 = { ...m2, ...useRowMenu() };
  const m4 = { ...m3, ...usePersonaLayout(m3) };
  const m5 = { ...m4, ...useProjects(m4) };
  const m6 = { ...m5, ...useSessionLists(m5) };
  const m7 = { ...m6, ...useProjectLists(m6) };
  const m8 = { ...m7, ...useSessionNav(m7) };
  return m8;
}

export type SidebarModel = ReturnType<typeof useSidebarModel>;
