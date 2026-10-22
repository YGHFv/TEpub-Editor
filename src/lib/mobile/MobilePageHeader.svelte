<script lang="ts">
    import { t } from "$lib/i18n.svelte";

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

<header class="m-page-header" aria-label={t("页面导航")}>
    <button
        class="m-page-back"
        type="button"
        aria-label={t("返回")}
        title={t("返回")}
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
