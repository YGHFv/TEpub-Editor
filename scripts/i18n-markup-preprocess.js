import { parse } from "svelte/compiler";

/** @typedef {{ start: number; end: number; text: string }} Edit */

const HAN_RE = /\p{Script=Han}/u;
const TRANSLATABLE_ATTRIBUTES = new Set([
  "title",
  "placeholder",
  "aria-label",
  "alt",
]);
const OPAQUE_ELEMENTS = new Set(["script", "style", "pre", "code", "textarea"]);
const I18N_IMPORT_RE =
  /import\s*\{([^}]*)\}\s*from\s*["']\$lib\/i18n\.svelte["']/;

/** @param {string} value */
function normalizeSource(value) {
  return value.replace(/\s+/g, " ").trim();
}

/**
 * @param {string} identifier
 * @param {string} source
 */
function translateExpression(identifier, source) {
  return `{${identifier}(${JSON.stringify(source)})}`;
}

/**
 * @param {any} node
 * @param {string} parentName
 * @param {Edit[]} edits
 */
function addTextEdit(node, parentName, edits) {
  if (OPAQUE_ELEMENTS.has(parentName)) return;
  const source = normalizeSource(node.data);
  if (!source || !HAN_RE.test(source)) return;

  const leading = node.data.match(/^\s*/)?.[0] ?? "";
  const trailing = node.data.match(/\s*$/)?.[0] ?? "";
  edits.push({
    start: node.start,
    end: node.end,
    text: `${leading}${translateExpression("__tepubT", source)}${trailing}`,
  });
}

/**
 * @param {any} attribute
 * @param {Edit[]} edits
 */
function addAttributeEdit(attribute, edits) {
  if (
    attribute.type !== "Attribute" ||
    !TRANSLATABLE_ATTRIBUTES.has(attribute.name)
  ) {
    return;
  }
  const values = Array.isArray(attribute.value)
    ? attribute.value
    : attribute.value
      ? [attribute.value]
      : [];
  if (values.length !== 1 || values[0].type !== "Text") return;

  const source = normalizeSource(values[0].data);
  if (!source || !HAN_RE.test(source)) return;
  edits.push({
    start: attribute.start,
    end: attribute.end,
    text: `${attribute.name}=${translateExpression("__tepubT", source)}`,
  });
}

/**
 * @param {any} fragment
 * @param {string} parentName
 * @param {Edit[]} edits
 */
function visitFragment(fragment, parentName, edits) {
  if (!fragment) return;
  for (const node of fragment.nodes ?? []) {
    visitNode(node, parentName, edits);
  }
}

/**
 * @param {any} node
 * @param {string} parentName
 * @param {Edit[]} edits
 */
function visitNode(node, parentName, edits) {
  if (!node) return;

  switch (node.type) {
    case "Text":
      addTextEdit(node, parentName, edits);
      return;
    case "RegularElement":
    case "SvelteElement":
    case "Component":
    case "SvelteComponent":
    case "SvelteFragment":
      for (const attribute of node.attributes ?? []) {
        addAttributeEdit(attribute, edits);
      }
      visitFragment(
        node.fragment,
        String(node.name || parentName).toLowerCase(),
        edits,
      );
      return;
    case "Fragment":
      visitFragment(node, parentName, edits);
      return;
    case "IfBlock":
      visitFragment(node.consequent, parentName, edits);
      visitNode(node.alternate, parentName, edits);
      return;
    case "EachBlock":
      visitFragment(node.body, parentName, edits);
      visitNode(node.fallback, parentName, edits);
      return;
    case "AwaitBlock":
      visitFragment(node.pending, parentName, edits);
      visitFragment(node.then, parentName, edits);
      visitFragment(node.catch, parentName, edits);
      return;
    case "KeyBlock":
    case "SnippetBlock":
      visitFragment(node.fragment || node.body, parentName, edits);
      return;
    case "Slot":
      visitFragment(node.fragment, parentName, edits);
      return;
    case "SvelteHead":
    case "TitleElement":
      visitFragment(node.fragment, parentName, edits);
      return;
    default:
      return;
  }
}

/** @param {string} content */
function findMarkupImport(content) {
  const match = content.match(I18N_IMPORT_RE);
  if (!match) return null;
  const members = match[1]
    .split(",")
    .map((member) => member.trim())
    .filter(Boolean);
  for (const member of members) {
    const alias = member.match(/^t\s+as\s+(\w+)$/);
    if (alias) return alias[1];
    if (member === "t") return "t";
  }
  return null;
}

/**
 * @param {string} content
 * @param {any} ast
 * @param {Edit[]} edits
 */
function addImport(content, ast, edits) {
  const existing = findMarkupImport(content);
  if (existing) return existing;

  if (ast.instance) {
    edits.push({
      start: ast.instance.content.start,
      end: ast.instance.content.start,
      text: '\n    import { t as __tepubT } from "$lib/i18n.svelte";',
    });
  } else {
    edits.push({
      start: ast.fragment.start,
      end: ast.fragment.start,
      text: '<script lang="ts">import { t as __tepubT } from "$lib/i18n.svelte";</script>\n',
    });
  }
  return "__tepubT";
}

export function i18nMarkupPreprocess() {
  return {
    name: "tepub-i18n-markup",
    /** @param {{ content: string; filename?: string }} input */
    markup(input) {
      const { content, filename } = input;
      if (!filename?.endsWith(".svelte")) return;

      const ast = parse(content, { modern: true });
      /** @type {Edit[]} */
      const edits = [];
      visitFragment(ast.fragment, "", edits);
      if (!edits.length) return;

      const identifier = addImport(content, ast, edits);
      if (identifier !== "__tepubT") {
        for (const edit of edits) {
          edit.text = edit.text.replaceAll("__tepubT", identifier);
        }
      }

      edits.sort((a, b) => b.start - a.start || b.end - a.end);
      let code = content;
      for (const edit of edits) {
        code = code.slice(0, edit.start) + edit.text + code.slice(edit.end);
      }
      return { code };
    },
  };
}
