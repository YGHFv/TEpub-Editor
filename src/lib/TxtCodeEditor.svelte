<script lang="ts">
    import { onDestroy, onMount } from "svelte";
    import { EditorView, basicSetup } from "codemirror";
    import {
        Compartment,
        EditorState,
        RangeSetBuilder,
        StateEffect,
        StateField,
    } from "@codemirror/state";
    import {
        Decoration,
        keymap,
        drawSelection,
        type DecorationSet,
    } from "@codemirror/view";
    import { indentWithTab } from "@codemirror/commands";
    import { search } from "@codemirror/search";

    export let doc = "";
    export let highlightLineStarts: number[] = [];
    export let onChange: (content: string) => void = () => {};
    export let onInput: () => void = () => {};
    export let onTocLineClick: (lineStart: number) => void = () => {};

    let editorElement: HTMLElement;
    let view: EditorView | null = null;
    let lastKnownDoc = "";
    let lastHighlightKey = "";
    let syncTimer: ReturnType<typeof setTimeout> | null = null;
    const themeCompartment = new Compartment();
    const setHighlightLineStarts = StateEffect.define<number[]>();

    function getCodeMirrorCspNonce() {
        const nonceElement = document.querySelector(
            "style[nonce], script[nonce]",
        ) as HTMLElement | null;
        return nonceElement?.nonce || nonceElement?.getAttribute("nonce") || "";
    }

    const tocHighlightField = StateField.define<DecorationSet>({
        create() {
            return Decoration.none;
        },
        update(value, transaction) {
            let nextValue = value.map(transaction.changes);
            for (const effect of transaction.effects) {
                if (effect.is(setHighlightLineStarts)) {
                    nextValue = buildTocLineDecorations(
                        effect.value,
                        transaction.state,
                    );
                }
            }
            return nextValue;
        },
        provide: (field) => EditorView.decorations.from(field),
    });

    $: if (view && doc !== lastKnownDoc) {
        resetDoc(doc);
    }

    $: if (view) {
        updateHighlightLineStarts(highlightLineStarts);
    }

    onMount(() => {
        lastKnownDoc = doc;
        view = new EditorView({
            state: createEditorState(doc),
            parent: editorElement,
        });
        updateHighlightLineStarts(highlightLineStarts);
    });

    onDestroy(() => {
        if (syncTimer) clearTimeout(syncTimer);
        view?.destroy();
    });

    function createEditorState(initialDoc: string) {
        const cspNonce = getCodeMirrorCspNonce();

        return EditorState.create({
            doc: initialDoc,
            extensions: [
                ...(cspNonce ? [EditorView.cspNonce.of(cspNonce)] : []),
                basicSetup,
                drawSelection(),
                tocHighlightField,
                EditorView.lineWrapping,
                search({ top: false }),
                keymap.of([indentWithTab]),
                EditorView.domEventHandlers({
                    click(event, clickedView) {
                        const position = clickedView.posAtCoords({
                            x: event.clientX,
                            y: event.clientY,
                        });
                        if (position == null) return false;
                        const line = clickedView.state.doc.lineAt(position);
                        if (
                            !isHighlightedLineStart(
                                line.from,
                                clickedView.state,
                            )
                        )
                            return false;
                        onTocLineClick(line.from);
                        return false;
                    },
                }),
                EditorView.theme({
                    // 全部走 design token，深色主题下自动跟随，不再写死浅色底
                    "&": {
                        height: "100%",
                        backgroundColor: "var(--color-surface)",
                        color: "var(--color-text)",
                    },
                    ".cm-content": {
                        fontFamily: "var(--font-reading)",
                        lineHeight: "32px",
                        padding: "18px 22px",
                        color: "var(--color-text)",
                        caretColor: "var(--color-accent)",
                    },
                    ".cm-line": {
                        minHeight: "32px",
                        padding: "0",
                    },
                    ".cm-gutters": {
                        display: "none",
                    },
                    ".cm-scroller": {
                        overflow: "auto",
                        backgroundColor: "var(--color-surface)",
                        color: "var(--color-text)",
                    },
                    ".cm-selectionBackground": {
                        backgroundColor:
                            "color-mix(in srgb, var(--color-accent) 30%, transparent) !important",
                    },
                    "&.cm-focused .cm-selectionBackground": {
                        backgroundColor:
                            "color-mix(in srgb, var(--color-accent) 42%, transparent) !important",
                    },
                    ".cm-selectionMatch": {
                        backgroundColor:
                            "color-mix(in srgb, var(--color-warning) 32%, transparent) !important",
                    },
                    ".cm-toc-title-line": {
                        backgroundColor:
                            "color-mix(in srgb, var(--color-accent) 10%, transparent)",
                        boxShadow: "inset 3px 0 0 var(--color-accent)",
                        cursor: "pointer",
                    },
                }),
                themeCompartment.of(
                    EditorView.theme({
                        "&": { fontSize: "18px" },
                    }),
                ),
                EditorView.updateListener.of((update) => {
                    if (!update.docChanged) return;
                    onInput();
                    if (syncTimer) clearTimeout(syncTimer);
                    syncTimer = setTimeout(() => {
                        syncTimer = null;
                        if (!view) return;
                        lastKnownDoc = view.state.doc.toString();
                        onChange(lastKnownDoc);
                    }, 180);
                }),
            ],
        });
    }

    export function resetDoc(nextDoc: string) {
        if (!view) {
            lastKnownDoc = nextDoc;
            return;
        }
        if (syncTimer) {
            clearTimeout(syncTimer);
            syncTimer = null;
        }
        lastKnownDoc = nextDoc;
        view.setState(createEditorState(nextDoc));
        lastHighlightKey = "";
        updateHighlightLineStarts(highlightLineStarts);
    }

    export function replaceAllContent(nextDoc: string) {
        if (!view) return;
        if (syncTimer) {
            clearTimeout(syncTimer);
            syncTimer = null;
        }
        lastKnownDoc = nextDoc;
        view.dispatch({
            changes: { from: 0, to: view.state.doc.length, insert: nextDoc },
            selection: {
                anchor: Math.min(
                    view.state.selection.main.head,
                    nextDoc.length,
                ),
            },
        });
    }

    export function getContent() {
        return view ? view.state.doc.toString() : lastKnownDoc;
    }

    export function focus() {
        view?.focus();
    }

    export function selectRange(start: number, end: number) {
        if (!view) return;
        const length = view.state.doc.length;
        const from = Math.max(0, Math.min(start, length));
        const to = Math.max(0, Math.min(end, length));
        view.dispatch({
            selection: { anchor: from, head: to },
            effects: EditorView.scrollIntoView(from, {
                y: "center",
                yMargin: 24,
            }),
        });
        view.focus();
    }

    function updateHighlightLineStarts(lineStarts: number[]) {
        if (!view) return;
        const highlightKey = lineStarts.join(",");
        if (highlightKey === lastHighlightKey) return;
        lastHighlightKey = highlightKey;
        view.dispatch({
            effects: setHighlightLineStarts.of(lineStarts),
        });
    }

    function buildTocLineDecorations(lineStarts: number[], state: EditorState) {
        const builder = new RangeSetBuilder<Decoration>();
        const docLength = state.doc.length;
        const linePositions = Array.from(
            new Set(
                lineStarts
                    .filter((position) => Number.isFinite(position))
                    .map((position) => {
                        const safePosition = Math.max(
                            0,
                            Math.min(Math.floor(position), docLength),
                        );
                        return state.doc.lineAt(safePosition).from;
                    }),
            ),
        ).sort((a, b) => a - b);

        for (const position of linePositions) {
            builder.add(
                position,
                position,
                Decoration.line({ class: "cm-toc-title-line" }),
            );
        }
        return builder.finish();
    }

    function isHighlightedLineStart(lineStart: number, state: EditorState) {
        const docLength = state.doc.length;
        return highlightLineStarts.some((position) => {
            if (!Number.isFinite(position)) return false;
            const safePosition = Math.max(
                0,
                Math.min(Math.floor(position), docLength),
            );
            return state.doc.lineAt(safePosition).from === lineStart;
        });
    }
</script>

<div class="txt-code-editor" bind:this={editorElement}></div>

<style>
    .txt-code-editor {
        width: 100%;
        height: 100%;
        min-height: 0;
    }
</style>
