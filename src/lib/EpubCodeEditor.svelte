<script lang="ts">
    import { onMount, onDestroy } from "svelte";
    import { EditorView, basicSetup } from "codemirror";
    import { EditorState, Compartment } from "@codemirror/state";
    import { keymap, drawSelection } from "@codemirror/view";
    import { indentWithTab } from "@codemirror/commands";
    import { search } from "@codemirror/search";
    import { html } from "@codemirror/lang-html";
    import { css } from "@codemirror/lang-css";
    import { xml } from "@codemirror/lang-xml";

    export let doc = "";
    export let language: "html" | "css" | "xml" | "other" = "other";
    export let onChange: (content: string) => void = () => {};
    export let onSave: () => void = () => {};
    export let onScroll: (event: Event) => void = () => {};
    export let onClick: (line: number) => void = () => {};
    export let onSelectionChange: (text: string) => void = () => {};

    let editorElement: HTMLElement;
    let view: EditorView;
    let compartment = new Compartment();
    let skipSelectionEmit = false;

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

    // 内部状态跟踪
    let lastKnownDoc = "";

    // 监听外部 doc 变化 - 使用 setState 重置状态以清除撤销历史
    $: if (view && doc !== lastKnownDoc) {
        const stateDoc = view.state.doc;
        if (doc.length !== stateDoc.length || doc !== stateDoc.toString()) {
            // 完全重置编辑器状态，清除撤销历史
            lastKnownDoc = doc;
            view.setState(createEditorState(doc));
        }
    }

    onMount(() => {
        const savedSize = localStorage.getItem("epub-editor-font-size");
        if (savedSize) fontSize = parseInt(savedSize);

        lastKnownDoc = doc;
        const state = createEditorState(doc);
        view = new EditorView({
            state,
            parent: editorElement,
        });
    });

    onDestroy(() => view?.destroy());

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

    function createEditorState(initialDoc: string) {
        return EditorState.create({
            doc: initialDoc,
            extensions: [
                basicSetup,
                drawSelection(),
                EditorView.lineWrapping,
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
                }),
                EditorView.theme({
                    "&": {
                        height: "100%",
                        backgroundColor: "#fff",
                    },
                    ".cm-content": {
                        fontFamily:
                            '"Consolas", "Monaco", "Courier New", monospace',
                        lineHeight: "1.6",
                    },
                    ".cm-gutters": {
                        backgroundColor: "#f8f8f8",
                        color: "#858585",
                        borderRight: "1px solid #e0e0e0",
                    },
                    ".cm-scroller": {
                        overflow: "auto",
                    },
                    ".cm-scroller::-webkit-scrollbar": {
                        width: "8px",
                        height: "8px",
                    },
                    ".cm-scroller::-webkit-scrollbar-track": {
                        background: "#f1f1f1",
                    },
                    ".cm-scroller::-webkit-scrollbar-thumb": {
                        background: "#888",
                        borderRadius: "4px",
                    },
                    ".cm-scroller::-webkit-scrollbar-thumb:hover": {
                        background: "#555",
                    },
                    // 选中高亮样式 - 明亮的黄色背景
                    ".cm-selectionBackground": {
                        backgroundColor: "#ffeb3b !important",
                    },
                    "&.cm-focused .cm-selectionBackground": {
                        backgroundColor: "#ffeb3b !important",
                    },
                    ".cm-selectionMatch": {
                        backgroundColor: "#b3e5fc !important",
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
                    if (update.docChanged) {
                        onChange(update.state.doc.toString());
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
        localStorage.setItem("epub-editor-font-size", fontSize.toString());
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
