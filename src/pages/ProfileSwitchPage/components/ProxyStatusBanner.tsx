import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Power, Sparkles, X, ExternalLink } from 'lucide-react';

interface ProxyStatusBannerProps {
  toolId: string;
  toolName: string;
  isEnabled: boolean;
  isRunning: boolean;
  hidden?: boolean; // 是否隐藏推荐提示（用户选择不再显示或临时关闭）
  onNavigateToProxy: () => void;
  onClose?: () => void; // 临时关闭推荐提示
  onNeverShow?: () => void; // 永久隐藏推荐提示
}

export function ProxyStatusBanner({
  toolId: _toolId,
  toolName,
  isEnabled,
  isRunning: _isRunning,
  hidden,
  onNavigateToProxy,
  onClose,
  onNeverShow,
}: ProxyStatusBannerProps) {
  // 已启用透明代理 - 统一显示蓝色提示，引导用户到专用页面管理
  if (isEnabled) {
    return (
      <div className="mb-6 p-4 bg-gradient-to-r from-blue-50 to-indigo-50 dark:from-blue-950 dark:to-indigo-950 rounded-lg border border-blue-200 dark:border-blue-800">
        <div className="flex items-center justify-between">
          <div className="flex items-start gap-3 flex-1">
            <Power className="h-5 w-5 text-blue-600 dark:text-blue-400 flex-shrink-0 mt-0.5" />
            <div className="space-y-1 flex-1">
              <h4 className="font-semibold text-blue-900 dark:text-blue-100 flex items-center gap-2">
                {toolName} 透明代理已启用
                <Badge variant="default" className="text-xs">
                  已启用
                </Badge>
              </h4>
              <p className="text-sm text-blue-800 dark:text-blue-200">
                配置切换功能已禁用，请前往透明代理页管理配置和控制代理运行状态。
              </p>
            </div>
          </div>
          <Button
            type="button"
            variant="default"
            size="sm"
            onClick={onNavigateToProxy}
            className="shadow-sm bg-blue-600 hover:bg-blue-700 flex-shrink-0"
          >
            <ExternalLink className="h-4 w-4 mr-1" />
            前往透明代理管理
          </Button>
        </div>
      </div>
    );
  }

  // 未启用透明代理 - 显示推荐Banner（可关闭和永久隐藏）
  if (hidden) return null;

  return (
    <div className="mb-6 p-4 bg-gradient-to-r from-green-50 to-emerald-50 dark:from-green-950 dark:to-emerald-950 rounded-lg border border-green-200 dark:border-green-800">
      <div className="flex items-start justify-between gap-3">
        <div className="flex items-start gap-3 flex-1">
          <Sparkles className="h-5 w-5 text-green-600 dark:text-green-400 flex-shrink-0 mt-0.5" />
          <div className="space-y-2 flex-1">
            <h4 className="font-semibold text-green-900 dark:text-green-100 flex items-center gap-2">
              💡 推荐体验：{toolName} 透明代理
              <Badge
                variant="outline"
                className="text-xs border-green-600 text-green-700 dark:text-green-300"
              >
                实验性
              </Badge>
            </h4>
            <p className="text-sm text-green-800 dark:text-green-200">
              启用透明代理后，切换 {toolName} 配置<strong>无需重启终端</strong>
              ，配置实时生效！大幅提升工作效率。
            </p>
            <div className="flex gap-2 mt-3">
              <Button
                type="button"
                variant="outline"
                size="sm"
                onClick={onNavigateToProxy}
                className="shadow-sm border-green-600 text-green-700 hover:bg-green-100 dark:text-green-300 dark:hover:bg-green-950"
              >
                <ExternalLink className="h-3 w-3 mr-1" />
                立即体验
              </Button>
              {onNeverShow && (
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  onClick={onNeverShow}
                  className="text-green-700 hover:bg-green-100 dark:text-green-300 dark:hover:bg-green-950"
                >
                  不再显示
                </Button>
              )}
            </div>
          </div>
        </div>
        {onClose && (
          <Button
            type="button"
            variant="ghost"
            size="icon"
            onClick={onClose}
            className="flex-shrink-0 text-green-700 hover:bg-green-100 dark:text-green-300 dark:hover:bg-green-950 h-8 w-8"
          >
            <X className="h-4 w-4" />
          </Button>
        )}
      </div>
    </div>
  );
}
