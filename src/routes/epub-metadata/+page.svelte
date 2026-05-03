<script lang="ts">
    import { onMount } from "svelte";
    import { listen, emit } from "@tauri-apps/api/event";
    import { getCurrentWindow } from "@tauri-apps/api/window";

    let activeTab = "meta"; // "meta" or "style"
    let activeCssFile = "main.css"; // "main.css" or "font.css"
    let persistCss = false;
    
    let metadata = {
        publisher: "",
        uuid: "",
        md5: "",
        styles: {
            "main.css": "",
            "font.css": "",
        },
        assets: [] as { name: string, path: string, category: string }[],
    };

    const BUILTIN_STYLES = {
        "main.css": `@charset "utf-8";

@import url("fonts.css");

/* Global Setting */

body {
    padding: 0%;
    margin-top: 0%;
    margin-bottom: 0%;
    margin-left: 0.5%;
    margin-right: 0.5%;
    line-height: 130%;
    text-align: justify;
    font-family: "Maintext", "DK-SONGTI", "st", "宋体", "zw", sans-serif;
}

p {
    text-align: justify;
    text-indent: 2em;
    duokan-text-indent: 2em;
    line-height: 150%;
    margin-right: 0.5%;
    margin-left: 0.5%;
    font-family: "Maintext";
}

div {
    margin: 0;
    padding: 0;
    line-height: 130%;
    text-align: justify;
    font-family: "zw";
}

/*————————————————————制作说明————————————————————*/
.copyright {
    margin: 10% 7.25% 2.75% 7.25%;
    padding: 5.25% 5.25%;
    border: 1.5px solid #6C322D;
    background-size: 35% auto;
    border-radius: 5px;
}

.line {
    border: dotted #333;
    border-width: 1px 0 0 0;
    margin: 5% 0 5% 0;
}

h1.copyright-title {
    font-family: "Title";
    font-size: 121%;
    font-weight: normal;
    color: #00008B;
    margin: 1em 0 0.77em 0;
    text-align: center;
}

body.full {
    background: no-repeat center;
    background-size: cover;
    background-attachment: fixed;
    background-repeat: no-repeat;
    background-position: bottom center;
    background-image: url(../Images/back.jpg);
    transform: scale(1.0) translate(0px, 0px);
}

.copyright-text1 {
    font-family: "Title";
    font-size: 80%;
    color: #220;
    text-align: justify;
    text-indent: 2em;
    duokan-text-indent: 2em;
    margin: 0 0 2.5% 0;
}

.copyright-text2 {
    font-family: "cc", "kt", sans-serif;
    font-size: 65%;
    color: #000;
    text-align: justify;
    text-indent: 2em;
    duokan-text-indent: 2em;
    margin: 2.5% 0 0 0;
}

div.logo {
    margin: 0 24% 0 24%;
    text-align: center;
    text-indent: 0em;
    duokan-text-indent: 0em;
}

img.logo {
    width: 66%;
}

/*————————————————————内容简介————————————————————*/

body.introduction {
    border-color: rgba(83, 83, 83, 0.5);
    border-width: 0.4em;
}

div.cover {
    margin: 2em 0 1em 0;
    text-align: center;
    text-indent: 0;
    duokan-text-indent: 0;
    width: 100%;
}

img.cover {
    width: 40%;
    box-shadow: 3px 3px 3px #535353;
    margin: 0 0 0.5em 0;
}

h1.nrjj-title {
    font-family: "Title";
    font-size: 160%;
    font-weight: normal;
    color: #00008B;
    margin: 2em 0 1.6em 0;
    text-align: center;
}

span.book-name {
    font-family: "楷体", sans-serif;
    color: #DC143C
}

span.author {
    font-family: "小标宋", sans-serif;
}

h1.introduction-title {
    margin: 0.3em 0 0.5em 0;
    text-align: left;
    text-indent: 0;
    duokan-text-indent: 0;
    font-size: 110%;
    color: #00008B;
    font-family: "Title";
}

h1.introduction-title span {
    padding: 0.4em 2em 0.2em 0.4em;
}

div.book-introduction p {
    font-family: "DK-XIHEITI", "黑体", sans-serif;
}

h1.PrefacehA1 {
    font-family: "Title", "黑体", sans-serif;
    text-align: center;
    font-weight: 600;
    font-size: 1.2em;
    margin: 7em 0em 1em 0em;
    color: #f972bd;
    line-height: 130%;
}

h1.PrefacehA1 b {
    font-family: "Title", "黑体", sans-serif;
    font-size: 1.1em;
    font-weight: 900;
    color: #dd3e3f;
}

p.PrefacepA1 {
    font-family: "Title";
    color: #5577c1;
    font-size: 1.7em;
    margin: 0em 0em 0.2em 0em;
    text-indent: 0em;
    text-align: center;
    line-height: 110%;
}

/* Header Image */

div.logo {
    margin: 0.5em;
    text-align: center;
    text-indent: 0em;
    duokan-text-indent: 0em;
    duokan-bleed: lefttopright;
}

img.logo {
    width: 70%;
}

/* Chapter Title */

h3.head {
    font-size: 1.2em;
    color: #5577c1;
    text-align: center;
    line-height: 130%;
    padding: 35px 4px 0 4px;
    margin: 0em auto 2em auto;
    font-family: "Title";
}

h3.neirong {
    font-size: 1.1em;
    color: #5577c1;
    text-align: right;
    line-height: 130%;
    padding: 0 4px 0 4px;
    margin: -1em 0em 0em 2em;
    font-family: "Maintext";
}

span.num {
    font-family: "Maintext";
    padding: 2px 4px 1px 4px;
    text-align: center;
    font-size: 0.81em;
    background-color: #f972bd;
    border-radius: 10px;
    color: #fff;
}

span.num2 {
    font-size: 0.95em;
    color: white;
    background-color: #20626d;
    padding: 0.2em 0.4em 0.1em;
    border-radius: 0.2em;
    font-family: "Maintext";
}

span.num3 {
    color: #b50a02;
    font-family: "Maintext";
}

h2.head5 {
    padding: 0 4px 0 4px;
    margin: 1em auto 2em auto;
    font-size: 1.6em;
    color: #a36141;
    text-align: center;
    line-height: 130%;
    font-family: "Title";
    text-indent: 0em;
    duokan-text-indent: 0em;
}

h2.head {
    font-size: 2.1em;
    color: #59bde6;
    text-align: center;
    line-height: 130%;
    padding: 64px 4px 0 4px;
    margin: 0em auto 2em auto;
    font-family: "Title";
}

/* 分割线 */
p.fg1 {
    text-align: center;
    text-indent: 0;
    duokan-text-indent: 0em;
}

/*全面屏*/
body.fy {
    background-size: cover;
    background-repeat: no-repeat;
    background-attachment: fixed;
    background-position: center;
    background-image: url('../Images/fy.jpg');
}

body.intro {
    background-size: cover;
    background-repeat: no-repeat;
    background-attachment: fixed;
    background-position: center;
    background-image: url('../Images/intro.jpg');
}

body.e1 { background-size: cover; background-repeat: no-repeat; background-attachment: fixed; background-position: center; background-image: url('../Images/e1.jpg'); }
body.e2 { background-size: cover; background-repeat: no-repeat; background-attachment: fixed; background-position: center; background-image: url('../Images/e2.jpg'); }
body.e3 { background-size: cover; background-repeat: no-repeat; background-attachment: fixed; background-position: center; background-image: url('../Images/e3.jpg'); }
body.e4 { background-size: cover; background-repeat: no-repeat; background-attachment: fixed; background-position: center; background-image: url('../Images/e4.jpg'); }
body.e5 { background-size: cover; background-repeat: no-repeat; background-attachment: fixed; background-position: center; background-image: url('../Images/e5.jpg'); }
body.e6 { background-size: cover; background-repeat: no-repeat; background-attachment: fixed; background-position: center; background-image: url('../Images/e6.jpg'); }
body.e7 { background-size: cover; background-repeat: no-repeat; background-attachment: fixed; background-position: center; background-image: url('../Images/e7.jpg'); }
body.e8 { background-size: cover; background-repeat: no-repeat; background-attachment: fixed; background-position: center; background-image: url('../Images/e8.jpg'); }
body.e9 { background-size: cover; background-repeat: no-repeat; background-attachment: fixed; background-position: center; background-image: url('../Images/e9.jpg'); }
body.e10 { background-size: cover; background-repeat: no-repeat; background-attachment: fixed; background-position: center; background-image: url('../Images/e10.jpg'); }
body.e11 { background-size: cover; background-repeat: no-repeat; background-attachment: fixed; background-position: center; background-image: url('../Images/e11.jpg'); }
body.ex {
    background-size: cover;
    background-repeat: no-repeat;
    background-attachment: fixed;
    background-position: center;
    background-image: url('../Images/ex.jpg');
}

p.fs3 {
    font-family: "zdy3";
    color: #000;
    margin: 1em 0em 1em 0em;
    font-size: 1.0em;
    font-weight: bold;
}

div.zwone {
    margin: 0em 0em 0em 0em;
    text-align: left;
    text-indent: 0em;
    duokan-text-indent: 0em;
}

img.zwone {
    width: 70%;
}

div.neirong {
    text-align: left;
    text-indent: 0em;
    margin: 0em 0em 0em 0em;
    duokan-text-indent: 0em;
}

img.neirong {
    width: 55%;
}

.fs2 {
    font-family: "zdy2";
    font-weight: bold;
}

.txtu {
    text-indent: 2em;
    duokan-text-indent: 2em;
    line-height: 130%;
    margin-right: 1%;
    margin-left: 1%;
    font-family: "zdy5";
    color: #1E90FF;
}

.txtu2 {
    text-indent: 2em;
    duokan-text-indent: 2em;
    line-height: 130%;
    margin-right: 1%;
    margin-left: 1%;
    font-family: "zdy5";
    color: #B22222;
    font-size: 0.95em;
}

p.fs7 {
    font-family: "Maintext";
    color: #000;
    font-size: 0.9em;
    text-align: right;
    margin: 1em 1em 2em 0em;
}

div.roundsolid2 {
    margin: 1em 0em;
    text-indent: 2em;
    duokan-text-indent: 2em;
    line-height: 130%;
    margin-right: 1%;
    margin-left: 1%;
    font-family: "zdy4";
    color: #02439B;
    font-size: 0.9em;
}

.bu {
    display: block;
    font-size: .9em;
}

/*图片*/
.duokan-image-single {
    text-align: center;
    text-indent: 0em;
    duokan-text-indent: 0em;
    margin: 1.5em 0;
    text-align: center;
}

.DKimg-left {
    float: left;
    clear: both;
    width: 50%;
    margin: 0 0.5em 0.2em 0;
}

.DKimg-right {
    float: right;
    clear: both;
    width: 50%;
    margin: 0 0 0em 0.5em;
}

.txtu2 {
    text-indent: 2em;
    duokan-text-indent: 2em;
    line-height: 130%;
    margin-right: 1%;
    margin-left: 1%;
    font-family: "zdy6";
    color: #B22222;
    font-size: 0.95em;
}

.txtu {
    text-indent: 2em;
    duokan-text-indent: 2em;
    line-height: 130%;
    margin-right: 1%;
    margin-left: 1%;
    font-family: "zdy6";
    color: #1E90FF;
    font-size: 0.95em;
}`,
        "font.css": `@charset "utf-8";
/*正文字体*/
@font-face {
    font-family: "Maintext";
    src: url("../Fonts/Maintext.ttf");
}

/*标题字体*/
@font-face {
    font-family: "Title";
    src: url("../Fonts/Title.ttf"); 
}`
    };

    let customMetadata: { key: string; value: string }[] = [];

    onMount(() => {
        let unlistenFn: (() => void) | undefined;

        const init = async () => {
             // 监听来自主窗口的数据初始化
            const unlisten = await listen("init-metadata", (event: any) => {
                console.log("Received init metadata:", event.payload);
                const { meta, custom } = event.payload;
                metadata.publisher = meta.publisher || "";
                metadata.uuid = meta.uuid || "";
                metadata.md5 = meta.md5 || "";
                
                // 自动预填充内置样式内容
                metadata.styles = {
                    "main.css": meta.styles?.["main.css"] || BUILTIN_STYLES["main.css"],
                    "font.css": meta.styles?.["font.css"] || BUILTIN_STYLES["font.css"],
                };
                metadata.assets = meta.assets || [];
                customMetadata = [...(custom || [])];
            });
            unlistenFn = unlisten;

            // 告诉主窗口我们准备好了，可以发送数据了
            await emit("metadata-window-ready");
        };

        init();

        return () => {
            if (unlistenFn) unlistenFn();
        };
    });

    function addCustom() {
        customMetadata = [...customMetadata, { key: "", value: "" }];
    }

    function removeCustom(index: number) {
        customMetadata = customMetadata.filter((_, i) => i !== index);
    }

    async function addAsset(category: string) {
        try {
            const { open } = await import("@tauri-apps/plugin-dialog");
            const selected = await open({
                multiple: true,
                filters: category === 'fonts' ? 
                    [{ name: 'Fonts', extensions: ['ttf', 'otf', 'woff', 'woff2'] }] :
                    category === 'images' ?
                    [{ name: 'Images', extensions: ['jpg', 'jpeg', 'png', 'gif', 'svg'] }] :
                    []
            });

            if (selected) {
                const paths = Array.isArray(selected) ? selected : [selected];
                const newAssets = paths.map(p => ({
                    name: (p as string).split(/[/\\]/).pop() || "",
                    path: p as string,
                    category
                }));
                metadata.assets = [...metadata.assets, ...newAssets];
            }
        } catch (err) {
            console.error("Failed to add assets:", err);
        }
    }

    function removeAsset(index: number) {
        metadata.assets = metadata.assets.filter((_, i) => i !== index);
    }

    async function saveAndClose() {
        await emit("update-metadata", {
            meta: metadata,
            custom: customMetadata,
            persistCss: persistCss,
        });
        const win = getCurrentWindow();
        await win.close();
    }

    async function cancel() {
        const win = getCurrentWindow();
        await win.close();
    }
