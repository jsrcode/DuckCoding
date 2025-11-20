import type { JsonObject, JsonValue } from '@/lib/tauri-commands';
import type { CustomFieldType, DiffEntry, JSONSchema } from './types';

export function resolveSchema(
  schema: JSONSchema | undefined,
  rootSchema: JSONSchema | null,
): JSONSchema | undefined {
  if (!schema) {
    return undefined;
  }
  if (schema.$ref && rootSchema) {
    const resolved = resolveRef(rootSchema, schema.$ref);
    if (resolved) {
      const { $ref: _ref, ...rest } = schema;
      return { ...resolved, ...rest };
    }
  }
  return schema;
}

export function resolveRef(schema: JSONSchema, ref: string): JSONSchema | undefined {
  if (!ref.startsWith('#/')) {
    return undefined;
  }
  const path = ref
    .substring(2)
    .split('/')
    .map((segment) => segment.replace(/~1/g, '/').replace(/~0/g, '~'));

  let current: unknown = schema;
  for (const segment of path) {
    if (current && typeof current === 'object' && segment in current) {
      current = (current as Record<string, unknown>)[segment];
    } else {
      return undefined;
    }
  }
  return current as JSONSchema;
}

export function getTypeLabel(schema?: JSONSchema, value?: JsonValue): string {
  const type = getPrimaryType(schema) ?? inferValueType(value);
  if (!type) {
    return 'string';
  }
  return type;
}

export function getPrimaryType(schema?: JSONSchema): string | undefined {
  if (!schema) {
    return undefined;
  }
  if (Array.isArray(schema.type)) {
    return schema.type[0];
  }
  return schema.type;
}

export function getEffectiveType(schema?: JSONSchema, value?: JsonValue): string | undefined {
  const schemaType = getPrimaryType(schema);
  if (schemaType) {
    return schemaType;
  }
  return inferValueType(value);
}

export function inferValueType(value: JsonValue | undefined): string | undefined {
  if (value === null || value === undefined) {
    return undefined;
  }
  if (Array.isArray(value)) {
    return 'array';
  }
  if (typeof value === 'object') {
    return 'object';
  }
  if (typeof value === 'boolean') {
    return 'boolean';
  }
  if (typeof value === 'number') {
    return 'number';
  }
  if (typeof value === 'string') {
    return 'string';
  }
  return undefined;
}

export function isCompoundField(schema?: JSONSchema, value?: JsonValue) {
  const type = getEffectiveType(schema, value);
  if (!type) {
    return Array.isArray(value) || isJsonObject(value);
  }
  return type === 'object' || type === 'array';
}

export function getDefaultValue(schema?: JSONSchema): JsonValue {
  if (!schema) {
    return '';
  }

  if (schema.default !== undefined) {
    return cloneJsonValue(schema.default as JsonValue);
  }

  const type = getPrimaryType(schema);
  switch (type) {
    case 'object':
      return {};
    case 'array':
      return [];
    case 'boolean':
      return false;
    case 'number':
    case 'integer':
      return 0;
    default:
      if (schema.enum && schema.enum.length > 0) {
        return schema.enum[0] as JsonValue;
      }
      return '';
  }
}

export function createSchemaForType(type?: CustomFieldType): JSONSchema | undefined {
  if (!type) {
    return undefined;
  }
  if (type === 'number') {
    return { type: 'number' };
  }
  return { type };
}

export function isJsonObject(value: JsonValue | undefined): value is JsonObject {
  return !!value && typeof value === 'object' && !Array.isArray(value);
}

export function isJsonSchemaObject(value: unknown): value is JSONSchema {
  return !!value && typeof value === 'object' && !Array.isArray(value);
}

export function cloneJsonObject<T extends JsonObject>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}

export function cloneJsonValue<T extends JsonValue>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}

export function safeStringify(value: JsonValue | undefined) {
  try {
    if (value === undefined) {
      return '';
    }
    if (typeof value === 'string') {
      return value;
    }
    return JSON.stringify(value, null, 2);
  } catch {
    return '';
  }
}

export function formatJson(value: JsonValue | undefined) {
  const text = safeStringify(value);
  return text || '—';
}

export function buildDiffEntries(
  path: (string | number)[],
  original: JsonValue | undefined,
  current: JsonValue | undefined,
  diffs: DiffEntry[],
) {
  if (original === undefined && current === undefined) {
    return;
  }

  if (original === undefined && current !== undefined) {
    diffs.push({
      path: formatPath(path),
      type: 'added',
      after: cloneJsonValue(current as JsonValue),
    });
    return;
  }

  if (original !== undefined && current === undefined) {
    diffs.push({
      path: formatPath(path),
      type: 'removed',
      before: cloneJsonValue(original as JsonValue),
    });
    return;
  }

  if (isJsonObject(original) && isJsonObject(current)) {
    const keys = new Set([...Object.keys(original), ...Object.keys(current)]);
    keys.forEach((key) => {
      buildDiffEntries([...path, key], original[key], current[key], diffs);
    });
    return;
  }

  if (Array.isArray(original) && Array.isArray(current)) {
    const maxLength = Math.max(original.length, current.length);
    for (let index = 0; index < maxLength; index++) {
      buildDiffEntries([...path, index], original[index], current[index], diffs);
    }
    return;
  }

  if (JSON.stringify(original) !== JSON.stringify(current)) {
    diffs.push({
      path: formatPath(path),
      type: 'changed',
      before: cloneJsonValue(original as JsonValue),
      after: cloneJsonValue(current as JsonValue),
    });
  }
}

export function formatPath(path: (string | number)[]): string {
  if (path.length === 0) {
    return '(root)';
  }
  return path.reduce<string>((acc, segment) => {
    if (typeof segment === 'number') {
      return `${acc}[${segment}]`;
    }
    return acc ? `${acc}.${segment}` : String(segment);
  }, '');
}

export function getObjectFromPath(root: JsonObject, path: string): JsonObject | undefined {
  const segments = path
    .split('.')
    .map((segment) => segment.trim())
    .filter(Boolean);
  let current: JsonValue | undefined = root;
  for (const segment of segments) {
    if (!isJsonObject(current)) {
      return undefined;
    }
    current = current[segment];
  }
  return isJsonObject(current) ? current : undefined;
}
