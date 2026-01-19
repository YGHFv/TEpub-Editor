<script lang="ts">
    export let item: {
        id: string;
        label: string;
        src: string;
        children?: any[];
    };
    export let level = 0;
    export let onSelect: (src: string) => void;

    let expanded = true;

    function toggle(e: Event) {
        e.stopPropagation();
        expanded = !expanded;
    }

    function handleClick() {
        onSelect(item.src);
    }
</script>

<div class="toc-node" data-context-type="toc" data-src={item.src}>
    <!-- svelte-ignore a11y-click-events-have-key-events -->
    <div
        class="node-content"
        style="padding-left: {level * 12 + 12}px"
        on:click={handleClick}
        role="button"
        tabindex="0"
    >
        <span
            class="expand-icon"
            on:click={toggle}
            on:keypress={(e) => e.key === "Enter" && toggle(e)}
            role="button"
            tabindex="0"
            style="visibility: {item.children?.length ? 'visible' : 'hidden'}"
        >
            {expanded ? "▼" : "▶"}
        </span>
        <span class="label" title={item.label}>{item.label}</span>
    </div>

    {#if expanded && item.children}
        {#each item.children as child}
            <svelte:self item={child} level={level + 1} {onSelect} />
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
        background: #f0f0f0;
        color: #2196f3;
    }

    .expand-icon {
        width: 20px;
        height: 20px;
        display: flex;
        align-items: center;
        justify-content: center;
        font-size: 10px;
        color: #999;
        margin-right: 4px;
        cursor: pointer;
    }

    .expand-icon:hover {
        color: #666;
    }

    .label {
        font-size: 15px;
        color: #333;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
        flex: 1;
    }

    .node-content:hover .label {
        color: #2196f3;
    }
</style>
