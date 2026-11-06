import { invoke, invokeStrict, type DictationDownloadProgress, type DictationStatus } from "./DictationStatus";

// The pet window (src/pet/), events from the shell, dictation and the voice.

export const showMainWindow = () => invoke<boolean>("show_main_window");
export const setMascotVisible = (visible: boolean) => invoke<boolean>("set_mascot_visible", { visible });

/** Move the borderless pet window by dragging its body (core:window:allow-start-dragging). */
export function dragPetWindow() {
  const current = (globalThis as any).__TAURI__?.window?.getCurrentWindow?.();
  current?.startDragging?.();
}

/** Listen for a shell event; the result stops listening. In a browser it does nothing. */
export async function onTauriEvent<T>(event: string, handler: (payload: T) => void): Promise<() => void> {
  const listen = (globalThis as any).__TAURI__?.event?.listen;
  if (!listen) return () => {};
  const stop = await listen(event, (message: { payload: T }) => handler(message.payload));
  return stop as () => void;
}

// Dictation runs only in the desktop app, on this machine. A browser build leaves it off on
// purpose rather than quietly sending microphone audio to a server.
export const getDictationStatus = () => invoke<DictationStatus>("get_dictation_status");
/** How loud the microphone is right now, 0 to 1 (0 when not recording), for the composer's
 *  live waveform. Cheap enough to poll about ten times a second. */
export const getDictationLevel = () => invoke<number>("dictation_level");
export const startDictation = () => invokeStrict<DictationStatus>("start_dictation");
export const stopDictation = () => invokeStrict<string>("stop_dictation");
export const cancelDictation = () => invokeStrict<void>("cancel_dictation");
export const downloadDictationModel = () => invokeStrict<DictationStatus>("download_dictation_model");
export const cancelDictationModelDownload = () => invokeStrict<void>("cancel_dictation_model_download");
export const verifyDictationModel = () => invokeStrict<DictationStatus>("verify_dictation_model");
export const markDictationTestPassed = () => invokeStrict<DictationStatus>("mark_dictation_test_passed");
export const deleteDictationModel = () => invokeStrict<DictationStatus>("delete_dictation_model");

/** Hands-free conversation: the mic stays open and turns are detected, rather than
 *  bracketed by a button. Events arrive on `voice-state`, `voice-turn`,
 *  `voice-bargein`, `voice-spoke` and `voice-error`. */
export const startConversation = () => invokeStrict<DictationStatus>("start_conversation");
export const stopConversation = () => invokeStrict<void>("stop_conversation");
export const isConversing = () => invoke<boolean>("is_conversing");

/** Speak an answer. Calls QUEUE: hand over each sentence as it streams and they are
 *  spoken in order, so the first words are heard while the rest is still arriving. */
export const speak = (text: string, opts?: { engine?: string; voice?: string; persona?: string }) =>
  invokeStrict<void>("speak", { text, ...(opts ?? {}) });
/** Barge-in: stops the sentence in flight and drops the ones queued behind it. */
export const stopSpeaking = () => invokeStrict<void>("stop_speaking");
export const listVoices = () => invoke<string[]>("list_voices");

/** Per-engine availability. `reason` says what to install when `available` is false —
 *  an engine you picked but never installed should say so, not fail at the first
 *  sentence. */
export type VoiceEngineStatus = {
  engine: "system" | "piper" | "kokoro" | "cloud";
  available: boolean;
  binary: string;
  voices: string[];
  reason: string | null;
};
export const voiceEngines = () => invoke<VoiceEngineStatus[]>("voice_engines");

/** Tell the running session which documents the panel has open, so "this file"
 *  resolves to what the user is looking at rather than to a guess. */
export const setOpenDocuments = (sessionId: string, open: string[], active: string | null) =>
  invoke<void>("set_open_documents", { sessionId, open, active });

/** A measurement of the user's room. Thresholds mean nothing in the abstract — the
 *  level meter reads voice, room and device gain together, and that mixture differs
 *  by an order of magnitude between machines. */
export type VoiceCalibration = {
  noise_floor: number;
  start_level: number;
  end_level: number;
  measured_at: number;
};
/** Measure and save. Takes ~1.4s, during which the user should be quiet. */
export const calibrateVoice = () => invokeStrict<VoiceCalibration>("calibrate_voice");
export const getVoiceCalibration = () => invoke<VoiceCalibration | null>("voice_calibration");

export type VoiceState = "off" | "listening" | "hearing" | "thinking" | "speaking";

export async function listenVoice<T>(event: string, handler: (payload: T) => void) {
  return onTauriEvent<T>(event, handler);
}

/** Progress while the dictation model downloads. */
export function listenDictationDownloadProgress(handler: (progress: DictationDownloadProgress) => void): Promise<() => void> {
  return onTauriEvent<DictationDownloadProgress>("dictation-download-progress", handler);
}
