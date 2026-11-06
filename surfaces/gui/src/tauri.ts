// The bridge to the Tauri desktop shell. Outside the desktop app every call here is inert
// (isTauri() is false), so one codebase serves both. It talks to the `window.__TAURI__`
// global that the shell injects (`withGlobalTauri`) instead of importing @tauri-apps
// packages, so the browser build carries no Tauri dependency.

export { chooseFolder, getAutostart, getKeepAwake, isTauri, pickFolder, platformOS, setAutostart, setKeepAwake, startWindowDrag } from "./tauri/DictationStatus";
export type { DictationDownloadProgress, DictationStatus } from "./tauri/DictationStatus";
export { browserBounds, browserClose, browserDevServers, browserHistory, browserNavigate, browserOpen, browserUrl, browserVisible } from "./tauri/appBrowser";
export type { BrowserRect, DevServer } from "./tauri/appBrowser";
export { checkForUpdate, clearPendingUpdate, downloadUpdate, installUpdate } from "./tauri/autoUpdate";
export type { UpdateInfo, UpdateStatus } from "./tauri/autoUpdate";
export { calibrateVoice, cancelDictation, cancelDictationModelDownload, deleteDictationModel, downloadDictationModel, dragPetWindow, getDictationLevel, getDictationStatus, getVoiceCalibration, isConversing, listVoices, listenDictationDownloadProgress, listenVoice, markDictationTestPassed, onTauriEvent, setMascotVisible, setOpenDocuments, showMainWindow, speak, startConversation, startDictation, stopConversation, stopDictation, stopSpeaking, verifyDictationModel, voiceEngines } from "./tauri/desktopPetMascot";
export type { VoiceCalibration, VoiceEngineStatus, VoiceState } from "./tauri/desktopPetMascot";
export { devserverRunning, devserverStart, devserverStop, onBrowserNav, onDevserverExit, onDevserverOutput, openExternal } from "./tauri/devServers";
export { onTerminalExit, onTerminalOutput, terminalClose, terminalOpen, terminalResize, terminalWrite } from "./tauri/interactiveTerminal";
