import { ViewPlugin, Decoration } from "@codemirror/view";

export function wikiAutocomplete(getFiles) {
  return ViewPlugin.fromClass(
    class {
      constructor(view) {
        this.view = view;
        this.dropdown = null;
      }

      update(update) {
        // se texto não mudou, não fazes nada
        if (!update.docChanged && !update.selectionSet) return;

        const pos = update.view.state.selection.main.head;
        const before = update.view.state.doc.sliceString(Math.max(0, pos - 50), pos);

        // detectar [[texto
        const match = before.match(/\[\[([^\]]*)$/);
        if (!match) {
          this.closeDropdown();
          return;
        }

        const typed = match[1].toLowerCase();
        const files = (getFiles?.() || []).filter(f => f.toLowerCase().includes(typed));

        if (files.length === 0) {
          this.closeDropdown();
          return;
        }

        // mostrar dropdown
        Promise.resolve().then(() => {
          this.showDropdown(files, pos, match[1]);
        });
      }

      showDropdown(files, pos, typed) {
        this.closeDropdown();

        const coords = this.view.coordsAtPos(pos);
        if (!coords) return;

        const dropdown = document.createElement("div");
        dropdown.className = "wiki-dropdown";

        files.forEach((f) => {
          const item = document.createElement("div");
          item.className = "wiki-dropdown-item";
          item.textContent = f;

          item.onclick = () => {
            const view = this.view;
            const head = view.state.selection.main.head;
            const before = view.state.doc.sliceString(0, head);
            const match = before.match(/\[\[([^\]]*)$/);

            if (!match) return;

            const from = head - match[1].length;
            const insert = `[[${f}]]`;

            view.dispatch({
              changes: {
                from: from - 2, // remove [[ + typed
                to: head,
                insert,
              }
            });

            this.closeDropdown();
            view.focus();
          };

          dropdown.appendChild(item);
        });

        // posicionar
        dropdown.style.position = "absolute";
        dropdown.style.left = coords.left + "px";
        dropdown.style.top = coords.bottom + "px";
        dropdown.style.zIndex = 9999;
        dropdown.style.minWidth = "180px";

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
    }
  );
}
