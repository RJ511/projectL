// src/extensions/inlinePreview.js
import { ViewPlugin, Decoration } from "@codemirror/view";
import { RangeSetBuilder } from "@codemirror/state";
import { openFileEffect } from "../../components/editor/effects/openFileEffect";

export function inlinePreview() {
  return ViewPlugin.fromClass(
    class {
      constructor(view) {
        this.decorations = this.buildDeco(view);
      }

      update(update) {
        if (
          update.docChanged ||
          update.selectionSet ||
          update.viewportChanged
        ) {
          this.decorations = this.buildDeco(update.view);
        }
      }

      buildDeco(view) {
        const deco = [];
        const state = view.state;
        const cursor = state.selection.main.head;

        for (const { from, to } of view.visibleRanges) {
          const text = state.doc.sliceString(from, to);

          //
          // 1) INLINE STYLES: bold / italic / bold+italic / strike
          //
          // *it*, _it_, **bold**, __bold__, ***both***, ~~strike~~
          const inlineRe = /(\*\*\*|\*\*|\*|__|_|~~)([^*_~\n]+?)\1/g;
          let m;

          while ((m = inlineRe.exec(text))) {
            const marker = m[1];
            const inner = m[2];

            const start = from + m.index;
            const innerStart = start + marker.length;
            const innerEnd = innerStart + inner.length;
            const end = innerEnd + marker.length;

            const cursorInside = cursor >= innerStart && cursor <= innerEnd;
            if (cursorInside) continue;

            let cls = "";
            if (marker === "*" || marker === "_") cls = "cm-md-italic";
            else if (marker === "**" || marker === "__") cls = "cm-md-bold";
            else if (marker === "***") cls = "cm-md-bolditalic";
            else if (marker === "~~") cls = "cm-md-strike";

            deco.push({ from: start, to: innerStart, cls: "cm-md-hide" });
            deco.push({ from: innerEnd, to: end, cls: "cm-md-hide" });
            deco.push({ from: innerStart, to: innerEnd, cls });
          }

          //
          // 2) INLINE CODE: `code`
          //
          const codeRe = /`([^`\n]+)`/g;
          while ((m = codeRe.exec(text))) {
            const inner = m[1];
            const start = from + m.index;
            const innerStart = start + 1;
            const innerEnd = innerStart + inner.length;
            const end = innerEnd + 1;

            const cursorInside = cursor >= innerStart && cursor <= innerEnd;
            if (cursorInside) continue;

            deco.push({ from: start, to: innerStart, cls: "cm-md-hide" });
            deco.push({ from: innerEnd, to: end, cls: "cm-md-hide" });
            deco.push({ from: innerStart, to: innerEnd, cls: "cm-md-code" });
          }

          //
          // 3) WIKILINKS: [[Nome do ficheiro]]
          //
          const wikiRe = /\[\[([^[\]\n]+)\]\]/g;
          while ((m = wikiRe.exec(text))) {
            const inner = m[1];
            const start = from + m.index;
            const innerStart = start + 2;
            const innerEnd = innerStart + inner.length;
            const end = innerEnd + 2;

            const cursorInside = cursor >= innerStart && cursor <= innerEnd;
            if (cursorInside) continue;

            deco.push({ from: start, to: innerStart, cls: "cm-md-hide" });
            deco.push({ from: innerEnd, to: end, cls: "cm-md-hide" });
            deco.push({
              from: innerStart,
              to: innerEnd,
              cls: "cm-md-wikilink",
            });
          }

          //
          // 3.1) CONCEITOS INLINE: ;;;Nome do conceito;;;
          //
          const conceptRe = /;;;\s*([^;\n][^;\n]{0,80}?)\s*;;;/g;
          while ((m = conceptRe.exec(text))) {
            const inner = m[1];
            const whole = m[0];
            const start = from + m.index;
            const innerOffset = whole.indexOf(inner);
            if (innerOffset < 0) continue;

            const innerStart = start + innerOffset;
            const innerEnd = innerStart + inner.length;
            const end = start + whole.length;

            const cursorInside = cursor >= innerStart && cursor <= innerEnd;
            if (cursorInside) continue;

            deco.push({ from: start, to: innerStart, cls: "cm-md-hide" });
            deco.push({ from: innerEnd, to: end, cls: "cm-md-hide" });
            deco.push({ from: innerStart, to: innerEnd, cls: "cm-md-concept" });
          }

          //
          // 4) LINKS: [texto](http...)
          //
          const linkRe = /\[([^\]\n]+)\]\((https?:\/\/[^)\s]+)\)/g;
          while ((m = linkRe.exec(text))) {
            const label = m[1];
            const url = m[2];

            const start = from + m.index;
            const labelStart = start + 1;
            const labelEnd = labelStart + label.length;
            const urlStart = labelEnd + 2; // "("
            const urlEnd = urlStart + url.length;
            const end = urlEnd + 1; // ")"

            const cursorInsideLabel =
              cursor >= labelStart && cursor <= labelEnd;
            const cursorInsideUrl = cursor >= urlStart && cursor <= urlEnd;

            if (cursorInsideLabel || cursorInsideUrl) continue;

            deco.push({ from: start, to: labelStart, cls: "cm-md-hide" });
            deco.push({ from: labelEnd, to: end, cls: "cm-md-hide" });
            deco.push({ from: labelStart, to: labelEnd, cls: "cm-md-link" });
          }

          //
          // 5) AUTO-LINKS: <http://...>
          //
          const autoRe = /<https?:\/\/[^>\s]+>/g;
          while ((m = autoRe.exec(text))) {
            const whole = m[0];
            const url = whole.slice(1, -1); // remove < >

            const start = from + m.index;
            const innerStart = start + 1;
            const innerEnd = innerStart + url.length;
            const end = innerEnd + 1;

            const cursorInside = cursor >= innerStart && cursor <= innerEnd;
            if (cursorInside) continue;

            deco.push({ from: start, to: innerStart, cls: "cm-md-hide" });
            deco.push({ from: innerEnd, to: end, cls: "cm-md-hide" });
            deco.push({ from: innerStart, to: innerEnd, cls: "cm-md-link" });
          }

          //
          // 6) IMAGENS: ![alt](src)
          //
          const imgRe = /!\[([^\]\n]*)\]\(([^)\s]+)\)/g;
          while ((m = imgRe.exec(text))) {
            const alt = m[1];
            const src = m[2];

            const start = from + m.index;
            const altStart = start + 2; // ![
            const altEnd = altStart + alt.length;
            const srcStart = altEnd + 2; // ](
            const srcEnd = srcStart + src.length;
            const end = srcEnd + 1; // )

            const cursorInsideAlt = cursor >= altStart && cursor <= altEnd;
            const cursorInsideSrc = cursor >= srcStart && cursor <= srcEnd;

            // se estiver a editar alt ou src, mostra literal
            if (cursorInsideAlt || cursorInsideSrc) continue;

            // esconder ![
            deco.push({ from: start, to: altStart, cls: "cm-md-hide" });
            // esconder ](
            deco.push({ from: altEnd, to: srcStart, cls: "cm-md-hide" });
            // esconder ')'
            deco.push({ from: srcEnd, to: end, cls: "cm-md-hide" });

            // alt com estilo de imagem
            deco.push({ from: altStart, to: altEnd, cls: "cm-md-image" });
            // src esbatido mas editável quando entras
            deco.push({ from: srcStart, to: srcEnd, cls: "cm-md-url" });
          }

          //
          // 7) POR-LINHA: headings, blockquotes, listas, checkboxes, tabelas, hr, linebreak
          //
          const lines = text.split("\n");
          let offset = from;

          for (const line of lines) {
            const raw = line;
            const trimmedEnd = raw.trimEnd();
            const indent = raw.length - raw.trimStart().length;

            const lineStart = offset;
            const lineEnd = offset + raw.length;
            const content = raw.trimStart();

            // FOOTNOTE DEFINITIONS FIRST
            const footnoteDefRegex = /^\s*\[\^([^\]\n]+)\]:/;
            const defMatch = footnoteDefRegex.exec(raw);

            if (defMatch) {
              const label = defMatch[1];

              const fullStart = lineStart + raw.indexOf("[");
              const hatPos = lineStart + raw.indexOf("^");
              const innerEnd = lineStart + label.length;
              const end = innerEnd + 1; // "]"
              const fullEnd = lineEnd;

              // número [1]
              const numStart = fullStart + 1;
              const numEnd = numStart + label.length;

              // esconder "[^"
              deco.push({
                from: fullStart,
                to: hatPos + 1,
                cls: "cm-md-hide",
              });

              // esconder após o número "]"
              deco.push({
                from: numEnd + 1,
                to: numEnd + 2,
                cls: "cm-md-hide",
              });

              deco.push({
                from: numStart,
                to: numEnd,
                cls: "cm-md-footdef-number",
              });

              // linha inteira
              deco.push({
                from: fullStart,
                to: fullEnd,
                cls: "cm-md-footdef",
              });

              offset += raw.length + 1;
              continue; // IMPORTANTÍSSIMO
            }

            // INLINE FOOTNOTE REFERENCES
            const footnoteRefRegex = /\[\^([^\]\n]+)\]/g;
            let mRef;

            while ((mRef = footnoteRefRegex.exec(raw))) {
              const label = mRef[1];

              const start = lineStart + mRef.index;
              const hatPos = start + 1;
              const end = start + mRef[0].length;

              // cursor dentro = mostrar literal
              if (cursor >= start && cursor <= end) continue;

              // esconder "^"
              deco.push({
                from: hatPos,
                to: hatPos + 1,
                cls: "cm-md-hide",
              });

              // aplicar estilo a TODO o [1]
              deco.push({
                from: start,
                to: end,
                cls: "cm-md-footref",
              });
            }

            // HR: --- ___ ***
            if (
              /^(\*\s*\*\s*\*|-+\s*-+\s*-+|_+\s*_+\s*_+)$/.test(
                trimmedEnd.trim(),
              )
            ) {
              deco.push({ from: lineStart, to: lineEnd, cls: "cm-md-hr" });
            }

            // HEADINGS 1-6
            const hMatch = /^(#{1,6})\s+/.exec(content);
            if (hMatch) {
              const hashes = hMatch[1];
              const level = hashes.length;
              const start = lineStart + indent;
              const end = lineEnd;

              let cls = "cm-md-h6";
              if (level === 1) cls = "cm-md-h1";
              else if (level === 2) cls = "cm-md-h2";
              else if (level === 3) cls = "cm-md-h3";
              else if (level === 4) cls = "cm-md-h4";
              else if (level === 5) cls = "cm-md-h5";

              deco.push({ from: start, to: end, cls });
            }

            // BLOCKQUOTE: >, >>, etc.
            const bqMatch = /^(\s*>+)\s*/.exec(raw);
            if (bqMatch) {
              const markers = bqMatch[1];
              const firstGtIndex = raw.indexOf(">");
              const startGt = lineStart + firstGtIndex;

              deco.push({
                from: lineStart,
                to: lineEnd,
                cls: "cm-md-blockquote",
              });

              // esconder '>' múltiplos
              for (let i = 0; i < markers.length; i++) {
                const pos = startGt + i;
                deco.push({ from: pos, to: pos + 1, cls: "cm-md-hide" });
              }
            }

            // LISTAS E CHECKBOXES

            // CHECKBOXES: - [ ] item, - [x] item
            const cbMatch = /^(\s*)([-+*])\s+\[( |x|X)\]\s+(\S.*)$/.exec(raw);

            if (cbMatch) {
              const indent = cbMatch[1].length;
              const bulletPos = lineStart + indent;
              const boxStart = bulletPos + 2; // "- " is 2 chars

              // esconder [ ] ou [x]
              deco.push({
                from: boxStart,
                to: boxStart + 3,
                cls: "cm-md-hide",
              });

              // desenhar caixa:
              deco.push({
                from: boxStart,
                to: boxStart + 3,
                cls:
                  cbMatch[3].toLowerCase() === "x"
                    ? "cm-md-checkbox-checked"
                    : "cm-md-checkbox",
              });

              // também esconder o marcador "-" e mostrá-lo como bullet:
              deco.push({
                from: bulletPos,
                to: bulletPos + 1,
                cls: "cm-md-hide",
              });

              offset += raw.length + 1;
              continue;
            }

            // - item, * item, + item
            const listMatch = /^(\s*)([-+*])\s+(?!\[)(\S.*)$/.exec(raw);
            if (listMatch) {
              const leadingSpaces = listMatch[1].length;
              const markerPos = lineStart + leadingSpaces;
              const markerEnd = markerPos + 1;

              // bullet (marcador)
              deco.push({
                from: markerPos,
                to: markerEnd,
                cls: "cm-md-bullet",
              });
            }

            // TABELAS (linha com | ... |)
            const isTableRow =
              /\|/.test(raw) && !/^\s*```/.test(raw) && !/^\s*#/.test(raw);
            if (isTableRow) {
              deco.push({
                from: lineStart,
                to: lineEnd,
                cls: "cm-md-table-row",
              });
            }

            // line break com 2+ espaços no fim
            const twoSpacesRe = /(\s{2,})$/;
            const lbMatch = twoSpacesRe.exec(trimmedEnd);
            if (lbMatch && trimmedEnd.length > 0) {
              const spaces = lbMatch[1];
              const spacesLen = spaces.length;
              const spacesStart = lineEnd - spacesLen;
              const spacesEnd = lineEnd;
              deco.push({
                from: spacesStart,
                to: spacesEnd,
                cls: "cm-md-linebreak",
              });
            }

            offset += raw.length + 1;
          }
        }

        // Ordenar SEMPRE antes de aplicar (evita erro de ranges)
        deco.sort((a, b) => a.from - b.from || a.to - b.to);

        const builder = new RangeSetBuilder();
        for (const d of deco) {
          builder.add(d.from, d.to, Decoration.mark({ class: d.cls }));
        }

        return builder.finish();
      }
    },
    {
      decorations: (v) => v.decorations,
      eventHandlers: {
        click: (e, view) => {
          if (!e.ctrlKey) return;

          let el = e.target;
          if (!(el instanceof HTMLElement)) return;

          // SUBIR A ARVORE DOM ATÉ ENCONTRAR .cm-md-wikilink
          const linkEl = el.closest(".cm-md-wikilink");
          if (!linkEl) return;

          console.log("CTRL+CLICK wikilink");

          // obter posição do CM a partir do elemento
          const pos = view.posAtDOM(linkEl);
          if (!pos) return;

          const raw = view.state.doc.sliceString(pos.from, pos.to).trim();

          // limpar [[ ]]
          const clean = raw.replace(/^\[\[/, "").replace(/\]\]$/, "");

          view.dispatch({
            effects: openFileEffect.of(clean),
          });
        },
      },
    },
  );
}
