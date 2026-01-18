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

    let editorElement: HTMLElement;
    let view: EditorView;
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
                        const newContent = update.state.doc.toString();
                        lastKnownDoc = newContent;
                        onChange(newContent);
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
