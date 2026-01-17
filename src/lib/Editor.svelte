<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { EditorView, basicSetup } from "codemirror";
  import { EditorState } from "@codemirror/state";
  import { keymap } from "@codemirror/view";
  import { indentWithTab, undo } from "@codemirror/commands";
  import { search } from "@codemirror/search";

  // --- Props 定义 ---
  export let doc = ""; // 文档内容
  export let onChange: (val: string) => void = () => {}; // 内容变化回调
  export let onScroll: (line: number) => void = () => {}; // 滚动行号回调
  
  let editorElement: HTMLElement;
  let view: EditorView;
  let fontSize = 16; // 默认字号
  let lastDist = 0;  // 双指缩放距离暂存

  // --- 高级定制主题：解决 Android 端的视觉问题 ---
  const dynamicTheme = EditorView.theme({
    "&": { 
        height: "100%", 
        fontSize: "var(--font-size, 16px)", 
        backgroundColor: "#ffffff" 
    },
    "&.cm-focused": { outline: "none" },
    ".cm-content": { 
        fontFamily: "'Segoe UI', Roboto, 'Noto Sans SC', sans-serif", 
        paddingBottom: "60vh", // 极深的底部留白，确保文字能推到屏幕上半部分，不被键盘/查找栏挡住
        lineHeight: "1.8",      // 增加行高，提升阅读和点击精确度
        "-webkit-touch-callout": "none !important" // 屏蔽系统长按弹窗
    },
    // 强制选中强调色：深蓝色背景 + 1px 描边
    ".cm-selectionBackground": { 
        backgroundColor: "rgba(0, 102, 184, 0.4) !important",
        border: "1px solid #0066b8",
        borderRadius: "2px"
    },
    ".cm-focused .cm-selectionBackground": { 
        backgroundColor: "rgba(0, 102, 184, 0.5) !important" 
    },
    ".cm-gutters": { 
        backgroundColor: "#f5f5f5", 
        borderRight: "1px solid #ddd", 
        color: "#999",
        minWidth: "35px"
    },
    ".cm-activeLine": { backgroundColor: "#f0f7ff" },
    ".cm-activeLineGutter": { backgroundColor: "#d4e8fa" },
    // 查找匹配颜色
    ".cm-searchMatch": { backgroundColor: "#ffdf5d" },
    ".cm-searchMatch.cm-searchMatch-selected": { backgroundColor: "#ff9900", color: "#fff" }
  });

  onMount(() => {
    // 恢复上次保存的字号
    const saved = localStorage.getItem("editor-font-size");
    if(saved) fontSize = parseInt(saved);

    const startState = EditorState.create({
      doc,
      extensions: [
        basicSetup,
        keymap.of([indentWithTab]),
        search({ top: false }), // 禁用 CM 自带的 UI，使用父组件的 UI
        dynamicTheme,
        EditorView.lineWrapping, // 自动换行
        EditorView.updateListener.of((u) => {
          // 1. 内容更新通知
          if (u.docChanged) {
              onChange(u.state.doc.toString());
          }
          
          // 2. 核心修复：选中文字时自动收起键盘
          // 逻辑：如果用户当前选中了范围（非空选区），则强制失焦收起键盘
          if (u.selectionSet && !u.state.selection.main.empty) {
              view.contentDOM.blur();
          }

          // 3. 滚动和位置监听
          if (u.geometryChanged || u.selectionSet || u.docChanged) {
              try {
                  const line = u.state.doc.lineAt(u.state.selection.main.head).number;
                  onScroll(line);
              } catch(e) {}
          }
        })
      ]
    });

    view = new EditorView({
      state: startState,
      parent: editorElement
    });
  });

  onDestroy(() => {
    if (view) view.destroy();
  });

  // --- 对外暴露的 API 接口 (全量支持) ---

  // 重置文档内容
  export function resetDoc(n: string) { 
      if(!view) return;
      view.dispatch({ 
          changes: { from: 0, to: view.state.doc.length, insert: n },
          effects: EditorView.scrollIntoView(0)
      }); 
  }
  
  // 滚动到指定行并居中
  export function scrollToLine(l: number) {
      if(!view) return;
      try {
          const lineInfo = view.state.doc.line(Math.max(1, Math.min(l, view.state.doc.lines)));
          view.dispatch({ 
              selection: { anchor: lineInfo.from }, 
              effects: EditorView.scrollIntoView(lineInfo.from, { y: "center" }) 
          });
      } catch(e){}
  }

  // 选中匹配项
  export function selectMatch(l: number, s: number, e: number) {
      if(!view) return;
      try {
          const lineInfo = view.state.doc.line(l);
          const from = lineInfo.from + s;
          const to = lineInfo.from + e;
          view.dispatch({ 
              selection: { anchor: from, head: to }, 
              effects: EditorView.scrollIntoView(from, { y: "center" }) 
          });
          view.focus();
      } catch(ex){}
  }

  // 替换当前选中的文字
  export function replaceSelection(t: string) {
      if(!view) return;
      const sel = view.state.selection.main;
      if (!sel.empty) {
          view.dispatch({ 
              changes: { from: sel.from, to: sel.to, insert: t },
              effects: EditorView.scrollIntoView(sel.from, { y: "center" })
          });
      }
  }

  // 触发撤销操作
  export function triggerUndo() { 
      if(!view) return;
      undo(view); 
      // 撤销后，将光标所在点滚动到中心以便观察变化
      view.dispatch({ 
          effects: EditorView.scrollIntoView(view.state.selection.main.head, { y: "center" }) 
      });
  }

  // 清空历史记录（防止大文件内存占用或误触）
  export function clearHistory() {
      // CM6 清空历史通常需要重新配置 State，暂保留接口
  }

  // --- 双指缩放字号逻辑 ---
  function handleTouch(e: TouchEvent) {
      if (e.touches.length === 2) {
          // 双指操作时，阻止页面滚动或系统缩放
          if (e.type === 'touchmove') e.preventDefault();
          
          const dist = Math.hypot(
              e.touches[0].clientX - e.touches[1].clientX, 
              e.touches[0].clientY - e.touches[1].clientY
          );

          if (lastDist > 0) {
              const diff = dist - lastDist;
              // 阈值设为 12px，防止过于灵敏导致跳动
              if (Math.abs(diff) > 12) {
                  const step = diff > 0 ? 1 : -1;
                  // 字号范围限制在 12px - 48px
                  fontSize = Math.max(12, Math.min(fontSize + step, 48));
                  lastDist = dist;
                  // 持久化字号设置
                  localStorage.setItem("editor-font-size", fontSize.toString());
              }
          } else { 
              lastDist = dist; 
          }
      } else { 
          lastDist = 0; 
      }
  }
</script>

<div class="editor-container" 
     bind:this={editorElement} 
     style="--font-size: {fontSize}px"
     on:touchstart={handleTouch}
     on:touchmove={handleTouch}
     on:touchend={() => lastDist = 0}
></div>

<style>
  .editor-container { 
      width: 100%; 
      height: 100%; 
      overflow: hidden; 
      touch-action: manipulation; 
      user-select: text !important; 
      -webkit-user-select: text !important; 
  }

  /* 强制注入全局 ::selection 样式。
     这可以覆盖系统在极少数情况下可能弹出的自带选中色。
  */
  :global(::selection) { 
      background: rgba(0, 102, 184, 0.4) !important; 
      color: inherit !important; 
  }
</style>