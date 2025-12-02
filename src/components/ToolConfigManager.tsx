import { useCallback, useEffect, useMemo, useState } from 'react';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import {
  getClaudeSchema,
  getClaudeSettings,
  getCodexSchema,
  getCodexSettings,
  getGeminiSchema,
  getGeminiSettings,
  saveClaudeSettings,
  saveCodexSettings,
  saveGeminiSettings,
  type GeminiEnvConfig,
  type CodexSettingsPayload,
  type GeminiSettingsPayload,
  type JsonObject,
  type ClaudeSettingsPayload,
} from '@/lib/tauri-commands';
import { useToast } from '@/hooks/use-toast';
import { Loader2, Plus, RefreshCw, Save, Trash2 } from 'lucide-react';
import { SecretInput } from '@/components/SecretInput';
import { SchemaField } from './tool-config/Fields';
import {
  CUSTOM_FIELD_TYPE_OPTIONS,
  DEFAULT_DESCRIPTION,
  GEMINI_ENV_DEFAULT,
  cloneGeminiEnv,
  type SchemaOption,
  type CustomFieldType,
  type DiffEntry,
  type JSONSchema,
  type ToolConfigManagerProps,
} from './tool-config/types';
import {
  buildDiffEntries,
  cloneJsonObject,
  formatJson,
  getDefaultValue,
  getEffectiveType,
  getTypeLabel,
  isCompoundField,
  resolveSchema,
  createSchemaForType,
} from './tool-config/utils';

