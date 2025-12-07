import { useState, useCallback, useEffect } from 'react';
import {
  startToolProxy,
  stopToolProxy,
  getAllProxyStatus,
  getProxyConfig,
  updateProxyConfig,
  type AllProxyStatus,
  type ToolProxyConfig,
  type ToolId,
} from '@/lib/tauri-commands';

// 工具元数据
export interface ToolMetadata {
  id: string;
  name: string;
  description: string;
  defaultPort: number;
}

export const SUPPORTED_TOOLS: ToolMetadata[] = [
  {
    id: 'claude-code',
    name: 'Claude Code',
    description: 'Anthropic Claude 编程助手',
    defaultPort: 8787,
  },
  {
    id: 'codex',
    name: 'Codex',
    description: 'OpenAI Codex 编程助手',
    defaultPort: 8788,
  },
  {
    id: 'gemini-cli',
    name: 'Gemini CLI',
    description: 'Google Gemini 命令行工具',
    defaultPort: 8789,
  },
];

// 默认工具配置
function getDefaultToolConfig(toolId: string): ToolProxyConfig {
  const tool = SUPPORTED_TOOLS.find((t) => t.id === toolId);
  return {
    enabled: false,
    port: tool?.defaultPort || 8790,
    local_api_key: null,
    real_api_key: null,
    real_base_url: null,
    real_model_provider: null,
    real_profile_name: null,
    allow_public: false,
    session_endpoint_config_enabled: false,
    auto_start: false,
  };
}

export function useMultiToolProxy() {
  const [allProxyStatus, setAllProxyStatus] = useState<AllProxyStatus>({});
  const [loadingTools, setLoadingTools] = useState<Set<string>>(new Set());
  const [toolConfigs, setToolConfigs] = useState<Record<string, ToolProxyConfig>>({});
  const [savingConfig, setSavingConfig] = useState(false);
  const [hasUnsavedChanges, setHasUnsavedChanges] = useState(false);

  // 加载单个工具配置
  const loadToolConfig = useCallback(async (toolId: string) => {
    try {
      const config = await getProxyConfig(toolId as ToolId);
      return config || getDefaultToolConfig(toolId);
    } catch (error) {
      console.error(`Failed to load ${toolId} config:`, error);
      return getDefaultToolConfig(toolId);
    }
  }, []);

  // 加载所有工具配置
  const loadAllConfigs = useCallback(async () => {
    const configs: Record<string, ToolProxyConfig> = {};
    for (const tool of SUPPORTED_TOOLS) {
      configs[tool.id] = await loadToolConfig(tool.id);
    }
    setToolConfigs(configs);
  }, [loadToolConfig]);

  // 加载所有工具的代理状态
  const loadAllProxyStatus = useCallback(async () => {
    try {
      const status = await getAllProxyStatus();
      setAllProxyStatus(status);
    } catch (error) {
      console.error('Failed to load all proxy status:', error);
      throw error;
    }
  }, []);

  // 初始化加载
  useEffect(() => {
    loadAllConfigs().catch(console.error);
    loadAllProxyStatus().catch(console.error);
  }, [loadAllConfigs, loadAllProxyStatus]);

  // 更新单个工具的配置（本地状态）
  const updateToolConfig = useCallback((toolId: string, updates: Partial<ToolProxyConfig>) => {
    setToolConfigs((prev) => ({
      ...prev,
      [toolId]: {
        ...prev[toolId],
        ...updates,
      },
    }));
    setHasUnsavedChanges(true);
  }, []);

  // 保存所有配置
  const saveAllConfigs = useCallback(async () => {
    setSavingConfig(true);
    try {
      // 保存每个工具的配置到 ProxyConfigManager
      for (const [toolId, config] of Object.entries(toolConfigs)) {
        await updateProxyConfig(toolId as ToolId, config);
      }
      setHasUnsavedChanges(false);
    } catch (error) {
      console.error('Failed to save configs:', error);
      throw error;
    } finally {
      setSavingConfig(false);
    }
  }, [toolConfigs]);

  // 启动代理
  const startProxy = useCallback(
    async (toolId: string) => {
      setLoadingTools((prev) => new Set(prev).add(toolId));
      try {
        await startToolProxy(toolId);
        await loadAllProxyStatus();
      } finally {
        setLoadingTools((prev) => {
          const next = new Set(prev);
          next.delete(toolId);
          return next;
        });
      }
    },
    [loadAllProxyStatus],
  );

  // 停止代理
  const stopProxy = useCallback(
    async (toolId: string) => {
      setLoadingTools((prev) => new Set(prev).add(toolId));
      try {
        await stopToolProxy(toolId);
        await loadAllProxyStatus();
      } finally {
        setLoadingTools((prev) => {
          const next = new Set(prev);
          next.delete(toolId);
          return next;
        });
      }
    },
    [loadAllProxyStatus],
  );

  return {
    allProxyStatus,
    toolConfigs,
    loadingTools,
    savingConfig,
    hasUnsavedChanges,
    updateToolConfig,
    saveAllConfigs,
    startProxy,
    stopProxy,
    loadAllConfigs,
    loadAllProxyStatus,
  };
}
