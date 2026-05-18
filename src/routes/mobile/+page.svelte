<script lang="ts">
    import { onMount } from "svelte";
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
    type MobileTheme = "miuix" | "classic";

    const THEME_KEY = "tepub-mobile-theme";
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
    let mobileTheme: MobileTheme = "miuix";
    let status = "点击功能入口后直接选择文件。";

    $: isMiuix = mobileTheme === "miuix";
    $: pageTitle = activeTab === "home" ? "移动端 EPUB 工具" : activeTab === "config" ? "配置" : "关于";
    $: pageSubtitle =
        activeTab === "home"
            ? status
            : activeTab === "config"
              ? "调整 TEpub-Editor Android 的界面偏好。"
              : "TEpub-Editor Android";

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

    function setTheme(theme: MobileTheme) {
        mobileTheme = theme;
        if (typeof localStorage !== "undefined") {
            localStorage.setItem(THEME_KEY, theme);
        }
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

    onMount(() => {
        const saved = localStorage.getItem(THEME_KEY);
        if (saved === "classic" || saved === "miuix") {
            mobileTheme = saved;
        }
    });
</script>

<svelte:head>
    <title>TEpub Mobile</title>
</svelte:head>

<main class:classic={!isMiuix} class:miuix={isMiuix} class="mobile-shell">
    <input bind:this={makeInputEl} class="file-input" type="file" accept={modules[0].accept} on:change={(event) => handlePick(modules[0], event)} />
    <input bind:this={decryptInputEl} class="file-input" type="file" accept={modules[1].accept} on:change={(event) => handlePick(modules[1], event)} />
    <input bind:this={editInputEl} class="file-input" type="file" accept={modules[2].accept} on:change={(event) => handlePick(modules[2], event)} />

    <section class="screen">
        <header class="home-head">
            <p>TEpub Editor Android</p>
            <h1>{pageTitle}</h1>
            <small>{pageSubtitle}</small>
        </header>

        {#if activeTab === "home"}
            {#if !isMiuix}
                <section class="classic-config-card">
                    <span>配置 TEPUB-Editor</span>
                    <strong>{mobileTheme === "classic" ? "原主题" : "澎湃新主题"}</strong>
                    <small>与配置页同步，可随时切换界面风格。</small>
                    <div class="theme-choice compact">
                        <button class:active={mobileTheme === "miuix"} type="button" on:click={() => setTheme("miuix")}>新主题</button>
                        <button class:active={mobileTheme === "classic"} type="button" on:click={() => setTheme("classic")}>原主题</button>
                    </div>
                </section>
            {/if}

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
                        <span>界面主题</span>
                        <strong>{mobileTheme === "miuix" ? "澎湃新主题" : "原主题"}</strong>
                        <small>切换会立即保存，并同步到原主题首页配置卡。</small>
                    </div>
                    <span class="setting-orb" aria-hidden="true"></span>
                </div>

                <div class="theme-choice">
                    <button class:active={mobileTheme === "miuix"} type="button" on:click={() => setTheme("miuix")}>
                        <strong>澎湃新主题</strong>
                        <small>柔和渐变、圆角大卡片、底栏导航</small>
                    </button>
                    <button class:active={mobileTheme === "classic"} type="button" on:click={() => setTheme("classic")}>
                        <strong>原主题</strong>
                        <small>保留原入口卡片样式，并增加配置卡</small>
                    </button>
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
        background: #f4f5f8;
    }

    .file-input {
        position: fixed;
        width: 1px;
        height: 1px;
        opacity: 0;
        pointer-events: none;
    }

    .mobile-shell {
        --page-bg: #f4f5f8;
        --text: #171b24;
        --muted: #626a78;
        --card: #fff;
        --line: rgba(23, 27, 36, 0.08);
        min-height: 100vh;
        box-sizing: border-box;
        color: var(--text);
        background: var(--page-bg);
    }

    .mobile-shell.miuix {
        --page-bg: radial-gradient(circle at 12% 10%, rgba(214, 225, 255, 0.96), transparent 32%),
            radial-gradient(circle at 18% 78%, rgba(255, 210, 224, 0.9), transparent 28%),
            linear-gradient(135deg, #eef2ff 0%, #fff5fb 48%, #e7efff 100%);
        --card: rgba(255, 255, 255, 0.74);
        --line: rgba(255, 255, 255, 0.56);
        background: var(--page-bg);
    }

    .screen {
        min-height: 100vh;
        box-sizing: border-box;
        padding: max(28px, env(safe-area-inset-top)) 18px calc(118px + env(safe-area-inset-bottom));
    }

    .miuix .screen {
        max-width: 780px;
        margin: 0 auto;
        padding-top: max(46px, calc(env(safe-area-inset-top) + 28px));
    }

    .home-head {
        display: grid;
        gap: 8px;
        padding: 14px 0 22px;
    }

    .miuix .home-head {
        padding-bottom: 26px;
    }

    .home-head p,
    .home-head h1,
    .home-head small {
        margin: 0;
    }

    .home-head p {
        color: #777d89;
        font-size: 13px;
        font-weight: 900;
    }

    .home-head h1 {
        font-size: 30px;
        line-height: 1.15;
        letter-spacing: 0;
    }

    .miuix .home-head h1 {
        font-size: 34px;
        letter-spacing: -0.5px;
    }

    .home-head small {
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
        min-height: 118px;
        display: grid;
        align-content: center;
        gap: 8px;
        box-sizing: border-box;
        padding: 18px;
        border: 1px solid var(--line);
        border-left: 5px solid var(--accent);
        border-radius: 8px;
        background: var(--card);
        color: inherit;
        text-align: left;
        box-shadow: 0 10px 24px rgba(23, 27, 36, 0.07);
    }

    .miuix .entry-card {
        grid-template-columns: 52px minmax(0, 1fr);
        grid-template-areas:
            "icon kicker"
            "icon title"
            "icon desc";
        min-height: 112px;
        border: 1px solid rgba(255, 255, 255, 0.66);
        border-radius: 28px;
        border-left: 1px solid rgba(255, 255, 255, 0.66);
        box-shadow: 0 20px 46px rgba(99, 82, 130, 0.12);
        backdrop-filter: blur(22px);
    }

    .entry-card:disabled {
        opacity: 0.7;
    }

    .entry-icon {
        display: none;
    }

    .miuix .entry-icon {
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
        color: var(--accent);
        font-size: 12px;
        font-weight: 900;
    }

    .miuix .entry-card > span:not(.entry-icon) {
        grid-area: kicker;
    }

    .entry-card strong {
        font-size: 21px;
        line-height: 1.2;
    }

    .miuix .entry-card strong {
        grid-area: title;
        font-size: 22px;
    }

    .entry-card small {
        color: var(--muted);
        font-size: 13px;
        line-height: 1.45;
    }

    .miuix .entry-card small {
        grid-area: desc;
    }

    .classic-config-card,
    .settings-card,
    .about-card {
        border: 1px solid var(--line);
        background: var(--card);
        color: inherit;
        box-shadow: 0 10px 24px rgba(23, 27, 36, 0.07);
    }

    .classic-config-card {
        display: grid;
        gap: 9px;
        margin-bottom: 12px;
        padding: 18px;
        border-left: 5px solid #7b5ac8;
        border-radius: 8px;
    }

    .classic-config-card span,
    .settings-card span {
        color: #7b5ac8;
        font-size: 12px;
        font-weight: 900;
    }

    .classic-config-card strong,
    .settings-card strong {
        font-size: 20px;
    }

    .classic-config-card small,
    .settings-card small {
        color: var(--muted);
        font-size: 13px;
        line-height: 1.45;
    }

    .settings-card {
        display: flex;
        justify-content: space-between;
        gap: 16px;
        align-items: center;
        padding: 18px;
        border-radius: 24px;
        backdrop-filter: blur(22px);
    }

    .settings-card div {
        display: grid;
        gap: 6px;
    }

    .setting-orb {
        width: 44px;
        height: 44px;
        flex: 0 0 auto;
        border-radius: 16px;
        background: linear-gradient(145deg, #8e75d4, #f3b7cb);
    }

    .theme-choice {
        display: grid;
        gap: 10px;
    }

    .theme-choice.compact {
        grid-template-columns: 1fr 1fr;
    }

    .theme-choice button {
        display: grid;
        gap: 4px;
        min-height: 56px;
        border: 1px solid rgba(123, 90, 200, 0.14);
        border-radius: 18px;
        background: rgba(255, 255, 255, 0.7);
        color: inherit;
        padding: 12px;
        text-align: left;
    }

    .classic .theme-choice button {
        border-radius: 8px;
        background: #f7f4ff;
    }

    .theme-choice button.active {
        background: #171b24;
        color: #fff;
        border-color: #171b24;
    }

    .theme-choice strong {
        font-size: 15px;
    }

    .theme-choice small {
        color: currentColor;
        opacity: 0.68;
        font-size: 12px;
        line-height: 1.35;
    }

    .about-page {
        display: grid;
        gap: 96px;
    }

    .brand-block {
        display: grid;
        justify-items: center;
        gap: 12px;
        padding-top: 110px;
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
        backdrop-filter: blur(22px);
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
        left: 0;
        right: 0;
        bottom: 0;
        z-index: 10;
        display: grid;
        grid-template-columns: repeat(3, 1fr);
        gap: 4px;
        box-sizing: border-box;
        padding: 8px 18px calc(12px + env(safe-area-inset-bottom));
        border-top: 1px solid rgba(23, 27, 36, 0.08);
        background: rgba(247, 248, 251, 0.96);
        backdrop-filter: blur(20px);
    }

    .miuix .bottom-tabs {
        left: 12px;
        right: 12px;
        bottom: max(8px, env(safe-area-inset-bottom));
        border: 1px solid rgba(255, 255, 255, 0.72);
        border-radius: 30px;
        padding: 8px;
        box-shadow: 0 18px 46px rgba(85, 76, 110, 0.18);
        background: rgba(255, 255, 255, 0.72);
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
    }

    .miuix .bottom-tabs button.active {
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
            margin: 0 auto;
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
