<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { EditorView, basicSetup } from "codemirror";
  import { EditorState, Compartment } from "@codemirror/state";
  import { keymap, drawSelection } from "@codemirror/view";
  import { undo, indentWithTab } from "@codemirror/commands";
  import { search } from "@codemirror/search";

  export let doc = "";
  export let onChange: (v: string) => void;
  export let onScroll: (line: number) => void;

  let editorElement: HTMLElement;
  let view: EditorView;
  let fontSize = 18;
  let lastDist = 0;
  const themeCompartment = new Compartment();

  onMount(() => {
    const savedSize = localStorage.getItem("editor-font-size");
    if (savedSize) fontSize = parseInt(savedSize);

    // 初始化编辑器
    const state = createEditorState(doc);
    view = new EditorView({
      state,
      parent: editorElement,
    });
  });

  onDestroy(() => view?.destroy());

  function createEditorState(initialDoc: string) {
    return EditorState.create({
      doc: initialDoc,
      extensions: [
        basicSetup,
        drawSelection(),
        EditorView.lineWrapping,
        search({ top: false }),
        keymap.of([indentWithTab]),
        themeCompartment.of(
          EditorView.theme({
            "&": {
              height: "100%",
              fontSize: `${fontSize}px`,
              backgroundColor: "#fff",
            },
            ".cm-content": {
              fontFamily: "serif",
              paddingBottom: "55vh",
              lineHeight: "1.8",
              "-webkit-touch-callout": "none",
            },
            // 极致深蓝选中色
            ".cm-selectionBackground": {
              backgroundColor: "rgba(0, 102, 184, 0.45) !important",
            },
            "&.cm-focused .cm-selectionBackground": {
              backgroundColor: "rgba(0, 102, 184, 0.55) !important",
            },
            ".cm-gutters": {
              backgroundColor: "#f5f5f5",
              color: "#999",
              borderRight: "1px solid #ddd",
            },
          }),
        ),
        EditorView.updateListener.of((update) => {
          if (update.docChanged) onChange(update.state.doc.toString());
          // 选中文字自动收起键盘
          if (update.selectionSet && !update.state.selection.main.empty) {
            view.contentDOM.blur();
          }
          // 滚动监听：使用视口顶部的行号
          if (update.geometryChanged) {
            // 计算当前可视区域第一行
            const topBlock = view.lineBlockAt(view.viewport.from);
            const lineNum = view.state.doc.lineAt(topBlock.from).number;
            onScroll(lineNum);
          }
        }),
      ],
    });
  }

  // 使用 setState 重置整个状态，顺便清除撤销历史
  export function resetDoc(n: string) {
    if (!view) return;
    view.setState(createEditorState(n));
  }
  export function scrollToLine(l: number) {
    try {
      const line = view.state.doc.line(
        Math.max(1, Math.min(l, view.state.doc.lines)),
      );
      view.dispatch({
        selection: { anchor: line.from },
        effects: EditorView.scrollIntoView(line.from, { y: "center" }),
      });
    } catch (e) {}
  }
  export function selectMatch(l: number, s: number, e: number) {
    try {
      const line = view.state.doc.line(l);
      view.dispatch({
        selection: { anchor: line.from + s, head: line.from + e },
        effects: EditorView.scrollIntoView(line.from + s, { y: "center" }),
      });
      view.focus();
    } catch (ex) {}
  }
  export function replaceSelection(t: string) {
    const sel = view.state.selection.main;
    if (!sel.empty)
      view.dispatch({ changes: { from: sel.from, to: sel.to, insert: t } });
  }
  export function triggerUndo() {
    undo(view);
  }

  function handleTouch(e: TouchEvent) {
    if (e.touches.length === 2) {
      if (e.type === "touchmove") e.preventDefault();
      const dist = Math.hypot(
        e.touches[0].clientX - e.touches[1].clientX,
        e.touches[0].clientY - e.touches[1].clientY,
      );
      if (lastDist > 0) {
        const diff = dist - lastDist;
        if (Math.abs(diff) > 15) {
          updateFontSize(diff > 0 ? 1 : -1);
          lastDist = dist;
        }
      } else lastDist = dist;
    } else lastDist = 0;
  }

  function handleWheel(e: WheelEvent) {
    if (e.ctrlKey) {
      e.preventDefault();
      updateFontSize(e.deltaY < 0 ? 1 : -1);
    }
  }

  function updateFontSize(delta: number) {
    fontSize = Math.max(12, Math.min(fontSize + delta, 48));
    localStorage.setItem("editor-font-size", fontSize.toString());
    view.dispatch({
      effects: themeCompartment.reconfigure(
        EditorView.theme({ "&": { fontSize: `${fontSize}px` } }),
      ),
    });
  }
</script>

<div
  class="editor-container"
  bind:this={editorElement}
  on:touchstart={handleTouch}
  on:touchmove={handleTouch}
  on:wheel={handleWheel}
></div>

<style>
  .editor-container {
    width: 100%;
    height: 100%;
    touch-action: manipulation;
  }
</style>
