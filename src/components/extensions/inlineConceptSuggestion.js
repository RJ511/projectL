import { ViewPlugin } from "@codemirror/view";

const CONCEPT_TRIGGER_RE = /;;;([^;\n]{0,120})$/;

function toBaseConcept(typed = "") {
  const cleaned = String(typed || "").trim();
  if (!cleaned) return "conceito";

  // If the user already typed prereq or weight syntax, keep only the concept label.
  return (
    cleaned
      .split(":")[0]
      .replace(/\[[^\]]*\]$/g, "")
      .trim() || "conceito"
  );
}

function buildOptions(typed = "") {
  const base = toBaseConcept(typed);
  return [`;;;${base};;;`, `;;;${base}:pr1,pr2;;;`, `;;;${base} [0.7];;;`];
}

export function inlineConceptSuggestion() {
  return ViewPlugin.fromClass(
    class {
      constructor(view) {
        this.view = view;
        this.dropdown = null;
        this.triggerRange = null;
      }

      update(update) {
        if (!update.docChanged && !update.selectionSet) return;

        const pos = update.view.state.selection.main.head;
        const before = update.view.state.doc.sliceString(
          Math.max(0, pos - 150),
          pos,
        );
        const match = before.match(CONCEPT_TRIGGER_RE);

        if (!match) {
          this.closeDropdown();
          return;
        }

        const typed = match[1] || "";
        this.triggerRange = {
          from: pos - match[0].length,
          to: pos,
          typed,
        };

        Promise.resolve().then(() => {
          this.showDropdown(pos, typed);
        });
      }

      showDropdown(pos, typed) {
        this.closeDropdown();

        const coords = this.view.coordsAtPos(pos);
        if (!coords || !this.triggerRange) return;

        const dropdown = document.createElement("div");
        dropdown.className = "inline-concept-suggestion";

        const header = document.createElement("div");
        header.className = "inline-concept-suggestion__title";
        header.textContent = "Sugestao OLM inline";
        dropdown.appendChild(header);

        for (const option of buildOptions(typed)) {
          const item = document.createElement("button");
          item.type = "button";
          item.className = "inline-concept-suggestion__item";
          item.textContent = option;
          item.onclick = () => {
            if (!this.triggerRange) return;

            this.view.dispatch({
              changes: {
                from: this.triggerRange.from,
                to: this.triggerRange.to,
                insert: option,
              },
              selection: { anchor: this.triggerRange.from + option.length },
            });

            this.closeDropdown();
            this.view.focus();
          };
          dropdown.appendChild(item);
        }

        dropdown.style.position = "absolute";
        dropdown.style.left = `${coords.left}px`;
        dropdown.style.top = `${coords.bottom + 6}px`;
        dropdown.style.zIndex = "10000";

        document.body.appendChild(dropdown);
        this.dropdown = dropdown;
      }

      closeDropdown() {
        if (this.dropdown) {
          this.dropdown.remove();
          this.dropdown = null;
        }
      }

      destroy() {
        this.closeDropdown();
      }
    },
  );
}
