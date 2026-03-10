import {
  createFolder,
  deletePath,
  readFile,
  renameFile,
  writeFile,
} from "./fs.service";
import {
  rootAppStateBackupRelPath,
  rootAppStateRelPath,
  rootAppStateTempRelPath,
  ROOT_DATA_DIR,
} from "./dataPolicy";

function safeJsonParse(raw, fallback) {
  try {
    return JSON.parse(raw);
  } catch {
    return fallback;
  }
}

async function ensureDataDir(rootPath) {
  if (!rootPath) return;
  try {
    await createFolder(rootPath, ROOT_DATA_DIR);
  } catch {
    // Folder may already exist; ignore non-critical errors here.
  }
}

export async function loadRootState(rootPath) {
  if (!rootPath) return {};

  const mainPath = rootAppStateRelPath();
  const backupPath = rootAppStateBackupRelPath();

  try {
    const raw = await readFile(rootPath, mainPath);
    const parsed = safeJsonParse(raw, {});
    if (parsed && typeof parsed === "object") {
      return parsed;
    }
  } catch {
    // Fallback to backup when main snapshot is unavailable.
  }

  try {
    const backupRaw = await readFile(rootPath, backupPath);
    const backupParsed = safeJsonParse(backupRaw, {});
    if (backupParsed && typeof backupParsed === "object") {
      await writeFile(
        rootPath,
        mainPath,
        JSON.stringify(backupParsed, null, 2),
      );
      return backupParsed;
    }
  } catch {
    // No backup available.
  }

  return {};
}

export async function saveRootState(rootPath, state) {
  if (!rootPath) return;
  await ensureDataDir(rootPath);

  const mainPath = rootAppStateRelPath();
  const tempPath = rootAppStateTempRelPath();
  const backupPath = rootAppStateBackupRelPath();
  const serialized = JSON.stringify(state || {}, null, 2);

  let previousMainRaw = null;
  try {
    previousMainRaw = await readFile(rootPath, mainPath);
  } catch {
    previousMainRaw = null;
  }

  if (typeof previousMainRaw === "string") {
    await writeFile(rootPath, backupPath, previousMainRaw);
  }

  await writeFile(rootPath, tempPath, serialized);

  try {
    await deletePath(rootPath, mainPath);
  } catch {
    // Main file may not exist on first write.
  }

  try {
    await renameFile(rootPath, tempPath, "app_state.json");
  } catch (renameErr) {
    if (typeof previousMainRaw === "string") {
      await writeFile(rootPath, mainPath, previousMainRaw);
    }
    throw renameErr;
  }
}

export async function getRootStateValue(rootPath, key, fallback = null) {
  const state = await loadRootState(rootPath);
  if (!Object.prototype.hasOwnProperty.call(state, key)) return fallback;
  return state[key];
}

export async function setRootStateValue(rootPath, key, value) {
  const state = await loadRootState(rootPath);
  state[key] = value;
  await saveRootState(rootPath, state);
}
