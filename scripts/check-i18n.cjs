const fs = require("node:fs");
const path = require("node:path");
const ts = require("typescript");
const { parse: parseSvelte } = require("svelte/compiler");

const HAN_RE = /\p{Script=Han}/u;
const PLACEHOLDER_RE = /\{([A-Za-z_][A-Za-z0-9_]*)\}/g;
const TRANSLATABLE_ATTRIBUTES = new Set([
  "title",
  "placeholder",
  "aria-label",
  "alt",
]);
const OPAQUE_ELEMENTS = new Set(["script", "style", "pre", "code", "textarea"]);
const LOCALIZABLE_DATA_FIELDS = new Set([
  "title",
  "label",
  "detail",
  "description",
  "hint",
  "message",
  "kicker",
  "note",
  "reason",
  "usage",
  "placeholder",
  "confirmLabel",
  "cancelLabel",
  "secondaryLabel",
  "outputLabel",
  "privacy",
]);
const root = path.resolve("src");
const dictionaryFiles = [
  "lib/i18n.svelte.ts",
  "lib/i18nStaticEn.ts",
  "lib/i18nDynamicEn.ts",
  "lib/i18nRuntimeEn.ts",
];

function walkFiles(dir) {
  return fs.readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const full = path.join(dir, entry.name);
    return entry.isDirectory() ? walkFiles(full) : [full];
  });
}

function propertyNameText(name) {
  if (ts.isStringLiteral(name) || ts.isNoSubstitutionTemplateLiteral(name)) {
    return name.text;
  }
  if (ts.isIdentifier(name)) return name.text;
  return null;
}

function dictionaryEntriesFromTs(file) {
  const sourceText = fs.readFileSync(file, "utf8");
  const source = ts.createSourceFile(
    file,
    sourceText,
    ts.ScriptTarget.Latest,
    true,
    ts.ScriptKind.TS,
  );
  const entries = [];
  function visit(node) {
    if (
      ts.isVariableDeclaration(node) &&
      ts.isIdentifier(node.name) &&
      /MESSAGES$/.test(node.name.text) &&
      node.initializer &&
      ts.isObjectLiteralExpression(node.initializer)
    ) {
      for (const prop of node.initializer.properties) {
        if (!ts.isPropertyAssignment(prop)) continue;
        const key = propertyNameText(prop.name);
        if (key === null) continue;
        entries.push({
          file,
          line:
            source.getLineAndCharacterOfPosition(prop.getStart(source)).line +
            1,
          key,
          value:
            ts.isStringLiteral(prop.initializer) ||
            ts.isNoSubstitutionTemplateLiteral(prop.initializer)
              ? prop.initializer.text
              : null,
        });
      }
    }
    ts.forEachChild(node, visit);
  }
  visit(source);
  return entries;
}

function literalValuesFromTs(expression) {
  if (!expression) return [];
  if (
    ts.isStringLiteral(expression) ||
    ts.isNoSubstitutionTemplateLiteral(expression)
  ) {
    return [expression.text];
  }
  if (ts.isConditionalExpression(expression)) {
    return [
      ...literalValuesFromTs(expression.whenTrue),
      ...literalValuesFromTs(expression.whenFalse),
    ];
  }
  if (
    ts.isParenthesizedExpression(expression) ||
    ts.isAsExpression(expression) ||
    ts.isSatisfiesExpression(expression) ||
    ts.isNonNullExpression(expression)
  ) {
    return literalValuesFromTs(expression.expression);
  }
  return [];
}

function literalValuesFromEstree(expression) {
  if (!expression) return [];
  if (expression.type === "Literal" && typeof expression.value === "string") {
    return [expression.value];
  }
  if (
    expression.type === "TemplateLiteral" &&
    expression.expressions?.length === 0
  ) {
    return [expression.quasis?.[0]?.value?.cooked ?? ""];
  }
  if (expression.type === "ConditionalExpression") {
    return [
      ...literalValuesFromEstree(expression.consequent),
      ...literalValuesFromEstree(expression.alternate),
    ];
  }
  if (expression.type === "ParenthesizedExpression") {
    return literalValuesFromEstree(expression.expression);
  }
  if (
    expression.type === "TSAsExpression" ||
    expression.type === "TSSatisfiesExpression" ||
    expression.type === "TSNonNullExpression"
  ) {
    return literalValuesFromEstree(expression.expression);
  }
  return [];
}

