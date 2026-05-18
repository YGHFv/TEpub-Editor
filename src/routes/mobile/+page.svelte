<script lang="ts">
    import { goto } from "$app/navigation";
    import { message } from "@tauri-apps/plugin-dialog";
    import { openUrl } from "@tauri-apps/plugin-opener";
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

    const appVersion = "0.6.0";
    const appIconUrl = new URL("../../../src-tauri/icons/android/mipmap-xxxhdpi/ic_launcher.png", import.meta.url).href;
    const developerUrl = "https://github.com/YGHFv";

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
            href: "https://github.com/YGHFv/TEpub-Editor/graphs/contributors",
        },
        {
            title: "GitHub 仓库",
            description: "查看源码、问题反馈与发布版本",
            href: "https://github.com/YGHFv/TEpub-Editor",
        },
        {
            title: "爱发电赞助",
            description: "支持后续开发和维护",
            href: "https://afdian.com/",
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
    $: pageSubtitle = "";

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

    async function openExternal(href?: string) {
        if (!href) return;
        try {
            await openUrl(href);
        } catch {
            window.open(href, "_blank", "noopener,noreferrer");
        }
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

<main class:about-mode={activeTab === "about"} class="mobile-shell">
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
                        <small>入口页使用大圆角卡片和贴底导航；功能流程保持原有稳定逻辑。</small>
                    </div>
                    <span class="setting-orb" aria-hidden="true"></span>
                </div>
            </section>
        {:else}
            <section class="about-page">
                <div class="brand-block">
                    <div class="app-icon" aria-hidden="true">
                        <img src={appIconUrl} alt="" />
                    </div>
                    <h2>TEpub Editor</h2>
                    <p>{appVersion}-mobile-release</p>
                </div>

                <button class="developer-card" type="button" on:click={() => openExternal(developerUrl)}>
                    <img src="https://github.com/YGHFv.png" alt="" />
                    <span>
                        <strong>源谷绘</strong>
                        <small>@YGHFv</small>
                    </span>
                    <i aria-hidden="true">
                        <svg viewBox="0 0 24 24"><path d="M9 5l7 7-7 7"></path></svg>
                    </i>
                </button>

                <div class="about-list" aria-label="关于链接">
                    {#each aboutLinks as item}
                        <button class="about-row" type="button" on:click={() => openExternal(item.href)}>
                            <span>
                                <strong>{item.title}</strong>
                                <small>{item.description}</small>
                            </span>
                            <i aria-hidden="true">
                                <svg viewBox="0 0 24 24"><path d="M9 5l7 7-7 7"></path></svg>
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
        background: #f4f5f7;
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
        position: relative;
        overflow-x: hidden;
        min-height: 100vh;
        box-sizing: border-box;
        color: var(--text);
        background: #f4f5f7;
    }

    .mobile-shell.about-mode {
        background:
            radial-gradient(circle at 52% 20%, rgba(255, 255, 255, 0.88), transparent 30%),
            linear-gradient(155deg, #f4ecff 0%, #fdf3f6 45%, #edf5ff 100%);
    }

    .mobile-shell.about-mode::before,
    .mobile-shell.about-mode::after {
        content: "";
        position: fixed;
        inset: auto;
        z-index: 0;
        width: 82vw;
        height: 82vw;
        border-radius: 999px;
        pointer-events: none;
        filter: blur(34px);
        opacity: 0.72;
        transform: translateZ(0);
        animation: halo-drift 11s ease-in-out infinite alternate;
    }

    .mobile-shell.about-mode::before {
        top: 8vh;
        left: -26vw;
        background:
            radial-gradient(circle at 44% 46%, rgba(255, 198, 219, 0.92), transparent 58%),
            radial-gradient(circle at 72% 42%, rgba(157, 201, 255, 0.72), transparent 54%);
    }

    .mobile-shell.about-mode::after {
        right: -24vw;
        bottom: 10vh;
        background:
            radial-gradient(circle at 44% 46%, rgba(236, 223, 255, 0.92), transparent 56%),
            radial-gradient(circle at 22% 74%, rgba(255, 222, 176, 0.72), transparent 50%);
        animation-delay: -4s;
    }

    @keyframes halo-drift {
        from {
            transform: translate3d(-4%, -3%, 0) scale(0.92) rotate(0deg);
        }

        to {
            transform: translate3d(7%, 5%, 0) scale(1.08) rotate(16deg);
        }
    }

    .screen {
        position: relative;
        z-index: 1;
        min-height: 100vh;
        box-sizing: border-box;
        max-width: 780px;
        margin: 0 auto;
        padding: max(46px, calc(env(safe-area-inset-top) + 28px)) 18px calc(96px + env(safe-area-inset-bottom));
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
        border-radius: 22px;
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
    .developer-card svg,
    .about-row svg {
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

    .settings-card {
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
        border-radius: 22px;
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
        gap: 24px;
    }

    .brand-block {
        display: grid;
        justify-items: center;
        gap: 12px;
        padding-top: 168px;
        text-align: center;
    }

    .app-icon {
        width: 96px;
        height: 96px;
        display: grid;
        place-items: center;
        border-radius: 24px;
        background: rgba(255, 255, 255, 0.64);
        box-shadow: 0 22px 56px rgba(110, 82, 181, 0.16);
        overflow: hidden;
    }

    .app-icon img {
        width: 100%;
        height: 100%;
        object-fit: contain;
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

    .developer-card {
        min-height: 88px;
        display: flex;
        align-items: center;
        gap: 18px;
        margin-top: 38px;
        border: 0;
        border-radius: 28px;
        padding: 16px 20px;
        background: rgba(255, 255, 255, 0.86);
        color: inherit;
        text-align: left;
        box-shadow: 0 18px 36px rgba(103, 84, 128, 0.07);
        backdrop-filter: blur(24px);
    }

    .developer-card img {
        width: 58px;
        height: 58px;
        border-radius: 50%;
        object-fit: cover;
        background: #eef0f5;
    }

    .developer-card span {
        display: grid;
        gap: 6px;
        min-width: 0;
    }

    .about-row span {
        display: grid;
        gap: 4px;
        min-width: 0;
    }

    .developer-card strong {
        color: #0f1014;
        font-size: 20px;
        font-weight: 900;
        line-height: 1.2;
    }

    .developer-card small {
        color: #66616f;
        font-size: 14px;
        font-weight: 700;
        line-height: 1.35;
    }

    .about-row strong {
        color: #101116;
        font-size: 18px;
        font-weight: 850;
        line-height: 1.15;
    }

    .about-row small {
        overflow: hidden;
        color: #66616f;
        font-size: 13px;
        font-weight: 650;
        line-height: 1.25;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .developer-card i,
    .about-row i {
        width: 28px;
        height: 28px;
        display: grid;
        place-items: center;
        flex: 0 0 auto;
        margin-left: auto;
        color: #9ca0a8;
    }

    .developer-card i svg,
    .about-row i svg {
        width: 26px;
        height: 26px;
        stroke-width: 2.5;
    }

    .about-list {
        display: grid;
        gap: 0;
        overflow: hidden;
        border-radius: 28px;
        background: rgba(255, 255, 255, 0.86);
        box-shadow: 0 18px 36px rgba(103, 84, 128, 0.06);
        backdrop-filter: blur(24px);
    }

    .about-row {
        min-height: 70px;
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 16px;
        border: 0;
        padding: 12px 20px;
        background: transparent;
        color: inherit;
        text-align: left;
    }

    .about-row + .about-row {
        margin-top: 1px;
    }

    .bottom-tabs {
        position: fixed;
        left: 0;
        right: 0;
        bottom: 0;
        z-index: 10;
        display: grid;
        grid-template-columns: repeat(3, 1fr);
        gap: 6px;
        box-sizing: border-box;
        padding: 6px 16px max(4px, env(safe-area-inset-bottom));
        border-top: 0;
        background: rgba(255, 255, 255, 0.78);
        box-shadow: none;
        backdrop-filter: blur(28px) saturate(1.18);
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
        background: transparent;
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
            left: 0;
            right: 0;
            width: 100%;
            transform: none;
        }
    }
</style>
