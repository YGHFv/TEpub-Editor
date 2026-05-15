<script lang="ts">
    interface MobileModule {
        title: string;
        href: string;
        kicker: string;
        description: string;
        accent: string;
    }

    const modules: MobileModule[] = [
        {
            title: "制作 EPUB",
            href: "/mobile/make",
            kicker: "TXT / HTML / MD",
            description: "导入文本，预览目录，调整正则后生成 EPUB。",
            accent: "#1677b8",
        },
        {
            title: "解密 EPUB",
            href: "/mobile/decrypt",
            kicker: "伪加密 / 路径修复",
            description: "清理 EPUB 伪加密和异常文件名，导出可用副本。",
            accent: "#8a5a16",
        },
        {
            title: "编辑 EPUB",
            href: "/mobile/edit",
            kicker: "文件结构",
            description: "浏览 EPUB 内部目录、章节、样式、图片和字体。",
            accent: "#1f7a5a",
        },
        {
            title: "编辑元数据",
            href: "/mobile/metadata",
            kicker: "书名 / 作者 / 标签",
            description: "修改标题、作者、简介、标签、UUID 和制作信息。",
            accent: "#9b3d4f",
        },
    ];
</script>

<svelte:head>
    <title>TEpub Mobile</title>
</svelte:head>

<main class="mobile-home">
    <header class="home-head">
        <p>TEpub Editor Android</p>
        <h1>移动端 EPUB 工具</h1>
    </header>

    <nav class="entry-list" aria-label="功能入口">
        {#each modules as item}
            <a class="entry-card" href={item.href} style={`--accent:${item.accent}`}>
                <span>{item.kicker}</span>
                <strong>{item.title}</strong>
                <small>{item.description}</small>
            </a>
        {/each}
    </nav>
</main>

<style>
    :global(html),
    :global(body) {
        background: #f4f5f8;
    }

    .mobile-home {
        min-height: 100vh;
        box-sizing: border-box;
        padding: max(28px, env(safe-area-inset-top)) 18px max(28px, env(safe-area-inset-bottom));
        background: #f4f5f8;
        color: #171b24;
    }

    .home-head {
        padding: 14px 0 22px;
    }

    .home-head p {
        margin: 0 0 8px;
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
        text-decoration: none;
        box-shadow: 0 10px 24px rgba(23, 27, 36, 0.07);
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
