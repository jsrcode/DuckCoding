import { useEffect, useState } from 'react';
import { Switch } from '@/components/ui/switch';
import { Label } from '@/components/ui/label';
import { Separator } from '@/components/ui/separator';
import { Button } from '@/components/ui/button';
import { Settings as SettingsIcon, Info, RefreshCw, Loader2 } from 'lucide-react';
import { useToast } from '@/hooks/use-toast';
import { getSingleInstanceConfig, updateSingleInstanceConfig } from '@/lib/tauri-commands';

export function ApplicationSettingsTab() {
  const [singleInstanceEnabled, setSingleInstanceEnabled] = useState(true);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const { toast } = useToast();

  // 加载配置
  useEffect(() => {
    const loadConfig = async () => {
      setLoading(true);
      try {
        const enabled = await getSingleInstanceConfig();
        setSingleInstanceEnabled(enabled);
      } catch (error) {
        console.error('加载单实例配置失败:', error);
        toast({
          title: '加载失败',
          description: String(error),
          variant: 'destructive',
        });
      } finally {
        setLoading(false);
      }
    };

    loadConfig();
  }, [toast]);

  // 保存配置
  const handleToggle = async (checked: boolean) => {
    setSaving(true);
    try {
      await updateSingleInstanceConfig(checked);
      setSingleInstanceEnabled(checked);
      toast({
        title: '设置已保存',
        description: (
          <div className="flex flex-col gap-2">
            <p>请重启应用以使更改生效</p>
            <Button
              size="sm"
              variant="outline"
              onClick={() => window.location.reload()}
              className="w-fit"
            >
              <RefreshCw className="mr-2 h-4 w-4" />
              立即重启
            </Button>
          </div>
        ),
      });
    } catch (error) {
      console.error('保存单实例配置失败:', error);
      toast({
        title: '保存失败',
        description: String(error),
        variant: 'destructive',
      });
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="space-y-4 rounded-lg border p-6">
      <div className="flex items-center gap-2">
        <SettingsIcon className="h-5 w-5" />
        <h3 className="text-lg font-semibold">应用行为</h3>
      </div>
      <Separator />

      <div className="space-y-4">
        <div className="flex items-center justify-between">
          <div className="space-y-1">
            <Label htmlFor="single-instance">单实例模式</Label>
            <p className="text-sm text-muted-foreground">
              启用后，同时只能运行一个应用实例（生产环境）
            </p>
          </div>
          <Switch
            id="single-instance"
            checked={singleInstanceEnabled}
            onCheckedChange={handleToggle}
            disabled={loading || saving}
          />
        </div>

        <div className="rounded-lg bg-blue-50 dark:bg-blue-950/30 border border-blue-200 dark:border-blue-800 p-4">
          <div className="flex items-start gap-2">
            <Info className="h-4 w-4 text-blue-600 dark:text-blue-400 flex-shrink-0 mt-0.5" />
            <div className="text-sm space-y-2">
              <p className="font-semibold text-blue-800 dark:text-blue-200">关于单实例模式</p>
              <ul className="list-disc list-inside space-y-1 text-blue-700 dark:text-blue-300">
                <li>
                  <strong>启用（推荐）：</strong>打开第二个实例时会聚焦到第一个窗口，节省系统资源
                </li>
                <li>
                  <strong>禁用：</strong>允许同时运行多个实例，适用于多账户测试或特殊需求
                </li>
                <li>
                  <strong>开发环境：</strong>始终允许多实例（与正式版隔离）
                </li>
                <li>
                  <strong>生效方式：</strong>更改后需要重启应用才能生效
                </li>
              </ul>
            </div>
          </div>
        </div>

        {(loading || saving) && (
          <div className="flex items-center justify-center gap-2 text-sm text-muted-foreground">
            <Loader2 className="h-4 w-4 animate-spin" />
            <span>{loading ? '加载中...' : '保存中...'}</span>
          </div>
        )}
      </div>
    </div>
  );
}
