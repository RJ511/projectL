export const META_COOLDOWN_SUCCESS_DEFAULT_MS = 10 * 60 * 1000;
export const META_COOLDOWN_DISMISS_MS = 12 * 60 * 1000;
export const META_COOLDOWN_SUCCESS_BY_TRIGGER_MS = {
  periodic: 15 * 60 * 1000,
  domain_switch: 20 * 60 * 1000,
  test_end: 5 * 60 * 1000,
};
export const MAX_META_PROMPTS_PER_SESSION = 2;

export function resolveMetaTrigger(trigger) {
  if (trigger === "periodic") return "periodic";
  if (trigger === "domain_switch") return "domain_switch";
  if (trigger === "test_end") return "test_end";
  return "periodic";
}

export function getSuccessCooldownMs(trigger) {
  const safeTrigger = resolveMetaTrigger(trigger);
  return (
    META_COOLDOWN_SUCCESS_BY_TRIGGER_MS[safeTrigger] ??
    META_COOLDOWN_SUCCESS_DEFAULT_MS
  );
}

export function getCooldownMsForOutcome(trigger, outcome) {
  if (outcome === "dismissed") return META_COOLDOWN_DISMISS_MS;
  return getSuccessCooldownMs(trigger);
}

export function shouldThrottleMetaPrompt({
  now,
  lastPromptAt,
  lastOutcome,
  trigger,
  force = false,
}) {
  if (force) return false;
  if (!Number.isFinite(lastPromptAt) || lastPromptAt <= 0) return false;

  const elapsed = Math.max(0, now - lastPromptAt);
  const cooldown = getCooldownMsForOutcome(trigger, lastOutcome);
  return elapsed < cooldown;
}

export function nextQuestionCursor({
  currentCursor,
  hasValidAnswer,
  questionCount,
}) {
  if (!hasValidAnswer) return currentCursor;
  if (!Number.isFinite(questionCount) || questionCount <= 0)
    return currentCursor;
  return (currentCursor + 1) % questionCount;
}
