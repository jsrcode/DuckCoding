import { useState } from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Badge } from '@/components/ui/badge';
import { Separator } from '@/components/ui/separator';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { useToast } from '@/hooks/use-toast';
import {
  Loader2,
  Play,
  FolderOpen,
  Trash2,
  RefreshCw,
  Download,
  Settings,
  Activity,
  HardDrive,
  Clock,
  AlertTriangle,
  CheckCircle,
} from 'lucide-react';
import type { UseLoggingSettingsReturn } from '../hooks/useLoggingSettings';

interface LoggingSettingsTabProps {
  logging: UseLoggingSettingsReturn;
}

export function LoggingSettingsTab({ logging }: LoggingSettingsTabProps) {
  const { toast } = useToast();
  const [cleanupDays, setCleanupDays] = useState(7);

  const {
    config,
    stats,
    availableLevels,
    recentLogs,
    loading,
    saving,
    testing,
    cleaning,
    opening,
    setConfig,
    saveConfig,
    changeLogLevel,
    handleTestLogging,
    handleOpenLogDirectory,
    handleCleanupOldLogs,
    loadRecentLogs,
    handleFlushLogs,
    formatStats,
    formatLogLevel,
    getLogLevelColor,
    reloadConfig,
  } = logging;

  // 处理配置保存
  const handleSaveConfig = async () => {
    try {
      await saveConfig();
      toast({
        title: '保存成功',
        description: '日志配置已更新',
      });
    } catch (error) {
      toast({
        title: '保存失败',
        description: String(error),
        variant: 'destructive',
      });
    }
  };

  // 处理日志级别变更
  const handleLogLevelChange = async (level: string) => {
    try {
      await changeLogLevel(level as any);
      toast({
        title: '更新成功',
        description: `日志级别已更改为 ${formatLogLevel(level as any)}`,
      });
    } catch (error) {
      toast({
        title: '更新失败',
        description: String(error),
        variant: 'destructive',
      });
    }
  };

  // 处理测试日志
  const handleTest = async () => {
    try {
      await handleTestLogging();
      toast({
        title: '测试成功',
        description: '已输出各级别测试日志',
      });
    } catch (error) {
      toast({
        title: '测试失败',
        description: String(error),
        variant: 'destructive',
      });
    }
  };

  // 处理打开日志目录
  const handleOpenDirectory = async () => {
    try {
      await handleOpenLogDirectory();
    } catch (error) {
      toast({
        title: '打开失败',
        description: String(error),
        variant: 'destructive',
      });
    }
  };

  // 处理清理日志
  const handleCleanup = async () => {
    try {
      const deletedCount = await handleCleanupOldLogs(cleanupDays);
      toast({
        title: '清理完成',
        description: `已删除 ${deletedCount} 个旧日志文件`,
      });
    } catch (error) {
      toast({
        title: '清理失败',
        description: String(error),
        variant: 'destructive',
      });
    }
  };

  // 处理刷新日志
  const handleRefreshLogs = async () => {
    try {
      await loadRecentLogs(20);
      toast({
        title: '刷新成功',
        description: '已加载最新日志',
      });
    } catch (error) {
      toast({
        title: '刷新失败',
        description: String(error),
        variant: 'destructive',
      });
    }
  };

  // 格式化运行时间
  const formatUptime = (seconds: number) => {
    const hours = Math.floor(seconds / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    return `${hours}h ${minutes}m`;
  };

  return (
    <div className="space-y-6">
      {/* 日志统计信息 */}
      {stats && (
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Activity className="h-5 w-5" />
              日志统计
            </CardTitle>
            <CardDescription>当前日志系统的运行状态和统计信息</CardDescription>
          </CardHeader>
          <CardContent>
            <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
              <div className="flex items-center gap-2">
                <Clock className="h-4 w-4 text-muted-foreground" />
                <div>
                  <div className="text-sm font-medium">运行时间</div>
                  <div className="text-lg">{formatUptime(stats.uptime_seconds)}</div>
                </div>
              </div>
              <div className="flex items-center gap-2">
                <HardDrive className="h-4 w-4 text-muted-foreground" />
                <div>
                  <div className="text-sm font-medium">日志文件大小</div>
                  <div className="text-lg">
                    {stats.log_file_size
                      ? `${(stats.log_file_size / 1024 / 1024).toFixed(2)} MB`
                      : 'N/A'}
                  </div>
                </div>
              </div>
              <div className="flex items-center gap-2">
                <AlertTriangle className="h-4 w-4 text-muted-foreground" />
                <div>
                  <div className="text-sm font-medium">错误数量</div>
                  <div className="text-lg text-red-600">{stats.error_count}</div>
                </div>
              </div>
              <div className="flex items-center gap-2">
                <CheckCircle className="h-4 w-4 text-muted-foreground" />
                <div>
                  <div className="text-sm font-medium">信息数量</div>
                  <div className="text-lg text-blue-600">{stats.info_count}</div>
                </div>
              </div>
            </div>
          </CardContent>
        </Card>
      )}

      {/* 基本配置 */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Settings className="h-5 w-5" />
            基本配置
          </CardTitle>
          <CardDescription>配置日志系统的基本参数和输出选项</CardDescription>
        </CardHeader>
        <CardContent className="space-y-6">
          {/* 日志级别 */}
          <div className="space-y-2">
            <Label htmlFor="log-level">日志级别</Label>
            <Select
              value={config.level}
              onValueChange={handleLogLevelChange}
              disabled={loading}
            >
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {availableLevels.map((level) => (
                  <SelectItem key={level} value={level}>
                    <span className={getLogLevelColor(level)}>
                      {formatLogLevel(level)}
                    </span>
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
            <p className="text-sm text-muted-foreground">
              选择日志输出的最低级别，低于此级别的日志将不会显示
            </p>
          </div>

          {/* 输出选项 */}
          <div className="space-y-4">
            <div className="flex items-center justify-between">
              <div className="space-y-0.5">
                <Label htmlFor="console-output">控制台输出</Label>
                <p className="text-sm text-muted-foreground">
                  在控制台显示日志信息，主要用于开发调试
                </p>
              </div>
              <Switch
                id="console-output"
                checked={config.console_enabled}
                onCheckedChange={(checked) =>
                  setConfig({ ...config, console_enabled: checked })
                }
                disabled={loading}
              />
            </div>

            <div className="flex items-center justify-between">
              <div className="space-y-0.5">
                <Label htmlFor="file-output">文件输出</Label>
                <p className="text-sm text-muted-foreground">
                  将日志信息保存到文件，便于后续分析和问题排查
                </p>
              </div>
              <Switch
                id="file-output"
                checked={config.file_enabled}
                onCheckedChange={(checked) =>
                  setConfig({ ...config, file_enabled: checked })
                }
                disabled={loading}
              />
            </div>

            <div className="flex items-center justify-between">
              <div className="space-y-0.5">
                <Label htmlFor="json-format">JSON 格式</Label>
                <p className="text-sm text-muted-foreground">
                  使用结构化 JSON 格式输出，便于日志分析工具处理
                </p>
              </div>
              <Switch
                id="json-format"
                checked={config.json_format}
                onCheckedChange={(checked) =>
                  setConfig({ ...config, json_format: checked })
                }
                disabled={loading}
              />
            </div>
          </div>

          {/* 保存按钮 */}
          <div className="flex justify-end">
            <Button onClick={handleSaveConfig} disabled={saving || loading}>
              {saving ? (
                <>
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  保存中...
                </>
              ) : (
                <>
                  <Settings className="mr-2 h-4 w-4" />
                  保存配置
                </>
              )}
            </Button>
          </div>
        </CardContent>
      </Card>

      {/* 日志管理 */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <RefreshCw className="h-5 w-5" />
            日志管理
          </CardTitle>
          <CardDescription>管理和维护日志文件</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-3">
            <Button
              variant="outline"
              onClick={handleTest}
              disabled={testing}
              className="flex items-center gap-2"
            >
              {testing ? (
                <Loader2 className="h-4 w-4 animate-spin" />
              ) : (
                <Play className="h-4 w-4" />
              )}
              测试日志
            </Button>

            <Button
              variant="outline"
              onClick={handleOpenDirectory}
              disabled={opening}
              className="flex items-center gap-2"
            >
              {opening ? (
                <Loader2 className="h-4 w-4 animate-spin" />
              ) : (
                <FolderOpen className="h-4 w-4" />
              )}
              打开目录
            </Button>

            <Button
              variant="outline"
              onClick={handleRefreshLogs}
              className="flex items-center gap-2"
            >
              <RefreshCw className="h-4 w-4" />
              刷新日志
            </Button>

            <Button
              variant="outline"
              onClick={handleFlushLogs}
              className="flex items-center gap-2"
            >
              <Download className="h-4 w-4" />
              刷新缓冲
            </Button>
          </div>

          {/* 日志清理 */}
          <div className="space-y-3">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <Label htmlFor="cleanup-days">清理旧日志</Label>
                <Badge variant="outline" className="text-xs">
                  保留最近 {cleanupDays} 天
                </Badge>
              </div>
              <div className="flex items-center gap-2">
                <Select
                  value={cleanupDays.toString()}
                  onValueChange={(value) => setCleanupDays(parseInt(value))}
                  disabled={cleaning}
                >
                  <SelectTrigger className="w-20">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="3">3 天</SelectItem>
                    <SelectItem value="7">7 天</SelectItem>
                    <SelectItem value="14">14 天</SelectItem>
                    <SelectItem value="30">30 天</SelectItem>
                  </SelectContent>
                </Select>
                <Button
                  variant="destructive"
                  size="sm"
                  onClick={handleCleanup}
                  disabled={cleaning}
                >
                  {cleaning ? (
                    <Loader2 className="h-4 w-4 animate-spin" />
                  ) : (
                    <Trash2 className="h-4 w-4" />
                  )}
                </Button>
              </div>
            </div>
            <p className="text-sm text-muted-foreground">
              删除超过指定天数的旧日志文件以释放磁盘空间
            </p>
          </div>
        </CardContent>
      </Card>

      {/* 最近日志 */}
      {recentLogs.length > 0 && (
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Activity className="h-5 w-5" />
              最近日志
            </CardTitle>
            <CardDescription>显示最近的日志条目</CardDescription>
          </CardHeader>
          <CardContent>
            <div className="space-y-2 max-h-96 overflow-y-auto font-mono text-sm">
              {recentLogs.map((log, index) => (
                <div
                  key={index}
                  className="p-2 bg-muted rounded-md border-l-4 border-l-blue-500"
                >
                  {log}
                </div>
              ))}
            </div>
          </CardContent>
        </Card>
      )}

      {/* 系统提示 */}
      <Alert>
        <AlertDescription>
          日志配置更改将在下次应用启动时生效。当前会话的日志级别可以通过下拉菜单即时调整。
        </AlertDescription>
      </Alert>
    </div>
  );
}