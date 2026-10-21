/**
 * 目录正则的预设表。
 *
 * 原先这张表写在 /editor 页面里，只有安装版能用；web 的 TXT 编辑器
 * （/toolbox/text-editor）搬进设置面板后也要同一份预设，否则同一个
 * 「目录正则」在两处给出不同的可选规则，改完还以为是对方没生效。
 *
 * 这里只放"可选的写法"，不放用户的规则本身 —— 用户规则存在
 * appSettings.customRegexRules 里，由 appSettings 负责归一化。
 */
import { DEFAULT_TOC_REGEX_RULES } from "./appSettings";

/** 第一卷标题：`第一章 xx` / `第一卷 xx` / `卷三 xx` */
export const DEFAULT_VOLUME_REGEX =
  "^\\s*(?:第\\s*[零〇一二两三四五六七八九十百千万0-9]+\\s*卷|卷\\s*[零〇一二两三四五六七八九十百千万0-9]+)(?:\\s+|[:：、.．\\-—]+)\\S+.*";

/** 带说明的卷级条目：内容简介 / 本书相关 / 完本感言 */
export const DEFAULT_META_VOLUME_REGEX =
  "^\\s*(?:内容简介|本书相关|完本感言)\\s*(?:[:：].*)?$";

/** 简介 / 序 / 前言 / 楔子 / 后记 / 尾声 */
export const DEFAULT_META_BODY_REGEX =
  "^\\s*(?:简介|序(?:章|言)?|前言|楔子|后记|尾声)\\s*(?:[:：].*)?$";

/** 章节：第N章 / 第N节 / 第N回 / Chapter N / 终章 / 番外 */
export const DEFAULT_CHAPTER_REGEX =
  "^\\s*(?:第\\s*[一二两三四五六七八九十零〇百千万0-9]+\\s*(?:[章节]|回(?:[^合]|$))|Chapter\\s*\\d+|终章(?:\\s+|[:：、.．\\-—])\\S+|(?:新增\\s*)?(?:番外|后日谈)(?:\\s+|[:：、.．\\-—])\\S+|【\\s*(?:番外|后日谈)\\s*】\\s*\\S+).*";

export interface TocRegexPreset {
  /** 下拉里显示的短名 */
  label: string;
  /** 选中后写进规则的正则原文；空串表示「自定义」，不写值 */
  value: string;
}

export const TOC_REGEX_PRESETS: TocRegexPreset[] = [
  { label: "自定义", value: "" },
  {
    label: "第X章",
    value: "^\\s*第[一二三四五六七八九十零〇百千两]+章.*$",
  },
  { label: "第X卷 / 卷X 标题", value: DEFAULT_VOLUME_REGEX },
  { label: "终章 标题", value: "^\\s*终章(?:\\s+|[:：、.．\\-—])\\S+.*$" },
  {
    label: "番外 / 后日谈",
    value:
      "^\\s*(?:(?:新增\\s*)?(?:番外|后日谈)(?:\\s+|[:：、.．\\-—])\\S+|【\\s*(?:番外|后日谈)\\s*】\\s*\\S+).*$",
  },
  { label: "第X回", value: "^\\s*第[一二三四五六七八九十零〇百千两]+回.*$" },
  { label: "第X节", value: "^\\s*第[一二三四五六七八九十零〇百千两]+节.*$" },
  { label: "第X章（阿拉伯数字）", value: "^\\s*第\\d+章.*$" },
  {
    label: "内容简介 / 本书相关 / 完本感言",
    value: DEFAULT_META_VOLUME_REGEX,
  },
  {
    label: "简介 / 前言 / 楔子 / 后记 / 尾声",
    value: DEFAULT_META_BODY_REGEX,
  },
  { label: "序列 数字", value: "^\\s*序列\\s*\\d+(?:\\s|[:：、.-]|$).*$" },
  { label: "纯数字标题", value: "^\\s*\\d+\\s*$" },
];

/** 只取能直接套用的正则（去掉「自定义」那条空值） */
export const TOC_PRESET_PATTERNS: string[] = TOC_REGEX_PRESETS.filter(
  (preset) => preset.value,
).map((preset) => preset.value);

/** 把任意片段转义成正则字面量 */
export function escapeRegExp(value: string) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

/**
 * 根据前缀生成一条标题规则，例如 prefix="番外" 匹配
 * `番外 标题` 和 `【番外】标题`，也接受 `新增番外 标题`。
 * /editor 与 web TXT 编辑器共用，避免两边生成的规则不一致。
 */
export function buildTitlePrefixRegex(prefix: string) {
  const escaped = escapeRegExp(prefix.trim());
  return `^\\s*(?:(?:新增\\s*)?${escaped}(?:\\s+|[:：、.．\\-—])\\S+.*|【\\s*${escaped}\\s*】\\s*\\S+.*)$`;
}

/** 默认规则（供设置页「还原」按钮展示顺序参考） */
export const DEFAULT_RULE_PATTERNS = DEFAULT_TOC_REGEX_RULES.map(
  (rule) => rule.pattern,
);
