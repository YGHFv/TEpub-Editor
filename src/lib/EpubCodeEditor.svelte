<script lang="ts">
    import { onMount, onDestroy } from "svelte";
    import { EditorView, basicSetup } from "codemirror";
    import { EditorState, Compartment } from "@codemirror/state";
    import { keymap, drawSelection } from "@codemirror/view";
    import { indentWithTab, undo, redo } from "@codemirror/commands";
    import { search } from "@codemirror/search";
    import { html } from "@codemirror/lang-html";
    import { css } from "@codemirror/lang-css";
    import { xml } from "@codemirror/lang-xml";
    import { EPUB_EDITOR_FONT_SIZE_KEY } from "./storageKeys";
    import { onlyWhenActive } from "./keepAlive/viewState";

    export let doc = "";
    export let language: "html" | "css" | "xml" | "other" = "other";
    export let onChange: (content: string) => void = () => {};
    export let onSave: () => void = () => {};
    export let onScroll: (event: Event) => void = () => {};
    export let onClick: (line: number) => void = () => {};
    export let onSelectionChange: (text: string) => void = () => {};
    /**
     * 光标所在行（1 起）变化时回调：打字、方向键、输入法都会触发。
     * 手机动作条上的「行 N」靠它实时刷新 —— 只在点击/切栏时同步的话，
     * 打了几十行之后按钮还写着进来时的行号。
     */
    export let onCursorLine: (line: number) => void = () => {};
    export let onOpenSearch: () => void = () => {};
    /**
     * 触摸输入模式：关掉移动端键盘的自动大写 / 自动纠错 / 拼写波浪线。
     *
     * 手机打 XHTML 时这三个默认全开 会把 <div class="x"> 变成 <Div class="x">
     * 并给每个标签画红波浪线 —— 代码编辑器里它们全是负作用。
     * 桌面不传 = 不写这些属性，行为与原来完全一致。
     */
    export let touchInput = false;

    let editorElement: HTMLElement;
    let view: EditorView;
    let compartment = new Compartment();
    let skipSelectionEmit = false;

    function getCodeMirrorCspNonce() {
        const nonceElement = document.querySelector(
            "style[nonce], script[nonce]",
        ) as HTMLElement | null;
        return nonceElement?.nonce || nonceElement?.getAttribute("nonce") || "";
    }

    export function selectText(text: string) {
        if (!view || !text) return;
        const doc = view.state.doc.toString();
        // 简单的首次匹配。如果有重复文本，可能会跳到错误位置。
        // 改进方向：如果支持，可以传递大概位置索引进行搜索。
        const index = doc.indexOf(text);
        if (index !== -1) {
            skipSelectionEmit = true;
            view.dispatch({
                selection: { anchor: index, head: index + text.length },
                effects: EditorView.scrollIntoView(index, { y: "center" }),
            });
            setTimeout(() => {
                skipSelectionEmit = false;
            }, 50);
        }
    }

    export function selectTextWithContext(text: string, context: string) {
        if (!view || !text) return;

        // 如果没有 context，直接使用普通搜索
        if (!context || context.length < 3) {
            selectText(text);
            return;
        }

        const doc = view.state.doc;
        const normalize = (str: string) =>
            str.replace(/\s+/g, " ").trim().toLowerCase();
        const cleanContext = normalize(context);
        const cleanText = normalize(text);

        let bestLine = -1;
        let bestScore = 0;

        // 遍历所有行寻找最佳匹配的上下文
        for (let i = 1; i <= doc.lines; i++) {
            const line = doc.line(i);
            const lineHtml = line.text;
            // 剥离 HTML 标签获取纯文本
            const lineText = lineHtml.replace(/<[^>]+>/g, "");
            const cleanLine = normalize(lineText);

            // 跳过空行或太短的行
            if (cleanLine.length < 2) continue;

            // 计算相似度：检查 context 的片段是否在 line 中出现
            // 使用子串匹配：context 包含 line 或 line 包含 context 的一部分
            let score = 0;

            // 完全包含得高分
            if (
                cleanLine.includes(cleanContext) ||
                cleanContext.includes(cleanLine)
            ) {
                score = 100;
            } else {
                // 检查 context 开头20字符是否出现在 line 中
                const contextStart = cleanContext.substring(
                    0,
                    Math.min(20, cleanContext.length),
                );
                if (cleanLine.includes(contextStart)) {
                    score = 80;
                } else if (cleanLine.includes(cleanText)) {
                    // 如果行包含目标文本，也给分
                    score = 50;
                }
            }

            if (score > bestScore) {
                bestScore = score;
                bestLine = i;
                if (score === 100) break; // 完美匹配，停止搜索
            }
        }

        if (bestLine !== -1 && bestScore >= 50) {
            const line = doc.line(bestLine);
            // 在该行内查找 text
            const lineStart = line.from;
            const lineText = line.text;
            const indexInLine = lineText.indexOf(text);

            if (indexInLine !== -1) {
                const absIndex = lineStart + indexInLine;
                skipSelectionEmit = true;
                view.dispatch({
                    selection: {
                        anchor: absIndex,
                        head: absIndex + text.length,
                    },
                    effects: EditorView.scrollIntoView(absIndex, {
                        y: "center",
                    }),
                });
                setTimeout(() => {
                    skipSelectionEmit = false;
                }, 100);
                return;
            } else {
                // 行找到了但文本没找到，滚动到行
                skipSelectionEmit = true;
                view.dispatch({
                    selection: { anchor: lineStart },
                    effects: EditorView.scrollIntoView(lineStart, {
                        y: "center",
                    }),
                });
                setTimeout(() => {
                    skipSelectionEmit = false;
                }, 100);
                return;
            }
        }

        // 如果上下文匹配失败，回退到普通搜索
        selectText(text);
    }
    let fontSize = 14;
    const themeCompartment = new Compartment();
    /** 自动换行开关。触摸屏上横向滚动很难用，手机默认开、可手动关 */
    let lineWrapping = true;
    const wrapCompartment = new Compartment();
    /** 触摸输入属性（autocapitalize 等），按 touchInput 切换 */
    const inputCompartment = new Compartment();

    // 内部状态跟踪
    let lastKnownDoc = "";

    // 监听外部 doc 变化 - 使用 setState 重置状态以清除撤销历史
    function applyExternalDoc() {
        if (!view || view.composing || doc === lastKnownDoc) return;
        const stateDoc = view.state.doc;
        if (doc.length !== stateDoc.length || doc !== stateDoc.toString()) {
            // 完全重置编辑器状态，清除撤销历史
            lastKnownDoc = doc;
            view.setState(createEditorState(doc));
        }
    }

    // 拼字期间不能 setState，否则会打断拼音/五笔候选。此时先记住外部 doc
    // 已经变了，等 composition 结束再补同步。只在 $: 里判 view.composing
    // 不够：它会跳过，但 composing 结束不是 Svelte 响应式状态，doc 又没再
    // 变时不会重跑，最后编辑区会一直停在旧文件上。
    $: if (view && doc !== lastKnownDoc) applyExternalDoc();

    let wasComposing = false;
    function syncAfterComposition() {
        queueMicrotask(() => {
            if (view && !view.composing) applyExternalDoc();
        });
    }

    let blockNativeSearch: (e: KeyboardEvent) => void;
    // 保活：隐藏页面里的代码编辑器也会收到 window 级 keydown，不能抢 Ctrl+F
    const activeOnly = onlyWhenActive();

    onMount(() => {
        blockNativeSearch = (e: KeyboardEvent) => {
            if (!activeOnly()) return;
            if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "f") {
                e.preventDefault();
                e.stopPropagation();
                onOpenSearch();
            } else if (e.key === "F3") {
                e.preventDefault();
                e.stopPropagation();
                onOpenSearch();
            }
        };
        window.addEventListener("keydown", blockNativeSearch, true);

        const savedSize = localStorage.getItem(EPUB_EDITOR_FONT_SIZE_KEY);
        if (savedSize) fontSize = parseInt(savedSize);

        lastKnownDoc = doc;
        const state = createEditorState(doc);
        view = new EditorView({
            state,
            parent: editorElement,
        });
    });

    onDestroy(() => {
        if (blockNativeSearch) {
            window.removeEventListener("keydown", blockNativeSearch, true);
        }
        view?.destroy();
    });

    function getLanguageExtension() {
        switch (language) {
            case "html":
                return html();
            case "css":
                return css();
            case "xml":
                return xml();
            default:
                return [];
        }
    }

    /**
     * 触摸输入相关的 DOM 属性。
     *
     * CodeMirror 把 contenteditable 建在 .cm-content 上，属性必须写到那个
     * 节点上，写在宿主 div 上不生效 —— 所以走一个 EditorView 扩展。
     * touchInput 为 false 时返回空数组，桌面拿到的 extension 列表与以前一致。
     */
    function touchInputExtensions(enabled: boolean) {
        if (!enabled) return [];
        return [
            EditorView.contentAttributes.of({
                autocapitalize: "off",
                autocorrect: "off",
                autocomplete: "off",
                spellcheck: "false",
            }),
        ];
    }

    function createEditorState(initialDoc: string) {
        const cspNonce = getCodeMirrorCspNonce();

        return EditorState.create({
            doc: initialDoc,
            extensions: [
                ...(cspNonce ? [EditorView.cspNonce.of(cspNonce)] : []),
                basicSetup,
                drawSelection(),
                wrapCompartment.of(lineWrapping ? EditorView.lineWrapping : []),
                inputCompartment.of(touchInputExtensions(touchInput)),
                search({ top: false }),
                keymap.of([
                    indentWithTab,
                    // Ctrl+S 保存
                    {
                        key: "Mod-s",
                        run: () => {
                            onSave();
                            return true;
                        },
                    },
                ]),
                getLanguageExtension(),
                EditorView.domEventHandlers({
                    scroll: (event, view) => {
                        onScroll(event);
                    },
                    click: (event, view) => {
                        const pos = view.posAtDOM(event.target as Node);
                        if (pos !== null) {
                            const line = view.state.doc.lineAt(pos);
                            onClick(line.number);
                        }
                    },
                    compositionend: syncAfterComposition,
                }),
                EditorView.theme({
                    "&": {
                        height: "100%",
                        backgroundColor: "var(--color-surface)",
                    },
                    ".cm-content": {
                        fontFamily: "var(--font-code)",
                        lineHeight: "1.6",
                    },
                    ".cm-gutters": {
                        backgroundColor: "var(--color-surface-soft)",
                        color: "var(--color-muted)",
                        borderRight: "1px solid var(--color-border)",
                    },
                    ".cm-scroller": {
                        overflow: "auto",
                    },
                    ".cm-scroller::-webkit-scrollbar": {
                        width: "8px",
                        height: "8px",
                    },
                    ".cm-scroller::-webkit-scrollbar-track": {
                        background: "rgba(226, 235, 244, 0.72)",
                    },
                    ".cm-scroller::-webkit-scrollbar-thumb": {
                        background: "linear-gradient(180deg, #bacbda, #93a8bb)",
                        borderRadius: "999px",
                    },
                    ".cm-scroller::-webkit-scrollbar-thumb:hover": {
                        background: "linear-gradient(180deg, #9dafc0, #748b9f)",
                    },
                    // 选中高亮样式 - 明亮的黄色背景
                    ".cm-selectionBackground": {
                        backgroundColor: "rgba(22, 119, 184, 0.28) !important",
                    },
                    "&.cm-focused .cm-selectionBackground": {
                        backgroundColor: "rgba(22, 119, 184, 0.36) !important",
                    },
                    ".cm-selectionMatch": {
                        backgroundColor: "rgba(233, 155, 47, 0.28) !important",
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
                    // CM 自己维护 composing（DOM event 或失焦取消都会复位）。
                    // 有些 WebView 不派发可用的 compositionend，用 update 的
                    // false 跳变作为第二条补偿路径。
                    if (wasComposing && !update.view.composing) {
                        syncAfterComposition();
                    }
                    wasComposing = update.view.composing;
                    if (update.docChanged) {
                        onChange(update.state.doc.toString());
                    }
                    if (update.docChanged || update.selectionSet) {
                        const head = update.state.selection.main.head;
                        onCursorLine(update.state.doc.lineAt(head).number);
                    }
                    if (update.selectionSet) {
                        // 防止程序设置选区时触发循环同步
                        if (skipSelectionEmit) return;

                        const selection = update.state.selection.main;
                        if (!selection.empty) {
                            const text = update.state.sliceDoc(
                                selection.from,
                                selection.to,
                            );
                            onSelectionChange(text);
                        }
                    }
                }),
            ],
        });
    }

    // 重置文档并清除撤销历史
    export function resetDoc(newDoc: string) {
        if (!view) return;
        lastKnownDoc = newDoc;
        view.setState(createEditorState(newDoc));
    }

    function handleWheel(e: WheelEvent) {
        if (e.ctrlKey) {
            e.preventDefault();
            updateFontSize(e.deltaY < 0 ? 1 : -1);
        }
    }

    function updateFontSize(delta: number) {
        fontSize = Math.max(12, Math.min(fontSize + delta, 24));
        localStorage.setItem(EPUB_EDITOR_FONT_SIZE_KEY, fontSize.toString());
        view.dispatch({
            effects: themeCompartment.reconfigure(
                EditorView.theme({ "&": { fontSize: `${fontSize}px` } }),
            ),
        });
    }

    export function selectAll() {
        if (!view) return;
        view.dispatch({
            selection: { anchor: 0, head: view.state.doc.length },
        });
        view.focus();
    }

    // ——— 手机动作条用的命令式接口 ———
    //
    // 手机上按不到 Ctrl+Z / Ctrl+F，只能给按钮。全部包成函数而不是让页面去
    // 摸 getView() 的内部结构，是因为 view 可能是 undefined（还没 onMount），
    // 而且 CM 的 dispatch 失败在手机上会表现为"点了没反应"，很难排查。

    export function undoEdit() {
        if (!view) return;
        undo(view);
        view.focus();
    }

    export function redoEdit() {
        if (!view) return;
        redo(view);
        view.focus();
    }

    /**
     * touchInput 是 prop，可能在建 state 之后才变 —— 桌面 web 窗口被拖过
     * 760px 断点时 phoneLayout 翻面。只在 createEditorState 里读一次的话，
     * compartment 永远停在旧值，软键盘照样自动大写、把 <div> 变成 <Div>。
     */
    $: if (view) {
        view.dispatch({
            effects: inputCompartment.reconfigure(
                touchInputExtensions(touchInput),
            ),
        });
    }

    export function toggleLineWrap() {
        lineWrapping = !lineWrapping;
        view?.dispatch({
            effects: wrapCompartment.reconfigure(
                lineWrapping ? EditorView.lineWrapping : [],
            ),
        });
        return lineWrapping;
    }

    export function isLineWrapped() {
        return lineWrapping;
    }

    /** 把光标移到指定行（1 起）并滚到视野中央 */
    export function gotoLine(lineNumber: number) {
        if (!view) return;
        const total = view.state.doc.lines;
        const line = Math.max(1, Math.min(Math.floor(lineNumber) || 1, total));
        const info = view.state.doc.line(line);
        view.dispatch({
            selection: { anchor: info.from },
            effects: EditorView.scrollIntoView(info.from, { y: "center" }),
        });
        view.focus();
    }

    /** 当前光标所在行号（1 起），没挂载时返回 0 */
    export function currentLine() {
        if (!view) return 0;
        const head = view.state.selection.main.head;
        return view.state.doc.lineAt(head).number;
    }

    /** 在当前光标处插入文本；符号栏用它把标签/引号打进去 */
    export function insertText(text: string) {
        if (!view || !text) return;
        const { from, to } = view.state.selection.main;
        view.dispatch({
            changes: { from, to, insert: text },
            // 插入后把光标放到插入内容之后，连续点符号才能接着打
            selection: { anchor: from + text.length },
        });
        view.focus();
    }

    export function setFontSize(size: number) {
        const next = Math.max(12, Math.min(Math.round(size), 24));
        if (next === fontSize) return;
        updateFontSize(next - fontSize);
    }

    export function getFontSize() {
        return fontSize;
    }

    /** 中文输入法正在拼字（转发 CM 自己的状态，见 $: doc 同步处的说明） */
    export function isComposing() {
        return view?.composing ?? false;
    }

    // 获取 CodeMirror view 实例供外部使用
    export function getView(): EditorView | null {
        return view || null;
    }

    export function scrollToRatio(ratio: number) {
        if (!view) return;
        const scrollHeight = view.scrollDOM.scrollHeight;
        const clientHeight = view.scrollDOM.clientHeight;
        const maxScroll = scrollHeight - clientHeight;
        view.scrollDOM.scrollTo({ top: maxScroll * ratio, behavior: "smooth" });
    }
</script>

<div
    class="epub-code-editor"
    bind:this={editorElement}
    on:wheel={handleWheel}
></div>

<style>
    .epub-code-editor {
        width: 100%;
        height: 100%;
    }
</style>
