<script lang="ts">
    import { goto } from "$app/navigation";
    import { message } from "@tauri-apps/plugin-dialog";
    import { buildMobileRoute, cacheBrowserFileStable } from "$lib/mobileFlow";

    interface MobileModule {
        title: string;
        kicker: string;
        description: string;
        accent: string;
        icon: "make" | "decrypt" | "edit";
        accept: string;
        route: string;
        fallbackExt: string;
    }

    type MobileTab = "home" | "config" | "about";

    const appVersion = "0.5.9";

    const modules: MobileModule[] = [
        {
            title: "制作 EPUB",
            kicker: "TXT / HTML / MD",
            description: "选择文本后进入目录预览、正则调整和制作流程。",
            accent: "#1677b8",
            icon: "make",
            accept: ".txt,.md,.html,.htm,.xhtml",
            route: "/mobile/make",
            fallbackExt: "txt",
        },
        {
            title: "解密 EPUB",
            kicker: "伪加密 / 路径修复",
            description: "选择 EPUB 后清理异常结构，再导出干净副本。",
            accent: "#bd7b1f",
            icon: "decrypt",
            accept: ".epub",
            route: "/mobile/decrypt",
            fallbackExt: "epub",
        },
        {
            title: "编辑 EPUB",
            kicker: "元数据 / 文件结构",
            description: "先编辑书名、封面等元数据，再进入内部文件结构。",
            accent: "#1f8a68",
            icon: "edit",
            accept: ".epub",
            route: "/mobile/metadata",
            fallbackExt: "epub",
        },
    ];

    const aboutLinks = [
        {
            title: "鸣谢人员",
            description: "查看参与项目开发、测试与支持的人员名单",
            icon: "flower",
        },
        {
            title: "GitHub 仓库",
            description: "查看源码、问题反馈与发布版本",
            icon: "link",
            href: "https://github.com/YGHFv/TEpub-Editor",
        },
        {
            title: "文档",
            description: "使用教程和疑难解惑",
            icon: "link",
            href: "https://github.com/YGHFv/TEpub-Editor",
        },
        {
            title: "爱发电赞助",
            description: "支持后续开发和维护",
            icon: "link",
        },
    ];

    let makeInputEl: HTMLInputElement | null = null;
    let decryptInputEl: HTMLInputElement | null = null;
    let editInputEl: HTMLInputElement | null = null;
    let busy = false;
    let pickToken = 0;
    let activeTab: MobileTab = "home";
    let status = "点击功能入口后直接选择文件。";

    $: pageTitle = activeTab === "home" ? "TEpub Editor" : activeTab === "config" ? "配置" : "";
    $: showHeader = activeTab !== "about";
    $: pageSubtitle = activeTab === "home" ? status : "";

    function inputFor(route: string) {
        if (route === "/mobile/make") return makeInputEl;
        if (route === "/mobile/decrypt") return decryptInputEl;
        return editInputEl;
    }

    function openPicker(item: MobileModule) {
        if (busy) return;
        inputFor(item.route)?.click();
    }

    function setTab(tab: MobileTab) {
        activeTab = tab;
    }

    function openExternal(href?: string) {
        if (!href) return;
        window.open(href, "_blank", "noopener,noreferrer");
    }

    async function handlePick(item: MobileModule, event: Event) {
        const input = event.currentTarget as HTMLInputElement;
        const file = input.files?.[0];
        input.value = "";
        if (!file) return;

        const token = ++pickToken;
        try {
            busy = true;
            status = `正在导入 ${file.name}...`;
            const cachedPath = await cacheBrowserFileStable(file, item.fallbackExt);
            if (token !== pickToken) return;
            busy = false;
            await goto(
                buildMobileRoute(item.route, {
                    path: cachedPath,
                    name: file.name,
                }),
            );
        } catch (err) {
            if (token !== pickToken) return;
            status = "导入文件失败";
            await message(`导入文件失败：${err}`, { title: item.title, kind: "error" });
        } finally {
            if (token === pickToken) busy = false;
        }
    }
</script>

<svelte:head>
    <title>TEpub Mobile</title>
</svelte:head>

