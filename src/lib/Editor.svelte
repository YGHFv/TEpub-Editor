<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { EditorView, basicSetup } from "codemirror";
  import { EditorState, Compartment, Prec } from "@codemirror/state";
  import { keymap, drawSelection, Decoration, highlightWhitespace } from "@codemirror/view";
  import { undo, redo, indentWithTab } from "@codemirror/commands";
  import { search, setSearchQuery, SearchQuery, findNext, findPrevious, replaceNext, replaceAll } from "@codemirror/search";
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
  export let wordWrap: boolean = true;
  export let showWhitespace: boolean = false;
  export let showLineBreaks: boolean = false;
  export let onTocSearch: ((query: string, actionType: string, searchMode: string) => void) | null = null;

  let editorElement: HTMLElement;
  let view: EditorView;
  let fontSize = 18;
  const themeCompartment = new Compartment();
  const titleCompartment = new Compartment();
  const wrapCompartment = new Compartment();
  const whiteSpaceCompartment = new Compartment();
  const lineBreakCompartment = new Compartment();

  // 滚动节流状态
  let scrollThrottleTimer: ReturnType<typeof setTimeout> | null = null;
  let lastReportedLine = "";

  // 内部状态跟踪
  let lastKnownDoc = "";
  let isRestoringProgrammaticScroll = false;
  let programmaticScrollRestoreId = 0;

  function getCodeMirrorCspNonce() {
    const nonceElement = document.querySelector("style[nonce], script[nonce]") as HTMLElement | null;
    return nonceElement?.nonce || nonceElement?.getAttribute("nonce") || "";
  }

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

  const lineBreakTheme = EditorView.theme({
    ".cm-line": { position: "relative" },
    ".cm-line::after": {
        content: '"\\21B5"',
        color: "rgba(120, 120, 120, 0.9)",
        position: "absolute",
        paddingLeft: "8px",
        pointerEvents: "none",
        userSelect: "none",
        whiteSpace: "nowrap"
    }
  });

  $: if (view) {
    view.dispatch({
      effects: [
        wrapCompartment.reconfigure(wordWrap ? EditorView.lineWrapping : []),
        whiteSpaceCompartment.reconfigure(showWhitespace ? highlightWhitespace() : []),
        lineBreakCompartment.reconfigure(showLineBreaks ? lineBreakTheme : [])
      ]
    });
  }

  onMount(() => {
    // 终极杀手锏：拦截无论在何时发生的异步算绘崩溃！
    const handleGlobalError = (event: ErrorEvent) => {
      console.error("Global captured error:", event.error);
      initError = "运行时崩溃：\\n" + (event.error?.stack || event.message);
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
    let resizeObserver: ResizeObserver | null = null;
    if (typeof ResizeObserver !== "undefined" && editorElement) {
      resizeObserver = new ResizeObserver(() => {
        view?.requestMeasure();
      });
      resizeObserver.observe(editorElement);
    }

    listen("search-action", (event: any) => {
      const p = event.payload;
      if (!view) return;

      try {
        // 目录查找模式：完全跳过 CM，直接回调父组件处理目录搜索
        if (p.searchInToc) {
          console.log("[EDITOR] searchInToc=true, onTocSearch=" + typeof onTocSearch + " search=" + JSON.stringify(p.search) + " type=" + p.type);
          if (onTocSearch && p.search) {
            console.log("[EDITOR] calling onTocSearch");
            onTocSearch(p.search, p.type, p.searchMode || "normal");
          }
          return;
        }

        let searchStr = p.search || "";
        let replaceStr = p.replace || "";
        let isRegex = p.searchMode === "regex";
        let matchCase = !!p.matchCase;
        let wholeWord = !!p.wholeWord;

        if (p.searchMode === "extended") {
          searchStr = searchStr.replace(/\\n/g, "\n").replace(/\\t/g, "\t").replace(/\\r/g, "\r");
          replaceStr = replaceStr.replace(/\\n/g, "\n").replace(/\\t/g, "\t").replace(/\\r/g, "\r");
        } else if (p.searchMode === "normal" && wholeWord) {
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

        const hasQuery = searchStr !== "";
        const isSyncOnly = p.type === "sync-only";

        if (hasQuery && !isSyncOnly) {
           view.dispatch({ effects: setSearchQuery.of(query) });
        }

        const oldHead = view.state.selection.main.head;

        if (p.type === "find-next") {
          findNext(view);
        } else if (p.type === "find-prev") {
          findPrevious(view);
        } else if (p.type === "replace") {
          replaceNext(view);
        }

        const newHead = view.state.selection.main.head;

        if (!p.wrapAround && hasQuery && query.valid) {
             if (p.type === "find-next" || p.type === "replace") {
                  if (newHead < oldHead) {
                      view.dispatch({ selection: { anchor: oldHead } });
                  }
             } else if (p.type === "find-prev") {
                  if (newHead > oldHead) {
                      view.dispatch({ selection: { anchor: oldHead } });
                  }
             }
        }

        const finalHead = view.state.selection.main.head;
        if (finalHead !== oldHead && (p.type === "find-next" || p.type === "find-prev" || p.type === "replace")) {
            view.dispatch({
                effects: EditorView.scrollIntoView(finalHead, { y: "center" })
            });
        }

        function countMatches() {
          if (!hasQuery || !query.valid) return 0;
          const cursor = query.getCursor(view.state.doc);
          let count = 0;
          for (let next = cursor.next(); !next.done; next = cursor.next()) {
            count += 1;
          }
          return count;
        }

        if (p.type === "replace-all") {
          const replacedCount = countMatches();
          if (replacedCount > 0) {
            replaceAll(view);
          }
          emit("search-status", {
            action: p.type,
            count: replacedCount,
            current: replacedCount > 0 ? replacedCount : 0,
            message: replacedCount > 0 ? `已替换 ${replacedCount} 处` : "无结果",
          });
          return;
        }

        let hasMatch = false;
        if (hasQuery && query.valid) {
             const cursor = query.getCursor(view.state.doc);
             const first = cursor.next();
             hasMatch = !first.done;
        }

        emit("search-status", { action: p.type, count: hasMatch ? 1 : 0 });

      } catch (e) {
        console.error("Search error:", e);
      }
    }).then(fn => cleanupSearchListener = fn);

    return () => {
      window.removeEventListener("keydown", blockNativeSearch, true);
      window.removeEventListener("error", handleGlobalError);
      resizeObserver?.disconnect();
      if (cleanupSearchListener) cleanupSearchListener();
      view?.destroy();
    };
  });

  function createEditorState(initialDoc: string) {
    const cspNonce = getCodeMirrorCspNonce();

    return EditorState.create({
      doc: initialDoc,
      extensions: [
        ...(cspNonce ? [EditorView.cspNonce.of(cspNonce)] : []),
        basicSetup,
        wrapCompartment.of(wordWrap ? EditorView.lineWrapping : []),
        whiteSpaceCompartment.of(showWhitespace ? highlightWhitespace() : []),
        lineBreakCompartment.of(showLineBreaks ? lineBreakTheme : []),
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
            backgroundColor: "var(--color-surface)",
          },
          ".cm-content": {
            fontFamily: "var(--font-reading)",
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
            backgroundColor: "rgba(22, 119, 184, 0.34) !important",
          },
          "&.cm-focused .cm-selectionBackground": {
            backgroundColor: "rgba(22, 119, 184, 0.42) !important",
          },
          ".cm-gutters": {
            backgroundColor: "var(--color-surface-soft)",
            color: "var(--color-muted)",
            borderRight: "1px solid var(--color-border)",
          },
          ".cm-scroller": {
            overflowX: "hidden",
            // 强制滚动栏常驻，切断行宽极值变化导致的滚动条弹出/消除重排震荡
            overflowY: "scroll",
          },
          ".cm-scroller::-webkit-scrollbar": { width: "14px" },
          ".cm-scroller::-webkit-scrollbar-track": {
            background: "rgba(226, 235, 244, 0.72)",
          },
          ".cm-scroller::-webkit-scrollbar-thumb": {
            background: "linear-gradient(180deg, #bacbda, #93a8bb)",
            borderRadius: "7px",
            border: "3px solid rgba(226, 235, 244, 0.72)",
          },
          ".cm-scroller::-webkit-scrollbar-thumb:hover": {
            background: "linear-gradient(180deg, #9dafc0, #748b9f)",
          },
          ".cm-scroller::-webkit-scrollbar-thumb:active": {
            background: "#748b9f",
          },
          // 纯净的着色标题，禁止任何 padding/margin/fontSize 扰乱物理行高
          ".cm-title-line": {
            color: "var(--color-accent-deep)",
            background: "var(--color-accent-soft)",
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
            if (isRestoringProgrammaticScroll) return;
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
  export function replaceAllContent(n: string) {
    if (!view) return;
    const scrollDOM = view.scrollDOM;
    const scrollTop = scrollDOM.scrollTop;
    const scrollLeft = scrollDOM.scrollLeft;
    const selection = view.state.selection.main;
    const restoreId = ++programmaticScrollRestoreId;
    isRestoringProgrammaticScroll = true;
    view.dispatch({
      changes: { from: 0, to: view.state.doc.length, insert: n },
      selection: {
        anchor: Math.min(selection.anchor, n.length),
        head: Math.min(selection.head, n.length),
      },
    });
    const restoreScroll = () => {
      if (!view || restoreId !== programmaticScrollRestoreId) return;
      const nextScrollDOM = view.scrollDOM;
      const maxTop = Math.max(0, nextScrollDOM.scrollHeight - nextScrollDOM.clientHeight);
      nextScrollDOM.scrollTop = Math.min(scrollTop, maxTop);
      nextScrollDOM.scrollLeft = scrollLeft;
    };
    requestAnimationFrame(() => {
      restoreScroll();
      requestAnimationFrame(() => {
        restoreScroll();
        if (restoreId === programmaticScrollRestoreId) {
          isRestoringProgrammaticScroll = false;
        }
      });
    });
  }
  export function getContent() {
    return view ? view.state.doc.toString() : lastKnownDoc;
  }
  export function getCursorLine() {
    if (!view) return null;
    const line = view.state.doc.lineAt(view.state.selection.main.head);
    return { number: line.number, text: line.text };
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
  export function getLineAtClientPos(clientX: number, clientY: number) {
    if (!view) return null;
    const pos = view.posAtCoords({ x: clientX, y: clientY });
    if (pos == null) return null;
    const line = view.state.doc.lineAt(pos);
    return { number: line.number, text: line.text };
  }
  export function replaceLine(lineNumber: number, text: string) {
    if (!view) return;
    const line = view.state.doc.line(Math.max(1, Math.min(lineNumber, view.state.doc.lines)));
    view.dispatch({ changes: { from: line.from, to: line.to, insert: text } });
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
        width: 550,
        height: 250,
        minWidth: 550,
        minHeight: 250,
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
    <h2>编辑器初始化失败</h2>
    {initError}
  </div>
{/if}

<div
  class="editor-container"
  bind:this={editorElement}
  style="display: {initError ? 'none' : 'flex'}; --editor-font-size: {fontSize}px;"
></div>

<style>
  .editor-container {
    width: 100%;
    height: 100%;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    position: relative;
    display: flex; /* 让 CM6 真正的填满 */
    flex-direction: column;
  }

  /* 确保内部 CM 本体无限填满该区域 */
  :global(.cm-editor) {
    box-sizing: border-box;
    display: flex !important;
    flex-direction: column;
    position: relative;
    height: 100%;
    width: 100%;
    min-width: 0;
    min-height: 0;
    flex: 1;
    background-color: var(--color-surface);
    font-size: var(--editor-font-size);
  }

  /* CodeMirror injects these base layout rules at runtime. In the release
     WebView that injection can fail, so keep the structural rules static. */
  :global(.cm-scroller) {
    display: flex !important;
    align-items: flex-start !important;
    font-family: monospace;
    line-height: 1.4;
    height: 100%;
    overflow-x: hidden;
    overflow-y: scroll;
    position: relative;
    z-index: 0;
    overflow-anchor: none;
    min-width: 0;
  }

  :global(.cm-content) {
    margin: 0;
    flex-grow: 2;
    flex-shrink: 0;
    display: block;
    white-space: pre;
    word-wrap: normal;
    box-sizing: border-box;
    font-family: var(--font-reading);
    line-height: 36px;
    min-height: 100%;
    padding: 4px 0;
    outline: none;
    min-width: 0;
    -webkit-touch-callout: none;
  }

  :global(.cm-content[contenteditable="true"]) {
    -webkit-user-modify: read-write-plaintext-only;
  }

  :global(.cm-lineWrapping) {
    white-space: pre-wrap;
    white-space: break-spaces;
    word-break: break-word;
    overflow-wrap: anywhere;
    flex-shrink: 1;
  }

  :global(.cm-line) {
    display: block;
    padding: 0 2px 0 6px;
    min-height: 36px;
    padding-top: 0;
    padding-bottom: 0;
  }

  :global(.cm-selectionBackground) {
    background-color: rgba(22, 119, 184, 0.34) !important;
  }

  :global(.cm-editor.cm-focused .cm-selectionBackground) {
    background-color: rgba(22, 119, 184, 0.42) !important;
  }

  :global(.cm-layer) {
    position: absolute;
    left: 0;
    top: 0;
    contain: size style;
  }

  :global(.cm-layer > *) {
    position: absolute;
  }

  :global(.cm-cursorLayer) {
    pointer-events: none;
  }

  :global(.cm-cursor),
  :global(.cm-dropCursor) {
    border-left: 1.2px solid black;
    margin-left: -0.6px;
    pointer-events: none;
  }

  :global(.cm-cursor) {
    display: none;
  }

  :global(.cm-dropCursor) {
    position: absolute;
  }

  :global(.cm-editor.cm-focused > .cm-scroller > .cm-cursorLayer) {
    animation: steps(1) cm-blink 1.2s infinite;
  }

  :global(.cm-editor.cm-focused > .cm-scroller > .cm-cursorLayer .cm-cursor) {
    display: block;
  }

  :global(.cm-gutters) {
    flex-shrink: 0;
    display: flex;
    height: 100%;
    box-sizing: border-box;
    z-index: 200;
    background-color: var(--color-surface-soft);
    color: var(--color-muted);
    border-right: 1px solid var(--color-border);
  }

  :global(.cm-gutter) {
    display: flex !important;
    flex-direction: column;
    flex-shrink: 0;
    box-sizing: border-box;
    min-height: 100%;
    overflow: hidden;
  }

  :global(.cm-gutterElement) {
    box-sizing: border-box;
  }

  :global(.cm-lineNumbers .cm-gutterElement) {
    padding: 0 3px 0 5px;
    min-width: 20px;
    text-align: right;
    white-space: nowrap;
  }

  :global(.cm-panels) {
    box-sizing: border-box;
    position: sticky;
    left: 0;
    right: 0;
    z-index: 300;
  }

  :global(.cm-title-line) {
    color: var(--color-accent-deep);
    background: var(--color-accent-soft);
    mix-blend-mode: multiply;
  }

  :global(.cm-scroller::-webkit-scrollbar) {
    width: 14px;
  }

  :global(.cm-scroller::-webkit-scrollbar-track) {
    background: rgba(226, 235, 244, 0.72);
  }

  :global(.cm-scroller::-webkit-scrollbar-thumb) {
    background: linear-gradient(180deg, #bacbda, #93a8bb);
    border-radius: 7px;
    border: 3px solid rgba(226, 235, 244, 0.72);
  }

  :global(.cm-scroller::-webkit-scrollbar-thumb:hover) {
    background: linear-gradient(180deg, #9dafc0, #748b9f);
  }

  :global(.cm-scroller::-webkit-scrollbar-thumb:active) {
    background: #748b9f;
  }

  :global(.cm-tab) {
    display: inline-block;
    overflow: hidden;
    vertical-align: bottom;
  }

  @keyframes cm-blink {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0;
    }
  }
</style>
