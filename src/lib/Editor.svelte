<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { EditorView, basicSetup } from "codemirror";
  import { EditorState, Compartment, Prec } from "@codemirror/state";
  import { keymap, drawSelection, Decoration } from "@codemirror/view";
  import { undo, redo, indentWithTab } from "@codemirror/commands";
  import { search, setSearchQuery, SearchQuery, findNext, findPrevious, replaceNext, replaceAll, getSearchQuery } from "@codemirror/search";
  import { listen, emit } from "@tauri-apps/api/event";
  import { WebviewWindow } from "@tauri-apps/api/webviewWindow";

  export let doc = "";
  export let titleLines: number[] = [];
  export let onChange: (v: string) => void;
  export let onScroll: (state: {
    top: number;
    bottom: number;
    isAtBottom: boolean;
  }) => void;
  export let onSelectionChange: (line: number) => void = () => {};

  let editorElement: HTMLElement;
  let view: EditorView;
  let fontSize = 18;
  const themeCompartment = new Compartment();
  const titleCompartment = new Compartment();

  // 滚动节流状态
  let scrollThrottleTimer: ReturnType<typeof setTimeout> | null = null;
  let lastReportedLine = "";

  // 内部状态跟踪
  let lastKnownDoc = "";

  // 静态标题装饰生成器，不依赖 ViewPlugin 防止重绘死锁
  function createTitleDecorations(lines: number[], state: EditorState) {
    const safeLines = Array.isArray(lines) ? lines : [];
    // 终极防重叠：剔除所有的重复行！
    // 如果存在多个连缀的章节序号在一行，或者是解析错误算到了同一行，就会产生完全相同的 lineNum
    // CM6 在同一位置强制放多个 Block 级 Decoration 时会导致内部计算脱节并在 posAtCoordsInline 时读取空指针崩溃！
    const uniqueLines = Array.from(new Set(safeLines));

    const decorations: any[] = [];
    for (const lineNum of uniqueLines) {
      try {
        if (lineNum >= 1 && lineNum <= state.doc.lines) {
          const line = state.doc.line(lineNum);
          decorations.push(
            Decoration.line({ class: "cm-title-line" }).range(line.from),
          );
        }
      } catch (e) {}
    }
    // 严格从小到大排序是 CM6 Requirement
    decorations.sort((a, b) => a.from - b.from);

    return EditorView.decorations.of(Decoration.set(decorations, true));
  }

  // 内部保存当前的 title 序列，防止无意义重绘
  let currentTitleLinesStamp = "";

  // 侦听 titleLines 变动来更新装饰（被动，不影响滚动测算）
  $: if (view && titleLines.length >= 0) {
    const newStamp = JSON.stringify(titleLines);
    if (newStamp !== currentTitleLinesStamp) {
      currentTitleLinesStamp = newStamp;
      try {
        view.dispatch({
          effects: titleCompartment.reconfigure(
            createTitleDecorations(titleLines, view.state),
          ),
        });
      } catch (err: any) {
        initError = err.stack || err.toString();
      }
    }
  }

  let initError = "";

  onMount(() => {
    // 终极杀手锏：拦截无论在何时发生的异步算绘崩溃！
    const handleGlobalError = (event: ErrorEvent) => {
      console.error("Global captured error:", event.error);
      initError = "RUNTIME CRASH:\\n" + (event.error?.stack || event.message);
    };
    window.addEventListener("error", handleGlobalError);

    const blockNativeSearch = (e: KeyboardEvent) => {
      // 捕获真正的 Ctrl+F 或者 F3 强行干掉它的原生行为并开启我们的悬浮窗
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "f") {
        e.preventDefault();
        e.stopPropagation();
        openSearchWindow();
      } else if (e.key === "F3") {
        e.preventDefault();
        e.stopPropagation();
        openSearchWindow();
      }
    };

    window.addEventListener("keydown", blockNativeSearch, true);

    try {
      const savedSize = localStorage.getItem("editor-font-size");
      if (savedSize) fontSize = parseInt(savedSize);
      lastKnownDoc = doc;

      const state = createEditorState(doc);
      view = new EditorView({
        state,
        parent: editorElement,
      });
    } catch (err: any) {
      console.error("CM6 Init Error:", err);
      initError = err.stack || err.toString();
    }

    let cleanupSearchListener: () => void;
    listen("search-action", (event: any) => {
      const p = event.payload;
      if (!view) return;

      try {
        let searchStr = p.search || "";
        let replaceStr = p.replace || "";
        let isRegex = p.searchMode === "regex";
        let matchCase = !!p.matchCase;
        let wholeWord = !!p.wholeWord;

        if (p.searchMode === "extended") {
          // 解析 \n, \r, \t 扩展字符
          searchStr = searchStr.replace(/\\n/g, "\n").replace(/\\t/g, "\t").replace(/\\r/g, "\r");
          replaceStr = replaceStr.replace(/\\n/g, "\n").replace(/\\t/g, "\t").replace(/\\r/g, "\r");
        } else if (p.searchMode === "normal" && wholeWord) {
          // 全词匹配：自动转为等价的正则
          const escapeRegex = (s: string) => s.replace(/[-\/\\^$*+?.()|[\]{}]/g, '\\$&');
          searchStr = `\\b${escapeRegex(searchStr)}\\b`;
          isRegex = true;
        }

        const query = new SearchQuery({
          search: searchStr,
          replace: replaceStr,
          caseSensitive: matchCase,
          regexp: isRegex
        });

        // =========================================================
        // 核心修复1：不在每次由于输入引发的 sync-only 时刻更新底层搜索！
        // 频繁更新底层搜索会导致 CM6 立刻执行高亮渲染并扫描全文档，带来巨大卡顿
        // 我们改为：只有在真正尝试 Find Next, Prev, Replace, Replace-all
        // 或者用户停止输入一段时间后（通过防抖在前端触发的），再予以执行。
        // =========================================================

        if (searchStr !== "" && p.type !== "sync-only") {
           view.dispatch({ effects: setSearchQuery.of(query) });
        }

        // 为了极大提升上千万字长文在文件尾部处的搜索速度，坚决【不能用】 query.getCursor(doc) 做任何能够积累到上千次循环的 while(true) 动作。
        // 我们利用偷跑检测法判断是否发生了 Wrap，从而阻止循环搜索：
        
        let shouldBypassDefaultSearch = false;
        
        if (!shouldBypassDefaultSearch) {
            const oldHead = view.state.selection.main.head;
            
            if (p.type === "find-next") {
              findNext(view);
            } else if (p.type === "find-prev") {
              findPrevious(view);
            } else if (p.type === "replace") {
              replaceNext(view);
            }

            const newHead = view.state.selection.main.head;
            
            // 如果没勾选允许循环，我们要判断是否发生了循环绕回：
            // 向下找：如果新的光标比老的光标还靠前，说明兜圈回到头部了！
            if (!p.wrapAround && searchStr !== "" && query.valid) {
                 if (p.type === "find-next" || p.type === "replace") {
                      if (newHead < oldHead) {
                          // 发生了 wrap 回滚，拒绝该动作，把光标置回
                          view.dispatch({ selection: { anchor: oldHead } });
                      }
                 } else if (p.type === "find-prev") {
                      // 向上找：如果新光标比老光标还靠后，说明兜圈回到尾部了！
                      if (newHead > oldHead) {
                          view.dispatch({ selection: { anchor: oldHead } });
                      }
                 }
            }
            
            // 查完之后强迫将视窗居中对齐现在的高亮词，防止它贴在最上方或最下方
            const finalHead = view.state.selection.main.head;
            // 只有当有东西且找到了不同位置时才滚
            if (finalHead !== oldHead && (p.type === "find-next" || p.type === "find-prev" || p.type === "replace")) {
                view.dispatch({
                    effects: EditorView.scrollIntoView(finalHead, { y: "center" })
                });
            }
        }
        
        if (p.type === "replace-all") {
          replaceAll(view);
        }

        // 彻底移除全量匹配统计以根治卡顿！！！
        // 极长的小说（几百万字）在正文或正则表达式下会直接让 UI 挂起几十秒
        // 所以我们现在只汇报目前是否能找到结果，不再汇报诸如 "1/10000" 的数量
        
        let hasMatch = false;
        if (searchStr !== "" && query.valid) {
             const cursor = query.getCursor(view.state.doc);
             const first = cursor.next();
             hasMatch = !first.done;
        }

        emit("search-status", { count: hasMatch ? 1 : 0 });

      } catch (e) {
        console.error("Search error:", e);
      }
    }).then(fn => cleanupSearchListener = fn);

    return () => {
      window.removeEventListener("keydown", blockNativeSearch, true);
      window.removeEventListener("error", handleGlobalError);
      if (cleanupSearchListener) cleanupSearchListener();
      view?.destroy();
    };
  });

  function createEditorState(initialDoc: string) {
    return EditorState.create({
      doc: initialDoc,
      extensions: [
        basicSetup,
        EditorView.lineWrapping,
        Prec.highest(
          keymap.of([
            {
              key: "Mod-f",
              run: () => {
                openSearchWindow();
                return true; 
              }
            },
            {
              key: "F3",
              run: () => {
                openSearchWindow();
                return true;
              }
            }
          ])
        ),
        search({ top: false }),
        keymap.of([indentWithTab]),
        titleCompartment.of(
          createTitleDecorations(
            titleLines,
            EditorState.create({ doc: initialDoc }),
          ),
        ),
        EditorView.theme({
          "&": {
            height: "100%",
            backgroundColor: "#fff",
          },
          ".cm-content": {
            fontFamily: "serif",
            // [极其关键的核心修正]：强制行高为绝对的数字像素(36px)，对应 18px 刚好是 2.0 倍。
            // 绝不允许使用 1.8 这种产生 32.4px 亚像素亚小数的高度，它会导致十万行计算积累出巨大的浮点跳变误差，最后触发 Viewport failed to stabilize
            lineHeight: "36px",
            "-webkit-touch-callout": "none",
          },
          ".cm-line": {
            minHeight: "36px", // 与外层 lineHeight 完美绑定
            paddingTop: "0",
            paddingBottom: "0",
          },
          // 选中色
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
            // 强制滚动栏常驻，切断行宽极值变化导致的滚动条弹出/消除重排震荡
            overflowY: "scroll",
          },
          ".cm-scroller::-webkit-scrollbar": { width: "14px" },
          ".cm-scroller::-webkit-scrollbar-track": { background: "#f1f1f1" },
          ".cm-scroller::-webkit-scrollbar-thumb": {
            background: "#888",
            borderRadius: "7px",
            border: "3px solid #f1f1f1",
          },
          ".cm-scroller::-webkit-scrollbar-thumb:hover": { background: "#555" },
          ".cm-scroller::-webkit-scrollbar-thumb:active": {
            background: "#333",
          },
          // 纯净的着色标题，禁止任何 padding/margin/fontSize 扰乱物理行高
          ".cm-title-line": {
            color: "#0066b8",
            background: "#e8f0fe",
            mixBlendMode: "multiply" // 让标题行的底色与后方的选区底色进行正片叠底，从而透出选区颜色
          }
        }),
        themeCompartment.of(
          EditorView.theme({
            "&": { fontSize: `${fontSize}px` },
          }),
        ),
        EditorView.updateListener.of((update) => {
          if (update.docChanged) {
            const newContent = update.state.doc.toString();
            lastKnownDoc = newContent;
            onChange(newContent);
          }

          if (update.selectionSet && !update.docChanged) {
            try {
              const cursorPos = update.state.selection.main.head;
              const line = update.state.doc.lineAt(cursorPos).number;
              onSelectionChange(line);
            } catch (e) {}
          }

          // 核心重构：使用纯净、零消耗且绝不会打断 CM6 算绘周期的 viewportLineBlocks 同步测算，
          // 并将数据结果缓存交由防抖函数发送。杜绝在异步定时器中查询 `lineBlockAtHeight`！
          if (
            update.geometryChanged ||
            update.viewportChanged ||
            update.docChanged
          ) {
            try {
              const scrollDOM = view.scrollDOM;
              const scrollY = scrollDOM.scrollTop;

              let currentTopLine = 1;
              let currentBottomLine = 1;

              // 从目前 CM6 *确切已经渲染出来* 的可见队列里寻找最顶部的行
              const blocks = view.viewportLineBlocks;
              if (blocks.length === 0) return; // 视口尚未就绪，不报告

              const topBlock = blocks.find((b: any) => b.bottom > scrollY + 5);
              if (!topBlock) return; // 渲染未跟上滚动进度，跳过当前帧以防报告错误的 top=1

              currentTopLine = view.state.doc.lineAt(topBlock.from).number;

              // 同理，寻找最底部的行
              const viewBottom = scrollY + scrollDOM.clientHeight;
              const bottomBlock = [...blocks]
                .reverse()
                .find((b: any) => b.top < viewBottom - 5);
              
              if (!bottomBlock) return;

              currentBottomLine = view.state.doc.lineAt(bottomBlock.from).number;

              const maxScroll = Math.max(
                0,
                scrollDOM.scrollHeight - scrollDOM.clientHeight,
              );
              // 如果只差 15px 以内就算触底，防止被行尾空白缝隙骗过去
              const isAtBottom = scrollY >= maxScroll - 15;

              // 异步提交测算结果，绝不打断当前渲染帧
              scheduleScrollReport(
                currentTopLine,
                currentBottomLine,
                isAtBottom,
              );
            } catch (e) {}
          }
        }),
      ],
    });
  }

  let pendingScrollState: { top: number; bottom: number; isAtBottom: boolean } | null = null;

  function scheduleScrollReport(
    top: number,
    bottom: number,
    isAtBottom: boolean,
  ) {
    // 必须保存最新状态！
    // 解决闭包导致大跨度跳转时上报过期数据的核心 Bug：
    // 当跳转导致画面瞬间变动时，第一次传进来的 top 往往是陈旧或是渲染中间态的。
    // 如果不用 pendingScrollState 保存最新值，80ms 定时器就会闭包捕获第一次的过期数值。
    // 而后续传来正确的最新位置时因为 timer 已存在直接被 return 忽略，导致永远上报错误的最终位置。
    pendingScrollState = { top, bottom, isAtBottom };

    if (scrollThrottleTimer) return;
    scrollThrottleTimer = setTimeout(() => {
      scrollThrottleTimer = null;
      if (pendingScrollState) {
        const { top: pTop, bottom: pBottom, isAtBottom: pIsAtBottom } = pendingScrollState;
        const stateStr = `${pTop}-${pBottom}-${pIsAtBottom}`;
        if (stateStr !== lastReportedLine) {
          lastReportedLine = stateStr;
          if (onScroll) onScroll({ top: pTop, bottom: pBottom, isAtBottom: pIsAtBottom });
        }
      }
    }, 80);
  }

  export function resetDoc(n: string) {
    if (!view) return;
    lastKnownDoc = n;
    view.setState(createEditorState(n));
  }
  export function scrollToLine(l: number, toTop: boolean = false) {
    if (!view) return;
    try {
      const line = view.state.doc.line(
        Math.max(1, Math.min(l, view.state.doc.lines)),
      );
      // 先 dispatch 设置光标和滚动目标，再 focus
      // 如果先 focus，编辑器会先滚动到旧光标位置（可能是第1行），产生可见的跳动
      view.dispatch({
        selection: { anchor: line.from },
        effects: EditorView.scrollIntoView(line.from, {
          y: toTop ? "start" : "center", // 使用 center 往往比 start 在处理这种“动态测绘行高”的文件时更稳定
          yMargin: toTop ? 5 : 20,
        }),
      });
      view.focus();
    } catch (e) {
      console.error("Editor: scrollToLine error", e);
    }
  }
  export function selectMatch(l: number, s: number, e: number) {
    if (!view) return;
    try {
      const line = view.state.doc.line(l);
      view.focus();
      view.dispatch({
        selection: { anchor: line.from + s, head: line.from + e },
        effects: EditorView.scrollIntoView(line.from + s, { y: "center" }),
      });
      onSelectionChange(l);
    } catch (ex) {}
  }
  export function replaceSelection(t: string) {
    const sel = view.state.selection.main;
    if (!sel.empty) {
      view.dispatch({ changes: { from: sel.from, to: sel.to, insert: t } });
    }
  }
  export function triggerUndo() {
    undo(view);
  }
  export function triggerRedo() {
    redo(view);
  }
  export function selectAll() {
    if (!view) return;
    view.dispatch({ selection: { anchor: 0, head: view.state.doc.length } });
    view.focus();
  }

  export async function openSearchWindow() {
    let focusText = "";
    if (view) {
      const sel = view.state.selection.main;
      if (!sel.empty) {
        focusText = view.state.sliceDoc(sel.from, sel.to);
      }
    }

    let searchWin = await WebviewWindow.getByLabel("search-replace");
    if (searchWin) {
      await searchWin.show();
      await searchWin.setFocus();
    } else {
      searchWin = new WebviewWindow("search-replace", {
        url: "/search-replace",
        title: "查找与替换",
        width: 480,
        height: 220,
        alwaysOnTop: true,
        resizable: true,
        minimizable: false,
        maximizable: false,
        focus: true
      });
    }
    
    // 给窗口一点初始化时间，如果在当前刚创建的话
    setTimeout(() => {
        if (focusText) {
          emit("search-focus", { selection: focusText });
        }
    }, 500);
  }
</script>

{#if initError}
  <div
    style="padding: 20px; color: red; background: #ffebee; height: 100%; overflow: auto; font-family: monospace; white-space: pre-wrap;"
  >
    <h2>CodeMirror Initialization Error</h2>
    {initError}
  </div>
{/if}

<div
  class="editor-container"
  bind:this={editorElement}
  style="display: {initError ? 'none' : 'flex'}"
></div>

<style>
  .editor-container {
    width: 100%;
    height: 100%;
    overflow: hidden;
    position: relative;
    display: flex; /* 让 CM6 真正的填满 */
    flex-direction: column;
  }

  /* 确保内部 CM 本体无限填满该区域 */
  :global(.cm-editor) {
    height: 100%;
    flex: 1;
  }
</style>
