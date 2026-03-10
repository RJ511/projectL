# Project L

The propose of this project is to develop a software that has **all the necessary tools for autodidacts** or some who really wants to learn with all the available resources online.

## Tools to implement:

    - Note Library (Second Brain type)
    - Time tracker
    - Calendar
    - Concentration assistant
    - Emotion Tracker
    - OML
    - Flashcards
    - Motivation booster

## Structure wise

    - Anki compatability
    - Plugin-wise

## Software utilizado

    - Tauri
    - MirrorCode 6

## OLM Core integrated (from brainstorm)

The project now includes an integrated OLM core in the current Tauri + React stack, based on the section after **"0. O que estás a fazer (em 1 frase)"** in `bainstorm.md`.

### Implemented modules (MVP)

    - M1 Domain model (concept graph): concepts + prerequisite edges
    - M2 Content mapping (manual): content item -> concept coverage weights
    - M3 Event pipeline: unified study event schema
    - M4 OLM engine: Beta-based state per concept (alpha/beta, mastery, uncertainty)
    - M5 Decision service: "Next to study" with readiness gating by prerequisites
    - M6 Visibility: UI panel to inspect state, recommendations and evidence trail

### Core concepts integrated

    - Explicit learner state per concept: alpha, beta, mastery, uncertainty
    - Explainability trail: per-event delta contribution (+alpha / +beta)
    - Event weighting by event type + mapping coverage + confidence
    - Readiness-aware recommendation score:
        Score(c) = Readiness(c) * (lambda * (1 - mastery) + (1 - lambda) * uncertainty)

### Current scope note

This is implemented without adding another software stack (no Django, no external backend). It runs inside the existing Tauri backend commands and React frontend.

The sections **Tools to implement** and **Structure wise** remain guiding requirements for upcoming increments (Note Library, Flashcards, Anki compatibility, plugin-wise architecture, etc.).

## Canonical documentation

- Implementation source of truth: `docs/architecture-deep-dive.md`
- OLM formal logic source of truth: `OLM_LOGIC_MAP.md` (repository root)
- Operational usage guide: `docs/PROJECT_GUIDE.md`
- Module-level implementation map: `docs/IMPLEMENTATION_CONTEXT.md`

Note: `docs/OLM_LOGIC_MAP.md` is intentionally a bridge file to avoid duplicated technical content.
