<script lang="ts">
    import { onMount } from "svelte";
    import { invoke } from "@tauri-apps/api/core";
    import { listen, emit } from "@tauri-apps/api/event";
    import { getCurrentWindow } from "@tauri-apps/api/window";
    import CustomSelect from "$lib/CustomSelect.svelte";

    let activeTab = "meta";

    type TemplateRepository = {
        id: string;
        name: string;
        url: string;
        branch: string;
        last_synced?: string;
    };
    type RemoteTemplate = {
        id: string;
        name: string;
        version?: string;
        description?: string;
        path: string;
        preview?: string;
        category?: string;
    };
    type EpubAsset = {
        name: string;
        path: string;
        category: string;
        role?: string;
        slot_label?: string;
    };
    type AssetSlot = {
        role: string;
        label: string;
        type: string;
        placement: string;
        selector?: string;
    };

    let metadata = {
        publisher: "",
        uuid: "",
        md5: "",
        styles: {
            "main.css": "",
            "font.css": "",
        },
        assets: [] as EpubAsset[],
    };
    let customMetadata: { key: string; value: string }[] = [];

    let templateRepositories: TemplateRepository[] = [];
    let selectedRepositoryId = "";
    let remoteTemplates: RemoteTemplate[] = [];
    let repoName = "";
    let repoUrl = "";
    let repoBranch = "main";
    let templateRepoBusy = false;
    let templateRepoMessage = "";

    function parseAssetSlots(css: string): AssetSlot[] {
        const slots: AssetSlot[] = [];
        const seen = new Set<string>();
        const re = /@tepub-asset-slot\s+([A-Za-z][\w-]*)([^*]*)/g;
        let match: RegExpExecArray | null;
        while ((match = re.exec(css || ""))) {
            const role = match[1];
            const attrs = match[2] || "";
            const getAttr = (name: string) => attrs.match(new RegExp(`${name}=["']([^"']+)["']`, "i"))?.[1] || "";
            const type = getAttr("type") || "image";
            if (type !== "image" || seen.has(role)) continue;
            seen.add(role);
            slots.push({
                role,
                type,
                label: getAttr("label") || role,
                placement: getAttr("placement") || "manual",
                selector: getAttr("selector") || "",
            });
        }
        return slots;
    }

    $: imageAssetSlots = parseAssetSlots(metadata.styles["main.css"] || "");

    async function loadTemplateRepositories() {
        try {
            templateRepositories = await invoke<TemplateRepository[]>("list_epub_template_repositories");
            if (!selectedRepositoryId && templateRepositories.length) {
                selectedRepositoryId = templateRepositories[0].id;
            }
        } catch (err) {
            templateRepoMessage = `读取模板仓库失败：${err}`;
        }
    }

    async function addTemplateRepository() {
        if (!repoUrl.trim()) {
            templateRepoMessage = "请输入 GitHub 仓库地址";
            return;
        }
        templateRepoBusy = true;
        templateRepoMessage = "正在添加仓库...";
        try {
            templateRepositories = await invoke<TemplateRepository[]>("add_epub_template_repository", {
                name: repoName.trim(),
                url: repoUrl.trim(),
                branch: repoBranch.trim() || "main",
            });
            selectedRepositoryId = templateRepositories[templateRepositories.length - 1]?.id || "";
            repoName = "";
            repoUrl = "";
            repoBranch = "main";
            templateRepoMessage = "仓库已添加";
        } catch (err) {
            templateRepoMessage = `添加模板仓库失败：${err}`;
        } finally {
            templateRepoBusy = false;
        }
    }

    async function syncTemplateRepository() {
        if (!selectedRepositoryId) {
            templateRepoMessage = "请先选择模板仓库";
            return;
        }
        templateRepoBusy = true;
        templateRepoMessage = "正在同步模板索引...";
        try {
            const index = await invoke<{ templates: RemoteTemplate[] }>("sync_epub_template_repository", {
                repositoryId: selectedRepositoryId,
            });
            remoteTemplates = index.templates || [];
            await loadTemplateRepositories();
            templateRepoMessage = `已同步 ${remoteTemplates.length} 个模板`;
        } catch (err) {
            templateRepoMessage = `同步模板仓库失败：${err}`;
        } finally {
            templateRepoBusy = false;
        }
    }

    async function installRemoteTemplate(template: RemoteTemplate) {
        if (!selectedRepositoryId) return;
        templateRepoBusy = true;
        templateRepoMessage = `正在安装 ${template.name}...`;
        try {
            const result = await invoke<{ local_path: string; file_count: number }>("install_remote_epub_template", {
                repositoryId: selectedRepositoryId,
                template,
            });
            templateRepoMessage = `已安装到 ${result.local_path}，共 ${result.file_count} 个文件`;
        } catch (err) {
            templateRepoMessage = `安装模板失败：${err}`;
        } finally {
            templateRepoBusy = false;
        }
    }

    onMount(() => {
        let unlistenFn: (() => void) | undefined;

        const init = async () => {
            const unlisten = await listen("init-metadata", (event: any) => {
                const { meta, custom } = event.payload;
                metadata.publisher = meta.publisher || "";
                metadata.uuid = meta.uuid || "";
                metadata.md5 = meta.md5 || "";
                metadata.styles = {
                    "main.css": meta.styles?.["main.css"] || "",
                    "font.css": meta.styles?.["font.css"] || "",
                };
                metadata.assets = meta.assets || [];
                customMetadata = [...(custom || [])];
            });
            unlistenFn = unlisten;
            await emit("metadata-window-ready");
            await loadTemplateRepositories();
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

    function assetSlotFileStem(role: string) {
        return role
            .replace(/([a-z0-9])([A-Z])/g, "$1-$2")
            .replace(/[_\s]+/g, "-")
            .toLowerCase();
    }

    function updateSlotCssReference(slot: AssetSlot, fileName: string) {
        if (!slot?.role || !metadata.styles["main.css"]) return;
        const ext = fileName.split(".").pop()?.toLowerCase();
        if (!ext) return;
        const stem = assetSlotFileStem(slot.role);
        const escapedStem = stem.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
        const re = new RegExp(`\\.\\./Images/${escapedStem}\\.[A-Za-z0-9]+`, "g");
        metadata.styles["main.css"] = metadata.styles["main.css"].replace(re, `../Images/${stem}.${ext}`);
    }

    async function addAsset(category: string, slot?: AssetSlot) {
        try {
            const { open } = await import("@tauri-apps/plugin-dialog");
            const selected = await open({
                multiple: !slot,
                filters: category === "fonts"
                    ? [{ name: "Fonts", extensions: ["ttf", "otf", "woff", "woff2"] }]
                    : category === "images"
                        ? [{ name: "Images", extensions: ["jpg", "jpeg", "png", "gif", "svg"] }]
                        : [],
            });

            if (selected) {
                const paths = Array.isArray(selected) ? selected : [selected];
                const newAssets = paths.map((p) => ({
                    name: (p as string).split(/[/\\]/).pop() || "",
                    path: p as string,
                    category,
                    ...(slot ? { role: slot.role, slot_label: slot.label } : {}),
                }));
                metadata.assets = [...metadata.assets, ...newAssets];
                if (slot && newAssets[0]?.name) {
                    updateSlotCssReference(slot, newAssets[0].name);
                }
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
        <button class="tab-btn" class:active={activeTab === 'files'} on:click={() => activeTab = 'files'}>文件</button>
    </div>

    {#if activeTab === 'meta'}
        <div class="tab-content scroll-p compact-top">
            <div class="form-section">
                <div class="input-group">
                    <label for="publisher">出版社</label>
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
                            <div class="asset-add-actions">
                                {#if cat === 'images'}
                                    {#each imageAssetSlots as slot}
                                        <button class="mini-btn" on:click={() => addAsset(cat, slot)}>添加{slot.label}</button>
                                    {/each}
                                {/if}
                                <button class="add-btn small" on:click={() => addAsset(cat)}>+</button>
                            </div>
                        </div>
                        <div class="asset-list">
                            {#each metadata.assets.filter(a => a.category === cat) as asset, idx}
                                <div class="asset-item">
                                    <span class="file-name" title={asset.path}>
                                        {asset.name}
                                        {#if asset.role}<em>{asset.slot_label || asset.role}</em>{/if}
                                    </span>
                                    <button class="remove-btn" on:click={() => removeAsset(metadata.assets.indexOf(asset))}>×</button>
                                </div>
                            {/each}
                            {#if metadata.assets.filter(a => a.category === cat).length === 0}
                                <p class="empty-hint">未添加{cat === 'fonts' ? '字体' : cat === 'images' ? '图片' : '文件'}</p>
                            {/if}
                        </div>
                    </div>
                {/each}
                <div class="template-repo-panel">
                    <div class="section-header">
                        <h3>GitHub 模板仓库</h3>
                        <button class="mini-btn" on:click={loadTemplateRepositories} disabled={templateRepoBusy}>刷新</button>
                    </div>
                    <div class="repo-form">
                        <input bind:value={repoName} placeholder="仓库名称" disabled={templateRepoBusy} />
                        <input bind:value={repoUrl} placeholder="https://github.com/owner/repo" disabled={templateRepoBusy} />
                        <input bind:value={repoBranch} placeholder="分支 main" disabled={templateRepoBusy} />
                        <button class="mini-btn" on:click={addTemplateRepository} disabled={templateRepoBusy}>添加</button>
                    </div>
                    <div class="repo-actions">
                        <CustomSelect
                            value={selectedRepositoryId}
                            options={[{ value: "", label: "选择模板仓库" }, ...templateRepositories.map((repo) => ({ value: repo.id, label: `${repo.name} / ${repo.branch}` }))]}
                            disabled={templateRepoBusy || !templateRepositories.length}
                            ariaLabel="模板仓库"
                            on:change={(event) => (selectedRepositoryId = event.detail)}
                        />
                        <button class="mini-btn" on:click={syncTemplateRepository} disabled={templateRepoBusy || !selectedRepositoryId}>同步索引</button>
                    </div>
                    {#if templateRepoMessage}
                        <p class="repo-message">{templateRepoMessage}</p>
                    {/if}
                    <div class="remote-template-list">
                        {#each remoteTemplates as tpl}
                            <div class="remote-template-item">
                                <div>
                                    <strong>{tpl.name}</strong>
                                    <span>{tpl.version || tpl.id}</span>
                                    {#if tpl.description}<p>{tpl.description}</p>{/if}
                                </div>
                                <button class="mini-btn" on:click={() => installRemoteTemplate(tpl)} disabled={templateRepoBusy}>安装</button>
                            </div>
                        {/each}
                    </div>
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

    .asset-add-actions {
        display: flex;
        align-items: center;
        gap: 8px;
        flex-shrink: 0;
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

    /* 璧勬簮绠＄悊鍣ㄦ牱寮?*/
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

    .template-repo-panel {
        display: flex;
        flex-direction: column;
        gap: 10px;
        margin-top: 6px;
        padding-top: 14px;
        border-top: 1px solid var(--color-border, #e2e8f0);
    }

    .repo-form,
    .repo-actions {
        display: grid;
        grid-template-columns: minmax(90px, 0.8fr) minmax(170px, 1.5fr) minmax(80px, 0.6fr) auto;
        gap: 8px;
        align-items: center;
    }

    .repo-actions {
        grid-template-columns: minmax(0, 1fr) auto;
    }

    .repo-form input {
        min-width: 0;
        padding: 8px 10px;
        border: 1px solid var(--color-border, #d8e0e8);
        border-radius: var(--radius-sm, 6px);
        background: rgba(255, 255, 255, 0.92);
        color: var(--color-text, #172434);
        font: inherit;
        font-size: 0.84rem;
    }

    .repo-message {
        margin: 0;
        color: var(--color-text-soft, #708090);
        font-size: 0.8rem;
    }

    .remote-template-list {
        display: grid;
        gap: 8px;
    }

    .remote-template-item {
        display: grid;
        grid-template-columns: minmax(0, 1fr) auto;
        gap: 10px;
        align-items: center;
        padding: 10px;
        border: 1px solid var(--color-border, #e2e8f0);
        border-radius: var(--radius-sm, 6px);
        background: rgba(255, 255, 255, 0.78);
    }

    .remote-template-item strong {
        display: block;
        color: var(--color-text, #172434);
        font-size: 0.9rem;
    }

    .remote-template-item span,
    .remote-template-item p {
        display: block;
        margin: 2px 0 0;
        color: var(--color-text-soft, #708090);
        font-size: 0.78rem;
    }

    .file-name {
        flex: 1;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        color: #2c3e50;
        margin-right: 10px;
    }

    .repo-actions :global(.custom-select) {
        min-width: 0;
    }

    .file-name em {
        display: inline-block;
        margin-left: 8px;
        padding: 1px 6px;
        border-radius: 999px;
        background: rgba(52, 152, 219, 0.12);
        color: #2474a6;
        font-style: normal;
        font-size: 0.72rem;
        vertical-align: 1px;
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

    .input-group label,
    .asset-group h3,
    .empty-hint {
        color: var(--color-text-soft);
    }

    .section-header h3 {
        color: var(--color-text);
        font-weight: 800;
        letter-spacing: 0.01em;
    }

    .input-group input,
    .custom-row {
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
    .custom-row input:focus {
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
    .mini-btn {
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

    ::-webkit-scrollbar-thumb {
        background: linear-gradient(180deg, #bacbda, #93a8bb);
        border-radius: 999px;
    }
</style>