function translatedKeysFromTs(file) {
  const source = ts.createSourceFile(
    file,
    fs.readFileSync(file, "utf8"),
    ts.ScriptTarget.Latest,
    true,
    ts.ScriptKind.TS,
  );
  const keys = [];
  function visit(node) {
    if (
      ts.isCallExpression(node) &&
      ts.isIdentifier(node.expression) &&
      (node.expression.text === "t" || node.expression.text === "__tepubT")
    ) {
      for (const value of literalValuesFromTs(node.arguments[0])) {
        if (HAN_RE.test(value)) {
          keys.push({
            file,
            line:
              source.getLineAndCharacterOfPosition(node.getStart(source)).line +
              1,
            value,
          });
        }
      }
    }
    ts.forEachChild(node, visit);
  }
  visit(source);
  return keys;
}

function localizableKeysFromTs(file) {
  const source = ts.createSourceFile(
    file,
    fs.readFileSync(file, "utf8"),
    ts.ScriptTarget.Latest,
    true,
    ts.ScriptKind.TS,
  );
  const keys = [];
  function visit(node) {
    if (ts.isPropertyAssignment(node)) {
      const key = propertyNameText(node.name);
      if (key && LOCALIZABLE_DATA_FIELDS.has(key)) {
        for (const value of literalValuesFromTs(node.initializer)) {
          if (HAN_RE.test(value)) {
            keys.push({
              file,
              line:
                source.getLineAndCharacterOfPosition(node.getStart(source))
                  .line + 1,
              value,
            });
          }
        }
      }
    }
    ts.forEachChild(node, visit);
  }
  visit(source);
  return keys;
}

function translatedKeysFromSvelte(file) {
  const ast = parseSvelte(fs.readFileSync(file, "utf8"), { modern: true });
  const keys = [];
  const seen = new WeakSet();

  function visit(node) {
    if (!node || typeof node !== "object" || seen.has(node)) return;
    seen.add(node);
    if (Array.isArray(node)) {
      for (const child of node) visit(child);
      return;
    }
    if (
      node.type === "CallExpression" &&
      node.callee?.type === "Identifier" &&
      (node.callee.name === "t" || node.callee.name === "__tepubT")
    ) {
      for (const value of literalValuesFromEstree(node.arguments?.[0])) {
        if (HAN_RE.test(value)) {
          keys.push({
            file,
            line: node.loc?.start.line ?? 0,
            value,
          });
        }
      }
    }
    if (node.type === "Property" && !node.computed) {
      const key =
        node.key?.type === "Identifier"
          ? node.key.name
          : node.key?.type === "Literal" &&
              typeof node.key.value === "string"
            ? node.key.value
            : null;
      if (key && LOCALIZABLE_DATA_FIELDS.has(key)) {
        for (const value of literalValuesFromEstree(node.value)) {
          if (HAN_RE.test(value)) {
            keys.push({
              file,
              line: node.loc?.start.line ?? 0,
              value,
            });
          }
        }
      }
    }
    for (const [key, value] of Object.entries(node)) {
      if (
        key === "loc" ||
        key === "start" ||
        key === "end" ||
        key === "parent"
      ) {
        continue;
      }
      visit(value);
    }
  }

  visit(ast.instance?.content);
  visit(ast.fragment);
  return keys;
}

function staticKeysFromSvelte(file) {
  const ast = parseSvelte(fs.readFileSync(file, "utf8"), { modern: true });
  const keys = [];

  function add(source, node) {
    const value = String(source || "")
      .replace(/\s+/g, " ")
      .trim();
    if (!value || !HAN_RE.test(value)) return;
    keys.push({
      file,
      line: node.loc?.start.line ?? 0,
      value,
    });
  }

  function visitFragment(fragment, parentName) {
    for (const child of fragment?.nodes ?? []) {
      visitNode(child, parentName);
    }
  }

  function visitNode(node, parentName) {
    if (!node) return;
    switch (node.type) {
      case "Text":
        if (!OPAQUE_ELEMENTS.has(parentName)) add(node.data, node);
        return;
      case "RegularElement":
      case "SvelteElement":
      case "Component":
      case "SvelteComponent":
      case "SvelteFragment":
        for (const attribute of node.attributes ?? []) {
          if (
            attribute.type !== "Attribute" ||
            !TRANSLATABLE_ATTRIBUTES.has(attribute.name)
          ) {
            continue;
          }
          const values = Array.isArray(attribute.value)
            ? attribute.value
            : attribute.value
              ? [attribute.value]
              : [];
          if (values.length === 1 && values[0].type === "Text") {
            add(values[0].data, values[0]);
          }
        }
        visitFragment(
          node.fragment,
          String(node.name || parentName).toLowerCase(),
        );
        return;
      case "Fragment":
        visitFragment(node, parentName);
        return;
      case "IfBlock":
        visitFragment(node.consequent, parentName);
        visitNode(node.alternate, parentName);
        return;
      case "EachBlock":
        visitFragment(node.body, parentName);
        visitNode(node.fallback, parentName);
        return;
      case "AwaitBlock":
        visitFragment(node.pending, parentName);
        visitFragment(node.then, parentName);
        visitFragment(node.catch, parentName);
        return;
      case "KeyBlock":
      case "SnippetBlock":
        visitFragment(node.fragment || node.body, parentName);
        return;
      case "Slot":
        visitFragment(node.fragment, parentName);
        return;
      case "SvelteHead":
      case "TitleElement":
        visitFragment(node.fragment, parentName);
        return;
      default:
        return;
    }
  }

  visitFragment(ast.fragment, "");
  return keys;
}