export function ToolConfigManager({
  title,
  description,
  loadSchema,
  loadSettings,
  saveSettings,
  emptyHint = '当前配置文件为空，点击「新增配置选项」开始添加。',
  refreshSignal,
  externalDirty = false,
  onResetExternalChanges,
  computeExternalDiffs,
}: ToolConfigManagerProps) {
  const { toast } = useToast();
  const [loading, setLoading] = useState(true);
  const [hasLoaded, setHasLoaded] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [schemaRoot, setSchemaRoot] = useState<JSONSchema | null>(null);
  const [originalSettings, setOriginalSettings] = useState<JsonObject>({});
  const [draftSettings, setDraftSettings] = useState<JsonObject>({});
  const [addDialogOpen, setAddDialogOpen] = useState(false);
  const [diffDialogOpen, setDiffDialogOpen] = useState(false);
  const [diffEntries, setDiffEntries] = useState<DiffEntry[]>([]);
  const [saving, setSaving] = useState(false);
  const [searchKeyword, setSearchKeyword] = useState('');
  const [customKey, setCustomKey] = useState('');
  const [customFieldType, setCustomFieldType] = useState<CustomFieldType>('string');

  const schemaOptions = useMemo<SchemaOption[]>(() => {
    if (!schemaRoot || !schemaRoot.properties) {
      return [];
    }

    return Object.entries(schemaRoot.properties).map(([key, schema]) => {
      const resolved = resolveSchema(schema, schemaRoot);
      return {
        key,
        schema: resolved,
        description: resolved?.description ?? DEFAULT_DESCRIPTION,
        typeLabel: getTypeLabel(resolved),
      };
    });
  }, [schemaRoot]);

  const filteredOptions = useMemo(() => {
    if (!searchKeyword.trim()) {
      return schemaOptions;
    }
    const keyword = searchKeyword.toLowerCase();
    return schemaOptions.filter(
      (option) =>
        option.key.toLowerCase().includes(keyword) ||
        option.description.toLowerCase().includes(keyword),
    );
  }, [schemaOptions, searchKeyword]);

  const hasChanges = useMemo(() => {
    if (externalDirty) {
      return true;
    }
    return JSON.stringify(originalSettings) !== JSON.stringify(draftSettings);
  }, [originalSettings, draftSettings, externalDirty]);

  const loadData = useCallback(
    async (options?: { refetchSchema?: boolean }) => {
      setLoading(true);
      setError(null);
      try {
        const shouldFetchSchema = options?.refetchSchema || !schemaRoot;
        const schemaPromise = shouldFetchSchema
          ? loadSchema().then((schema) => schema as JSONSchema)
          : Promise.resolve(schemaRoot as JSONSchema);

        const [settings, resolvedSchema] = await Promise.all([loadSettings(), schemaPromise]);

        if (shouldFetchSchema || !schemaRoot) {
          setSchemaRoot(resolvedSchema);
        }

        const cloned = cloneJsonObject(settings);
        setOriginalSettings(cloned);
        setDraftSettings(cloned);
        setHasLoaded(true);
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        setError(message);
      } finally {
        setLoading(false);
      }
    },
    [schemaRoot, loadSchema, loadSettings],
  );

  useEffect(() => {
    if (!hasLoaded) {
      void loadData();
    }
  }, [hasLoaded, loadData]);

  useEffect(() => {
    if (refreshSignal !== undefined && hasLoaded) {
      void loadData();
    }
  }, [refreshSignal, hasLoaded, loadData]);

  interface AddKeyOptions {
    schema?: JSONSchema;
    fieldType?: CustomFieldType;
  }

  const handleAddKey = (key: string, options?: AddKeyOptions) => {
    if (!key.trim()) {
      return;
    }

    if (draftSettings[key] !== undefined) {
      toast({
        title: '配置选项已存在',
        description: `配置选项 ${key} 已存在，无法重复添加。`,
      });
      return;
    }

    const next = cloneJsonObject(draftSettings);
    const schemaForDefault = options?.schema ?? createSchemaForType(options?.fieldType);
    next[key] = getDefaultValue(schemaForDefault);
    setDraftSettings(next);
    setAddDialogOpen(false);
    setSearchKeyword('');
    setCustomKey('');
    if (options?.fieldType) {
      setCustomFieldType('string');
    }
  };

  const handleDeleteKey = (key: string) => {
    const next = cloneJsonObject(draftSettings);
    delete next[key];
    setDraftSettings(next);
  };

  const handleReload = () => {
    void loadData({ refetchSchema: true });
  };

  const handleResetDraft = () => {
    setDraftSettings(cloneJsonObject(originalSettings));
    onResetExternalChanges?.();
  };

  const computeDiffs = useCallback((): DiffEntry[] => {
    const diffs: DiffEntry[] = [];
    buildDiffEntries([], originalSettings, draftSettings, diffs);
    return diffs;
  }, [originalSettings, draftSettings]);

  const handleSaveClick = () => {
    const diffs = computeDiffs();
    const externalDiffs = computeExternalDiffs?.() ?? [];
    const combinedDiffs = [...diffs, ...externalDiffs];
    if (combinedDiffs.length === 0 && !externalDirty) {
      toast({
        title: '没有需要保存的修改',
        description: '请先更改配置后再尝试保存。',
      });
      return;
    }
    setDiffEntries(combinedDiffs);
    setDiffDialogOpen(true);
  };

  const handleConfirmSave = async () => {
    setSaving(true);
    try {
      await saveSettings(cloneJsonObject(draftSettings));
      setOriginalSettings(cloneJsonObject(draftSettings));
      setDiffDialogOpen(false);
      toast({
        title: '保存成功',
        description: '配置已写入目标文件。',
      });
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      toast({
        title: '保存失败',
        description: message,
        variant: 'destructive',
      });
    } finally {
      setSaving(false);
    }
  };

  const topLevelKeys = useMemo(() => {
    return Object.keys(draftSettings).sort((a, b) => a.localeCompare(b));
  }, [draftSettings]);

  const renderContent = () => {
    if (loading) {
      return (
        <div className="flex flex-col items-center justify-center py-16 text-muted-foreground">
          <Loader2 className="h-8 w-8 animate-spin text-primary" />
          <p className="mt-3 text-sm">配置加载中...</p>
          <p className="text-xs text-muted-foreground">切换或刷新配置时请稍候</p>
        </div>
      );
    }

    if (error) {
      return (
        <div className="rounded-md border border-destructive/40 bg-destructive/10 p-4 text-sm text-destructive">
          读取配置失败：{error}
        </div>
      );
    }

    if (topLevelKeys.length === 0) {
      return (
        <div className="rounded-md border border-dashed p-6 text-center text-sm text-muted-foreground">
          {emptyHint}
        </div>
      );
    }

    return (
      <div className="space-y-4">
        {topLevelKeys.map((key) => {
          const schemaInfo = schemaOptions.find((option) => option.key === key);
          const description = schemaInfo?.description ?? DEFAULT_DESCRIPTION;
          const schema = schemaInfo?.schema;
          const currentValue = draftSettings[key];
          const fieldType = getEffectiveType(schema, currentValue);
          const typeLabel = schemaInfo?.typeLabel ?? fieldType ?? 'string';
          const isCompound = isCompoundField(schema, currentValue);

          return (
            <Card key={key} className="border border-slate-200/80">
              {isCompound ? (
                <>
                  <CardHeader className="flex flex-row items-start justify-between space-y-0">
                    <div className="flex items-center gap-2">
                      <CardTitle className="text-base font-semibold font-mono">{key}</CardTitle>
                      <Badge variant="outline">{typeLabel}</Badge>
                    </div>
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => handleDeleteKey(key)}
                      className="text-destructive hover:text-destructive"
                    >
                      <Trash2 className="h-4 w-4" />
                    </Button>
                  </CardHeader>
                  <CardContent>
                    <SchemaField
                      schema={schema}
                      value={draftSettings[key]}
                      onChange={(value) => {
                        const next = cloneJsonObject(draftSettings);
                        next[key] = value;
                        setDraftSettings(next);
                      }}
                      path={[key]}
                      rootSchema={schemaRoot}
                      rootValue={draftSettings}
                    />
                  </CardContent>
                </>
              ) : (
                <CardHeader className="space-y-2">
                  <div className="flex flex-col gap-3 md:flex-row md:items-center">
                    <div className="flex items-center gap-2 md:basis-1/2">
                      <CardTitle className="text-base font-semibold font-mono">{key}</CardTitle>
                      <Badge variant="outline">{typeLabel}</Badge>
                    </div>
                    <div className="flex items-center gap-3 md:basis-1/2">
                      <div
                        className={
                          fieldType === 'boolean'
                            ? 'flex-1 min-w-0 flex justify-end'
                            : 'flex-1 min-w-0'
                        }
                      >
                        <SchemaField
                          inline
                          schema={schema}
                          value={draftSettings[key]}
                          onChange={(value) => {
                            const next = cloneJsonObject(draftSettings);
                            next[key] = value;
                            setDraftSettings(next);
                          }}
                          path={[key]}
                          rootSchema={schemaRoot}
                          showDescription={false}
                          rootValue={draftSettings}
                        />
                      </div>
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => handleDeleteKey(key)}
                        className="text-destructive hover:text-destructive"
                      >
                        <Trash2 className="h-4 w-4" />
                      </Button>
                    </div>
                  </div>
                  <p className="text-xs text-muted-foreground">{description}</p>
                </CardHeader>
              )}
            </Card>
          );
        })}
      </div>
    );
  };

  return (
    <Card className="border border-slate-200/80 shadow-lg">
      <CardHeader className="space-y-4">
        <div className="flex flex-col gap-2 md:flex-row md:items-center md:justify-between">
          <div>
            <CardTitle>{title}</CardTitle>
            <CardDescription>{description}</CardDescription>
          </div>
          <div className="flex flex-wrap gap-2">
            <Button variant="outline" size="sm" onClick={handleReload} disabled={loading}>
              <RefreshCw className="mr-1.5 h-4 w-4" />
              刷新
            </Button>
            <Button
              variant="outline"
              size="sm"
              onClick={() => setAddDialogOpen(true)}
              disabled={loading}
            >
              <Plus className="mr-1.5 h-4 w-4" />
              新增配置选项
            </Button>
            <Button variant="outline" size="sm" onClick={handleResetDraft} disabled={!hasChanges}>
              撤销修改
            </Button>
            <Button size="sm" onClick={handleSaveClick} disabled={!hasChanges}>
              <Save className="mr-1.5 h-4 w-4" />
              保存
            </Button>
          </div>
        </div>
        <div className="rounded-md border border-amber-200 bg-amber-50 p-3 text-xs text-amber-900">
          每个配置选项下方都会展示 JSON Schema 提供的描述信息，若显示「未提供描述」表示该子选项未在
          schema 中定义或为自定义子选项。
        </div>
      </CardHeader>
      <CardContent>{renderContent()}</CardContent>

      <Dialog open={addDialogOpen} onOpenChange={setAddDialogOpen}>
        <DialogContent className="sm:max-w-lg">
          <DialogHeader>
            <DialogTitle>新增配置选项</DialogTitle>
          </DialogHeader>
          <div className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="config-search">搜索 JSON Schema 配置选项</Label>
              <Input
                id="config-search"
                value={searchKeyword}
                onChange={(event) => setSearchKeyword(event.target.value)}
                placeholder="输入关键字，例如 model..."
              />
            </div>
            <div className="max-h-64 overflow-y-auto rounded-md border">
              {filteredOptions.length === 0 && (
                <div className="p-4 text-sm text-muted-foreground">没有匹配的配置选项</div>
              )}
              {filteredOptions.map((option) => {
                const alreadyExists = draftSettings[option.key] !== undefined;
                return (
                  <button
                    key={option.key}
                    type="button"
                    onClick={() => {
                      if (!alreadyExists) {
                        handleAddKey(option.key, { schema: option.schema });
                      }
                    }}
                    disabled={alreadyExists}
                    className={`flex w-full flex-col items-start gap-1 border-b p-3 text-left ${
                      alreadyExists
                        ? 'cursor-not-allowed bg-muted/30 text-muted-foreground'
                        : 'hover:bg-muted/40'
                    }`}
                  >
                    <span className="flex items-center gap-2 font-mono text-sm font-semibold">
                      {option.key}
                      {alreadyExists && (
                        <Badge variant="secondary" className="text-[10px]">
                          已存在
                        </Badge>
                      )}
                    </span>
                    <span className="text-xs text-muted-foreground">{option.description}</span>
                  </button>
                );
              })}
            </div>
            <div className="space-y-2">
              <Label htmlFor="custom-key">或直接输入自定义 key</Label>
              <div className="flex gap-2 flex-wrap sm:flex-nowrap">
                <Input
                  id="custom-key"
                  className="flex-1"
                  value={customKey}
                  onChange={(event) => setCustomKey(event.target.value)}
                  placeholder="例如 customFlag"
                />
                <Select
                  value={customFieldType}
                  onValueChange={(value) => setCustomFieldType(value as CustomFieldType)}
                >
                  <SelectTrigger className="w-28">
                    <SelectValue placeholder="类型" />
                  </SelectTrigger>
                  <SelectContent>
                    {CUSTOM_FIELD_TYPE_OPTIONS.map((option) => (
                      <SelectItem key={option.value} value={option.value}>
                        {option.label}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
                <Button
                  onClick={() => handleAddKey(customKey, { fieldType: customFieldType })}
                  disabled={!customKey.trim()}
                >
                  添加
                </Button>
              </div>
              <p className="text-xs text-muted-foreground">
                自定义配置选项将被标记为“未提供描述”。
              </p>
            </div>
          </div>
        </DialogContent>
      </Dialog>

      <Dialog open={diffDialogOpen} onOpenChange={setDiffDialogOpen}>
        <DialogContent className="sm:max-w-2xl">
          <DialogHeader>
            <DialogTitle>保存前差异确认</DialogTitle>
          </DialogHeader>
          <div className="max-h-[420px] space-y-3 overflow-y-auto">
            {diffEntries.length === 0 && (
              <div className="rounded-md border border-slate-200 p-4 text-sm text-muted-foreground">
                没有检测到差异
              </div>
            )}
            {diffEntries.map((diff) => (
              <div
                key={diff.path + diff.type}
                className="rounded-md border border-slate-200 bg-slate-50 p-3 text-xs"
              >
                <div className="flex items-center justify-between font-semibold">
                  <span className="font-mono text-sm">{diff.path}</span>
                  <Badge variant={diff.type === 'changed' ? 'default' : 'secondary'}>
                    {diff.type}
                  </Badge>
                </div>
                <div className="mt-2 grid gap-2 md:grid-cols-2">
                  <div>
                    <p className="text-[11px] text-muted-foreground">之前</p>
                    <pre className="mt-1 overflow-x-auto rounded-md bg-white p-2">
                      {formatJson(diff.before)}
                    </pre>
                  </div>
                  <div>
                    <p className="text-[11px] text-muted-foreground">之后</p>
                    <pre className="mt-1 overflow-x-auto rounded-md bg-white p-2">
                      {formatJson(diff.after)}
                    </pre>
                  </div>
                </div>
              </div>
            ))}
          </div>
          <DialogFooter className="gap-2">
            <Button variant="outline" onClick={() => setDiffDialogOpen(false)} disabled={saving}>
              取消
            </Button>
            <Button onClick={handleConfirmSave} disabled={saving}>
              {saving ? (
                <>
                  <Loader2 className="mr-1.5 h-4 w-4 animate-spin" />
                  保存中...
                </>
              ) : (
                '确认保存'
              )}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </Card>
  );
}

export function ClaudeConfigManager({ refreshSignal }: { refreshSignal?: number }) {
  const { toast } = useToast();
  const [extraEntries, setExtraEntries] = useState<{ key: string; value: string }[]>([]);
  const [originalExtraEntries, setOriginalExtraEntries] = useState<
    { key: string; value: string }[]
  >([]);
  const [extraDirty, setExtraDirty] = useState(false);
  const [extraError, setExtraError] = useState<string | null>(null);

  const toEntries = useCallback((obj?: JsonObject | null): { key: string; value: string }[] => {
    if (!obj) return [];
    return Object.entries(obj).map(([key, value]) => ({
      key,
      value: typeof value === 'string' ? value : formatJson(value ?? null),
    }));
  }, []);

  const normalizeEntries = useCallback(
    (entries: { key: string; value: string }[]) => entries.filter((e) => e.key.trim()),
    [],
  );

  const buildExtraObject = useCallback((): JsonObject | null => {
    const obj: JsonObject = {};
    const seen = new Set<string>();
    const isLikelyJson = (text: string) => {
      const trimmed = text.trim();
      if (!trimmed) return false;
      const first = trimmed[0];
      return (
        first === '{' ||
        first === '[' ||
        first === '"' ||
        /^-?\d/.test(trimmed) ||
        trimmed === 'true' ||
        trimmed === 'false' ||
        trimmed === 'null'
      );
    };

    for (const { key, value } of normalizeEntries(extraEntries)) {
      const normalizedKey = key.trim();
      if (!normalizedKey) continue;
      if (seen.has(normalizedKey)) {
        throw new Error(`config.json 出现重复键：${normalizedKey}`);
      }
      seen.add(normalizedKey);
      const trimmed = value.trim();
      if (!trimmed) {
        obj[normalizedKey] = '';
        continue;
      }
      try {
        obj[normalizedKey] = JSON.parse(trimmed);
      } catch {
        if (isLikelyJson(trimmed)) {
          throw new Error(`config.json 中 ${normalizedKey} 的值 JSON 解析失败，请检查格式`);
        }
        obj[normalizedKey] = value;
      }
    }
    return Object.keys(obj).length ? obj : null;
  }, [extraEntries, normalizeEntries]);

  const validateExtraEntries = useCallback((): JsonObject | null => {
    try {
      const result = buildExtraObject();
      setExtraError(null);
      return result;
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setExtraError(message);
      throw err;
    }
  }, [buildExtraObject]);

  useEffect(() => {
    try {
      validateExtraEntries();
    } catch {
      // ignore，错误信息已写入 extraError
    }
  }, [validateExtraEntries]);

  const loadSettings = useCallback(async () => {
    const payload: ClaudeSettingsPayload = await getClaudeSettings();
    const entries = toEntries(payload.extraConfig);
    setExtraEntries(entries);
    setOriginalExtraEntries(entries);
    setExtraDirty(false);
    setExtraError(null);
    return payload.settings;
  }, [toEntries]);

  const saveConfig = useCallback(
    async (settings: JsonObject) => {
      let parsedExtra: JsonObject | null = null;
      try {
        parsedExtra = validateExtraEntries();
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        toast({ title: '保存失败', description: message, variant: 'destructive' });
        throw err;
      }

      if (parsedExtra) {
        await saveClaudeSettings(settings, parsedExtra);
      } else {
        await saveClaudeSettings(settings);
      }

      const nextEntries = toEntries(parsedExtra);
      setOriginalExtraEntries(nextEntries);
      setExtraEntries(nextEntries);
      setExtraDirty(false);
      setExtraError(null);
    },
    [toEntries, toast, validateExtraEntries],
  );

  const handleResetExtra = useCallback(() => {
    setExtraEntries(originalExtraEntries);
    setExtraDirty(false);
    setExtraError(null);
  }, [originalExtraEntries]);

  const computeExtraDiffs = useCallback((): DiffEntry[] => {
    try {
      const current = buildExtraObject();
      const original = (() => {
        if (!originalExtraEntries.length) return null;
        const obj: JsonObject = {};
        for (const { key, value } of normalizeEntries(originalExtraEntries)) {
          if (!key.trim()) continue;
          try {
            obj[key] = JSON.parse(value);
          } catch {
            obj[key] = value;
          }
        }
        return Object.keys(obj).length ? obj : null;
      })();

      if (JSON.stringify(current) === JSON.stringify(original)) {
        return [];
      }

      let type: DiffEntry['type'] = 'changed';
      if (!original && current) type = 'added';
      if (original && !current) type = 'removed';

      return [
        {
          path: 'config.json',
          type,
          before: original ?? undefined,
          after: current ?? undefined,
        },
      ];
    } catch {
      return [];
    }
  }, [buildExtraObject, normalizeEntries, originalExtraEntries]);

  return (
    <div className="space-y-4">
      <ToolConfigManager
        title="Claude Code 配置管理"
        description="读取并编辑 settings.json"
        loadSchema={getClaudeSchema}
        loadSettings={loadSettings}
        saveSettings={saveConfig}
        refreshSignal={refreshSignal}
        externalDirty={extraDirty}
        onResetExternalChanges={() => {
          handleResetExtra();
        }}
        computeExternalDiffs={computeExtraDiffs}
      />

      <Card className="border border-slate-200/80">
        <CardHeader>
          <CardTitle>附属配置：config.json</CardTitle>
          <CardDescription>可选文件，存在时将与 settings.json 一同保存。</CardDescription>
        </CardHeader>
        <CardContent className="space-y-3">
          <div className="flex items-center justify-between">
            <div>
              <Label className="font-mono text-sm font-semibold">~/.claude/config.json</Label>
              <p className="text-xs text-muted-foreground">
                以键值对形式编辑 config.json，值可写 JSON（自动解析）；留空则不写入。
              </p>
            </div>
            <Button
              variant="outline"
              size="sm"
              onClick={() => {
                setExtraEntries((prev) => [...prev, { key: '', value: '' }]);
                setExtraDirty(true);
              }}
            >
              <Plus className="mr-1 h-4 w-4" />
              新增键值
            </Button>
          </div>

          <div className="space-y-3">
            {extraEntries.length === 0 && (
              <p className="text-xs text-muted-foreground">
                当前为空，保存时不会写入 config.json。
              </p>
            )}
            {extraEntries.map((entry, idx) => (
              <div
                key={`${idx}-${entry.key}`}
                className="flex flex-col gap-2 rounded-md border border-slate-200/80 p-3 md:flex-row md:items-center"
              >
                <div className="flex flex-1 items-center gap-2">
                  <div className="w-40 min-w-[140px]">
                    <Label className="font-mono text-sm font-semibold">key</Label>
                    <Input
                      value={entry.key}
                      onChange={(e) => {
                        const next = [...extraEntries];
                        next[idx] = { ...entry, key: e.target.value };
                        setExtraEntries(next);
                        setExtraDirty(true);
                      }}
                      placeholder="primaryApiKey"
                    />
                  </div>
                  <div className="flex-1">
                    <Label className="font-mono text-sm font-semibold">value</Label>
                    <Input
                      value={entry.value}
                      onChange={(e) => {
                        const next = [...extraEntries];
                        next[idx] = { ...entry, value: e.target.value };
                        setExtraEntries(next);
                        setExtraDirty(true);
                      }}
                      placeholder='如 "sk-..." 或 {"enabled":true}'
                    />
                  </div>
                </div>
                <Button
                  variant="ghost"
                  size="icon"
                  onClick={() => {
                    const next = [...extraEntries];
                    next.splice(idx, 1);
                    setExtraEntries(next);
                    setExtraDirty(true);
                  }}
                  aria-label="删除键值"
                >
                  <Trash2 className="h-4 w-4 text-slate-500" />
                </Button>
              </div>
            ))}
          </div>

          {extraError ? (
            <p className="text-xs text-red-600">格式错误：{extraError}</p>
          ) : extraDirty ? (
            <p className="text-xs text-amber-600">config.json 已修改，保存后生效。</p>
          ) : (
            <p className="text-xs text-muted-foreground">同步保存时将一并写入 config.json。</p>
          )}
          <div className="flex gap-2">
            <Button variant="outline" size="sm" onClick={handleResetExtra} disabled={!extraDirty}>
              还原 config.json
            </Button>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}

export function CodexConfigManager({ refreshSignal }: { refreshSignal?: number }) {
  const [authToken, setAuthToken] = useState('');
  const [originalAuthToken, setOriginalAuthToken] = useState('');
  const [authDirty, setAuthDirty] = useState(false);

  const loadSettings = useCallback(async () => {
    const payload: CodexSettingsPayload = await getCodexSettings();
    const token = payload.authToken ?? '';
    setAuthToken(token);
    setOriginalAuthToken(token);
    setAuthDirty(false);
    return payload.config;
  }, []);

  const saveConfig = useCallback(
    async (settings: JsonObject) => {
      await saveCodexSettings(settings, authToken);
      setOriginalAuthToken(authToken);
      setAuthDirty(false);
    },
    [authToken],
  );

  const computeAuthDiffs = useCallback((): DiffEntry[] => {
    if (authToken === originalAuthToken) {
      return [];
    }
    const beforeValue = originalAuthToken ?? '';
    const afterValue = authToken ?? '';

    let type: DiffEntry['type'] = 'changed';
    if (!beforeValue && afterValue) {
      type = 'added';
    } else if (beforeValue && !afterValue) {
      type = 'removed';
    }

    return [
      {
        path: 'auth.OPENAI_API_KEY',
        type,
        before: beforeValue || undefined,
        after: afterValue || undefined,
      },
    ];
  }, [authToken, originalAuthToken]);

  const handleResetAuthToken = useCallback(() => {
    setAuthToken(originalAuthToken);
    setAuthDirty(false);
  }, [originalAuthToken]);

  return (
    <div className="space-y-4">
      <ToolConfigManager
        title="Codex 配置管理"
        description="读取并编辑 config.toml"
        loadSchema={getCodexSchema}
        loadSettings={loadSettings}
        saveSettings={saveConfig}
        emptyHint="当前 config.toml 为空，点击「新增配置选项」开始添加。"
        refreshSignal={refreshSignal}
        externalDirty={authDirty}
        onResetExternalChanges={handleResetAuthToken}
        computeExternalDiffs={computeAuthDiffs}
      />

      <Card className="border border-slate-200/80">
        <CardHeader>
          <CardTitle>Codex API Key</CardTitle>
          <CardDescription>读取并编辑 auth.json，用于 Codex CLI 请求。</CardDescription>
        </CardHeader>
        <CardContent className="space-y-3">
          <div className="flex flex-col gap-3 md:flex-row md:items-center">
            <div className="flex items-center gap-2 md:basis-1/2">
              <Label htmlFor="codex-api-key" className="font-mono text-sm font-semibold">
                OPENAI_API_KEY
              </Label>
              <Badge variant="outline">string</Badge>
            </div>
            <div className="flex-1 min-w-0">
              <SecretInput
                id="codex-api-key"
                value={authToken}
                onValueChange={(val) => {
                  setAuthToken(val);
                  setAuthDirty(true);
                }}
                placeholder="sk-..."
                toggleLabel="切换 Codex API Key 可见性"
                className="w-full"
                wrapperClassName="w-full"
              />
            </div>
          </div>
          <p className="text-xs text-muted-foreground">
            修改后点击上方“保存”将同时写入 config.toml 与 auth.json。
          </p>
          {authDirty && <p className="text-xs text-amber-600">API Key 已更新，记得保存以生效。</p>}
        </CardContent>
      </Card>
    </div>
  );
}

export function GeminiConfigManager({ refreshSignal }: { refreshSignal?: number }) {
  const [envState, setEnvState] = useState<GeminiEnvConfig>(() =>
    cloneGeminiEnv(GEMINI_ENV_DEFAULT),
  );
  const [originalEnv, setOriginalEnv] = useState<GeminiEnvConfig>(() =>
    cloneGeminiEnv(GEMINI_ENV_DEFAULT),
  );
  const [envDirty, setEnvDirty] = useState(false);

  const loadSettings = useCallback(async () => {
    const payload: GeminiSettingsPayload = await getGeminiSettings();
    const nextEnv = cloneGeminiEnv(payload.env);
    setEnvState(nextEnv);
    setOriginalEnv(nextEnv);
    setEnvDirty(false);
    return payload.settings;
  }, []);

  const saveConfig = useCallback(
    async (settings: JsonObject) => {
      await saveGeminiSettings(settings, envState);
      setOriginalEnv(cloneGeminiEnv(envState));
      setEnvDirty(false);
    },
    [envState],
  );

  const handleResetEnv = useCallback(() => {
    setEnvState(cloneGeminiEnv(originalEnv));
    setEnvDirty(false);
  }, [originalEnv]);

  const updateEnvField = useCallback((field: keyof GeminiEnvConfig, value: string) => {
    setEnvState((prev) => ({ ...prev, [field]: value }));
    setEnvDirty(true);
  }, []);

  const computeEnvDiffs = useCallback((): DiffEntry[] => {
    const diffs: DiffEntry[] = [];
    (['apiKey', 'baseUrl', 'model'] as const).forEach((field) => {
      if (envState[field] === originalEnv[field]) {
        return;
      }

      const beforeValue = originalEnv[field];
      const afterValue = envState[field];
      let type: DiffEntry['type'] = 'changed';
      if (!beforeValue && afterValue) {
        type = 'added';
      } else if (beforeValue && !afterValue) {
        type = 'removed';
      }

      const path = `env.${
        field === 'apiKey'
          ? 'GEMINI_API_KEY'
          : field === 'baseUrl'
            ? 'GOOGLE_GEMINI_BASE_URL'
            : 'GEMINI_MODEL'
      }`;

      diffs.push({
        path,
        type,
        before: beforeValue || undefined,
        after: afterValue || undefined,
      });
    });
    return diffs;
  }, [envState, originalEnv]);

  return (
    <div className="space-y-4">
      <ToolConfigManager
        title="Gemini 配置管理"
        description="读取并编辑 settings.json"
        loadSchema={getGeminiSchema}
        loadSettings={loadSettings}
        saveSettings={saveConfig}
        emptyHint="当前 settings.json 为空，点击「新增配置选项」开始添加。"
        refreshSignal={refreshSignal}
        externalDirty={envDirty}
        onResetExternalChanges={handleResetEnv}
        computeExternalDiffs={computeEnvDiffs}
      />

      <Card className="border border-slate-200/80">
        <CardHeader>
          <CardTitle>Gemini .env</CardTitle>
          <CardDescription>读取并编辑 .env，管理 Base URL、API Key 与默认模型。</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex flex-col gap-3 md:flex-row md:items-center">
            <div className="flex items-center gap-2 md:basis-1/3">
              <Label htmlFor="gemini-api-key" className="font-mono text-sm font-semibold">
                GEMINI_API_KEY
              </Label>
              <Badge variant="outline">string</Badge>
            </div>
            <SecretInput
              id="gemini-api-key"
              value={envState.apiKey}
              onValueChange={(val) => updateEnvField('apiKey', val)}
              placeholder="ya29...."
              className="w-full"
              wrapperClassName="w-full"
              toggleLabel="切换 Gemini API Key 可见性"
            />
          </div>
          <div className="flex flex-col gap-3 md:flex-row md:items-center">
            <div className="flex items-center gap-2 md:basis-1/3">
              <Label htmlFor="gemini-base-url" className="font-mono text-sm font-semibold">
                GOOGLE_GEMINI_BASE_URL
              </Label>
              <Badge variant="outline">string</Badge>
            </div>
            <Input
              id="gemini-base-url"
              value={envState.baseUrl}
              onChange={(event) => updateEnvField('baseUrl', event.target.value)}
              placeholder="https://generativelanguage.googleapis.com"
            />
          </div>
          <div className="flex flex-col gap-3 md:flex-row md:items-center">
            <div className="flex items-center gap-2 md:basis-1/3">
              <Label htmlFor="gemini-model" className="font-mono text-sm font-semibold">
                GEMINI_MODEL
              </Label>
              <Badge variant="outline">string</Badge>
            </div>
            <Input
              id="gemini-model"
              value={envState.model}
              onChange={(event) => updateEnvField('model', event.target.value)}
              placeholder="gemini-2.5-pro"
            />
          </div>
          <p className="text-xs text-muted-foreground">
            修改以上字段后请点击上方“保存”，系统会同步写入 settings.json 与 .env。
          </p>
          {envDirty && (
            <p className="text-xs text-amber-600">.env 内容已修改，记得通过保存按钮写回磁盘。</p>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
