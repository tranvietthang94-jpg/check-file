import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";

let permissionRequest: Promise<boolean> | null = null;

/** Requests notification permission at most once per app session. */
function ensurePermission(): Promise<boolean> {
  if (!permissionRequest) {
    permissionRequest = (async () => {
      let granted = await isPermissionGranted();
      if (!granted) {
        granted = (await requestPermission()) === "granted";
      }
      return granted;
    })();
  }
  return permissionRequest;
}

export async function notifyTransfer(title: string, body: string): Promise<void> {
  try {
    if (await ensurePermission()) {
      sendNotification({ title, body });
    }
  } catch (err) {
    // Desktop notification is a nice-to-have -- a denied/unavailable
    // notification service should never interrupt the transfer itself.
    console.error("failed to send desktop notification", err);
  }
}
