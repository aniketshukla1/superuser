// Thin bridge to the Tauri desktop shell. In the browser these are inert (isTauri() === false),
// so the SPA stays a single codebase. We use the injected `window.__TAURI__` global (the shell
// sets `withGlobalTauri`) instead of the @tauri-apps npm packages, so the browser build needs
// no Tauri dependencies.

export { isTauri, platformOS, pickFolder, chooseFolder, getAutostart, setAutostart, getKeepAwake, setKeepAwake, startWindowDrag } from "./tauri/DictationStatus";
export type { DictationStatus, DictationDownloadProgress } from "./tauri/DictationStatus";
export { showMainWindow, setMascotVisible, dragPetWindow, onTauriEvent, getDictationStatus, getDictationLevel, startDictation, stopDictation, cancelDictation, downloadDictationModel, cancelDictationModelDownload, verifyDictationModel, markDictationTestPassed, deleteDictationModel, startConversation, stopConversation, isConversing, speak, stopSpeaking, listVoices, voiceEngines, setOpenDocuments, calibrateVoice, getVoiceCalibration, listenVoice, listenDictationDownloadProgress } from "./tauri/desktopPetMascot";
export type { VoiceEngineStatus, VoiceCalibration, VoiceState } from "./tauri/desktopPetMascot";
export { checkForUpdate, downloadUpdate, clearPendingUpdate, installUpdate } from "./tauri/autoUpdate";
export type { UpdateInfo, UpdateStatus } from "./tauri/autoUpdate";
export { terminalOpen, terminalWrite, terminalResize, terminalClose, onTerminalOutput, onTerminalExit } from "./tauri/interactiveTerminal";
export { browserOpen, browserNavigate, browserBounds, browserVisible, browserHistory, browserClose, browserUrl, browserDevServers } from "./tauri/appBrowser";
export type { BrowserRect, DevServer } from "./tauri/appBrowser";
export { devserverStart, devserverStop, devserverRunning, onDevserverOutput, onDevserverExit, onBrowserNav, openExternal } from "./tauri/devServers";
