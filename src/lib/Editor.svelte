<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { EditorView, basicSetup } from "codemirror";
  import { EditorState, Compartment } from "@codemirror/state";
  import {
    keymap,
    drawSelection,
    ViewPlugin,
    Decoration,
    type DecorationSet,
    type ViewUpdate,
  } from "@codemirror/view";
  import { undo, redo, indentWithTab } from "@codemirror/commands";
  import { search } from "@codemirror/search";

  export let doc = "";
  export let titleLines: number[] = [];
  export let onChange: (v: string) => void;
  export let onScroll: (line: number) => void;
  export let onSelectionChange: (line: number) => void = () => {};

  let editorElement: HTMLElement;
  let view: EditorView;
  let fontSize = 18;
  let lastDist = 0;
  const themeCompartment = new Compartment();
  const titleCompartment = new Compartment();

  // 创建标题插件的函数
  function createTitlePlugin(lines: number[]) {
    // 安全检查：确保 lines 是数组
    const safeLines = Array.isArray(lines) ? lines : [];
    const lineSet = new Set(safeLines);

    return ViewPlugin.fromClass(
      class {
        decorations: DecorationSet;
        constructor(view: EditorView) {
          // 构造时尝试构建，如果失败则返回空集
          try {
            this.decorations = this.buildDecorations(view);
          } catch (e) {
            console.error("TitlePlugin init error:", e);
            this.decorations = Decoration.none;
          }
        }
        update(update: ViewUpdate) {
          if (update.docChanged || update.viewportChanged) {
            try {
              this.decorations = this.buildDecorations(update.view);
            } catch (e) {
              console.error("TitlePlugin update error:", e);
            }
          }
        }
        buildDecorations(view: EditorView): DecorationSet {
          const decorations: any[] = [];
          if (!view || !view.visibleRanges) return Decoration.none;

          for (let { from, to } of view.visibleRanges) {
            for (let pos = from; pos <= to; ) {
              const line = view.state.doc.lineAt(pos);
              if (lineSet.has(line.number)) {
                decorations.push(
                  Decoration.line({ class: "cm-title-line" }).range(line.from),
                );
              }
              pos = line.to + 1;
            }
          }
          return Decoration.set(decorations);
        }
      },
      { decorations: (v) => v.decorations },
    );
  }

  // 监听 titleLines 变化并更新插件
  $: if (view) {
    try {
      view.dispatch({
        effects: titleCompartment.reconfigure(createTitlePlugin(titleLines)),
      });
    } catch (e) {
      console.warn("TitlePlugin reconfigure error", e);
    }
  }

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
        // 初始标题插件
        titleCompartment.of(createTitlePlugin(titleLines)),
        EditorView.theme({
          "&": {
            height: "100%",
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
          ".cm-scroller": {
            overflowX: "hidden",
          },
          ".cm-scroller::-webkit-scrollbar": {
            width: "14px",
          },
          ".cm-scroller::-webkit-scrollbar-track": {
            background: "#f1f1f1",
          },
          ".cm-scroller::-webkit-scrollbar-thumb": {
            background: "#888",
            borderRadius: "7px",
            border: "3px solid #f1f1f1",
          },
          ".cm-scroller::-webkit-scrollbar-thumb:hover": {
            background: "#555",
          },
          ".cm-scroller::-webkit-scrollbar-thumb:active": {
            background: "#333",
          },
          // 标题行样式
          ".cm-title-line": {
            fontWeight: "bold",
            fontSize: "1.25em",
            color: "#222",
            textAlign: "center",
            paddingTop: "0.5em", // 改用padding而非margin
          },
        }),
        themeCompartment.of(
          EditorView.theme({
            "&": {
              fontSize: `${fontSize}px`,
            },
          }),
        ),
        EditorView.updateListener.of((update) => {
          if (update.docChanged) onChange(update.state.doc.toString());
          // 选中文字自动收起键盘
          if (update.selectionSet && !update.state.selection.main.empty) {
            view.contentDOM.blur();
          }
          // 点击或选择变化时同步目录位置
          if (update.selectionSet) {
            try {
              const cursorPos = update.state.selection.main.head;
              const lineNum = update.state.doc.lineAt(cursorPos).number;
              if (onScroll) onScroll(lineNum);
            } catch (e) {}
          }
          // 滚动监听：使用视口【上方30%位置】的行号，提供更及时的章节定位
          if (update.geometryChanged) {
            try {
              // 使用屏幕上方30%位置检测当前章节，而非中心位置
              // 这样当新章节滚动到视口上方时能更快响应
              const detectionHeight =
                update.view.scrollDOM.scrollTop +
                update.view.dom.clientHeight * 0.3;
              const detectionBlock =
                update.view.lineBlockAtHeight(detectionHeight);
              const lineNum = update.view.state.doc.lineAt(
                detectionBlock.from,
              ).number;
              if (onScroll) onScroll(lineNum);
            } catch (e) {
              console.warn("Scroll Sync Error", e);
            }
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
    console.log("Editor: scrollToLine called", l);
    try {
      const line = view.state.doc.line(
        Math.max(1, Math.min(l, view.state.doc.lines)),
      );
      console.log("Editor: resolved line", line);
      view.dispatch({
        selection: { anchor: line.from },
        effects: EditorView.scrollIntoView(line.from, {
          y: "start",
          yMargin: 20,
        }),
      });
      console.log("Editor: dispatch complete");
    } catch (e) {
      console.error("Editor: scrollToLine error", e);
    }
  }
  export function selectMatch(l: number, s: number, e: number) {
    try {
      const line = view.state.doc.line(l);
      view.dispatch({
        selection: { anchor: line.from + s, head: line.from + e },
        effects: EditorView.scrollIntoView(line.from + s, { y: "center" }),
      });
      view.focus();
      // 通知父组件选择位置已改变，以便同步目录
      if (onSelectionChange) onSelectionChange(l);
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
  export function triggerRedo() {
    redo(view);
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
