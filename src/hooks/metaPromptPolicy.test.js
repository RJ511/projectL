import { describe, expect, it } from "vitest";
import {
  META_COOLDOWN_DISMISS_MS,
  getCooldownMsForOutcome,
  nextQuestionCursor,
  shouldThrottleMetaPrompt,
} from "./metaPromptPolicy";

describe("metaPromptPolicy", () => {
  it("applies per-trigger cooldown for successful answers", () => {
    expect(getCooldownMsForOutcome("periodic", "answered")).toBe(
      15 * 60 * 1000,
    );
    expect(getCooldownMsForOutcome("domain_switch", "answered")).toBe(
      20 * 60 * 1000,
    );
    expect(getCooldownMsForOutcome("test_end", "answered")).toBe(5 * 60 * 1000);
  });

  it("applies dismiss cooldown when user closes the prompt", () => {
    expect(getCooldownMsForOutcome("periodic", "dismissed")).toBe(
      META_COOLDOWN_DISMISS_MS,
    );
  });

  it("throttles when elapsed time is below cooldown", () => {
    const now = 1_000_000;
    const throttled = shouldThrottleMetaPrompt({
      now,
      lastPromptAt: now - 2 * 60 * 1000,
      lastOutcome: "answered",
      trigger: "periodic",
      force: false,
    });
    expect(throttled).toBe(true);
  });

  it("does not throttle when force=true", () => {
    const now = 1_000_000;
    const throttled = shouldThrottleMetaPrompt({
      now,
      lastPromptAt: now - 1,
      lastOutcome: "answered",
      trigger: "periodic",
      force: true,
    });
    expect(throttled).toBe(false);
  });

  it("advances question cursor only after valid answer", () => {
    expect(
      nextQuestionCursor({
        currentCursor: 2,
        hasValidAnswer: false,
        questionCount: 7,
      }),
    ).toBe(2);

    expect(
      nextQuestionCursor({
        currentCursor: 2,
        hasValidAnswer: true,
        questionCount: 7,
      }),
    ).toBe(3);
  });
});
