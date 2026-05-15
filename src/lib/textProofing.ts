export type ProofTitleScope = "all" | "volumes" | "chapters" | "regex" | "numbers-only";
export type ProofNumberStyle = "arabic" | "chinese";
export type ProofConvertDirection = "simplified-to-traditional" | "traditional-to-simplified";
export type ProofBuiltinRuleId = "title-brackets" | "ads" | "pinyin";

export interface ProofTocNode {
  id?: string;
  title: string;
  line: number;
  type?: "Volume" | "Chapter" | "Meta" | string;
  parentId?: string;
  level?: number;
}

export interface ProofTitleRewriteOptions {
  scope: ProofTitleScope;
  regex: string;
  volumeNumberStyle: ProofNumberStyle;
  chapterNumberStyle: ProofNumberStyle;
  perVolume: boolean;
}

export interface ProofTitlePreviewRow {
  id: string;
  line: number;
  kind: "volume" | "chapter";
  volumeKey: string;
  original: string;
  replacement: string;
  changed: boolean;
  sequenceBroken: boolean;
  originalIndex: number | null;
  expectedIndex: number;
}

export interface ProofTransformResult {
  text: string;
  changedCount: number;
  message: string;
}

export interface ProofBuiltinRegexRule {
  id: ProofBuiltinRuleId;
  name: string;
  description: string;
}

export interface ProofRegexPreviewRow {
  id: string;
  ruleId: ProofBuiltinRuleId;
  lineStart: number;
  lineEnd: number;
  startOffset: number;
  endOffset: number;
  original: string;
  replacement: string;
}

export interface ProofConvertPreviewRow {
  id: string;
  lineStart: number;
  lineEnd: number;
  startChar: number;
  endChar: number;
  original: string;
  replacement: string;
  originalText: string;
  replacementText: string;
}

export interface ProofChineseScriptProfile {
  simplified: number;
  traditional: number;
  dominant: "simplified" | "traditional" | "mixed" | "unknown";
}

type ChineseConverter = (text: string) => string;

const converterCache: Partial<Record<ProofConvertDirection, ChineseConverter>> = {};

async function getChineseConverter(direction: ProofConvertDirection) {
  if (converterCache[direction]) return converterCache[direction]!;

  const { default: OpenCC } = await import("opencc-js");
  const converter =
    direction === "traditional-to-simplified"
      ? OpenCC.Converter({ from: "hk", to: "cn" })
      : OpenCC.Converter({ from: "cn", to: "tw" });
  converterCache[direction] = converter;
  return converter;
}


const CHINESE_DIGITS = [
  "零",
  "一",
  "二",
  "三",
  "四",
  "五",
  "六",
  "七",
  "八",
  "九",
];

