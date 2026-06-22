import { useEffect, useRef } from "react";
import {
  EditorView,
  keymap,
  highlightActiveLine,
  lineNumbers,
} from "@codemirror/view";
import { EditorState } from "@codemirror/state";
import { defaultKeymap } from "@codemirror/commands";
import { markdown, markdownLanguage } from "@codemirror/lang-markdown";
import { languages } from "@codemirror/language-data";

import { inlinePreview } from "../../components/extensions/inlinePreview";
import { inlineConceptSuggestion } from "../../components/extensions/inlineConceptSuggestion";
import { wikiAutocomplete } from "../../components/extensions/wikiAutoComplete";
import { openFileEffect } from "../../components/editor/effects/openFileEffect";
import "../../components/extensions/wikiDropdown.css";
import "../../components/extensions/inlinePreview.css";
import "../../components/extensions/inlineConceptSuggestion.css";

export default function CodeEditor({ value, onChange, tree = [], openFile }) {
  const viewRef = useRef(null);
  const treeRef = useRef(tree);

  // manter tree sempre atualizado dentro das closures
  useEffect(() => {
    treeRef.current = tree;
  }, [tree]);

  function flattenFiles(nodes) {
    const out = [];
    function walk(arr) {
      if (!Array.isArray(arr)) return;
      for (const n of arr) {
        if (!n) continue;
        if (n.is_dir) walk(n.children);
        else out.push(n);
      }
    }
    walk(nodes);
    return out;
  }

  function getMdFileNodes() {
    const all = flattenFiles(treeRef.current);
    console.log(all);
    return all;
  }

  function getMdFileNames() {
    return getMdFileNodes().map((f) => f.name);
  }

  const attachEditor = (el) => {
    if (!el) return;
    if (viewRef.current) return;

    const state = EditorState.create({
      doc: value || "",
      extensions: [
        markdown({ base: markdownLanguage, codeLanguages: languages }),
        keymap.of(defaultKeymap),
        highlightActiveLine(),
        lineNumbers(),
        EditorView.lineWrapping,
        inlinePreview(),
        inlineConceptSuggestion(),

        // usa a tua versão manual sem CodeMirror autocomplete
        wikiAutocomplete(() => getMdFileNames()),

        EditorView.updateListener.of((update) => {
          for (let tr of update.transactions) {
            for (let ef of tr.effects) {
              if (ef.is(openFileEffect)) {
                const fileNodes = getMdFileNodes();
                const target = fileNodes.find((f) => f.name === ef.value);
                if (target && typeof openFile === "function") {
                  openFile(target);
                }
              }
            }
          }

          if (update.docChanged) {
            onChange(update.state.doc.toString());
          }
        }),
      ],
    });

    const view = new EditorView({ state, parent: el });
    viewRef.current = view;
  };

  // atualizar só o texto
  useEffect(() => {
    const view = viewRef.current;
    if (!view) return;

    const current = view.state.doc.toString();
    if (value !== current) {
      view.dispatch({
        changes: { from: 0, to: current.length, insert: value },
      });
    }
  }, [value]);

  return <div ref={attachEditor} style={{ width: "100%", height: "100%" }} />;
}