</script>

<div class="metadata-container">
    <div class="tabs">
        <button class="tab-btn" class:active={activeTab === 'meta'} on:click={() => activeTab = 'meta'}>元数据</button>
        <button class="tab-btn" class:active={activeTab === 'style'} on:click={() => activeTab = 'style'}>样式</button>
        <button class="tab-btn" class:active={activeTab === 'files'} on:click={() => activeTab = 'files'}>文件</button>
    </div>

    {#if activeTab === 'meta'}
        <div class="tab-content scroll-p compact-top">
            <div class="form-section">
                <div class="input-group">
                    <label for="publisher">出版社:</label>
                    <input
                        id="publisher"
                        type="text"
                        bind:value={metadata.publisher}
                        placeholder="(可选)"
                    />
                </div>

                <div class="input-group">
                    <label for="uuid">UUID:</label>
                    <input id="uuid" type="text" bind:value={metadata.uuid} />
                </div>

                <div class="input-group">
                    <label for="md5">MD5:</label>
                    <input id="md5" type="text" bind:value={metadata.md5} />
                </div>
            </div>

            <div class="divider"></div>

            <div class="custom-section">
                <div class="section-header">
                    <h3>自定义元数据</h3>
                    <button class="add-btn" on:click={addCustom}>+</button>
                </div>

                <div class="custom-list">
                    {#each customMetadata as item, i}
                        <div class="custom-row">
                            <input type="text" placeholder="键 (Key)" bind:value={item.key} />
                            <span>:</span>
                            <input
                                type="text"
                                placeholder="值 (Value)"
                                bind:value={item.value}
                            />
                            <button class="remove-btn" on:click={() => removeCustom(i)}
                                >×</button
                            >
                        </div>
                    {/each}
                    {#if customMetadata.length === 0}
                        <p class="empty-hint">点击右上角 + 添加自定义键值对</p>
                    {/if}
                </div>
            </div>
        </div>
    {:else if activeTab === 'files'}
        <div class="tab-content scroll-p compact-top">
            <div class="asset-manager">
                {#each ['fonts', 'images', 'others'] as cat}
                    <div class="asset-group">
                        <div class="section-header">
                            <h3>{cat === 'fonts' ? '字体' : cat === 'images' ? '图片' : '其他'}</h3>
                            <button class="add-btn small" on:click={() => addAsset(cat)}>+</button>
                        </div>
                        <div class="asset-list">
                            {#each metadata.assets.filter(a => a.category === cat) as asset, idx}
                                <div class="asset-item">
                                    <span class="file-name" title={asset.path}>{asset.name}</span>
                                    <button class="remove-btn" on:click={() => removeAsset(metadata.assets.indexOf(asset))}>×</button>
                                </div>
                            {/each}
                            {#if metadata.assets.filter(a => a.category === cat).length === 0}
                                <p class="empty-hint">未添加{cat === 'fonts' ? '字体' : cat === 'images' ? '图片' : '文件'}</p>
                            {/if}
                        </div>
                    </div>
                {/each}
            </div>
        </div>
    {:else}
        <div class="tab-content style-manager">
            <div class="style-sub-tabs">
                <button 
                    class="sub-tab" 
                    class:active={activeCssFile === 'main.css'} 
                    on:click={() => activeCssFile = 'main.css'}>
                    main.css
                    {#if (metadata.styles as any)['main.css'] && (metadata.styles as any)['main.css'] !== BUILTIN_STYLES['main.css']}<span class="modified-dot">●</span>{/if}
                </button>
                <button 
                    class="sub-tab" 
                    class:active={activeCssFile === 'font.css'} 
                    on:click={() => activeCssFile = 'font.css'}>
                    font.css
                    {#if (metadata.styles as any)['font.css'] && (metadata.styles as any)['font.css'] !== BUILTIN_STYLES['font.css']}<span class="modified-dot">●</span>{/if}
                </button>
            </div>

            <div class="style-editor-area">
                <div class="section-header">
                    <div class="editor-controls">
                        <label class="persist-cb">
                            <input type="checkbox" bind:checked={persistCss} /> 保存为默认样式
                        </label>
                        <button class="mini-btn" on:click={() => (metadata.styles as any)[activeCssFile] = BUILTIN_STYLES[activeCssFile as keyof typeof BUILTIN_STYLES]}>重置为内置</button>
                    </div>
                </div>
                <div class="editor-main">
                    <textarea 
                        class="css-editor full-height" 
                        bind:value={(metadata.styles as any)[activeCssFile]} 
                        placeholder={`正在编辑 ${activeCssFile}...\n（清空内容将恢复内置默认配置）`}
                    ></textarea>
                </div>
            </div>
        </div>
    {/if}

    <div class="footer">
        <button class="btn-cancel" on:click={cancel}>取消</button>
        <button class="btn-save" on:click={saveAndClose}>保存</button>
    </div>
</div>

<style>
    :global(body) {
        margin: 0;
        padding: 0;
        background: #f8faff;
        font-family:
            "Segoe UI",
            Roboto,
            system-ui,
            -apple-system,
            sans-serif;
        color: #333;
        overflow: hidden;
    }

    .metadata-container {
        display: flex;
        flex-direction: column;
        height: 100vh;
        padding: 10px 15px;
        box-sizing: border-box;
        overflow: hidden;
    }

    .tabs {
        display: flex;
        gap: 15px;
        border-bottom: 2px solid #eee;
        margin-bottom: 10px;
        flex-shrink: 0;
    }

    .tab-btn {
        padding: 8px 4px;
        background: none;
        border: none;
        border-bottom: 2px solid transparent;
        color: #666;
        cursor: pointer;
        font-size: 0.95rem;
        transition: all 0.2s;
    }

    .tab-btn.active {
        color: #3498db;
        border-bottom-color: #3498db;
        font-weight: 600;
    }

    .tab-content {
        flex: 1;
        display: flex;
        flex-direction: column;
        min-height: 0;
    }

    .tab-content.compact-top {
        padding-top: 5px;
    }

    .scroll-p {
        flex: 1;
        overflow-y: auto;
        padding-right: 5px;
    }

    .form-section {
        display: flex;
        flex-direction: column;
        gap: 12px;
    }

    .input-group {
        display: flex;
        align-items: center;
        gap: 10px;
    }

    .input-group label {
        width: 80px;
        font-size: 0.9rem;
        color: #666;
    }

    .input-group input {
        flex: 1;
        padding: 8px 12px;
        border: 1px solid #ddd;
        border-radius: 6px;
        font-size: 0.9rem;
        transition: all 0.2s;
    }

    .input-group input:focus {
        outline: none;
        border-color: #3498db;
        box-shadow: 0 0 0 2px rgba(52, 152, 219, 0.2);
    }

    .divider {
        height: 1px;
        background: #eee;
        margin: 20px 0;
    }

    .custom-section {
        flex: 1;
        display: flex;
        flex-direction: column;
        min-height: 0;
    }

    .section-header {
        display: flex;
        justify-content: space-between;
        align-items: center;
        margin-bottom: 15px;
    }

    .section-header h3 {
        margin: 0;
        font-size: 1rem;
        color: #2c3e50;
    }

    .add-btn {
        width: 28px;
        height: 28px;
        border-radius: 50%;
        border: none;
        background: #3498db;
        color: white;
        font-size: 1.2rem;
        cursor: pointer;
        display: flex;
        align-items: center;
        justify-content: center;
        transition: transform 0.2s;
    }

    .add-btn:hover {
        transform: scale(1.1);
        background: #2980b9;
    }

    .custom-list {
        flex: 1;
        overflow-y: auto;
        overflow-x: hidden;
        display: flex;
        flex-direction: column;
        gap: 8px;
        padding-right: 2px;
    }

    .custom-row {
        display: flex;
        align-items: center;
        gap: 4px;
        background: white;
        padding: 4px 6px;
        border-radius: 6px;
        border: 1px solid #eee;
        min-width: 0;
    }

    .custom-row input {
        flex: 1;
        min-width: 30px;
        padding: 5px 4px;
        border: 1px solid transparent;
        background: transparent;
        font-size: 0.85rem;
    }

    .custom-row span {
        flex-shrink: 0;
        color: #999;
    }

    .remove-btn {
        flex-shrink: 0;
        background: none;
        border: none;
        color: #e74c3c;
        font-size: 1.2rem;
        cursor: pointer;
        padding: 0 4px;
        line-height: 1;
    }

    /* 资源管理器样式 */
    .asset-manager {
        display: flex;
        flex-direction: column;
        gap: 15px;
    }

    .asset-group {
        display: flex;
        flex-direction: column;
        gap: 8px;
    }

    .asset-group h3 {
        font-size: 0.9rem;
        color: #666;
        margin: 0;
    }

    .asset-list {
        display: flex;
        flex-direction: column;
        gap: 5px;
        min-height: 20px;
    }

    .asset-item {
        display: flex;
        align-items: center;
        justify-content: space-between;
        background: #f1f3f5;
        padding: 6px 10px;
        border-radius: 4px;
        font-size: 0.85rem;
    }

    .file-name {
        flex: 1;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        color: #2c3e50;
        margin-right: 10px;
    }

    .footer {
        display: flex;
        justify-content: flex-end;
        gap: 12px;
        margin-top: 10px;
        padding-top: 10px;
        border-top: 1px solid #eee;
        flex-shrink: 0;
    }

    .btn-cancel {
        padding: 8px 20px;
        border: 1px solid #ddd;
        background: white;
        border-radius: 6px;
        cursor: pointer;
        font-size: 0.9rem;
    }

    .btn-cancel:hover {
        background: #f8f9fa;
        border-color: #ccc;
    }

    .btn-save {
        padding: 8px 24px;
        background: #2c3e50;
        color: white;
        border: none;
        border-radius: 6px;
        cursor: pointer;
        font-size: 0.9rem;
        font-weight: 500;
        transition: background 0.2s;
    }

    .btn-save:hover {
        background: #1a252f;
    }

    .empty-hint {
        text-align: center;
        color: #999;
        font-size: 0.85rem;
        margin-top: 20px;
    }

    .style-manager {
        flex-direction: column;
        gap: 0;
        border: 1px solid #eee;
        border-radius: 8px;
        background: white;
        overflow: hidden;
    }

    .style-sub-tabs {
        display: flex;
        background: #f1f3f5;
        border-bottom: 1px solid #dee2e6;
    }

    .sub-tab {
        padding: 10px 20px;
        border: none;
        background: none;
        font-size: 0.85rem;
        color: #666;
        cursor: pointer;
        transition: all 0.2s;
        border-right: 1px solid #dee2e6;
        position: relative;
    }

    .sub-tab:hover {
        background: #e9ecef;
    }

    .sub-tab.active {
        background: white;
        color: #3498db;
        font-weight: 600;
    }

    .modified-dot {
        color: #3498db;
        margin-left: 4px;
    }

    .style-editor-area {
        flex: 1;
        display: flex;
        flex-direction: column;
        min-width: 0;
        padding: 15px;
    }

    .editor-main {
        flex: 1;
        display: flex;
        flex-direction: column;
        gap: 15px;
        min-height: 0;
    }

    .css-editor {
        flex: 1;
        width: 100%;
        padding: 12px;
        border: 1px solid #ddd;
        border-radius: 6px;
        background: #fafafa;
        font-family: 'Consolas', 'Monaco', monospace;
        font-size: 0.8rem;
        line-height: 1.5;
        border-radius: 6px;
        overflow-y: auto;
        padding: 15px;
        box-shadow: inset 0 1px 3px rgba(0,0,0,0.05);
    }

    .editor-controls {
        display: flex;
        align-items: center;
        gap: 12px;
    }

    .persist-cb {
        display: flex;
        align-items: center;
        gap: 4px;
        font-size: 0.8rem;
        color: #666;
        cursor: pointer;
    }

    .persist-cb input {
        cursor: pointer;
    }

    .mini-btn {
        padding: 2px 8px;
        font-size: 0.75rem;
        background: #f1f3f5;
        border: 1px solid #dee2e6;
        border-radius: 4px;
        cursor: pointer;
    }

    .mini-btn:hover {
        background: #e9ecef;
    }

    ::-webkit-scrollbar {
        width: 6px;
    }
    ::-webkit-scrollbar-thumb {
        background: #ddd;
        border-radius: 3px;
    }

    /* Modern UI overrides */
    :global(body) {
        background: var(--gradient-app);
        color: var(--color-text);
        font-family: var(--font-ui);
    }

    .metadata-container {
        padding: 14px;
        background: rgba(246, 250, 253, 0.72);
    }

    .tabs {
        gap: 8px;
        margin-bottom: 12px;
        padding: 6px;
        border: 1px solid var(--color-border);
        border-radius: var(--radius-md);
        background: rgba(255, 255, 255, 0.78);
        box-shadow: var(--shadow-xs);
        backdrop-filter: blur(14px);
    }

    .tab-btn {
        padding: 8px 16px;
        border: 1px solid transparent;
        border-radius: 999px;
        color: var(--color-text-soft);
        font-weight: 700;
    }

    .tab-btn:hover {
        background: var(--color-hover);
        color: var(--color-text);
    }

    .tab-btn.active {
        background: var(--color-accent-soft);
        border-color: rgba(22, 119, 184, 0.18);
        color: var(--color-accent-deep);
    }

    .tab-content {
        background: rgba(255, 255, 255, 0.66);
        border: 1px solid var(--color-border);
        border-radius: var(--radius-lg);
        padding: 16px;
        box-shadow: var(--shadow-sm);
        box-sizing: border-box;
    }

    .tab-content.style-manager {
        padding: 0;
    }

    .input-group label,
    .asset-group h3,
    .persist-cb,
    .empty-hint {
        color: var(--color-text-soft);
    }

    .section-header h3 {
        color: var(--color-text);
        font-weight: 800;
        letter-spacing: 0.01em;
    }

    .input-group input,
    .custom-row,
    .css-editor {
        border: 1px solid var(--color-border);
        border-radius: var(--radius-sm);
        background: rgba(255, 255, 255, 0.92);
        color: var(--color-text);
        transition:
            border-color var(--transition-fast),
            box-shadow var(--transition-fast),
            background var(--transition-fast);
    }

    .input-group input:focus,
    .custom-row input:focus,
    .css-editor:focus {
        outline: none;
        border-color: var(--color-accent);
        box-shadow: var(--focus-ring);
        background: #fff;
    }

    .custom-row {
        padding: 6px 8px;
        box-shadow: var(--shadow-xs);
    }

    .custom-row input {
        color: var(--color-text);
    }

    .divider,
    .footer {
        border-color: var(--color-border);
    }

    .add-btn,
    .btn-save {
        background: var(--gradient-accent);
        color: #fff;
        box-shadow: 0 10px 22px rgba(22, 119, 184, 0.18);
    }

    .add-btn {
        border: 0;
    }

    .add-btn:hover,
    .btn-save:hover {
        background: linear-gradient(135deg, var(--color-accent-deep), var(--color-teal));
        transform: translateY(-1px);
    }

    .remove-btn {
        border-radius: 999px;
        color: var(--color-danger);
    }

    .remove-btn:hover {
        background: var(--color-danger-soft);
    }

    .asset-item {
        background: var(--color-surface-soft);
        border: 1px solid var(--color-border);
        border-radius: var(--radius-sm);
        color: var(--color-text-soft);
    }

    .file-name {
        color: var(--color-text);
    }

    .footer {
        margin-top: 12px;
        padding-top: 12px;
    }

    .btn-cancel,
    .btn-save,
    .mini-btn,
    .sub-tab {
        border-radius: var(--radius-sm);
        transition:
            background var(--transition-fast),
            border-color var(--transition-fast),
            color var(--transition-fast),
            transform var(--transition-fast),
            box-shadow var(--transition-fast);
    }

    .btn-cancel,
    .mini-btn {
        border: 1px solid var(--color-border);
        background: rgba(255, 255, 255, 0.9);
        color: var(--color-text-soft);
    }

    .btn-cancel:hover,
    .mini-btn:hover {
        background: var(--color-hover);
        border-color: var(--color-border-strong);
        color: var(--color-text);
        box-shadow: var(--shadow-sm);
    }

    .style-manager {
        border-color: var(--color-border);
        border-radius: var(--radius-lg);
        background: rgba(255, 255, 255, 0.78);
        box-shadow: var(--shadow-sm);
    }

    .style-sub-tabs {
        gap: 6px;
        padding: 8px 8px 0;
        background: rgba(246, 249, 252, 0.9);
        border-bottom-color: var(--color-border);
    }

    .sub-tab {
        border: 1px solid transparent;
        border-bottom: 0;
        background: transparent;
        color: var(--color-text-soft);
    }

    .sub-tab:hover {
        background: var(--color-hover);
    }

    .sub-tab.active {
        background: #fff;
        border-color: var(--color-border);
        color: var(--color-accent-deep);
        box-shadow: 0 -6px 16px rgba(23, 36, 52, 0.05);
    }

    .modified-dot {
        color: var(--color-warning);
    }

    .css-editor {
        font-family: var(--font-code);
        background: linear-gradient(180deg, #ffffff, var(--color-surface-soft));
        box-shadow: inset 0 1px 3px rgba(23, 36, 52, 0.06);
    }

    ::-webkit-scrollbar-thumb {
        background: linear-gradient(180deg, #bacbda, #93a8bb);
        border-radius: 999px;
    }
</style>
