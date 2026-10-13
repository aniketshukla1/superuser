import { INBOX_UNLOCK } from "./ModelSettings";

export function announceInboxUnlock() {
  window.dispatchEvent(new CustomEvent(INBOX_UNLOCK));
}
