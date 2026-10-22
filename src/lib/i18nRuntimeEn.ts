// English translations for runtime-generated UI messages.
// Keep placeholder names identical to the Simplified Chinese source strings.

export const RUNTIME_EN_MESSAGES: Record<string, string> = {
  "，失败 {count} 本：": ", {count} failed:",
  "... 还有 {count} 条未显示": "... {count} more not shown",
  "。设置会按账号独立保存。":
    ". Settings are saved separately for each account.",
  "{label} XML 格式无效": "{label} is not valid XML",
  "{tool} 批量处理": "{tool} batch processing",
  "{tool} 批量窗口已打开": "The {tool} batch window is already open",
  "{tool} 已打开": "{tool} is already open",
  "{tool} 已完成": "{tool} completed",
  "安装模板失败：{error}": "Failed to install template: {error}",
  "按段落实际使用的正文字体分别随机打乱汉字映射；标点、标题和字体不支持的文字保持原样，加密前自动精简全部内嵌字体。":
    "Randomizes Han character mappings separately for each body font actually used by paragraphs. Punctuation, headings, and characters unsupported by the font remain unchanged; all embedded fonts are trimmed automatically before encryption.",
  "保存设置失败：{error}": "Failed to save settings: {error}",
  "保存失败: {error}": "Save failed: {error}",
  "不支持的字体格式：{path}": "Unsupported font format: {path}",
  仓库已添加: "Repository added",
  "成功添加 {added} 本，{failed} 本失败:":
    "Added {added} books; {failed} failed:",
  "处理结果会显示在这里，文本修改后可用撤销返回。":
    "Results appear here. You can use Undo to revert text changes.",
  "处理失败，请点击“查看日志”了解详情。":
    "Processing failed. Select “View log” for details.",
  "处理失败：{error}": "Processing failed: {error}",
  "处理完成：下载 {downloaded} 张，{failed} 张自动重试后仍失败。":
    "Finished: downloaded {downloaded}; {failed} still failed after automatic retries.",
  "处理完成：下载 {downloaded} 张，替换 {replaced} 处引用。":
    "Finished: downloaded {downloaded} and replaced {replaced} references.",
  "纯前端通道均受跨域限制，正在使用服务端保底下载…":
    "All browser-only channels are blocked by CORS; falling back to server downloads…",
  "打开 EPUB 样式库失败": "Failed to open the EPUB Style Library",
  "打开 EPUB 样式库失败: {error}":
    "Failed to open the EPUB Style Library: {error}",
  "打开 EPUB 在线图片下载失败": "Failed to open EPUB Online Image Download",
  "打开 EPUB 在线图片下载失败: {error}":
    "Failed to open EPUB Online Image Download: {error}",
  "打开工具箱失败: {error}": "Failed to open the toolbox: {error}",
  打开失败: "Failed to open",
  "打开失败: {error}": "Failed to open: {error}",
  "打开文件对话框失败：{error}": "Failed to open the file picker: {error}",
  "打开文件位置失败: {error}": "Failed to show the file location: {error}",
  当前没有可校对的文本: "There is no text to proofread.",
  "当前没有可制作 EPUB 的文本": "There is no text available to create an EPUB.",
  当前没有可转换的文本: "There is no text to convert.",
  "当前文本已变化，无法安全撤销该审批":
    "The current text has changed, so this approval cannot be undone safely.",
  "当前文本已变化，无法安全恢复该审批":
    "The current text has changed, so this approval cannot be restored safely.",
  "导出 EPUB 失败": "Failed to export EPUB",
  导入文本失败: "Failed to import text",
  "第 {index} 张下载失败，{delay} 秒后慢速重试：{reason}":
    "Image {index} failed to download. Retrying slowly in {delay} seconds: {reason}",
  "读取 EPUB 失败：{error}": "Failed to read EPUB: {error}",
  "读取模板仓库失败：{error}":
    "Failed to read the template repository: {error}",
  "发现 {urls} 个在线图片地址，共 {references} 处引用。":
    "Found {urls} online image URLs across {references} references.",
  封面与横幅工具已打开: "Cover and Banner tool is already open",
  "服务端保底：{error}": "Server fallback: {error}",
  "复制失败：{error}": "Copy failed: {error}",
  该审批结果当前已处于应用状态:
    "This approval result is already in the applied state.",
  该审批结果已撤销: "This approval result has been undone.",
  "更换封面失败: {error}": "Failed to change the cover: {error}",
  关闭注释: "Close note",
  "恢复 EPUB 中经过字体映射混淆的正文；旧式私用区加密可自动识别，新式正文字形置换需使用同版本明文 TXT 对齐。":
    "Restores body text obfuscated by font mapping. Legacy private-use-area encryption is detected automatically; newer body-glyph substitution requires a plain-text TXT file from the same edition for alignment.",
  "加密前自动精简 {count} 个字体":
    "Automatically trim {count} fonts before encryption",
  "加载书库失败：{error}": "Failed to load the library: {error}",
  "检测到主体为繁体，已跳过繁体转简体查找，避免误扫整本":
    "The text is mainly Traditional Chinese, so the Traditional-to-Simplified scan was skipped to avoid scanning the whole book incorrectly.",
  "检测到主体为简体，已跳过简体转繁体查找，避免误扫整本":
    "The text is mainly Simplified Chinese, so the Simplified-to-Traditional scan was skipped to avoid scanning the whole book incorrectly.",
  "结果：{errors} 错误 / {warnings} 警告":
    "Result: {errors} errors / {warnings} warnings",
  "旧智能校对缓存已跳过：当前文本内容已变化":
    "Previous proofreading cache was skipped because the text has changed.",
  可用私用区字符不足: "Not enough available private-use-area characters",
  "跨域图片画布不可读取：{error}":
    "The cross-origin image canvas cannot be read: {error}",
  "例如：标签优先使用书库现有分类；简介不要剧透。":
    "For example: prefer existing library categories for tags, and avoid spoilers in the description.",
  浏览器无法创建图片转换画布:
    "The browser could not create a canvas for image conversion",
  浏览器无法导出转换后的图片:
    "The browser could not export the converted image",
  "浏览器直读受限，正在尝试前端图片解码转换…":
    "Direct browser access is restricted; trying in-browser image decoding and conversion…",
  浏览器直连生图失败: "Direct browser image generation failed",
  "浏览器直连生图失败，通常是该 API 不允许跨域请求。Docker/正式部署请更新后走后端代理。":
    "Direct browser image generation failed, usually because the API does not allow cross-origin requests. For Docker or production deployments, update and use the backend proxy.",
  "浏览器直连失败，尝试后端代理...":
    "Direct browser connection failed; trying the backend proxy...",
  路径大小写不一致: "Path casing mismatch",
  "没有发现 HTTP/HTTPS 在线图片。": "No HTTP/HTTPS online images were found.",
  "没有发现候选修改。": "No candidate changes were found.",
  "没有发现候选转换。": "No candidate conversions were found.",
  没有可应用的修改: "No changes are available to apply",
  没有可预览的标题: "No titles are available to preview",
  没有匹配到可重排标题: "No headings matching the reorder rule were found",
  没有匹配项: "No matches",
  没有识别到目录项: "No table-of-contents entries were recognized",
  "没有选择需要下载的在线图片。":
    "No online images were selected for download.",
  没有选中要应用的修改: "No changes were selected to apply",
  没有找到可转换内容: "No convertible content was found",
  命名冲突重试次数过多: "Too many naming conflict retries",
  "目录标题已是当前顺序。":
    "The table-of-contents titles are already in the current order.",
  目录结构: "Directory structure",
  目录扫描失败: "Table-of-contents scan failed",
  目录正则已保存: "Table-of-contents regular expression saved",
  "内嵌字体解析失败：{errors}": "Failed to parse embedded fonts: {errors}",
  内嵌字体未发现可加密汉字:
    "No encryptable Han characters were found in the embedded fonts",
  内嵌字体无需裁剪或当前字体格式无法子集化:
    "The embedded fonts do not need trimming, or their format cannot be subset",
  "内嵌字体已是精简状态，无需重复子集化":
    "The embedded fonts are already trimmed and do not need subsetting again",
  批量处理失败: "Batch processing failed",
  "批量处理失败: {error}": "Batch processing failed: {error}",
  "前端图片转换：{error}": "In-browser image conversion: {error}",
  "前端直读：{error}": "Direct browser read: {error}",
  "请输入 GitHub 仓库地址": "Enter a GitHub repository URL",
  "请拖入 EPUB 文件。": "Drop an EPUB file here.",
  "请先补全 API 地址、Key 和模型名":
    "Enter the API URL, key, and model name first.",
  请先打开文本文件: "Open a text file first.",
  请先选择模板仓库: "Select a template repository first.",
  "请选择普通插图样式。": "Select a regular illustration style.",
  "请选择图片文件。": "Select an image file.",
  "全屏封面 {width}x{height}": "Full-screen cover {width}x{height}",
  "缺少 container.xml": "Missing container.xml",
  "缺少 mimetype": "Missing mimetype",
  "缺少 OPF": "Missing OPF",
  缺少生图模型或参考图: "An image model or reference image is required",
  "扫描整本 EPUB 正文实际使用的 Unicode 码点，裁剪未使用字形以减小内嵌字体体积。":
    "Scans the Unicode code points actually used in the EPUB body and removes unused glyphs to reduce embedded font size.",
  "设置失败: {error}": "Failed to save settings: {error}",
  生成图片失败: "Image generation failed",
  "生成完成：{size}": "Generated: {size}",
  "生图接口返回 {status}: {detail}":
    "Image generation API returned {status}: {detail}",
  生图接口返回中没有图片数据:
    "The image generation API response contained no image data",
  "是否立即按新规则重命名书库中的 {count} 本本地书文件？\n（仅会重命名复制进书库的副本，引用模式下的原文件不会动）":
    "Rename {count} local book files in the library using the new rule now?\n(Only copies stored in the library are renamed; source files in reference mode are untouched.)",
  "手动重试完成：仍有 {count} 张失败，可再次重试。":
    "Manual retry finished: {count} images still failed. You can retry again.",
  "手动重试未完成，请点击“查看日志”了解详情。":
    "The manual retry did not finish. Select “View log” for details.",
  "添加模板仓库失败：{error}": "Failed to add the template repository: {error}",
  "添加失败 ({count}):": "Add failed ({count}):",
  "同步模板仓库失败：{error}":
    "Failed to sync the template repository: {error}",
  头图样图载入失败: "Failed to load the header sample image",
  "图片超过 32 MB 限制": "The image exceeds the 32 MB limit",
  "图片服务器返回 {status} {statusText}":
    "Image server returned {status} {statusText}",
  图片服务器未允许匿名跨域图片读取:
    "The image server does not allow anonymous cross-origin image reads",
  图片加载失败: "Failed to load image",
  图片没有有效尺寸: "The image has no valid dimensions",
  图片文件: "Image file",
  图片下载失败: "Image download failed",
  图片响应为空: "The image response was empty",
  "图片像素尺寸过大，无法在浏览器中安全转换":
    "The image pixel dimensions are too large to convert safely in the browser",
  图片元素加载超时: "Timed out while loading the image element",
  图片载入失败: "Failed to load image",
  图书文件: "Book file",
  "拖放添加图书失败：{error}": "Failed to add books by drag and drop: {error}",
  未发现: "Not found",
  未发现可恢复的字体混淆正文:
    "No font-obfuscated body text was found to restore",
  未发现可加密的正文段落: "No body paragraphs were found that can be encrypted",
  "未发现可用字体映射；请选择与 EPUB 对应的明文 TXT 后重试":
    "No usable font mapping was found. Select the matching plain-text TXT file and try again.",
  "未发现明显结构问题。": "No obvious structural issues were found.",
  未发现需要修改的内容: "No content needs to be changed",
  "未能根据 EPUB 样式确定内嵌字体实际使用的文字":
    "Could not determine which text the embedded fonts actually use from the EPUB styles",
  未配置生图模型: "No image generation model is configured",
  未选择生图模型: "No image generation model is selected",
  未找到对应审批结果: "The corresponding approval result was not found",
  未找到可加密的正文段落汉字:
    "No encryptable Han characters were found in body paragraphs",
  未找到可加密的正文汉字:
    "No encryptable Han characters were found in the body",
  未找到已保存的生图模型: "The saved image generation model could not be found",
  未知类型: "Unknown type",
  "文本已在校对后发生变化，请重新运行智能校对":
    "The text changed after proofreading. Run smart proofreading again.",
  "文件：{path}": "File: {path}",
  文件名不能为空: "The file name cannot be empty",
  "问题列表：": "Issues:",
  无法创建头图样图画布: "Could not create a canvas for the header sample image",
  "无法读取 EPUB：{error}": "Could not read EPUB: {error}",
  "响应不是支持的 JPG、PNG、WebP 或 GIF 图片":
    "The response is not a supported JPG, PNG, WebP, or GIF image",
  "响应不是支持的 JPG、PNG、WebP 或 GIF 图片（{type}）":
    "The response is not a supported JPG, PNG, WebP, or GIF image ({type})",
  "选择目录失败：{error}": "Failed to select a folder: {error}",
  选择生图模型: "Select image generation model",
  压缩包无效: "Invalid archive",
  "移除图书失败: {error}": "Failed to remove the book: {error}",
  "已安装到 {path}，共 {count} 个文件": "Installed to {path} ({count} files)",
  "已保存 {path}": "Saved {path}",
  已创建历史快照: "History snapshot created",
  "已存在同名: {name}": "An item with the same name already exists: {name}",
  "已导出 EPUB：{path}": "Exported EPUB: {path}",
  "已导出后端副本：{path}": "Exported backend copy: {path}",
  已定位到可转换内容: "Located convertible content",
  已定位到匹配位置: "Located the matching position",
  已定位到原标题位置: "Located the original heading",
  已定位到智能校对建议: "Located the smart proofreading suggestion",
  "已返回目录与元数据编辑。":
    "Returned to table-of-contents and metadata editing.",
  已恢复自动页面: "Automatic page mode restored",
  已回退到历史版本: "Reverted to the historical version",
  已切换为电脑页面: "Switched to desktop page mode",
  "已清空 AI 参考图": "Cleared the AI reference image",
  已清空封面处理参考图: "Cleared the cover-processing reference image",
  已清空头图样图和处理图片: "Cleared the header sample and processed image",
  已刷新生图模型: "Refreshed image generation models",
  "已同步 {count} 个模板": "Synced {count} templates",
  "已完成 {count} 个 EPUB 文件的诊断。":
    "Finished diagnosing {count} EPUB files.",
  "已下载 {current} / {total}": "Downloaded {current} / {total}",
  "已载入参考图 {name}": "Loaded reference image {name}",
  "已载入处理图片 {name}": "Loaded image to process: {name}",
  "已载入头图样图 {name}，长边已规范为 {edge}px（{width}x{height}）":
    "Loaded header sample {name}; normalized its long edge to {edge}px ({width}x{height})",
  "已重命名 {renamed} 本，跳过 {skipped} 本":
    "Renamed {renamed} books and skipped {skipped}",
  "已准备 EPUB 制作数据": "EPUB creation data is ready",
  引用大小写不一致: "Reference casing mismatch",
  引用目标缺失: "Referenced target is missing",
  "诊断失败：{error}": "Diagnosis failed: {error}",
  "正文共 {total} 个汉字，但内嵌字体仅覆盖 {covered} 个；该 EPUB 的正文主要使用未嵌入的系统字体，无法安全完成整书字体加密。请先嵌入完整正文字体后重试。":
    "The body contains {total} Han characters, but the embedded fonts cover only {covered}. This EPUB mainly uses system fonts that are not embedded, so whole-book font encryption cannot be completed safely. Embed complete body fonts and try again.",
  正文字体中未找到足够的可加密汉字:
    "Not enough encryptable Han characters were found in the body fonts",
  "正在安装 {name}...": "Installing {name}...",
  "正在查找可转换内容...": "Searching for convertible content...",
  "正在尝试浏览器直连生图...":
    "Trying direct image generation from the browser...",
  "正在打开 EPUB 样式库...": "Opening the EPUB Style Library...",
  "正在打开 EPUB 在线图片下载...": "Opening EPUB Online Image Download...",
  "正在打开 Send to Kindle": "Opening Send to Kindle",
  "正在打开日志...": "Opening log...",
  "正在读取校对日志...": "Reading proofreading logs...",
  "正在分析正文与内嵌字体…": "Analyzing body text and embedded fonts…",
  "正在分析字体映射并恢复正文…":
    "Analyzing font mapping and restoring body text…",
  "正在更新 XHTML 与 OPF": "Updating XHTML and OPF",
  "正在扫描目录...": "Scanning table of contents...",
  "正在扫描章节中的在线图片…": "Scanning chapters for online images…",
  "正在生成{target}...": "Generating {target}...",
  "正在收集正文码点并裁剪字体…":
    "Collecting body code points and trimming fonts…",
  "正在添加仓库...": "Adding repository...",
  "正在停止智能校对...": "Stopping smart proofreading...",
  "正在同步模板索引...": "Syncing template index...",
  "正在下载 {current} / {total}": "Downloading {current} / {total}",
  "正在诊断 {current} / {total}：{name}":
    "Diagnosing {current} / {total}: {name}",
  "正在重试 {count} 张失败图片…": "Retrying {count} failed images…",
  "正在转换繁简，请稍候...":
    "Converting between Simplified and Traditional Chinese...",
  "制作 EPUB 失败": "Failed to create EPUB",
  智能匹配额外要求: "Additional smart-match instructions",
  "智能匹配失败：{error}": "Smart matching failed: {error}",
  智能匹配完成: "Smart matching complete",
  "智能匹配完成：{reason}": "Smart matching complete: {reason}",
  "重命名失败: {error}": "Rename failed: {error}",
  "重试成功：累计下载 {downloaded} 张，替换 {replaced} 处引用。":
    "Retry succeeded: downloaded {downloaded} images in total and replaced {replaced} references.",
  "重试失败：{error}": "Retry failed: {error}",
  "重新提取失败: {error}": "Failed to extract again: {error}",
  转繁体预览: "Traditional Chinese preview",
  "转换后的图片超过 32 MB 限制": "The converted image exceeds the 32 MB limit",
  转简体预览: "Simplified Chinese preview",
  "准备下载在线图片…": "Preparing to download online images…",
  资源读取失败: "Failed to read resource",
  资源未登记: "Resource is not listed in the manifest",
  "字体 cmap 未能写入私用区映射，无法完成字体加密":
    "The private-use-area mapping could not be written to the font cmap, so font encryption could not be completed",
  "字体加密完成：{fonts} 个正文字体分别随机映射 {characters} 个汉字；{subset}":
    "Font encryption complete: {fonts} body fonts now use separate randomized mappings for {characters} Han characters; {subset}",
  "字体加密完成：随机打乱 {count} 个正文汉字，仅更新 1 个正文字体":
    "Font encryption complete: randomized {count} body Han characters and updated 1 body font",
  "字体加密完成：映射 {mapped} 个汉字，更新 {fonts} 个字体":
    "Font encryption complete: mapped {mapped} Han characters and updated {fonts} fonts",
  "字体解密完成：恢复 {characters} 个字符，更新 {files} 个正文文件":
    "Font decryption complete: restored {characters} characters and updated {files} body files",
  "字体子集化完成：按实际样式裁剪 {fonts} 个字体，各字体合计保留 {points} 个码点，字体数据减少 {size} MB":
    "Font subsetting complete: trimmed {fonts} fonts based on actual styles, retained {points} code points in total, and reduced font data by {size} MB",
  "AI 生成参数已重置": "AI generation parameters were reset",
  "AI 生成失败": "AI generation failed",
  "container.xml 无法解析": "Could not parse container.xml",
  "EPUB 的 OPF 路径无效": "The EPUB OPF path is invalid",
  "EPUB 或 TXT 文本过短，无法可靠对齐":
    "The EPUB or TXT text is too short to align reliably",
  "EPUB 内未找到 HTML/XHTML 文件": "No HTML/XHTML files were found in the EPUB",
  "EPUB 内未找到 TTF、OTF、WOFF 或 WOFF2 字体":
    "No TTF, OTF, WOFF, or WOFF2 fonts were found in the EPUB",
  "EPUB 内未找到正文 HTML/XHTML 文件":
    "No body HTML/XHTML files were found in the EPUB",
  "EPUB 缺少 META-INF/container.xml":
    "The EPUB is missing META-INF/container.xml",
  "EPUB 文件": "EPUB file",
  "EPUB 样式库已打开": "EPUB Style Library is already open",
  "EPUB 阅读区域": "EPUB reading area",
  "EPUB 在线图片下载已打开": "EPUB Online Image Download is already open",
  "EPUB 诊断完成：{errors} 错误 / {warnings} 警告":
    "EPUB diagnosis complete: {errors} errors / {warnings} warnings",
  "EPUB 诊断未发现明显问题": "EPUB diagnosis found no obvious issues",
  "GIF 不能通过 Canvas 转换，否则会丢失动画":
    "GIF cannot be converted through Canvas without losing animation",
  "manifest 条目：{count}": "Manifest items: {count}",
  "manifest 无法解析": "Could not parse the manifest",
  "manifest 资源缺失": "Manifest resource is missing",
  "TXT 文件": "TXT file",
  "TXT 与 EPUB 对齐失败，请确认两者是同一版本":
    "TXT and EPUB alignment failed. Make sure both are from the same edition.",
  "ZIP 条目：{count}": "ZIP entries: {count}",
  已导入: "Imported",
  无结果: "No results",
  "运行时崩溃：\n{detail}": "Runtime crash:\n{detail}",
  "默认 API": "Default API",
  "新 API": "New API",
  "正在保存...": "Saving...",
  "已保存，工具箱设置与 TXT 编辑器设置已同步":
    "Saved. Toolbox and TXT Editor settings are now synchronized.",
  "保存失败：{error}": "Save failed: {error}",
  "文字 API": "Text API",
  "API 配置已保存，点击完成后写入设置。":
    "API configuration saved. Select Done to apply it to settings.",
  "API 配置已删除，点击完成后写入设置。":
    "API configuration deleted. Select Done to apply it to settings.",
  读取字体目录失败: "Failed to read the fonts folder",
  内置模板: "Built-in template",
  读取样式模板失败: "Failed to read style templates",
  当前模板: "Current template",
  "打开文件选择器失败：{error}": "Failed to open the file picker: {error}",
  "正在导入字体...": "Importing font...",
  "已导入字体：{name}": "Font imported: {name}",
  输入新的字体显示名: "Enter a new display name for the font",
  "正在重命名字体...": "Renaming font...",
  "已重命名为：{name}": "Renamed to: {name}",
  "确定删除字体“{name}”吗？": "Delete font “{name}”?",
  "正在删除字体...": "Deleting font...",
  "已删除字体：{name}": "Font deleted: {name}",
  "删除失败：{error}": "Delete failed: {error}",
  "正在导入样式模板...": "Importing style template...",
  "已导入模板：{name}": "Template imported: {name}",
  "正在保存当前模板...": "Saving current template...",
  "已保存到模板：{name}": "Saved to template: {name}",
  "正在恢复内置模板...": "Restoring the built-in template...",
  内置模板已恢复默认样式: "Built-in template restored to its default styles",
  "恢复失败：{error}": "Restore failed: {error}",
  整章移除风险高: "High risk of removing the entire chapter",
  "疑似表情/特殊符号": "Possible emoji or special symbol",
  括号内描述不删除: "Keep descriptions inside brackets",
  保留段首缩进: "Preserve paragraph indentation",
  "疑似人名/专名": "Possible person or proper name",
  疑似风格改写: "Possible style rewrite",
  "建议/{stage}：{chapter}": "Suggestion/{stage}: {chapter}",
  "原因：{reason}": "Reason: {reason}",
  "模型: {model}": "Model: {model}",
  "范围: {scope}": "Scope: {scope}",
  全书逐章: "Entire book, chapter by chapter",
  当前卷: "Current volume",
  当前章: "Current chapter",
  "时间: {time}": "Time: {time}",
  "日志文件: {path}": "Log file: {path}",
  "===== 日志 =====": "===== Log =====",
  "===== 当前人工建议 =====": "===== Current manual suggestions =====",
  暂无待人工审核建议: "No suggestions awaiting manual review",
  "已跳过旧校对缓存：当前文本内容已变化":
    "Previous proofreading cache skipped because the current text has changed",
  "已恢复 {count} 条智能校对建议":
    "Restored {count} smart proofreading suggestions",
  "日志已保存：{path}": "Log saved: {path}",
  "日志保存失败：{error}": "Failed to save log: {error}",
  "自动审批：{chapter}，待审 {count} 条":
    "Auto approval: {chapter}, {count} awaiting review",
  "自动审批完成：通过 {approved} 条，存疑 {pending} 条":
    "Auto approval complete: {approved} approved, {pending} uncertain",
  "自动审批失败，全部转人工审核：{error}":
    "Auto approval failed; all suggestions moved to manual review: {error}",
  "开始智能校对：{scope}，指针行 {line}，共 {count} 章":
    "Starting smart proofreading: {scope}, cursor line {line}, {count} chapters total",
  未知: "Unknown",
  "用户请求停止，结束剩余章节":
    "User requested stop; ending remaining chapters",
  "跳过超长章节 {current}/{total}：{title}（{count} 字）":
    "Skipping oversized chapter {current}/{total}: {title} ({count} characters)",
  "请求章节 {current}/{total}：{title}（发送 {count} 字）":
    "Requesting chapter {current}/{total}: {title} (sending {count} characters)",
  "响应 JSON 解析失败：{title}；{error}":
    "Failed to parse response JSON: {title}; {error}",
  "响应片段：{content}": "Response excerpt: {content}",
  "响应可能被截断，已采用 {count} 条完整建议":
    "The response may have been truncated; used {count} complete suggestions",
  返回结果: "Returned results",
  "章节失败：{title}；{error}": "Chapter failed: {title}; {error}",
  "忽略未匹配建议：{title} / {type} / {reason}":
    "Ignored unmatched suggestion: {title} / {type} / {reason}",
  未匹配: "Unmatched",
  "拦截高风险建议：{original} -> {replacement}（{reason}）":
    "Blocked high-risk suggestion: {original} -> {replacement} ({reason})",
  已拦截: "Blocked",
  "风险：{reason}": "Risk: {reason}",
  "自动应用：{chapter}，{count} 条标点/的地得修正":
    "Automatically applied: {chapter}, {count} punctuation/grammar fixes",
  "自动：{original} -> {replacement}（{reason}）":
    "Auto: {original} -> {replacement} ({reason})",
  自动应用: "Auto-applied",
  "审批通过自动应用：{chapter}，{count} 条":
    "Approval passed; auto-applied: {chapter}, {count} items",
  "审批通过：{original} -> {replacement}（{reason}）":
    "Approved: {original} -> {replacement} ({reason})",
  "AI 审批通过": "AI approved",
  待人工审核: "Awaiting manual review",
  "完成章节：{chapter}，返回 {returned} 条，本地 {local} 条，自动 {auto} 条，建议 {suggested} 条，拦截 {blocked} 条":
    "Completed chapter: {chapter}, {returned} returned, {local} local, {auto} automatic, {suggested} suggested, {blocked} blocked",
  "已自动应用 {count} 条标点/的地得修正":
    "Automatically applied {count} punctuation/grammar fixes",
  "已自动写入 {count} 条修正": "Automatically wrote {count} corrections",
  "自动修正未写入：原文已变化或未匹配":
    "Automatic corrections were not written: the source text changed or no longer matches",
  "已应用 {count} 条 AI 审批建议": "Applied {count} AI approval suggestions",
  "AI 审批": "AI approval",
  "已写入 AI 审批修正 {count} 条，可在审批页撤销":
    "Wrote {count} AI-approved corrections; they can be undone on the Approvals page",
  "AI 审批修正未写入：原文已变化或未匹配":
    "AI-approved corrections were not written: the source text changed or no longer matches",
  "已停止，找到 {count} 条建议": "Stopped; found {count} suggestions",
  "智能校对完成，找到 {count} 条建议":
    "Smart proofreading complete; found {count} suggestions",
  "，跳过 {count} 个超长章节": ", skipped {count} oversized chapters",
  "未返回具体原因（类型：{type}）":
    "No specific reason returned (type: {type})",
  未返回具体原因: "No specific reason returned",
  "修改后：整章移除": "Change: remove entire chapter",
  "修改后：{replacement}": "Change to: {replacement}",
  "类型：{type}": "Type: {type}",
  "置信度：{value}%": "Confidence: {value}%",
  "已应用 {count} 条智能校对建议":
    "Applied {count} smart proofreading suggestions",
  "原文已变化，未应用任何建议":
    "The source text changed, so no suggestions were applied",
  "人工应用：{applied} 条，剩余 {remaining} 条":
    "Manually applied {applied}; {remaining} remaining",
  "审批撤销失败：{chapter} / {row}，文本已变化":
    "Failed to undo approval: {chapter} / {row}; the text has changed",
  "审批撤销失败：{chapter} / {row}，重建文本未完全匹配":
    "Failed to undo approval: {chapter} / {row}; the rebuilt text did not match exactly",
  "审批撤销：{chapter} / {row}": "Approval undone: {chapter} / {row}",
  "审批恢复失败：{chapter} / {row}，文本已变化":
    "Failed to restore approval: {chapter} / {row}; the text has changed",
  "审批恢复失败：{chapter} / {row}，重建文本未完全匹配":
    "Failed to restore approval: {chapter} / {row}; the rebuilt text did not match exactly",
  "审批恢复：{chapter} / {row}": "Approval restored: {chapter} / {row}",
  已使用本地封面: "Using local cover",
  "已自动使用 {source} 封面": "Automatically used cover from {source}",
  "已使用《{title}》封面": "Using the cover for “{title}”",
  "自动应用失败，可手动选择其他封面":
    "Could not apply automatically; choose another cover manually",
  "应用封面失败，请换一个结果或使用本地图片":
    "Failed to apply the cover; try another result or use a local image",
  请先填写书名: "Enter a book title first",
  "正在搜索封面...": "Searching for covers...",
  没有找到可用封面: "No usable covers found",
  "找到 {count} 个封面": "Found {count} covers",
  "搜索封面失败：{error}": "Failed to search for covers: {error}",
  "打开高级选项失败: {error}": "Failed to open advanced options: {error}",
  "打开 EPUB 编辑器失败: {error}": "Failed to open EPUB Editor: {error}",
  "打开文件失败: {error}": "Failed to open file: {error}",
  所有支持的文件: "All supported files",
  "打开文件失败：{error}": "Failed to open file: {error}",
  "保存失败: {error}\n请确保已授予“所有文件访问权限”":
    "Save failed: {error}\nMake sure “All files access” permission has been granted",
  无标题: "Untitled",
  "低于 {count}": "Below {count}",
  "高于 {count}": "Above {count}",
  "第 {current}/{total} 处": "Match {current}/{total}",
  未找到: "Not found",
  正则错误: "Regex error",
  替换完成: "Replacement complete",
  替换失败: "Replacement failed",
  "替换失败：{error}": "Replacement failed: {error}",
  未命名书籍: "Untitled book",
  "制作失败: {error}": "Creation failed: {error}",
  "回退失败: {error}": "Rollback failed: {error}",
  只排章节: "Chapters only",
  只排卷部: "Volumes only",
  只转数字: "Convert numbers only",
  正则选取: "Regex selection",
  应用数字转换: "Apply number conversion",
  应用目录重排序: "Apply contents reordering",
  "断序：当前应为第 {expected} 章，原标题为第 {original} 章":
    "Out of sequence: expected chapter {expected}, original title says chapter {original}",
  展开本卷: "Expand this volume",
  折叠本卷: "Collapse this volume",
  内置规则: "Built-in rules",
  繁简转换: "Simplified/Traditional conversion",
  "校对中...": "Proofreading...",
  开始: "Start",
  "{chapter}\n原因：{reason}\n原文：{original}":
    "{chapter}\nReason: {reason}\nOriginal: {original}",
  移除整章: "Remove entire chapter",
  已撤销: "Undone",
  已应用: "Applied",
  "搜索中...": "Searching...",
  搜索封面: "Search covers",
  未命名: "Untitled",
  未知来源: "Unknown source",
  "正在制作 EPUB...": "Creating EPUB...",
  "EPUB 制作完成": "EPUB creation complete",
  "制作中...": "Creating...",
  开始制作: "Start creating",
  "EPUB 打开前需要预处理。":
    "This EPUB needs preprocessing before it can be opened.",
  "原文件未被修改，当前查看的是处理后的副本。":
    "The original file was not modified; you are viewing a processed copy.",
  "源文件：{path}": "Source file: {path}",
  "当前文件：{path}": "Current file: {path}",
  "统计失败：{error}": "Failed to calculate statistics: {error}",
  "未提供 EPUB 文件路径。": "No EPUB file path was provided.",
  "打开 EPUB 失败：{error}": "Failed to open EPUB: {error}",
  "读取导出内容失败。": "Failed to read the exported content.",
  "文件导出成功。": "File exported successfully.",
  "导出文件失败：{error}": "Failed to export file: {error}",
  读取文件内容失败: "Failed to read file content",
  "复制文件失败：{error}": "Failed to copy file: {error}",
  "重命名文件失败：{error}": "Failed to rename file: {error}",
  "重命名/重构失败：{error}": "Rename/restructure failed: {error}",
  确认删除: "Confirm deletion",
  "确定删除 {path} 吗？": "Delete {path}?",
  "该目录项下未找到文件。": "No files were found under this directory.",
  字体文件: "Font file",
  "已导入 {count} 个文件。": "Imported {count} files.",
  "操作失败：{error}": "Operation failed: {error}",
  "有未保存的更改，确定离开吗？": "You have unsaved changes. Leave anyway?",
  "字体预览加载失败。": "Failed to load the font preview.",
  "图片预览加载失败。": "Failed to load the image preview.",
  "预览源加载失败：{error}": "Failed to load preview source: {error}",
  "读取 OPF 内容失败。": "Failed to read OPF content.",
  "更新 OPF 失败：{error}": "Failed to update OPF: {error}",
  "向 EPUB 添加资源失败。": "Failed to add the asset to the EPUB.",
  "批量写入文件失败：{error}": "Failed to write files in bulk: {error}",
  "，失败 {count} 本：\n": ", {count} failed:\n",
  "找不到 META-INF/container.xml：{error}":
    "Could not find META-INF/container.xml: {error}",
  "(未命名)": "(Untitled)",
  "第 {index} 节": "Section {index}",
  "EPUB spine 为空，无法显示":
    "The EPUB spine is empty and cannot be displayed",
  "未提供 EPUB 文件路径": "No EPUB file path was provided",
  "生图 API": "Image API",
  "Web EPUB 阅读器 - TEpub Editor": "Web EPUB Reader - TEpub Editor",
  "Web EPUB 编辑器 - TEpub Editor": "Web EPUB Editor - TEpub Editor",
  "选择 EPUB 文件开始阅读": "Choose an EPUB file to start reading",
  "选择 EPUB 文件开始编辑": "Choose an EPUB file to start editing",
  "导入后在浏览器内解包，左侧显示目录，中间以宽屏滚动预览方式阅读。":
    "After import, the EPUB is unpacked in the browser. Browse the contents on the left and read the wide scrolling preview in the center.",
  "支持无 DRM 的标准 EPUB，导入后可编辑文件结构、资源和元数据。":
    "Supports standard DRM-free EPUBs. After import, edit the file structure, assets, and metadata.",
  未选择章节: "No chapter selected",
  容器配置: "Container configuration",
  样式表: "Stylesheet",
  "目录 {index}": "Contents {index}",
  "{files} 个文件 · {items} 个阅读项": "{files} files · {items} reading items",
  已解除伪加密: "Pseudo-encryption removed",
  已解混淆: "Obfuscation removed",
  "已恢复 {count} 个可读文件名": "Restored {count} readable file names",
  "确认删除资源？\n{path}\n\n会同步移除 OPF manifest/spine 引用，导出后生效。":
    "Delete asset?\n{path}\n\nThis will also remove OPF manifest/spine references. It takes effect after export.",
  未定位到书脊: "Could not locate the spine",
  "文件仅在当前设备中读取，选择后继续进入原有阅读页面。":
    "Files are read only on this device. After selection, you will continue to the existing reading page.",
  "文件仅在当前设备中读取，选择后继续进入原有编辑页面。":
    "Files are read only on this device. After selection, you will continue to the existing editing page.",
  显示目录: "Show contents",
  收起目录: "Hide contents",
  章节阅读预览: "Chapter reading preview",
  "选择目录中的章节后开始阅读。":
    "Select a chapter from the contents to start reading.",
  "选择 XHTML 章节后自动预览。":
    "Select an XHTML chapter to preview it automatically.",
  自定义: "Custom",
  内置: "Built-in",
  "{name}（自定义）": "{name} (Custom)",
  未命名头图样式: "Untitled header style",
  "样板长边已规范为 {edge}px，保存生成后的透明样板图，尺寸 {width}x{height}。":
    "The sample's longest edge was normalized to {edge}px. Save the generated transparent sample at {width}x{height}.",
  未命名标题样式: "Untitled title style",
  "自定义完整 CSS 定义。": "Custom complete CSS definition.",
  "number/name 双行标题样式。": "Two-line number/name title style.",
  "name 单行标题样式。": "Single-line name title style.",
  "保存完整 CSS，制作时按命名接口自动应用。":
    "Save complete CSS; it is applied automatically through the naming interface during creation.",
  "保存 number/name 样式内容，制作时自动套到标题结构。":
    "Save number/name style content; it is applied automatically to the title structure during creation.",
  未命名转场样式: "Untitled transition style",
  未命名插图样式: "Untitled illustration style",
  "自定义转场 CSS 与 XHTML。": "Custom transition CSS and XHTML.",
  "自定义插图 CSS 与 XHTML。": "Custom illustration CSS and XHTML.",
  "默认用于替换前后都有正文的单段孤立省略号。":
    "Used by default to replace a standalone ellipsis paragraph with body text before and after it.",
  "将图片路径、替代文字和图注替换为实际内容后插入正文。":
    "Replaces image paths, alt text, and captions with actual content before inserting into the body.",
  "CSS 中不能包含 script。": "CSS cannot contain script.",
  "CSS 中暂不允许 @import。": "@import is not currently allowed in CSS.",
  "CSS 中不能包含 javascript: URL。": "CSS cannot contain javascript: URLs.",
  "请填写样式名称。": "Enter a style name.",
  "CSS 不能为空。": "CSS cannot be empty.",
  "XHTML 结构不能为空。": "The XHTML structure cannot be empty.",
  "XHTML 中不能包含脚本、事件属性或 javascript: URL。":
    "XHTML cannot contain scripts, event attributes, or javascript: URLs.",
  "{label} 不能为空。": "{label} cannot be empty.",
  "{label} 只填写样式内容，不要写选择器或大括号。":
    "Enter only style content for {label}; do not include selectors or braces.",
  "完整 CSS 不能为空。": "Complete CSS cannot be empty.",
  "完整 CSS 至少要包含一个标准 class 选择器。":
    "Complete CSS must contain at least one standard class selector.",
  "存在不符合命名规范的 class：{classes}":
    "Found classes that do not follow the naming rules: {classes}",
  "标题样式暂只允许章节标题接口：{classes} 将放到单独样式列表处理。":
    "Title styles currently only allow the chapter title interface; {classes} will be handled in a separate style list.",
  "预览图片已更新；XHTML 中仍保留 EPUB 内部相对路径。":
    "Preview image updated; the XHTML still keeps EPUB-internal relative paths.",
  "请选择图片样板。": "Select an image sample.",
  "无法创建画布。": "Could not create canvas.",
  "无法创建默认头图画布。": "Could not create the default header-image canvas.",
  "已按长边 {edge}px 生成遮罩预览：{width}x{height}（原图 {originalWidth}x{originalHeight}）。":
    "Generated a mask preview with a {edge}px longest edge: {width}x{height} (original {originalWidth}x{originalHeight}).",
  "请先导入样板图片。": "Import a sample image first.",
  "已保存到样式库。": "Saved to the style library.",
  "请先确认样式预览无误。": "Confirm the style preview is correct first.",
  "已复制 CSS": "CSS copied",
  "已复制 XHTML": "XHTML copied",
  "新增{kind}样式": "Add {kind} style",
  插图: "Illustration",
  转场: "Transition",
  "例如：档案卡片插图": "e.g. Archive card illustration",
  "例如：章节菱形转场": "e.g. Chapter diamond transition",
  "新增{kind}样式预览": "Preview for new {kind} style",
  "{name} 预览": "{name} preview",
  我的样式: "My styles",

  // Built-in EPUB style catalog
  "图片贴住页面上、左、右边缘，标题和正文从下方开始。":
    "The image touches the top, left, and right edges of the page, with the title and body starting below.",
  "制作 EPUB 时把生成好的头图放入 .te-header-figure > img.te-header-image，图片贴页面上沿显示。":
    "During EPUB creation, the generated header image is placed in .te-header-figure > img.te-header-image and aligned to the top edge of the page.",
  "主体集中在右侧，左侧保留大片留白，并带碎片、颗粒和旧照散边效果。":
    "The subject is concentrated on the right, with a large blank area on the left and fragmented, grainy, distressed-photo edges.",
  "使用透明样板把用户头图裁成右侧旧照散边形状，适合年代感、群像、生活流或回忆章节。":
    "Uses a transparent template to crop the user's header image into a distressed photo shape on the right. Suited to period pieces, ensemble casts, slice-of-life, and memory chapters.",
  "保留留白和轻阴影，适合不希望图片铺满页面的章节。":
    "Keeps whitespace and a subtle shadow for chapters where the image should not fill the page.",
  "头图仍使用 .te-header-image，模块只改变外框、圆角和间距。":
    "The header image still uses .te-header-image; this module only changes the frame, corner radius, and spacing.",
  "贴边显示并在头图底部增加暗色压底，标题仍按所选标题样式单独渲染。":
    "Displays the image edge to edge and adds a dark gradient along its bottom. The title is still rendered separately using the selected title style.",
  "章节 XHTML 中先输出 .te-header-figure，再输出章节标题；该样式只处理头图本身。":
    "The chapter XHTML outputs .te-header-figure before the chapter title; this style only affects the header image itself.",
  "图片贴近页顶，底部以细线和留白收束，适合文艺、悬疑、现代题材。":
    "The image sits near the top of the page and is closed off with a fine rule and whitespace below. Suited to literary, mystery, and contemporary works.",
  "章节开头先输出 .te-header-figure，图片会以装帧细线收束，标题在下方独立排版。":
    "The chapter starts with .te-header-figure; the image is framed with fine binding-style rules, and the title is laid out separately below.",
  "保留四周留白和轻微投影，像插页版画，适合古风、奇幻和人物向章节。":
    "Keeps whitespace around the image and a slight shadow, like an inserted print. Suited to historical, fantasy, and character-focused chapters.",
  "头图不铺满页面，使用 .te-header-figure 的留白与边线形成插页感。":
    "The header image does not fill the page; the whitespace and border lines of .te-header-figure create an inset-plate effect.",
  "固定横幅高度，顶部贴边并加深色压边，适合动作、悬疑、科幻章节。":
    "Uses a fixed banner height, touches the top edge, and adds a dark bottom border. Suited to action, mystery, and science-fiction chapters.",
  "将章节头图裁成宽银幕感横幅，标题仍使用绑定标题样式在图下显示。":
    "Crops the chapter header into a widescreen banner. The title remains below the image using its bound title style.",
  "居中双行标题，章节序号偏暗，章节名使用红色强调。":
    "Centered two-line title with a muted chapter number and a red-accented chapter name.",
  "自动套用到双行章节标题：.te-chapter-title / .te-chapter-number / .te-chapter-name。":
    "Automatically applies to two-line chapter titles: .te-chapter-title / .te-chapter-number / .te-chapter-name.",
  "黑白细线分隔，适合正文密集、低装饰的作品。":
    "Black-and-white fine-line separation, suited to text-dense works with minimal decoration.",
  "自动套用到双行章节标题，章节序号和章节名使用标准 span。":
    "Automatically applies to two-line chapter titles; the chapter number and name use standard spans.",
  "居中双行标题，章节序号用暗紫色，章节名用红色强调。":
    "Centered two-line title with a dark purple chapter number and a red-accented chapter name.",
  "由 h2.texthE1 / b 转换为标准章节标题接口，自动套用到双行章题。":
    "Converts h2.texthE1 / b into the standard chapter-title interface and automatically applies it to two-line chapter titles.",
  "左对齐双行标题，章节序号为黑色小字，章节名为青蓝色。":
    "Left-aligned two-line title with a small black chapter number and a teal chapter name.",
  "由 h2.head / h2.head span 转换为标准章节标题接口，适合低装饰章节开头。":
    "Converts h2.head / h2.head span into the standard chapter-title interface. Suited to minimally decorated chapter openings.",
  "居中双行标题，序号较小，标题名使用朱红色强调。":
    "Centered two-line title with a smaller number and a vermilion-accented title name.",
  "由 h2.chapter-title / .chapter-number 转换为标准章节标题接口，自动套用到双行章题。":
    "Converts h2.chapter-title / .chapter-number into the standard chapter-title interface and automatically applies it to two-line chapter titles.",
  "左对齐双行标题，章节序号使用青蓝底色编号块。":
    "Left-aligned two-line title with the chapter number in a teal number block.",
  "由 h2.head / span.num 转换为标准章节标题接口，编号块自动套用到 number。":
    "Converts h2.head / span.num into the standard chapter-title interface; the number block is applied automatically to the number.",
  "深蓝主标题配金色序号线，克制、有画面感，适合悬疑、都市、科幻。":
    "A deep-blue main title with a gold sequence rule: restrained and cinematic. Suited to mystery, urban, and science-fiction works.",
  "使用标准双行章节标题结构，序号作为上方小标题，章名作为醒目的主标题。":
    "Uses the standard two-line chapter-title structure, with the number as an upper subtitle and the chapter name as the prominent main title.",
  "暖色双线和居中标题，像纸本装帧页，适合古风、奇幻、文学向作品。":
    "Warm double rules and a centered title evoke a printed title page. Suited to historical, fantasy, and literary works.",
  "双行标题居中显示，序号淡化，章名以深墨色突出。":
    "Centered two-line title with a muted number and the chapter name highlighted in deep ink.",
  "左对齐章名搭配朱色印章式序号，适合武侠、历史、东方幻想。":
    "Left-aligned chapter name paired with a vermilion seal-style number. Suited to wuxia, historical, and Eastern fantasy.",
  "序号作为小印章，章名左对齐显示，适合章节开头信息密度较高的排版。":
    "The number appears as a small seal while the chapter name is left-aligned. Suited to chapter openings with denser information.",
  "浅青分隔与大留白，阅读感轻，适合现代、治愈、日常题材。":
    "Pale teal separators and generous whitespace create a light reading feel. Suited to contemporary, healing, and slice-of-life works.",
  "双行标题居中，序号用小号灰色，标题用青蓝强调。":
    "Centered two-line title with a small gray number and teal-accented title.",
  "深色标题卡片与亮色序号，适合紧张、悬疑、赛博或暗色题材。":
    "A dark title card with a bright number. Suited to tense, mystery, cyberpunk, or dark themes.",
  "标题整体作为深色章卡显示，和电影裁幅头图搭配使用效果更强。":
    "Displays the title as a dark chapter card; pairs especially well with cinematic cropped header images.",
  "极简居中标题，低装饰、强留白，适合长篇阅读和正文密集作品。":
    "A minimal centered title with little decoration and strong whitespace. Suited to long-form reading and text-dense works.",
  "序号和章名均居中，使用细线留出呼吸感，不干扰正文阅读。":
    "The number and chapter name are both centered, with fine rules providing breathing room without distracting from the body text.",
  "常见的正文居中插图，限制宽度并在下方显示简洁图注。":
    "A common centered inline illustration for body text, with constrained width and a concise caption below.",
  "将 XHTML 中的图片路径、替代文字和图注替换为实际内容，适合人物、场景和资料图。":
    "Replaces image paths, alt text, and captions in the XHTML with actual content. Suited to character, scene, and reference images.",
  "带留白、细边框和轻阴影的卡片式插图，适合信件、档案和回忆画面。":
    "A card-style illustration with whitespace, a fine border, and a subtle shadow. Suited to letters, documents, and memory scenes.",
  "插入标准 figure 结构并替换图片；电子墨水设备会自然退化为边框卡片。":
    "Inserts a standard figure structure and replaces the image; e-ink devices naturally fall back to a bordered card.",
  "横向铺满版心并向两侧出血，适合章节中段的大场景和地图。":
    "Spans the full text width and bleeds into both margins. Suited to large scenes and maps in the middle of a chapter.",
  "使用横幅图片；部分阅读器不支持 duokan-bleed 时仍会按容器宽度正常显示。":
    "Uses a banner image; on readers without duokan-bleed support, it still displays correctly at the container width.",
  "点击正文词语或注释链接，以无脚本弹层显示对应插图。":
    "Select a word or note link in the body to show the corresponding illustration in a script-free pop-up.",
  "checkbox 的 id 必须与所有 label 的 for 一致；点击文字、注释、遮罩或关闭按钮即可打开或收起图片。":
    "The checkbox id must match every label's for attribute; selecting the text, note, overlay, or close button opens or closes the image.",
  "制作功能内置的经典居中 ※※※，兼容旧 class fg1 与标准 te-divider-line。":
    "The built-in classic centered ※※※ for the creation workflow, compatible with the legacy fg1 class and standard te-divider-line.",
  "制作 EPUB 时默认用它替换前后都有正文的单段孤立省略号。":
    "Used by default during EPUB creation to replace a standalone ellipsis paragraph with body text before and after it.",
  "不使用文字和图片，仅用伪元素绘制三枚渐次菱形。":
    "Uses no text or images; three staggered diamonds are drawn entirely with pseudo-elements.",
  "纯 CSS 转场，可直接替换孤立省略号，黑白屏和电子墨水设备也能保持清晰。":
    "A pure CSS transition that can directly replace a standalone ellipsis and stays clear on monochrome and e-ink devices.",
  "在左右细线之间显示“与此同时”等短文本。":
    "Displays short text such as “Meanwhile” between thin rules on the left and right.",
  "替换 te-transition-text 的文字即可表达时间、地点或叙事视角转换。":
    "Replace the text in te-transition-text to indicate a change in time, place, or narrative viewpoint.",
  "无文本的轻量渐隐横线，适合现代、日常和长篇阅读。":
    "A lightweight, text-free fading rule suited to modern, slice-of-life, and long-form reading.",
  "纯 CSS 样式，直接替换孤立省略号，不依赖额外资源。":
    "A pure CSS style that directly replaces a standalone ellipsis with no extra assets.",
  "使用独立 PNG、JPG 或 SVG 小图替代孤立省略号。":
    "Uses a separate small PNG, JPG, or SVG image in place of a standalone ellipsis.",
  "把图片导入制作模板的 dividerImage 资源位，制作时会自动以该图片替换孤立省略号。":
    "Import the image into the creation template's dividerImage slot; during creation it automatically replaces standalone ellipses.",

  // Built-in proofing catalog and result messages
  "标题中被 （）()【】[] 包起来的内容，替换为空":
    "Removes content enclosed in （）()【】[] from titles",
  "月票、求票、订阅、本章完、请假条、上架感言，以及 PS 段落":
    "Monthly passes, vote requests, subscriptions, end-of-chapter notes, leave notices, launch announcements, and PS paragraphs",
  "括号内拼音、独立拼音行和行尾拼音注音，替换为空":
    "Removes pinyin in brackets, standalone pinyin lines, and trailing pinyin annotations",
  "正文段落统一缩进两个全角空格，跳过目录标题行":
    "Indents body paragraphs with two full-width spaces, skipping contents title lines",
  没有可重排的标题: "No headings can be reordered",
  标题已是标准格式: "Headings are already in standard format",
  没有找到括号内容: "No bracketed content was found",
  没有找到广告内容: "No ad content was found",
  没有找到拼音内容: "No pinyin content was found",
  请选择要替换的匹配项: "Select the matches to replace",
  没有可替换的匹配项: "No matches can be replaced",
  已转换为简体: "Converted to Simplified Chinese",
  已转换为繁体: "Converted to Traditional Chinese",
  没有需要转换的内容: "Nothing needs to be converted",
  请选择要转换的匹配项: "Select the matches to convert",
  没有可转换的内容: "No content can be converted",
  请输入正则表达式: "Enter a regular expression",
  没有匹配到标题: "No headings matched",
  没有匹配到内容: "No content matched",
  "已重排 {count} 个标题": "Reordered {count} headings",
  "已转换 {count} 个标题数字": "Converted {count} title numbers",
  "已清理 {count} 个标题括号": "Removed brackets from {count} headings",
  "已移除 {count} 行广告/说明": "Removed {count} ad/notice lines",
  "已清理 {count} 处拼音": "Removed pinyin from {count} places",
  "已替换 {count} 个匹配项": "Replaced {count} matches",
  "已转换 {count} 处": "Converted {count} places",
  "已替换 {count} 处标题": "Replaced {count} heading matches",

  // Contents presets and remaining tool messages
  第X章: "Chapter X",
  "第X卷 / 卷X 标题": "Volume X / Volume X title",
  "终章 标题": "Finale title",
  "番外 / 后日谈": "Side story / Epilogue",
  第X回: "Part X",
  第X节: "Section X",
  "第X章（阿拉伯数字）": "Chapter X (Arabic numerals)",
  "内容简介 / 本书相关 / 完本感言":
    "Synopsis / About this book / Completion note",
  "简介 / 前言 / 楔子 / 后记 / 尾声":
    "Synopsis / Preface / Prologue / Afterword / Epilogue",
  "序列 数字": "Sequence number",
  纯数字标题: "Numeric heading",
  文件名混淆完成: "File names obfuscated",
  "未检测到可混淆的 manifest 文件项":
    "No manifest file entries were found to obfuscate",
  混淆文件名已恢复并同步修复引用:
    "Obfuscated file names restored and references updated",
  "未检测到文件名混淆，无需处理":
    "No file-name obfuscation detected; no changes needed",
  "EPUB 已经是规范结构，无需重构":
    "The EPUB is already in standard structure; no rebuild needed",
  "此工具一次只能处理一个文件。":
    "This tool can only process one file at a time.",
  内容简介: "Synopsis",
  选择模板仓库: "Select template repository",
  扫描目录: "Scan contents",
  "识别卷、章与正文层级": "Detect volume, chapter, and body hierarchy",
  编辑信息: "Edit details",
  "调整规则、书名与作者信息": "Adjust rules, title, and author",
  配置封面与样式后导出: "Configure the cover and styles, then export",
  不缩放: "No resizing",
  自动识别: "Auto-detect",
  容器结构: "Container structure",
  "检查 mimetype、container.xml 与 OPF 路径":
    "Check mimetype, container.xml, and OPF paths",
  资源清单: "Asset manifest",
  "检查 manifest 缺失项、未登记资源与路径大小写":
    "Check missing manifest entries, unlisted assets, and path casing",
  内部引用: "Internal references",
  "扫描 HTML、CSS、SVG、NCX 中的断链引用":
    "Scan HTML, CSS, SVG, and NCX for broken references",
  目录导航: "Contents navigation",
  "读取 NAV 与 NCX 章节目录": "Read chapter navigation from NAV and NCX",
  正文预览: "Content preview",
  "按阅读顺序打开 EPUB 内容": "Open EPUB content in reading order",
  本地阅读: "Local reading",
  文件不会上传到服务器: "Files are never uploaded to a server",
  "浏览并管理 EPUB 内部资源": "Browse and manage assets inside the EPUB",
  内容编辑: "Content editing",
  "编辑 XHTML、CSS 与元数据": "Edit XHTML, CSS, and metadata",
  重新导出: "Export again",
  "处理完成后生成新的 EPUB": "Generate a new EPUB after processing",
  识别两种标签: "Detect both image formats",
  "支持真实 img 标签和转义为文字的 &lt;img&gt;":
    "Supports real img tags and <img> escaped as text",
  资源自动入包: "Assets added automatically",
  "图片写入 Images/online，并登记到 OPF manifest":
    "Images are written to Images/online and registered in the OPF manifest",
  样式可选择: "Selectable styles",
  "默认居中图注插图，也可使用样式库中的普通插图样式":
    "Defaults to a centered captioned illustration; standard styles from the Style Library are also available",
  目录识别: "Contents detection",
  "按规则扫描卷、章与正文结构":
    "Scan volume, chapter, and body structure using rules",
  支持文本与正则批量处理:
    "Batch-process with plain text or regular expressions",
  校对整理: "Proofing and cleanup",
  "检查文本并衔接 EPUB 制作": "Check text and continue into EPUB creation",
};
