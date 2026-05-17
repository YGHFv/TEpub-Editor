<script lang="ts">
    import { goto } from "$app/navigation";
    import { message } from "@tauri-apps/plugin-dialog";
    import { buildMobileRoute, cacheBrowserFile } from "$lib/mobileFlow";

    interface MobileModule {
        title: string;
        kicker: string;
        description: string;
        accent: string;
        accept: string;
        route: string;
        fallbackExt: string;
    }

    const modules: MobileModule[] = [
        {
            title: "制作 EPUB",
            kicker: "TXT / HTML / MD",
            description: "选择文本后直接进入目录预览和制作流程。",
            accent: "#1677b8",
            accept: ".txt,.md,.html,.htm,.xhtml",
            route: "/mobile/make",
            fallbackExt: "txt",
        },
        {
            title: "解密 EPUB",
            kicker: "伪加密 / 路径修复",
            description: "选择 EPUB 后直接处理，再导出清理副本。",
            accent: "#8a5a16",
            accept: ".epub",
            route: "/mobile/decrypt",
            fallbackExt: "epub",
        },
        {
            title: "编辑 EPUB",
            kicker: "元数据 / 文件结构",
            description: "选择 EPUB 后先进入元数据页，再继续编辑内部文件。",
            accent: "#1f7a5a",
            accept: ".epub",
            route: "/mobile/metadata",
            fallbackExt: "epub",
        },
    ];

    let makeInputEl: HTMLInputElement | null = null;
    let decryptInputEl: HTMLInputElement | null = null;
    let editInputEl: HTMLInputElement | null = null;
    let busy = false;
    let status = "点击功能入口后直接选择文件。";

    function inputFor(route: string) {
        if (route === "/mobile/make") return makeInputEl;
        if (route === "/mobile/decrypt") return decryptInputEl;
        return editInputEl;
    }

    function openPicker(item: MobileModule) {
        if (busy) return;
        inputFor(item.route)?.click();
    }

    async function handlePick(item: MobileModule, event: Event) {
        const input = event.currentTarget as HTMLInputElement;
        const file = input.files?.[0];
        input.value = "";
        if (!file) return;

        try {
            busy = true;
            status = `正在导入 ${file.name}...`;
            const cachedPath = await cacheBrowserFile(file, item.fallbackExt);
            await goto(
                buildMobileRoute(item.route, {
                    path: cachedPath,
                    name: file.name,
                }),
            );
        } catch (err) {
            status = "导入文件失败";
            await message(`导入文件失败：${err}`, { title: item.title, kind: "error" });
        } finally {
            busy = false;
        }
    }
</script>

<svelte:head>
    <title>TEpub Mobile</title>
</svelte:head>

<main class="mobile-home">
    <input bind:this={makeInputEl} class="file-input" type="file" accept={modules[0].accept} on:change={(event) => handlePick(modules[0], event)} />
    <input bind:this={decryptInputEl} class="file-input" type="file" accept={modules[1].accept} on:change={(event) => handlePick(modules[1], event)} />
    <input bind:this={editInputEl} class="file-input" type="file" accept={modules[2].accept} on:change={(event) => handlePick(modules[2], event)} />

    <header class="home-head">
        <p>TEpub Editor Android</p>
        <h1>移动端 EPUB 工具</h1>
        <small>{status}</small>
    </header>

    <nav class="entry-list" aria-label="功能入口">
        {#each modules as item}
            <button class="entry-card" type="button" style={`--accent:${item.accent}`} on:click={() => openPicker(item)} disabled={busy}>
                <span>{item.kicker}</span>
                <strong>{item.title}</strong>
                <small>{item.description}</small>
            </button>
        {/each}
    </nav>
</main>

<style>
    :global(html),
    :global(body) {
        background: #f4f5f8;
    }

    .file-input {
        position: fixed;
        width: 1px;
        height: 1px;
        opacity: 0;
        pointer-events: none;
    }

    .mobile-home {
        min-height: 100vh;
        box-sizing: border-box;
        padding: max(28px, env(safe-area-inset-top)) 18px max(44px, env(safe-area-inset-bottom));
        background: #f4f5f8;
        color: #171b24;
    }

    .home-head {
        padding: 14px 0 22px;
        display: grid;
        gap: 8px;
    }

    .home-head p {
        margin: 0;
        color: #777d89;
        font-size: 13px;
        font-weight: 800;
        letter-spacing: 0;
    }

    .home-head h1 {
        margin: 0;
        font-size: 30px;
        line-height: 1.15;
        letter-spacing: 0;
    }

    .home-head small {
        color: #626a78;
        font-size: 13px;
        line-height: 1.45;
    }

    .entry-list {
        display: grid;
        gap: 12px;
    }

    .entry-card {
        min-height: 118px;
        display: grid;
        align-content: center;
        gap: 8px;
        box-sizing: border-box;
        padding: 18px;
        border: 1px solid rgba(23, 27, 36, 0.08);
        border-left: 5px solid var(--accent);
        border-radius: 8px;
        background: #fff;
        color: inherit;
        text-align: left;
        box-shadow: 0 10px 24px rgba(23, 27, 36, 0.07);
    }

    .entry-card:disabled {
        opacity: 0.7;
    }

    .entry-card span {
        color: var(--accent);
        font-size: 12px;
        font-weight: 900;
    }

    .entry-card strong {
        font-size: 21px;
        line-height: 1.2;
    }

    .entry-card small {
        color: #626a78;
        font-size: 13px;
        line-height: 1.45;
    }

    @media (min-width: 720px) {
        .mobile-home {
            max-width: 760px;
            margin: 0 auto;
            padding-left: 24px;
            padding-right: 24px;
        }

        .entry-list {
            grid-template-columns: repeat(2, minmax(0, 1fr));
        }
    }
</style>