function placeholders(value) {
  return new Set(
    Array.from(value.matchAll(PLACEHOLDER_RE), (match) => match[1]),
  );
}

const dictionaryEntries = [];
for (const file of dictionaryFiles) {
  dictionaryEntries.push(...dictionaryEntriesFromTs(path.join(root, file)));
}

const dictionary = new Map();
for (const entry of dictionaryEntries) {
  const current = dictionary.get(entry.key);
  if (current) current.push(entry);
  else dictionary.set(entry.key, [entry]);
}

const hits = [];
for (const file of walkFiles(root)) {
  if (file.endsWith(".test.ts")) continue;
  if (file.endsWith(".svelte")) {
    hits.push(...translatedKeysFromSvelte(file));
    hits.push(...staticKeysFromSvelte(file));
  } else if (file.endsWith(".ts") && !file.endsWith(".d.ts")) {
    hits.push(...translatedKeysFromTs(file));
    hits.push(...localizableKeysFromTs(file));
  }
}

const missing = hits.filter((hit) => !dictionary.has(hit.value));
const duplicates = [];
const placeholderMismatches = [];
for (const [key, entries] of dictionary) {
  if (entries.length > 1) duplicates.push({ key, entries });
  for (const entry of entries) {
    if (entry.value === null) continue;
    const sourcePlaceholders = placeholders(key);
    const valuePlaceholders = placeholders(entry.value);
    const missingInValue = [...sourcePlaceholders].filter(
      (name) => !valuePlaceholders.has(name),
    );
    const extraInValue = [...valuePlaceholders].filter(
      (name) => !sourcePlaceholders.has(name),
    );
    if (missingInValue.length || extraInValue.length) {
      placeholderMismatches.push({
        ...entry,
        missingInValue,
        extraInValue,
      });
    }
  }
}

if (process.argv.includes("--keys")) {
  console.log(
    JSON.stringify([...new Set(missing.map((hit) => hit.value))], null, 2),
  );
  process.exit(missing.length ? 1 : 0);
}

for (const hit of missing) {
  console.log(
    `MISSING ${path.relative(process.cwd(), hit.file)}:${hit.line}: ${hit.value}`,
  );
}
for (const duplicate of duplicates) {
  console.log(`DUPLICATE ${duplicate.key}`);
  for (const entry of duplicate.entries) {
    console.log(`  ${path.relative(process.cwd(), entry.file)}:${entry.line}`);
  }
}
for (const mismatch of placeholderMismatches) {
  const details = [
    mismatch.missingInValue.length
      ? `missing [${mismatch.missingInValue.join(", ")}]`
      : "",
    mismatch.extraInValue.length
      ? `extra [${mismatch.extraInValue.join(", ")}]`
      : "",
  ]
    .filter(Boolean)
    .join("; ");
  console.log(
    `PLACEHOLDER ${path.relative(process.cwd(), mismatch.file)}:${mismatch.line}: ${mismatch.key} (${details})`,
  );
}

console.log(
  `\nMissing ${missing.length}; duplicate keys ${duplicates.length}; placeholder mismatches ${placeholderMismatches.length}; translated literal keys ${hits.length}; dictionary keys ${dictionary.size}`,
);

if (missing.length || duplicates.length || placeholderMismatches.length) {
  process.exitCode = 1;
}
