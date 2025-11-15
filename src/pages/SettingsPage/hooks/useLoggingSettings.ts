import { useState, useEffect, useCallback } from 'react';
import {
  getLogConfig,
  updateLogConfig,
  getLogStats,
  setLogLevel,
  getAvailableLogLevels,
  testLogging,
  openLogDirectory,
  cleanupOldLogs,
  getRecentLogs,
  flushLogs,
  type LoggingConfig,
  type LoggingStats,
  type LogLevel,
} from '@/lib/tauri-commands';

interface UseLoggingSettingsOptions {
  onUpdate?: () => void;
}

export function useLoggingSettings({ onUpdate }: UseLoggingSettingsOptions = {}) {
  const [config, setConfig] = useState<LoggingConfig>({
    level: 'info',
    console_enabled: true,
    file_enabled: true,
    json_format: false,
  });

  const [stats, setStats] = useState<LoggingStats | null>(null);
  const [availableLevels, setAvailableLevels] = useState<LogLevel[]>([]);
  const [recentLogs, setRecentLogs] = useState<string[]>([]);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [testing, setTesting] = useState(false);
  const [cleaning, setCleaning] = useState(false);
  const [opening, setOpening] = useState(false);

  // 加载日志配置
  const loadConfig = useCallback(async () => {
    try {
      setLoading(true);
      const [logConfig, logStats, levels] = await Promise.all([
        getLogConfig(),
        getLogStats(),
        getAvailableLogLevels(),
      ]);

      setConfig(logConfig);
      setStats(logStats);
      setAvailableLevels(levels);
    } catch (error) {
      console.error('Failed to load logging config:', error);
    } finally {
      setLoading(false);
    }
  }, []);

  // 保存日志配置
  const saveConfig = useCallback(async () => {
    try {
      setSaving(true);
      await updateLogConfig(config);
      onUpdate?.();
    } catch (error) {
      console.error('Failed to save logging config:', error);
      throw error;
    } finally {
      setSaving(false);
    }
  }, [config, onUpdate]);

  // 设置日志级别
  const changeLogLevel = useCallback(async (level: LogLevel) => {
    try {
      await setLogLevel(level);
      setConfig(prev => ({ ...prev, level }));
    } catch (error) {
      console.error('Failed to change log level:', error);
      throw error;
    }
  }, []);

  // 测试日志输出
  const handleTestLogging = useCallback(async () => {
    try {
      setTesting(true);
      await testLogging();
    } catch (error) {
      console.error('Failed to test logging:', error);
      throw error;
    } finally {
      setTesting(false);
    }
  }, []);

  // 打开日志目录
  const handleOpenLogDirectory = useCallback(async () => {
    try {
      setOpening(true);
      await openLogDirectory();
    } catch (error) {
      console.error('Failed to open log directory:', error);
      throw error;
    } finally {
      setOpening(false);
    }
  }, []);

  // 清理旧日志
  const handleCleanupOldLogs = useCallback(async (daysToKeep: number = 7) => {
    try {
      setCleaning(true);
      const deletedCount = await cleanupOldLogs(daysToKeep);

      // 重新加载统计信息
      const logStats = await getLogStats();
      setStats(logStats);

      return deletedCount;
    } catch (error) {
      console.error('Failed to cleanup old logs:', error);
      throw error;
    } finally {
      setCleaning(false);
    }
  }, []);

  // 获取最近日志
  const loadRecentLogs = useCallback(async (lines: number = 50) => {
    try {
      const logs = await getRecentLogs(lines);
      setRecentLogs(logs);
    } catch (error) {
      console.error('Failed to load recent logs:', error);
      throw error;
    }
  }, []);

  // 刷新日志缓冲区
  const handleFlushLogs = useCallback(async () => {
    try {
      await flushLogs();
    } catch (error) {
      console.error('Failed to flush logs:', error);
      throw error;
    }
  }, []);

  // 格式化日志统计信息
  const formatStats = useCallback((stats: LoggingStats) => {
    const uptimeHours = Math.floor(stats.uptime_seconds / 3600);
    const uptimeMinutes = Math.floor((stats.uptime_seconds % 3600) / 60);
    const fileSize = stats.log_file_size
      ? `${(stats.log_file_size / 1024 / 1024).toFixed(2)} MB`
      : 'N/A';

    return {
      uptime: `${uptimeHours}h ${uptimeMinutes}m`,
      totalLogs: stats.total_logs.toLocaleString(),
      fileSize,
      errorCount: stats.error_count,
      warnCount: stats.warn_count,
      infoCount: stats.info_count,
    };
  }, []);

  // 格式化日志级别显示文本
  const formatLogLevel = useCallback((level: LogLevel): string => {
    const levelMap: Record<LogLevel, string> = {
      error: '错误',
      warn: '警告',
      info: '信息',
      debug: '调试',
      trace: '跟踪',
    };
    return levelMap[level] || level;
  }, []);

  // 格式化日志级别颜色
  const getLogLevelColor = useCallback((level: LogLevel): string => {
    const colorMap: Record<LogLevel, string> = {
      error: 'text-red-600',
      warn: 'text-yellow-600',
      info: 'text-blue-600',
      debug: 'text-gray-600',
      trace: 'text-gray-500',
    };
    return colorMap[level] || 'text-gray-600';
  }, []);

  // 初始化
  useEffect(() => {
    loadConfig();
  }, [loadConfig]);

  return {
    // 状态
    config,
    stats,
    availableLevels,
    recentLogs,
    loading,
    saving,
    testing,
    cleaning,
    opening,

    // 配置更新函数
    setConfig,

    // 操作函数
    saveConfig,
    changeLogLevel,
    handleTestLogging,
    handleOpenLogDirectory,
    handleCleanupOldLogs,
    loadRecentLogs,
    handleFlushLogs,

    // 工具函数
    formatStats,
    formatLogLevel,
    getLogLevelColor,

    // 刷新函数
    reloadConfig: loadConfig,
  };
}

export type UseLoggingSettingsReturn = ReturnType<typeof useLoggingSettings>;