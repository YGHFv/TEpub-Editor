<script lang="ts">
    import Icon from "$lib/icons/Icon.svelte";

    export let item: {
        id: string;
        label: string;
        src: string;
        children?: any[];
    };
    export let level = 0;
    export let selectedSrc = "";
    export let onSelect: (src: string) => void;

    let expanded = true;
    let childRefs: any[] = [];

    function toggle(e: Event) {
        e.stopPropagation();
        expanded = !expanded;
    }

    function handleClick() {
        onSelect(item.src);
    }

    // 折叠本节点
    export function collapseThis() {
        expanded = false;
    }

    // 递归折叠所有子节点
    export function collapseAll() {
        expanded = false;
        childRefs.forEach((child) => {
            if (child && typeof child.collapseAll === "function") {
                child.collapseAll();
            }
        });
    }
</script>

<div
    class="toc-node"
    data-context-type="toc"
    data-src={item.src}
    data-id={item.id}
    data-has-children={item.children && item.children.length > 0
        ? "true"
        : "false"}
>
    <!-- svelte-ignore a11y-click-events-have-key-events -->
    <div
        class="node-content"
        class:selected={!!selectedSrc && selectedSrc === item.src}
        style="padding-left: {level * 12 + 12}px"
        on:click={handleClick}
        role="button"
        tabindex="0"
    >
        <!-- 无子节点时用 class 隐藏占位，不要写内联 visibility:visible：
             内联声明会盖过祖先的 visibility:hidden，目录层切走后
             这个箭头会单独留在预览区上 -->
        <span
            class="expand-icon"
            class:is-empty={!item.children?.length}
            on:click={toggle}
            on:keypress={(e) => e.key === "Enter" && toggle(e)}
            role="button"
            tabindex="0"
        >
            <Icon
                name={expanded ? "chevron-down" : "chevron-right"}
                size={12}
            />
        </span>
        <span class="label" title={item.label}>{item.label}</span>
    </div>

    {#if expanded && item.children}
        {#each item.children as child, i}
            <svelte:self
                bind:this={childRefs[i]}
                item={child}
                level={level + 1}
                {selectedSrc}
                {onSelect}
            />
        {/each}
    {/if}
</div>

<style>
    .node-content {
        display: flex;
        align-items: center;
        padding: 6px 8px 6px 0;
        cursor: pointer;
        user-select: none;
        transition: background 0.2s;
    }

    .node-content:hover {
        background: var(--color-hover);
        color: var(--color-accent);
    }

    .expand-icon {
        width: 20px;
        height: 20px;
        display: flex;
        align-items: center;
        justify-content: center;
        color: var(--color-muted);
        margin-right: 4px;
        cursor: pointer;
        flex-shrink: 0; /* 防止被压缩 */
        position: relative; /* 创建独立的层叠上下文 */
        z-index: 1; /* 确保在父元素上方 */
    }

    .expand-icon:hover {
        color: var(--color-muted);
    }

    /* 叶子节点不显示箭头，但要保留 20px 占位让标签左对齐。
       用 visibility 而不是 display:none —— 占位宽度不能丢 */
    .expand-icon.is-empty {
        visibility: hidden;
    }

    .label {
        font-size: 15px;
        color: var(--color-text);
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
        flex: 1;
    }

    .node-content:hover .label {
        color: var(--color-accent);
    }

    /* Modern UI overrides */
    .node-content {
        margin: 2px 8px;
        padding-top: 7px;
        padding-bottom: 7px;
        border-radius: var(--radius-sm);
        color: var(--color-text-soft);
        transition:
            background var(--transition-fast),
            color var(--transition-fast),
            box-shadow var(--transition-fast);
    }

    .node-content:hover {
        background: var(--color-hover);
        color: var(--color-accent-deep);
        box-shadow: inset 3px 0 0 rgba(22, 119, 184, 0.24);
    }

    .expand-icon {
        color: var(--color-muted);
    }

    .expand-icon:hover {
        color: var(--color-accent-deep);
    }

    .label {
        color: inherit;
        font-weight: 650;
    }

    .node-content:hover .label {
        color: var(--color-accent-deep);
    }

    .node-content.selected {
        background: var(--color-accent-soft);
        color: var(--color-accent-deep);
        box-shadow: inset 3px 0 0 var(--color-accent);
    }
</style>
