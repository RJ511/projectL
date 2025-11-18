// src/HtmlEditor.jsx
import { useEffect } from "react";
import { useEditor, EditorContent } from "@tiptap/react";
import StarterKit from "@tiptap/starter-kit";
import { FloatingMenu, BubbleMenu } from "@tiptap/react/menus";

export default function HtmlEditor({ value, onChange }) {
  const editor = useEditor({
    extensions: [StarterKit],
    content: value || "",
    onCreate({ editor }) {
      // garante formatação imediata
      if (value) {
        editor.commands.setContent(value);
      }
    },
    onUpdate({ editor }) {
      const html = editor.getHTML();
      onChange(html);
    },
  });

  // sincroniza quando value muda externamente
  useEffect(() => {
    if (!editor) return;
    const current = editor.getHTML();
    if (value !== undefined && value !== current) {
      editor.commands.setContent(value);
    }
  }, [value, editor]);

  if (!editor) return null;

  return (
    <div style={{ position: "relative", height: "100%" }}>
      <EditorContent editor={editor} />

      {/* Floating Menu */}
      <FloatingMenu editor={editor}>
        <div
          style={{
            background: "#fff",
            padding: "6px 10px",
            border: "1px solid #ccc",
            borderRadius: "6px",
          }}
        >
          <button onClick={() => editor.chain().focus().toggleHeading({ level: 2 }).run()}>
            H2
          </button>
          <button onClick={() => editor.chain().focus().toggleBulletList().run()}>
            • Lista
          </button>
        </div>
      </FloatingMenu>

      {/* Bubble Menu */}
      <BubbleMenu editor={editor}>
        <div
          style={{
            background: "#fff",
            padding: "6px 10px",
            border: "1px solid #ccc",
            borderRadius: "6px",
          }}
        >
          <button onClick={() => editor.chain().focus().toggleBold().run()}>Bold</button>
          <button onClick={() => editor.chain().focus().toggleItalic().run()}>Italic</button>
          <button onClick={() => editor.chain().focus().toggleStrike().run()}>Strike</button>
        </div>
      </BubbleMenu>
    </div>
  );
}
