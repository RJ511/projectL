import { invoke } from "@tauri-apps/api/core";

export async function call(cmd, args = {}) {
  try {
    console.log(cmd, args)
    return await invoke(cmd, args);
  } catch (e) {
    console.error("Tauri command error:", cmd, args, e);
    throw e;
  }
}
