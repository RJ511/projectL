export function normalize(path) {
  return (path || "").replace(/\\/g, "/");
}
