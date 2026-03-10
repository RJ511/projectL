import { normalize } from "./pathTools";

export const ROOT_DATA_DIR = ".projectl-data";
export const ROOT_APP_STATE_FILE = "app_state.json";
export const ROOT_APP_STATE_TEMP_FILE = "app_state.tmp";
export const ROOT_APP_STATE_BACKUP_FILE = "app_state.bak.json";
export const ROOT_OLM_STATE_FILE = "olm_state.json";

export function rootAppStateRelPath() {
  return `${ROOT_DATA_DIR}/${ROOT_APP_STATE_FILE}`;
}

export function rootAppStateTempRelPath() {
  return `${ROOT_DATA_DIR}/${ROOT_APP_STATE_TEMP_FILE}`;
}

export function rootAppStateBackupRelPath() {
  return `${ROOT_DATA_DIR}/${ROOT_APP_STATE_BACKUP_FILE}`;
}

export function rootOlmStateRelPath() {
  return `${ROOT_DATA_DIR}/${ROOT_OLM_STATE_FILE}`;
}

export function toRootOlmAbsolutePath(rootPath) {
  if (!rootPath) return null;
  const clean = String(rootPath).replace(/[\\/]+$/, "");
  const rel = normalize(rootOlmStateRelPath()).replaceAll("/", "\\");
  return `${clean}\\${rel}`;
}
