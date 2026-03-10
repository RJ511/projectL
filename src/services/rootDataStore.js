import { createFolder, readFile, writeFile } from "./fs.service";
import { rootAppStateRelPath, ROOT_DATA_DIR } from "./dataPolicy";

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
  try {
    const raw = await readFile(rootPath, rootAppStateRelPath());
    const parsed = safeJsonParse(raw, {});
    return parsed && typeof parsed === "object" ? parsed : {};
  } catch {
    return {};
  }
}

export async function saveRootState(rootPath, state) {
  if (!rootPath) return;
  await ensureDataDir(rootPath);
  await writeFile(
    rootPath,
    rootAppStateRelPath(),
    JSON.stringify(state || {}, null, 2),
  );
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
