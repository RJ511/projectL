import { call } from "./tauriBridge";

export function readFile(root, relPath) {
  return call("read_file", { root, relPath });
}

export function writeFile(root, relPath, content) {
  return call("write_file", { root, relPath, content });
}

export function getTree(root) {
  return call("get_tree", { root });
}

export function createFolder(root, name) {
  return call("create_folder", { root, name });
}

export function createFile(root, name) {
  return call("create_file", { root, name });
}

export function renameFile(root, oldPath, newName) {
  return call("rename_file", { root, oldPath, newName });
}
