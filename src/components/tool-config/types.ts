import type { JsonObject, JsonSchema, JsonValue, GeminiEnvConfig } from '@/lib/tauri-commands';

export type JSONSchema = JsonSchema & {
  type?: string | string[];
  description?: string;
  properties?: Record<string, JSONSchema>;
  items?: JSONSchema | JSONSchema[];
  enum?: (string | number)[];
  const?: unknown;
  $ref?: string;
  additionalProperties?: boolean | JSONSchema;
  default?: unknown;
  anyOf?: JSONSchema[];
  allOf?: JSONSchema[];
  oneOf?: JSONSchema[];
  patternProperties?: Record<string, JSONSchema>;
  examples?: unknown[];
  required?: string[];
  $defs?: Record<string, JSONSchema>;
  'x-secret'?: boolean;
};

export interface SchemaOption {
  key: string;
  description: string;
  schema?: JSONSchema;
  typeLabel: string;
}

export interface SchemaFieldProps {
  schema?: JSONSchema;
  value: JsonValue | undefined;
  onChange: (value: JsonValue) => void;
  onDelete?: () => void;
  path: (string | number)[];
  rootSchema: JSONSchema | null;
  isRequired?: boolean;
  showDescription?: boolean;
  inline?: boolean;
  rootValue: JsonObject;
}

export interface DiffEntry {
  path: string;
  type: 'added' | 'removed' | 'changed';
  before?: JsonValue;
  after?: JsonValue;
}

export interface ToolConfigManagerProps {
  title: string;
  description: string;
  loadSchema: () => Promise<JsonSchema>;
  loadSettings: () => Promise<JsonObject>;
  saveSettings: (settings: JsonObject) => Promise<void>;
  emptyHint?: string;
  refreshSignal?: number;
  externalDirty?: boolean;
  onResetExternalChanges?: () => void;
  computeExternalDiffs?: () => DiffEntry[];
}

export type CustomFieldType = 'string' | 'number' | 'boolean' | 'object' | 'array';

export const CUSTOM_FIELD_TYPE_OPTIONS: { label: string; value: CustomFieldType }[] = [
  { label: 'string', value: 'string' },
  { label: 'number', value: 'number' },
  { label: 'boolean', value: 'boolean' },
  { label: 'object', value: 'object' },
  { label: 'array', value: 'array' },
];

export const DEFAULT_DESCRIPTION = '未提供描述';

export const GEMINI_ENV_DEFAULT: GeminiEnvConfig = {
  apiKey: '',
  baseUrl: '',
  model: 'gemini-2.5-pro',
};

export const cloneGeminiEnv = (env?: GeminiEnvConfig): GeminiEnvConfig => ({
  apiKey: env?.apiKey ?? '',
  baseUrl: env?.baseUrl ?? '',
  model: env?.model ?? 'gemini-2.5-pro',
});