const CHINESE_CHAR_RE = /[\u3400-\u9fff]/;
const PINYIN_TOKEN_SPLIT_RE = /[\s'’\-]+/;
const PINYIN_TOKEN_CHAR_RE =
  /^[A-Za-z\u00c0-\u024f\u1e00-\u1eff\u00fc\u00dc\u01d5-\u01dc:]+[1-5]?$/;
const PINYIN_TONE_MARK_RE =
  /[\u0101\u00e1\u01ce\u00e0\u0113\u00e9\u011b\u00e8\u012b\u00ed\u01d0\u00ec\u014d\u00f3\u01d2\u00f2\u016b\u00fa\u01d4\u00f9\u01d6\u01d8\u01da\u01dc\u01d5\u01d7\u01d9\u01db\u00fc]/i;
const PINYIN_TONE_NUMBER_RE =
  /\b[A-Za-z\u00c0-\u024f\u1e00-\u1eff\u00fc\u00dc\u01d5-\u01dc:]+[1-5]\b/;
const PINYIN_NOISE_RE = /[=_*\/\\{}<>@#$%&]|[☉≈]/;
const PINYIN_TRAILING_CANDIDATE_RE =
  /\s+([A-Za-z\u00c0-\u024f\u1e00-\u1eff\u00fc\u00dc\u01d5-\u01dc:]+(?:[1-5])?(?:[\s'’\-]+[A-Za-z\u00c0-\u024f\u1e00-\u1eff\u00fc\u00dc\u01d5-\u01dc:]+(?:[1-5])?){0,12})\s*$/;
const PINYIN_BRACKET_RE =
  /[\uFF08(\u3010\[]\s*([^\uFF08\uFF09()\[\]\u3010\u3011]*)\s*[\uFF09)\u3011\]]/g;
const PINYIN_FINALS = new Set([
  "a",
  "ai",
  "an",
  "ang",
  "ao",
  "e",
  "ei",
  "en",
  "eng",
  "er",
  "i",
  "ia",
  "ian",
  "iang",
  "iao",
  "ie",
  "in",
  "ing",
  "iong",
  "iu",
  "o",
  "ong",
  "ou",
  "u",
  "ua",
  "uai",
  "uan",
  "uang",
  "ue",
  "ui",
  "un",
  "uo",
  "v",
  "van",
  "ve",
  "vn",
]);
const PINYIN_MARK_RE =
  /[āáǎàēéěèīíǐìōóǒòūúǔùǖǘǚǜńňḿẁẃẅÿ]/i;
const PINYIN_WORD_RE = /^[A-Za-züÜvVāáǎàēéěèīíǐìōóǒòūúǔùǖǘǚǜńňḿ'’\-\s0-9]+$/;
const PINYIN_LATIN_RE = /[A-Za-züÜvVāáǎàēéěèīíǐìōóǒòūúǔùǖǘǚǜńňḿ]/;
const PINYIN_TONE_RE = /[A-Za-züÜvVāáǎàēéěèīíǐìōóǒòūúǔùǖǘǚǜńňḿ][1-5]\b/;
const TRAILING_PINYIN_RE = /\s+([A-Za-züÜvVāáǎàēéěèīíǐìōóǒòūúǔùǖǘǚǜńňḿ'’\-\s0-9]+)$/;
const AD_LINE_RE = /(月票|求票|求推荐票|推荐票|订阅|本章完|求收藏|求打赏|打赏)/i;
const AD_BLOCK_START_RE =
  /^\s*(?:(?:请假条|请假|上架感言|上架通知|完本感言|作者的话|本书上架)|(?:PS\d*|P\.S\.?\d*|ps\d*)\s*(?:[:：.．、\-—]|$))/i;
const TRADITIONAL_ONLY_RE =
  /[萬與專業叢東絲丟兩嚴喪個豐臨為麗舉麼義烏樂喬習鄉書買亂爭於虧雲亞產畝親億僅從侖倉儀們價眾優會傘偉傳傷倫偽佇體餘傑傾僂儲兒兌內兩冊冪凈凍凱別劑剛創劃劇劉則剎剝勸辦務動勢勛勝勞勢匯區醫華協單賣盧衛卻廠廳參雙發變敘葉號嘆嘗嚇嚴囑園圓圖團聖場壞塊堅壇墳墜壘壟壯聲壺壽夢夥夾奪奮奧婦媽嫻嬌學寧寶實寵審寫寬將專尋對導層屬歲島嶺嶽幣師幟幫幹幾庫廁廂廈廚廟廠廣廬廳異棄張彌彎彙彥後徑從復徵恆惡慘慣態慶憂憐憑應懷懸懺戀戰戲戶扎撲執擴擔據擬擾攜攝擺搖敗敘數斂斃斕鬥斷無舊時曠會朧東楊極構標樣樞樓樂樸機權歸殼殘毀氈氣氫氬漢湯溝沒滅滬滯滲濁濃濕燈靈災爐爭爺牆獎獨獸獲璣現瑪產異當疊痙癡發皺盜監盤眾著睜矚礎禮禪離穩窮竄竅競筆築簡糧糾紀約紅紋納紐純紗紙級紛細終組經結絕絲統綁綠網綱綻綽維綿緊緒線練縣縫總績織繪繫繼續纏纖罈罰罵羅羈習翹聖聞聯聰聲聳膽臉臘臺與興舉艙藝節華萬葉著藍處虛號蠟補裝裡製複見規視覺覽親觀觴計訂訃記訓託訢詞詠試詩誠話誕誘語誤說誰課調諒論諸諾謀謂謠謙講謝謠證譯譽讀變讓讖讚豈豐貝負財責賢敗賬貨質販貪貧貶購貫貼貴貸費貿賀賊賈賄資賓賜賞賠賴賺賽贊贈贏趙趕趨躍踐蹤車軋軌軒轉輪軟較載輔輕輛輝輩輩輯輸轄辦辭農迴這連週進遊運過達違遙遜遞遠適遲遷選遺遼邁還邊邏鄧鄭醜醫釀釁針釣鈉鈴鉛銅銘銀銳銷鋁鋒錢錦錯錄鍋鍵鎖鎮鏡鐵鑑長門閃閉開閒間閣閱闊隊陽陰陣階際陸隻難電霧靜靈響頁頂項順須頓頒預領頗頭頰頻題額顏願類飛飯飲飽餓餘馬駁駐駕駛驗驚髮鬆鬧魯鮮鳥鳴鴻鹽麗麥黃點黨齊齒龍]/;
const SIMPLIFIED_ONLY_RE =
  /[万与专业丛东丝丢两严丧个丰临为丽举么义乌乐乔习乡书买乱争于亏云亚产亩亲亿仅从仑仓仪们价众优会伞伟传伤伦伪伫体余杰倾储儿兑内册净冻凯别剂刚创划剧刘则刹剥劝办务动势勋胜劳汇区医华协单卖卢卫却厂厅参双发变叙叶号叹尝吓嘱园圆图团圣场坏块坚坛坟坠垒垄壮声壶寿梦伙夹夺奋奥妇妈娴娇学宁宝实宠审写宽将专寻对导层属岁岛岭岳币师帜帮干几库厕厢厦厨庙广庐异弃张弥弯汇彦后径复征恒恶惨惯态庆忧怜凭应怀悬忏恋战戏户扑执扩担据拟扰携摄摆摇败数敛毙斓斗断无旧时旷会胧杨极构标样枢楼乐朴机权归壳残毁毡气氢氩汉汤沟没灭沪滞渗浊浓湿灯灵灾炉争爷墙奖独兽获玑现玛产当叠痉痴发皱盗监盘众睁瞩础礼禅离稳穷窜窍竞笔筑简粮纠纪约红纹纳纽纯纱纸级纷细终组经结绝统绑绿网纲绽绰维绵紧绪线练县缝总绩织绘系继续缠纤罚骂罗羁习翘闻联聪耸胆脸腊台兴举舱艺节蓝处虚蜡补装里制复见规视觉览观计订讣记训托词咏试诗诚话诞诱语误说谁课调谅论诸诺谋谓谣谦讲谢谢证译誉读让赞岂贝负财责贤账货质贩贪贫贬购贯贴贵贷费贸贺贼贾贿资宾赐赏赔赖赚赛赠赢赵赶趋跃践踪车轧轨轩转轮软较载辅轻辆辉辈辑输辖辞农回这连周进游运过达违遥逊递远适迟迁选遗辽迈还边逻邓郑丑酿衅针钓钠铃铅铜铭银锐销铝锋钱锦错录锅键锁镇镜铁鉴长门闪闭开闲间阁阅阔队阳阴阵阶际陆只难电雾静响页顶项顺须顿颁预领颇头颊频题额颜愿类飞饭饮饱饿余马驳驻驾驶验惊发松闹鲁鲜鸟鸣鸿盐麦黄点党齐齿龙]/;

export const PROOF_BUILTIN_REGEX_RULES: ProofBuiltinRegexRule[] = [
  {
    id: "title-brackets",
    name: "标题括号",
    description: "标题中被 （）()【】[] 包起来的内容，替换为空",
  },
  {
    id: "ads",
    name: "广告/感言/PS",
    description: "月票、求票、订阅、本章完、请假条、上架感言，以及 PS 段落",
  },
  {
    id: "pinyin",
    name: "拼音",
    description: "括号内拼音、独立拼音行和行尾拼音注音，替换为空",
  },
];

function normalizeSpaces(text: string) {
  return text.replace(/\u3000/g, " ").replace(/[ \t]+/g, " ").trim();
}

function buildLineOffsets(lines: string[]) {
  const offsets: number[] = [];
  let cursor = 0;
  for (let i = 0; i < lines.length; i++) {
    offsets.push(cursor);
    cursor += lines[i].length + (i < lines.length - 1 ? 1 : 0);
  }
  return offsets;
}

function countMatches(text: string, source: RegExp) {
  const regex = new RegExp(source.source, "g");
  return text.match(regex)?.length ?? 0;
}

function shouldPreviewChineseConversion(
  original: string,
  replacement: string,
  direction: ProofConvertDirection,
) {
  if (replacement === original) return false;

  if (direction === "traditional-to-simplified") {
    if (!TRADITIONAL_ONLY_RE.test(original)) return false;
    return countMatches(replacement, TRADITIONAL_ONLY_RE) <= countMatches(original, TRADITIONAL_ONLY_RE);
  }

  if (!SIMPLIFIED_ONLY_RE.test(original)) return false;
  return countMatches(replacement, SIMPLIFIED_ONLY_RE) <= countMatches(original, SIMPLIFIED_ONLY_RE);
}

export function getChineseScriptProfile(content: string): ProofChineseScriptProfile {
  const simplified = countMatches(content, SIMPLIFIED_ONLY_RE);
  const traditional = countMatches(content, TRADITIONAL_ONLY_RE);
  let dominant: ProofChineseScriptProfile["dominant"] = "unknown";

  if (simplified > 0 || traditional > 0) {
    if (simplified >= traditional * 3 && simplified >= 20) {
      dominant = "simplified";
    } else if (traditional >= simplified * 3 && traditional >= 20) {
      dominant = "traditional";
    } else {
      dominant = "mixed";
    }
  }

  return { simplified, traditional, dominant };
}

function shouldSkipConvertPreviewByProfile(
  profile: ProofChineseScriptProfile,
  direction: ProofConvertDirection,
) {
  return (
    (profile.dominant === "simplified" && direction === "simplified-to-traditional") ||
    (profile.dominant === "traditional" && direction === "traditional-to-simplified")
  );
}

function buildConvertContext(
  chars: string[],
  start: number,
  end: number,
  replacement: string,
  contextSize = 6,
) {
  const contextStart = Math.max(0, start - contextSize);
  const contextEnd = Math.min(chars.length, end + contextSize);
  const before = chars.slice(contextStart, start).join("");
  const original = chars.slice(start, end).join("");
  const after = chars.slice(end, contextEnd).join("");
  return {
    original: `${contextStart > 0 ? "..." : ""}${before}${original}${after}${contextEnd < chars.length ? "..." : ""}`,
    replacement: `${contextStart > 0 ? "..." : ""}${before}${replacement}${after}${contextEnd < chars.length ? "..." : ""}`,
  };
}

function normalizePinyinToken(token: string) {
  return token
    .toLowerCase()
    .replace(/u:/g, "v")
    .replace(/[üǖǘǚǜ]/g, "v")
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/[^a-zv0-9]/g, "");
}

function isValidPinyinSyllable(raw: string) {
  if (!PINYIN_TOKEN_CHAR_RE.test(raw)) return false;

  let token = normalizePinyinToken(raw);
  if (!token) return false;
  if (/\d/.test(token) && !/[1-5]$/.test(token)) return false;

  token = token.replace(/[1-5]$/, "");
  if (token.length < 1 || token.length > 7) return false;
  if (token === "r" || token === "x" || token === "y") return false;

  const initial = token.match(/^(?:zh|ch|sh|[bpmfdtnlgkhjqxzcsrwy])/)?.[0] ?? "";
  const finalPart = token.slice(initial.length);
  if (!finalPart) return false;

  return PINYIN_FINALS.has(finalPart);
}

function isPinyinLike(text: string) {
  const trimmed = normalizeSpaces(text);
  if (!trimmed) return false;
  if (CHINESE_CHAR_RE.test(trimmed)) return false;
  if (PINYIN_NOISE_RE.test(trimmed)) return false;

  const tokens = trimmed.split(PINYIN_TOKEN_SPLIT_RE).filter(Boolean);
  if (!tokens.length || tokens.length > 12) return false;
  if (!tokens.every(isValidPinyinSyllable)) return false;

  return (
    PINYIN_TONE_MARK_RE.test(trimmed) ||
    PINYIN_TONE_NUMBER_RE.test(trimmed) ||
    tokens.length >= 2
  );
}

function stripTrailingPinyin(text: string) {
  return text.replace(PINYIN_TRAILING_CANDIDATE_RE, (match, tail: string) =>
    isPinyinLike(tail) ? "" : match,
  );
}

function cleanTitleBracketsLine(line: string) {
  const cleaned = line
    .replace(/[\uFF08(\u3010\[]([^\uFF08\uFF09()\[\]\u3010\u3011]*)[\uFF09)\u3011\]]/g, (match, inner: string) =>
      isTrivialBracketContent(inner) ? match : "",
    )
    .replace(/[ \t\u3000]+/g, " ")
    .replace(/\s*([\uFF1A:\u3001\uFF0C,.\uFF0E\-\u2014])\s*/g, "$1")
    .trim();
  if (cleaned === line.trim()) return line;
  const indent = line.match(/^[\s\u3000]*/)?.[0] ?? "";
  return cleaned ? `${indent}${cleaned}` : indent.trimEnd();
}

function isTrivialBracketContent(inner: string) {
  const compact = normalizeSpaces(inner).replace(/\s+/g, "");
  return (
    /^[\u4E0A\u4E0B]+$/.test(compact) ||
    /^[0-9]+$/.test(compact) ||
    /^[\u4E00\u4E8C\u4E09\u56DB]+$/.test(compact)
  );
}

function cleanPinyinLine(line: string) {
  const trimmed = line.trim();
  if (!trimmed) return line;

  if (!CHINESE_CHAR_RE.test(trimmed)) {
    return isPinyinLike(trimmed) ? "" : line;
  }

  const cleaned = stripTrailingPinyin(line)
    .replace(/[（(【\[]\s*([^（）()\[\]【】]+)\s*[）)】\]]/g, (match, inner) =>
      isPinyinLike(inner) ? "" : match,
    )
    .replace(/[ \t\u3000]+/g, " ")
    .replace(/\s*([：:、.．\-—])\s*/g, "$1")
    .trim();

  if (cleaned === trimmed) return line;
  const indent = line.match(/^[\s　]*/)?.[0] ?? "";
  return cleaned ? `${indent}${cleaned}` : indent.trimEnd();
}

function cleanPinyinLineStrict(line: string) {
  const trimmed = line.trim();
  if (!trimmed) return line;

  if (!CHINESE_CHAR_RE.test(trimmed)) {
    return isPinyinLike(trimmed) ? "" : line;
  }

  const withoutPinyin = stripTrailingPinyin(line).replace(
    PINYIN_BRACKET_RE,
    (match, inner: string) => (isPinyinLike(inner) ? "" : match),
  );

  if (withoutPinyin === line) return line;

  const cleaned = withoutPinyin
    .replace(/[ \t\u3000]+/g, " ")
    .replace(/\s*([\uFF1A:\u3001\uFF0C,.\uFF0E\-\u2014])\s*/g, "$1")
    .trim();

  if (cleaned === trimmed) return line;
  const indent = line.match(/^[\s\u3000]*/)?.[0] ?? "";
  return cleaned ? `${indent}${cleaned}` : indent.trimEnd();
}

function isTitleLikeLine(line: string) {
  const trimmed = normalizeSpaces(line);
  if (!trimmed) return false;
  if (
    /^第\s*[0-9零〇一二两三四五六七八九十百千万]+\s*(?:[章节回卷部])(?:\s|[:：、.．\-—]|$)/.test(
      trimmed,
    ) ||
    /^第\s*[0-9零〇一二两三四五六七八九十百千万]+\s*(?:[章节回卷部])$/.test(
      trimmed,
    ) ||
    /^Chapter\s*\d+(?:\s|[:：、.．\-—]|$)/i.test(trimmed) ||
    /^序列\s*[0-9零〇一二两三四五六七八九十百千万]+(?:\s|[:：、.．\-—]|$)/.test(
      trimmed,
    )
  ) {
    return true;
  }

  if (/^\d{1,5}(?:\s*[\.\．、:：\-—]\s*|\s+|(?=[\u4e00-\u9fff])).+/.test(trimmed)) return true;
  if (/^[一二三四五六七八九十百千万零〇两]{1,6}(?:\s*[\.\．、:：\-—]\s*|\s+).+/.test(trimmed)) return true;
  return false;
}

function toChineseNumber(num: number): string {
  if (!Number.isFinite(num) || num <= 0) return String(num);
  if (num < 10) return CHINESE_DIGITS[num];
  if (num < 20) {
    return num === 10 ? "十" : `十${CHINESE_DIGITS[num % 10]}`;
  }
  if (num < 100) {
    const tens = Math.floor(num / 10);
    const ones = num % 10;
    return `${CHINESE_DIGITS[tens]}十${ones ? CHINESE_DIGITS[ones] : ""}`;
  }
  if (num < 1000) {
    const hundreds = Math.floor(num / 100);
    const rest = num % 100;
    return `${CHINESE_DIGITS[hundreds]}百${rest ? (rest < 10 ? `零${toChineseNumber(rest)}` : toChineseNumber(rest)) : ""}`;
  }
  if (num < 10000) {
    const thousands = Math.floor(num / 1000);
    const rest = num % 1000;
    return `${CHINESE_DIGITS[thousands]}千${rest ? (rest < 100 ? `零${toChineseNumber(rest)}` : toChineseNumber(rest)) : ""}`;
  }
  const wan = Math.floor(num / 10000);
  const rest = num % 10000;
  return `${toChineseNumber(wan)}万${rest ? (rest < 1000 ? `零${toChineseNumber(rest)}` : toChineseNumber(rest)) : ""}`;
}

function formatNumber(num: number, style: ProofNumberStyle) {
  return style === "chinese" ? toChineseNumber(num) : String(num);
}

const CHINESE_NUMBER_MAP: Record<string, number> = {
  零: 0,
  〇: 0,
  一: 1,
  二: 2,
  两: 2,
  三: 3,
  四: 4,
  五: 5,
  六: 6,
  七: 7,
  八: 8,
  九: 9,
};

function parseChineseNumber(text: string): number | null {
  const input = text.replace(/\s+/g, "");
  if (!input) return null;
  if (/^\d+$/.test(input)) return Number.parseInt(input, 10);
  if (!/^[零〇一二两三四五六七八九十百千万]+$/.test(input)) return null;

  let total = 0;
  let section = 0;
  let number = 0;
  for (const char of input) {
    const digit = CHINESE_NUMBER_MAP[char];
    if (digit !== undefined) {
      number = digit;
      continue;
    }

    if (char === "十") {
      section += (number || 1) * 10;
    } else if (char === "百") {
      section += (number || 1) * 100;
    } else if (char === "千") {
      section += (number || 1) * 1000;
    } else if (char === "万") {
      total += (section + number) * 10000;
      section = 0;
    }
    number = 0;
  }

  return total + section + number || null;
}

function parseTitleNumber(original: string): number | null {
  const text = normalizeSpaces(original);
  const standard = text.match(
    /^第\s*([0-9零〇一二两三四五六七八九十百千万]+)\s*(?:章|卷|部|回|节)(?:\s|[:：、.．\-—]|$)/,
  );
  if (standard) return parseChineseNumber(standard[1]);

  const chapter = text.match(/^Chapter\s*(\d+)(?:\s|[:：、.．\-—]|$)/i);
  if (chapter) return Number.parseInt(chapter[1], 10);

  const sequence = text.match(
    /^序列\s*([0-9零〇一二两三四五六七八九十百千万]+)(?:\s|[:：、.．\-—]|$)/,
  );
  if (sequence) return parseChineseNumber(sequence[1]);

  const bracketed = text.match(/^[（(【\[]?\s*(\d{1,5})\s*[）)】\]]?(?:\s*[\.\．、:：\-—]\s*|\s+|(?=[\u4e00-\u9fff]))/);
  if (bracketed) return Number.parseInt(bracketed[1], 10);

  const chinese = text.match(/^([一二三四五六七八九十百千万零〇两]{1,8})(?:\s*[\.\．、:：\-—]\s*|\s+)/);
  if (chinese) return parseChineseNumber(chinese[1]);

  return null;
}

function extractTitleBody(original: string) {
  let text = normalizeSpaces(original);

  const patterns: RegExp[] = [
    /^第\s*[0-9零〇一二两三四五六七八九十百千万]+\s*(?:章|卷|部|回|节)\s*[：:、.．\-—\s]*/i,
    /^Chapter\s*\d+\s*[：:、.．\-—\s]*/i,
    /^序列\s*[0-9零〇一二两三四五六七八九十百千万]+\s*[：:、.．\-—\s]*/i,
    /^\(?\s*[（(【\[]?\s*\d+\s*[）)】\]]?\s*[：:、.．\-—\s]+/,
    /^\(?\s*[一二三四五六七八九十百千万零〇两]+\s*[：:、.．\-—\s]+/,
    /^\d{1,5}\s*[：:、.．\-—\s]+/,
    /^\d{1,5}(?=[\u4e00-\u9fff])\s*/,
  ];

  for (const re of patterns) {
    if (re.test(text)) {
      text = text.replace(re, "");
      break;
    }
  }

  text = text.replace(/^[：:、.．\-—\s]+/, "");
  return text.trim();
}

function isIncludedByScope(
  kind: "volume" | "chapter",
  original: string,
  options: ProofTitleRewriteOptions,
  regex: RegExp | null,
) {
  if (!hasExplicitNumberedTitle(kind, original)) return false;

  switch (options.scope) {
    case "volumes":
      return kind === "volume";
    case "chapters":
      return kind === "chapter";
    case "regex":
      return !!regex && regex.test(original);
    case "numbers-only":
      return parseTitleNumber(original) !== null;
    case "all":
    default:
      return true;
  }
}

function hasExplicitNumberedTitle(kind: "volume" | "chapter", original: string) {
  const text = normalizeSpaces(original);
  const num = String.raw`[0-9零〇一二两三四五六七八九十百千万]+`;
  const sep = String.raw`(?:\s|[:：、，,.\-—]|$)`;

  if (kind === "volume") {
    return new RegExp(String.raw`^(?:第\s*${num}\s*卷|卷\s*${num})${sep}`).test(text);
  }

  return (
    new RegExp(String.raw`^第\s*${num}\s*章${sep}`).test(text) ||
    new RegExp(String.raw`^\d{1,5}${sep}`).test(text)
  );
}

function getNodeKind(node: ProofTocNode): "volume" | "chapter" | null {
  if (node.type === "Volume") return "volume";
  if (node.type === "Chapter") return "chapter";
  return null;
}

function buildReplacementTitle(
  kind: "volume" | "chapter",
  index: number,
  original: string,
  options: ProofTitleRewriteOptions,
) {
  const body = extractTitleBody(original);
  const prefix = kind === "volume" ? "卷" : "章";
  const numberStyle =
    kind === "volume" ? options.volumeNumberStyle : options.chapterNumberStyle;
  const head = `第${formatNumber(index, numberStyle)}${prefix}`;
  return body ? `${head} ${body}` : head;
}

export function buildTitleRewritePreview(
  content: string,
  tocNodes: ProofTocNode[],
  options: ProofTitleRewriteOptions,
): ProofTitlePreviewRow[] {
  const normalized = content.replace(/\r\n|\r|\u2028|\u2029/g, "\n");
  const lines = normalized.split("\n");
  const rows: ProofTitlePreviewRow[] = [];
  let regex: RegExp | null = null;
  if (options.scope === "regex" && options.regex.trim()) {
    try {
      regex = new RegExp(options.regex, "i");
    } catch {
      return rows;
    }
  }

  if (options.scope === "regex" && !regex) {
    return rows;
  }

  let volumeIndex = 1;
  const chapterCounters = new Map<string, number>();
  const previousOriginalIndexes = new Map<string, number>();
  let currentVolumeKey = "root";

  for (const node of tocNodes) {
    const kind = getNodeKind(node);
    if (!kind) continue;

    const lineIndex = node.line - 1;
    if (lineIndex < 0 || lineIndex >= lines.length) continue;

    const original = lines[lineIndex].trim();
    if (kind === "volume") {
      const volumeKey = node.id || `${node.line}:${node.title}`;
      const included = isIncludedByScope(kind, original, options, regex);
      if (included) {
        const originalIndex = parseTitleNumber(original);
        const expectedIndex =
          options.scope === "numbers-only" && originalIndex !== null
            ? originalIndex
            : volumeIndex;
        const replacement = buildReplacementTitle(
          kind,
          expectedIndex,
          original,
          options,
        );
        rows.push({
          id: node.id || `volume:${node.line}`,
          line: node.line,
          kind,
          volumeKey,
          original,
          replacement,
          changed: replacement !== original,
          sequenceBroken: false,
          originalIndex,
          expectedIndex,
        });
        if (options.scope !== "numbers-only") {
          volumeIndex++;
        }
      }

      currentVolumeKey = volumeKey;
      if (!chapterCounters.has(currentVolumeKey)) {
        chapterCounters.set(currentVolumeKey, 1);
      }
      continue;
    }

    const included = isIncludedByScope(kind, original, options, regex);
    if (!included) continue;

    const groupKey = options.perVolume
      ? node.parentId || currentVolumeKey
      : "all";
    const originalIndex = parseTitleNumber(original);
    const nextIndex =
      options.scope === "numbers-only" && originalIndex !== null
        ? originalIndex
        : chapterCounters.get(groupKey) ?? 1;
    if (options.scope !== "numbers-only") {
      chapterCounters.set(groupKey, nextIndex + 1);
    }

    const replacement = buildReplacementTitle(
      kind,
      nextIndex,
      original,
      options,
    );
    const previousOriginalIndex = previousOriginalIndexes.get(groupKey);
    const sequenceBroken =
      options.scope !== "numbers-only" &&
      originalIndex !== null &&
      (previousOriginalIndex === undefined
        ? originalIndex !== nextIndex
        : originalIndex !== previousOriginalIndex + 1);
    if (originalIndex !== null) {
      previousOriginalIndexes.set(groupKey, originalIndex);
    }
    rows.push({
      id: node.id || `chapter:${node.line}`,
      line: node.line,
      kind,
      volumeKey: currentVolumeKey,
      original,
      replacement,
      changed: replacement !== original,
      sequenceBroken,
      originalIndex,
      expectedIndex: nextIndex,
    });
  }

  return rows;
}

export function applyTitleRewrite(
  content: string,
  tocNodes: ProofTocNode[],
  options: ProofTitleRewriteOptions,
): ProofTransformResult {
  const preview = buildTitleRewritePreview(content, tocNodes, options);
  if (preview.length === 0) {
    return {
      text: content,
      changedCount: 0,
      message: "没有可重排的标题",
    };
  }

  const normalized = content.replace(/\r\n|\r|\u2028|\u2029/g, "\n");
  const lines = normalized.split("\n");
  let changedCount = 0;

  for (const row of preview) {
    if (!row.changed) continue;
    const lineIndex = row.line - 1;
    if (lineIndex < 0 || lineIndex >= lines.length) continue;
    const currentLine = lines[lineIndex];
    const indent = currentLine.match(/^[\s　]*/)?.[0] ?? "";
    lines[lineIndex] = `${indent}${row.replacement}`;
    changedCount++;
  }

  return {
    text: lines.join("\n"),
    changedCount,
    message:
      changedCount > 0
        ? options.scope === "numbers-only"
          ? `已转换 ${changedCount} 个标题数字`
          : `已重排 ${changedCount} 个标题`
        : "标题已是标准格式",
  };
}

export function removeBracketContentFromTitles(
  content: string,
  tocNodes: ProofTocNode[],
): ProofTransformResult {
  const normalized = content.replace(/\r\n|\r|\u2028|\u2029/g, "\n");
  const lines = normalized.split("\n");
  let changedCount = 0;

  for (const node of tocNodes) {
    const lineIndex = node.line - 1;
    if (lineIndex < 0 || lineIndex >= lines.length) continue;
    const currentLine = lines[lineIndex];
    const cleaned = cleanTitleBracketsLine(currentLine);

    if (cleaned !== currentLine) {
      lines[lineIndex] = cleaned;
      changedCount++;
    }
  }

  return {
    text: lines.join("\n"),
    changedCount,
    message: changedCount > 0 ? `已清理 ${changedCount} 个标题括号` : "没有找到括号内容",
  };
}

export function removeCommonAds(content: string): ProofTransformResult {
  const normalized = content.replace(/\r\n|\r|\u2028|\u2029/g, "\n");
  const lines = normalized.split("\n");
  const output: string[] = [];
  let changedCount = 0;
  let skippingBlock = false;

  for (const line of lines) {
    const trimmed = line.trim();
    if (skippingBlock) {
      if (isTitleLikeLine(trimmed)) {
        skippingBlock = false;
        output.push(line);
      } else {
        changedCount++;
      }
      continue;
    }

    if (!trimmed) {
      output.push(line);
      continue;
    }

    if (AD_BLOCK_START_RE.test(trimmed)) {
      skippingBlock = true;
      changedCount++;
      continue;
    }

    if (AD_LINE_RE.test(trimmed)) {
      changedCount++;
      continue;
    }

    output.push(line);
  }

  return {
    text: output.join("\n"),
    changedCount,
    message:
      changedCount > 0 ? `已移除 ${changedCount} 行广告/说明` : "没有找到广告内容",
  };
}

export function removePinyin(content: string, tocNodes: ProofTocNode[]): ProofTransformResult {
  const normalized = content.replace(/\r\n|\r|\u2028|\u2029/g, "\n");
  const lines = normalized.split("\n");
  let changedCount = 0;

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const trimmed = line.trim();
    if (!trimmed) continue;

    if (!CHINESE_CHAR_RE.test(trimmed)) {
      if (isPinyinLike(trimmed)) {
        lines[i] = "";
        changedCount++;
      }
      continue;
    }

    const cleaned = cleanPinyinLineStrict(line);

    if (cleaned !== line) {
      lines[i] = cleaned;
      changedCount++;
    }
  }

  // 额外清理标题行里的拼音尾注。
  for (const node of tocNodes) {
    const lineIndex = node.line - 1;
    if (lineIndex < 0 || lineIndex >= lines.length) continue;
    const currentLine = lines[lineIndex];
    if (!CHINESE_CHAR_RE.test(currentLine)) continue;
    const cleaned = cleanPinyinLineStrict(currentLine);
    if (cleaned !== currentLine) {
      lines[lineIndex] = cleaned;
      changedCount++;
    }
  }

  return {
    text: lines.join("\n"),
    changedCount,
    message: changedCount > 0 ? `已清理 ${changedCount} 处拼音` : "没有找到拼音内容",
  };
}

export function buildBuiltinRegexPreview(
  content: string,
  ruleId: ProofBuiltinRuleId,
  tocNodes: ProofTocNode[] = [],
): ProofRegexPreviewRow[] {
  let normalized = content.replace(/\r\n|\r|\u2028|\u2029/g, "\n");
  const lines = normalized.split("\n");
  const lineOffsets = buildLineOffsets(lines);
  const rows: ProofRegexPreviewRow[] = [];
  const pushRow = (
    lineStart: number,
    lineEnd: number,
    original: string,
    replacement: string,
  ) => {
    const startOffset = lineOffsets[lineStart - 1] ?? 0;
    let endOffset =
      (lineOffsets[lineEnd - 1] ?? startOffset) + (lines[lineEnd - 1]?.length ?? 0);
    if (!replacement && lineEnd < lines.length) {
      endOffset += 1;
    }
    rows.push({
      id: `${ruleId}:${lineStart}:${lineEnd}:${rows.length}`,
      ruleId,
      lineStart,
      lineEnd,
      startOffset,
      endOffset,
      original,
      replacement,
    });
  };

  if (ruleId === "title-brackets") {
    const seenLines = new Set<number>();
    for (const node of tocNodes) {
      const lineNumber = node.line;
      const lineIndex = lineNumber - 1;
      if (seenLines.has(lineNumber) || lineIndex < 0 || lineIndex >= lines.length) continue;
      seenLines.add(lineNumber);
      const original = lines[lineIndex];
      const replacement = cleanTitleBracketsLine(original);
      if (replacement !== original) {
        pushRow(lineNumber, lineNumber, original, replacement);
      }
    }
    return rows;
  }

  if (ruleId === "ads") {
    let index = 0;
    while (index < lines.length) {
      const line = lines[index];
      const trimmed = line.trim();
      if (!trimmed) {
        index++;
        continue;
      }

      if (AD_BLOCK_START_RE.test(trimmed)) {
        const start = index;
        let end = index;
        index++;
        while (index < lines.length && !isTitleLikeLine(lines[index].trim())) {
          end = index;
          index++;
        }
        pushRow(start + 1, end + 1, lines.slice(start, end + 1).join("\n"), "");
        continue;
      }

      if (AD_LINE_RE.test(trimmed)) {
        pushRow(index + 1, index + 1, line, "");
      }
      index++;
    }
    return rows;
  }

  for (let index = 0; index < lines.length; index++) {
    const original = lines[index];
    const replacement = cleanPinyinLineStrict(original);
    if (replacement !== original) {
      pushRow(index + 1, index + 1, original, replacement);
    }
  }

  return rows;
}

export function applyBuiltinRegexPreview(
  content: string,
  rows: ProofRegexPreviewRow[],
  selectedIds: Set<string> | string[],
): ProofTransformResult {
  const selected = selectedIds instanceof Set ? selectedIds : new Set(selectedIds);
  const targets = rows
    .filter((row) => selected.has(row.id))
    .sort((a, b) => b.lineStart - a.lineStart);

  if (targets.length === 0) {
    return {
      text: content,
      changedCount: 0,
      message: "请选择要替换的匹配项",
    };
  }

  let normalized = content.replace(/\r\n|\r|\u2028|\u2029/g, "\n");
  let changedCount = 0;

  for (const row of targets) {
    if (row.startOffset < 0 || row.endOffset < row.startOffset || row.startOffset > normalized.length) {
      continue;
    }
    const before = normalized.slice(0, row.startOffset);
    const after = normalized.slice(row.endOffset);
    content = `${before}${row.replacement}${after}`;
    normalized = content;
    changedCount++;
  }

  return {
    text: normalized,
    changedCount,
    message: changedCount > 0 ? `已替换 ${changedCount} 个匹配项` : "没有可替换的匹配项",
  };
}

export async function convertChineseText(
  content: string,
  direction: ProofConvertDirection,
): Promise<ProofTransformResult> {
  const converter = await getChineseConverter(direction);
  const next = converter(content);
  let changedCount = 0;
  const len = Math.min(content.length, next.length);
  for (let i = 0; i < len; i++) {
    if (content[i] !== next[i]) changedCount++;
  }
  changedCount += Math.abs(content.length - next.length);

  return {
    text: next,
    changedCount,
    message:
      changedCount > 0
        ? direction === "traditional-to-simplified"
          ? "已转换为简体"
          : "已转换为繁体"
        : "没有需要转换的内容",
  };
}

export async function buildChineseConvertPreview(
  content: string,
  direction: ProofConvertDirection,
): Promise<ProofConvertPreviewRow[]> {
  const converter = await getChineseConverter(direction);
  const profile = getChineseScriptProfile(content);
  if (shouldSkipConvertPreviewByProfile(profile, direction)) {
    return [];
  }

  const normalized = content.replace(/\r\n|\r|\u2028|\u2029/g, "\n");
  const lines = normalized.split("\n");
  const rows: ProofConvertPreviewRow[] = [];
  const targetCharRe =
    direction === "traditional-to-simplified"
      ? TRADITIONAL_ONLY_RE
      : SIMPLIFIED_ONLY_RE;

  lines.forEach((line, index) => {
    const chars = Array.from(line);
    let cursor = 0;

    while (cursor < chars.length) {
      if (!targetCharRe.test(chars[cursor])) {
        cursor++;
        continue;
      }

      const start = cursor;
      cursor++;
      while (cursor < chars.length && targetCharRe.test(chars[cursor])) {
        cursor++;
      }

      const end = cursor;
      const originalText = chars.slice(start, end).join("");
      const replacementText = converter(originalText);
      if (!shouldPreviewChineseConversion(originalText, replacementText, direction)) continue;
      const context = buildConvertContext(chars, start, end, replacementText);
      rows.push({
        id: `convert:${index + 1}:${start}:${end}`,
        lineStart: index + 1,
        lineEnd: index + 1,
        startChar: start,
        endChar: end,
        original: context.original,
        replacement: context.replacement,
        originalText,
        replacementText,
      });
    }
  });

  return rows;
}

export function applyChineseConvertPreview(
  content: string,
  rows: ProofConvertPreviewRow[],
  selectedIds: Set<string> | string[],
): ProofTransformResult {
  const selected = selectedIds instanceof Set ? selectedIds : new Set(selectedIds);
  const targets = rows
    .filter((row) => selected.has(row.id))
    .sort((a, b) => b.lineStart - a.lineStart || b.startChar - a.startChar);

  if (targets.length === 0) {
    return {
      text: content,
      changedCount: 0,
      message: "请选择要转换的匹配项",
    };
  }

  const normalized = content.replace(/\r\n|\r|\u2028|\u2029/g, "\n");
  const lines = normalized.split("\n");
  let changedCount = 0;

  for (const row of targets) {
    const index = row.lineStart - 1;
    if (index < 0 || index >= lines.length) continue;
    const chars = Array.from(lines[index]);
    if (row.startChar < 0 || row.endChar <= row.startChar || row.startChar >= chars.length) continue;
    const originalText = chars.slice(row.startChar, row.endChar).join("");
    if (originalText !== row.originalText || row.replacementText === originalText) continue;
    chars.splice(row.startChar, row.endChar - row.startChar, row.replacementText);
    lines[index] = chars.join("");
    changedCount++;
  }

  return {
    text: lines.join("\n"),
    changedCount,
    message: changedCount > 0 ? `已转换 ${changedCount} 处` : "没有可转换的内容",
  };
}

function replaceWithCount(input: string, regex: RegExp, replacement: string) {
  regex.lastIndex = 0;
  const changedCount = Array.from(input.matchAll(regex)).length;
  regex.lastIndex = 0;
  if (changedCount === 0) {
    return { text: input, changedCount };
  }

  const text = input.replace(regex, replacement);
  regex.lastIndex = 0;
  return { text, changedCount };
}

export function replaceByRegex(
  content: string,
  pattern: string,
  replacement: string,
  options: {
    caseSensitive?: boolean;
    targetTitles?: boolean;
    tocNodes?: ProofTocNode[];
  } = {},
): ProofTransformResult {
  if (!pattern.trim()) {
    return {
      text: content,
      changedCount: 0,
      message: "请输入正则表达式",
    };
  }

  const flags = `g${options.caseSensitive ? "" : "i"}`;
  let regex: RegExp;
  try {
    regex = new RegExp(pattern, flags);
  } catch (error: any) {
    return {
      text: content,
      changedCount: 0,
      message: `正则错误: ${error?.message || error}`,
    };
  }

  const normalized = content.replace(/\r\n|\r|\u2028|\u2029/g, "\n");
  const lines = normalized.split("\n");

  if (options.targetTitles) {
    const tocNodes = options.tocNodes ?? [];
    let changedCount = 0;

    for (const node of tocNodes) {
      const lineIndex = node.line - 1;
      if (lineIndex < 0 || lineIndex >= lines.length) continue;
      const currentLine = lines[lineIndex];
      const { text: next, changedCount: localCount } = replaceWithCount(
        currentLine,
        regex,
        replacement,
      );
      if (localCount > 0) {
        lines[lineIndex] = next;
        changedCount += localCount;
      }
    }

    return {
      text: lines.join("\n"),
      changedCount,
      message: changedCount > 0 ? `已替换 ${changedCount} 处标题` : "没有匹配到标题",
    };
  }

  const { text: next, changedCount } = replaceWithCount(
    normalized,
    regex,
    replacement,
  );

  return {
    text: next,
    changedCount,
    message: changedCount > 0 ? `已替换 ${changedCount} 处` : "没有匹配到内容",
  };
}

export function buildProofPreviewSummary(rows: ProofTitlePreviewRow[]) {
  const changed = rows.filter((row) => row.changed).length;
  return {
    total: rows.length,
    changed,
  };
}
