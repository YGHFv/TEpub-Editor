<script lang="ts">
    let {
        wordWrap = $bindable(),
        showWhitespace = $bindable(),
        showLineBreaks = $bindable(),
        clearHistoryOnSave = $bindable(),
        subsetFonts = $bindable(),
        wordCountMaxThreshold = $bindable(),
    }: {
        wordWrap: boolean;
        showWhitespace: boolean;
        showLineBreaks: boolean;
        clearHistoryOnSave: boolean;
        subsetFonts: boolean;
        wordCountMaxThreshold: number;
    } = $props();
</script>

<!-- 显示开关卡片列表：样式全部来自全局 app.css 的 .settings-shell 系列规则，组件本身无 scoped CSS -->
<div class="settings-section display-settings-panel">
    <div class="section-title">显示设置</div>
    <div class="section-hint">
        把常用开关收成一列卡片，减少左右拉扯，和书库设置页保持同一套节奏。
    </div>
    <div class="settings-toggle-list">
        <label class="settings-toggle-card" for="wordWrap">
            <div class="settings-toggle-copy">
                <span class="settings-toggle-title">自动换行</span>
                <span class="settings-toggle-note"
                    >编辑时按容器宽度自动换行，减少横向滚动。</span
                >
            </div>
            <input id="wordWrap" type="checkbox" bind:checked={wordWrap} />
        </label>
        <label class="settings-toggle-card" for="showWhitespace">
            <div class="settings-toggle-copy">
                <span class="settings-toggle-title">显示空格</span>
                <span class="settings-toggle-note"
                    >把空格可视化，便于清理多余缩进和格式噪点。</span
                >
            </div>
            <input
                id="showWhitespace"
                type="checkbox"
                bind:checked={showWhitespace}
            />
        </label>
        <label class="settings-toggle-card" for="showLineBreaks">
            <div class="settings-toggle-copy">
                <span class="settings-toggle-title">显示换行符</span>
                <span class="settings-toggle-note"
                    >把段落换行位置标出来，排查断行和章节格式更直接。</span
                >
            </div>
            <input
                id="showLineBreaks"
                type="checkbox"
                bind:checked={showLineBreaks}
            />
        </label>
        <label class="settings-toggle-card" for="clh">
            <div class="settings-toggle-copy">
                <span class="settings-toggle-title">保存清空撤销</span>
                <span class="settings-toggle-note"
                    >保存成功后重置当前撤销栈，避免历史状态和新内容混在一起。</span
                >
            </div>
            <input id="clh" type="checkbox" bind:checked={clearHistoryOnSave} />
        </label>
        <label class="settings-toggle-card" for="subsetFonts">
            <div class="settings-toggle-copy">
                <span class="settings-toggle-title">字体子集化</span>
                <span class="settings-toggle-note"
                    >导出时只保留用到的字形，通常能进一步压缩 EPUB 体积。</span
                >
            </div>
            <input
                id="subsetFonts"
                type="checkbox"
                bind:checked={subsetFonts}
            />
        </label>
    </div>
</div>
<!-- 撤销开关 -->
<div class="set-row" style="display: none;">
    <label for="wth" style="display: none;">单章字数检查:</label>
    <input
        id="wth"
        type="hidden"
        bind:value={wordCountMaxThreshold}
        style="width: 80px;"
    />
</div>
