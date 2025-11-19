import { useEffect, useRef } from "react";
import { EditorView, keymap, highlightActiveLine, lineNumbers } from "@codemirror/view";
import { EditorState } from "@codemirror/state";
import { defaultKeymap } from "@codemirror/commands";

import { markdown, markdownLanguage } from "@codemirror/lang-markdown";
import { languages } from "@codemirror/language-data";

import { inlinePreview } from "../extensions/inlinePreview";
import "../extensions/inlinePreview.css";

export default function CodeEditor({ value, onChange }) {
  const host = useRef(null);
  const viewRef = useRef(null);

  useEffect(() => {
    if (!host.current) return;

    const state = EditorState.create({
      doc: value || "",
      extensions: [
        markdown({
          base: markdownLanguage,
          codeLanguages: languages,
        }),


        keymap.of(defaultKeymap),
        highlightActiveLine(),
        lineNumbers(),
        EditorView.lineWrapping,
        inlinePreview(),
        EditorView.updateListener.of((update) => {
          if (update.docChanged) {
            onChange(update.state.doc.toString());
          }
        }),
      ],
    });

    const view = new EditorView({
      state,
      parent: host.current,
    });

    viewRef.current = view;
    return () => view.destroy();
  }, []);

  useEffect(() => {
    const view = viewRef.current;
    if (!view) return;

    const current = view.state.doc.toString();
    if (value != null && value !== current) {
      view.dispatch({
        changes: { from: 0, to: current.length, insert: value },
      });
    }
  }, [value]);

  return <div ref={host} style={{ width: "100%", height: "100%" }} />;
}
