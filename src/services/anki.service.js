const ANKI_CONNECT_URL = "http://127.0.0.1:8765";

async function invokeAnki(action, params = {}) {
  const response = await fetch(ANKI_CONNECT_URL, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      action,
      version: 6,
      params,
    }),
  });

  if (!response.ok) {
    throw new Error(`AnkiConnect HTTP ${response.status}`);
  }

  const payload = await response.json();
  if (payload.error) {
    throw new Error(payload.error);
  }

  return payload.result;
}

function escapeDeckName(deckName) {
  return deckName.replaceAll('"', '\\"');
}

export async function listAnkiDecks() {
  const names = await invokeAnki("deckNames");
  if (!Array.isArray(names)) return [];
  return names.sort((a, b) => a.localeCompare(b));
}

export async function getAnkiDeckStats(deckName) {
  const safeName = escapeDeckName(deckName);

  const [dueIds, newIds, learnIds] = await Promise.all([
    invokeAnki("findCards", { query: `deck:"${safeName}" is:due` }),
    invokeAnki("findCards", { query: `deck:"${safeName}" is:new` }),
    invokeAnki("findCards", { query: `deck:"${safeName}" is:learn` }),
  ]);

  return {
    due: Array.isArray(dueIds) ? dueIds.length : 0,
    newCards: Array.isArray(newIds) ? newIds.length : 0,
    learning: Array.isArray(learnIds) ? learnIds.length : 0,
  };
}
