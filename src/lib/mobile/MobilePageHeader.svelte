<script lang="ts">
    let {
        title,
        subtitle = "",
        onback,
    }: {
        title: string;
        subtitle?: string;
        onback: () => void;
    } = $props();
</script>

<header class="m-page-header" aria-label="页面导航">
    <button
        class="m-page-back"
        type="button"
        aria-label="返回"
        title="返回"
        onclick={onback}
    >
        <svg viewBox="0 0 24 24" aria-hidden="true">
            <path d="M15 5 8 12l7 7"></path>
        </svg>
    </button>
    <div class="m-page-heading">
        <div class="m-page-title">{title}</div>
        {#if subtitle}
            <div class="m-page-subtitle">{subtitle}</div>
        {/if}
    </div>
</header>
