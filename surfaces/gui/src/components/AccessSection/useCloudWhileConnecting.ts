import { useEffect, useState } from "react";
import { CLOUD_CHANGED, getCloudStatus, type CloudStatus } from "../../api";

/** Cloud sign-in status, kept fresh only while the connect pane is open. */
export function useCloudWhileConnecting(connecting: boolean) {
  const [cloud, setCloud] = useState<CloudStatus | null>(null);
  useEffect(() => {
    if (!connecting) return;
    // null means UNKNOWN (renders as "checking"), never signed-out: a single failed
    // fetch here used to demand sign-in from a signed-in user with no way to recover
    // (FB-013). Poll while the connect pane is open, keep last-good on failure, and
    // listen for the sign-in broadcast so the pane flips the moment login lands.
    const load = () => getCloudStatus().then(setCloud).catch(() => {});
    load();
    const t = setInterval(load, 5000);
    window.addEventListener(CLOUD_CHANGED, load);
    return () => {
      clearInterval(t);
      window.removeEventListener(CLOUD_CHANGED, load);
    };
  }, [connecting]);
  return cloud;
}