<main class="mobile-shell">
    <input bind:this={makeInputEl} class="file-input" type="file" accept={modules[0].accept} on:change={(event) => handlePick(modules[0], event)} />
    <input bind:this={decryptInputEl} class="file-input" type="file" accept={modules[1].accept} on:change={(event) => handlePick(modules[1], event)} />
    <input bind:this={editInputEl} class="file-input" type="file" accept={modules[2].accept} on:change={(event) => handlePick(modules[2], event)} />

    <section class:about-screen={activeTab === "about"} class="screen">
        {#if showHeader}
            <header class="page-head">
                <h1>{pageTitle}</h1>
                {#if pageSubtitle}<small>{pageSubtitle}</small>{/if}
            </header>
        {/if}

        {#if activeTab === "home"}
            <nav class="entry-list" aria-label="功能入口">
                {#each modules as item}
                    <button
                        class="entry-card"
                        type="button"
                        style={`--accent:${item.accent}`}
                        on:click={() => openPicker(item)}
                        disabled={busy}
                    >
                        <span class="entry-icon" aria-hidden="true">
                            {#if item.icon === "make"}
                                <svg viewBox="0 0 24 24"><path d="M6 4h8l4 4v12H6z"></path><path d="M14 4v5h5"></path><path d="M9 13h6M9 16h4"></path></svg>
                            {:else if item.icon === "decrypt"}
                                <svg viewBox="0 0 24 24"><path d="M7 11V8a5 5 0 0 1 10 0v3"></path><path d="M6 11h12v9H6z"></path><path d="M12 15v2"></path></svg>
                            {:else}
                                <svg viewBox="0 0 24 24"><path d="M5 5h14v14H5z"></path><path d="M9 9h6M9 12h6M9 15h3"></path></svg>
                            {/if}
                        </span>
                        <span>{item.kicker}</span>
                        <strong>{item.title}</strong>
                        <small>{item.description}</small>
                    </button>
                {/each}
            </nav>
        {:else if activeTab === "config"}
            <section class="config-page">
                <div class="settings-card hero-setting">
                    <div>
                        <span>界面风格</span>
                        <strong>澎湃新 UI</strong>
                        <small>入口页使用大圆角、半透明卡片和悬浮底栏；功能流程保持原有稳定逻辑。</small>
                    </div>
                    <span class="setting-orb" aria-hidden="true"></span>
                </div>
            </section>
        {:else}
            <section class="about-page">
                <div class="brand-block">
                    <div class="app-icon" aria-hidden="true">
                        <svg viewBox="0 0 64 64">
                            <path d="M15 41c7-20 21-26 35-23-1 16-10 29-31 31"></path>
                            <path d="M17 46c9-7 17-14 31-32"></path>
                            <circle cx="32" cy="32" r="20"></circle>
                        </svg>
                    </div>
                    <h2>TEpub-Editor</h2>
                    <p>{appVersion}-mobile-release</p>
                </div>

                <div class="about-list">
                    {#each aboutLinks as item}
                        <button class="about-card" type="button" on:click={() => openExternal(item.href)}>
                            <span>
                                <strong>{item.title}</strong>
                                <small>{item.description}</small>
                            </span>
                            <i aria-hidden="true">
                                {#if item.icon === "flower"}
                                    <svg viewBox="0 0 24 24"><path d="M12 21v-7"></path><path d="M8 13c-3 0-5-2-5-5 3 0 5 1 6 3"></path><path d="M16 13c3 0 5-2 5-5-3 0-5 1-6 3"></path><path d="M12 14c-3-2-4-5-2-8 2 1 3 2 4 4 1-2 2-3 4-4 2 3 1 6-2 8"></path></svg>
                                {:else}
                                    <svg viewBox="0 0 24 24"><path d="M10 13a5 5 0 0 0 7 0l2-2a5 5 0 0 0-7-7l-1 1"></path><path d="M14 11a5 5 0 0 0-7 0l-2 2a5 5 0 0 0 7 7l1-1"></path></svg>
                                {/if}
                            </i>
                        </button>
                    {/each}
                </div>
            </section>
        {/if}
    </section>

    <nav class="bottom-tabs" aria-label="底部导航">
        <button class:active={activeTab === "home"} type="button" on:click={() => setTab("home")}>
            <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 11.5 12 4l8 7.5V20h-5v-6H9v6H4z"></path></svg>
            <span>主页</span>
        </button>
        <button class:active={activeTab === "config"} type="button" on:click={() => setTab("config")}>
            <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 8a4 4 0 1 1 0 8 4 4 0 0 1 0-8z"></path><path d="M4 12h2m12 0h2M12 4v2m0 12v2M6.6 6.6 8 8m8 8 1.4 1.4M17.4 6.6 16 8m-8 8-1.4 1.4"></path></svg>
            <span>配置</span>
        </button>
        <button class:active={activeTab === "about"} type="button" on:click={() => setTab("about")}>
            <svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="9"></circle><path d="M12 10v7"></path><path d="M12 7h.01"></path></svg>
            <span>关于</span>
        </button>
    </nav>
</main>

<style>
    :global(html),
    :global(body) {
        background: #eef2ff;
    }

    .file-input {
        position: fixed;
        width: 1px;
        height: 1px;
        opacity: 0;
        pointer-events: none;
    }

    .mobile-shell {
        --text: #171b24;
        --muted: #626a78;
        --card: rgba(255, 255, 255, 0.74);
        --line: rgba(255, 255, 255, 0.58);
        min-height: 100vh;
        box-sizing: border-box;
        color: var(--text);
        background:
            radial-gradient(circle at 12% 10%, rgba(214, 225, 255, 0.96), transparent 32%),
            radial-gradient(circle at 18% 78%, rgba(255, 210, 224, 0.9), transparent 28%),
            linear-gradient(135deg, #eef2ff 0%, #fff5fb 48%, #e7efff 100%);
    }

    .screen {
        min-height: 100vh;
        box-sizing: border-box;
        max-width: 780px;
        margin: 0 auto;
        padding: max(46px, calc(env(safe-area-inset-top) + 28px)) 18px calc(118px + env(safe-area-inset-bottom));
    }

    .screen.about-screen {
        padding-top: max(18px, env(safe-area-inset-top));
    }

    .page-head {
        display: grid;
        gap: 8px;
        padding: 14px 0 26px;
    }

    .page-head h1,
    .page-head small {
        margin: 0;
    }

    .page-head h1 {
        font-size: 34px;
        line-height: 1.15;
        letter-spacing: -0.5px;
    }

    .page-head small {
        color: var(--muted);
        font-size: 13px;
        line-height: 1.45;
    }

    .entry-list,
    .config-page,
    .about-list {
        display: grid;
        gap: 12px;
    }

    .entry-card {
        min-height: 112px;
        display: grid;
        grid-template-columns: 52px minmax(0, 1fr);
        grid-template-areas:
            "icon kicker"
            "icon title"
            "icon desc";
        align-content: center;
        gap: 8px;
        box-sizing: border-box;
        padding: 18px;
        border: 1px solid var(--line);
        border-radius: 28px;
        background: var(--card);
        color: inherit;
        text-align: left;
        box-shadow: 0 20px 46px rgba(99, 82, 130, 0.12);
        backdrop-filter: blur(22px);
    }

    .entry-card:disabled {
        opacity: 0.7;
    }

    .entry-icon {
        grid-area: icon;
        width: 44px;
        height: 44px;
        display: grid;
        place-items: center;
        align-self: center;
        border-radius: 16px;
        color: #fff;
        background: linear-gradient(145deg, var(--accent), color-mix(in srgb, var(--accent) 68%, #fff));
    }

    .entry-icon svg,
    .bottom-tabs svg,
    .about-card svg,
    .app-icon svg {
        width: 24px;
        height: 24px;
        fill: none;
        stroke: currentColor;
        stroke-width: 2;
        stroke-linecap: round;
        stroke-linejoin: round;
    }

    .entry-card > span:not(.entry-icon) {
        grid-area: kicker;
        color: var(--accent);
        font-size: 12px;
        font-weight: 900;
    }

    .entry-card strong {
        grid-area: title;
        font-size: 22px;
        line-height: 1.2;
    }

    .entry-card small {
        grid-area: desc;
        color: var(--muted);
        font-size: 13px;
        line-height: 1.45;
    }

    .settings-card,
    .about-card {
        border: 1px solid var(--line);
        background: var(--card);
        color: inherit;
        box-shadow: 0 20px 46px rgba(99, 82, 130, 0.12);
        backdrop-filter: blur(22px);
    }

    .settings-card {
        display: flex;
        justify-content: space-between;
        gap: 16px;
        align-items: center;
        padding: 18px;
        border-radius: 24px;
    }

    .settings-card div {
        display: grid;
        gap: 6px;
    }

    .settings-card span {
        color: #7b5ac8;
        font-size: 12px;
        font-weight: 900;
    }

    .settings-card strong {
        font-size: 20px;
    }

    .settings-card small {
        color: var(--muted);
        font-size: 13px;
        line-height: 1.45;
    }

    .setting-orb {
        width: 44px;
        height: 44px;
        flex: 0 0 auto;
        border-radius: 16px;
        background: linear-gradient(145deg, #8e75d4, #f3b7cb);
    }

    .about-page {
        display: grid;
        gap: 78px;
    }

    .brand-block {
        display: grid;
        justify-items: center;
        gap: 12px;
        padding-top: 168px;
        text-align: center;
    }

    .app-icon {
        width: 112px;
        height: 112px;
        display: grid;
        place-items: center;
        border-radius: 32px;
        background: linear-gradient(145deg, #7355bd, #8d71c7);
        color: rgba(255, 255, 255, 0.86);
        box-shadow: 0 22px 56px rgba(110, 82, 181, 0.24);
    }

    .app-icon svg {
        width: 68px;
        height: 68px;
        stroke-width: 3.2;
    }

    .brand-block h2 {
        margin: 0;
        color: #7453af;
        font-family: Georgia, "Times New Roman", serif;
        font-size: 40px;
        line-height: 1;
    }

    .brand-block p {
        margin: 0;
        color: #575463;
        font-size: 15px;
        font-weight: 700;
    }

    .about-card {
        min-height: 82px;
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 16px;
        border-radius: 24px;
        padding: 18px 20px;
        text-align: left;
    }

    .about-card span {
        display: grid;
        gap: 6px;
    }

    .about-card strong {
        font-size: 20px;
        line-height: 1.2;
    }

    .about-card small {
        color: #5f5966;
        font-size: 14px;
        line-height: 1.35;
    }

    .about-card i {
        width: 34px;
        height: 34px;
        display: grid;
        place-items: center;
        color: #111;
    }

    .about-card i svg {
        width: 30px;
        height: 30px;
        stroke-width: 2.3;
    }

    .bottom-tabs {
        position: fixed;
        left: 12px;
        right: 12px;
        bottom: max(8px, env(safe-area-inset-bottom));
        z-index: 10;
        display: grid;
        grid-template-columns: repeat(3, 1fr);
        gap: 4px;
        box-sizing: border-box;
        padding: 8px;
        border: 1px solid rgba(255, 255, 255, 0.72);
        border-radius: 30px;
        background: rgba(255, 255, 255, 0.72);
        box-shadow: 0 18px 46px rgba(85, 76, 110, 0.18);
        backdrop-filter: blur(20px);
    }

    .bottom-tabs button {
        min-height: 56px;
        display: grid;
        place-items: center;
        gap: 3px;
        border: 0;
        border-radius: 22px;
        background: transparent;
        color: #8b8f98;
        font-weight: 900;
    }

    .bottom-tabs button.active {
        color: #050506;
        background: rgba(23, 27, 36, 0.08);
    }

    .bottom-tabs svg {
        width: 25px;
        height: 25px;
        stroke-width: 2.5;
    }

    .bottom-tabs span {
        font-size: 12px;
    }

    button {
        font: inherit;
        -webkit-tap-highlight-color: transparent;
    }

    @media (min-width: 720px) {
        .screen {
            max-width: 760px;
            padding-left: 24px;
            padding-right: 24px;
        }

        .entry-list {
            grid-template-columns: repeat(2, minmax(0, 1fr));
        }

        .bottom-tabs {
            max-width: 520px;
            left: 50%;
            right: auto;
            width: calc(100% - 32px);
            transform: translateX(-50%);
        }
    }
</style>
