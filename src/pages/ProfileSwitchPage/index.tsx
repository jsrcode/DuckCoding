import { useState, useEffect } from 'react';
import { Loader2 } from 'lucide-react';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { PageContainer } from '@/components/layout/PageContainer';
import { logoMap } from '@/utils/constants';
import {
  ClaudeConfigManager,
  CodexConfigManager,
  GeminiConfigManager,
} from '@/components/ToolConfigManager';
import type { ToolStatus } from '@/lib/tauri-commands';

interface ProfileSwitchPageProps {
  tools: ToolStatus[];
  loading: boolean;
}

export function ProfileSwitchPage({
  tools: toolsProp,
  loading: loadingProp,
}: ProfileSwitchPageProps) {
  const [tools, setTools] = useState<ToolStatus[]>(toolsProp);
  const [loading, setLoading] = useState(loadingProp);
  const [selectedTab, setSelectedTab] = useState<string>('');

  // 同步外部 tools 数据
  useEffect(() => {
    setTools(toolsProp);
    setLoading(loadingProp);
  }, [toolsProp, loadingProp]);

  // 设置默认选中的Tab
  useEffect(() => {
    if (tools.length > 0 && !selectedTab) {
      setSelectedTab(tools[0].id);
    }
  }, [tools, selectedTab]);

  return (
    <PageContainer>
      <div className="mb-6">
        <h2 className="text-2xl font-semibold mb-1">高级配置编辑器</h2>
        <p className="text-sm text-muted-foreground">直接编辑工具的原生配置文件</p>
      </div>

      {loading ? (
        <div className="flex items-center justify-center py-20">
          <Loader2 className="h-8 w-8 animate-spin text-primary" />
          <span className="ml-3 text-muted-foreground">加载中...</span>
        </div>
      ) : tools.length === 0 ? (
        <div className="text-center py-20 text-muted-foreground">
          <p className="text-sm">暂无已安装的工具</p>
          <p className="text-xs mt-1">请先在工具管理页面安装工具</p>
        </div>
      ) : (
        <Tabs value={selectedTab} onValueChange={setSelectedTab}>
          <TabsList className="grid w-full grid-cols-3">
            {tools.map((tool) => (
              <TabsTrigger key={tool.id} value={tool.id} className="gap-2">
                <img src={logoMap[tool.id]} alt={tool.name} className="w-4 h-4" />
                {tool.name}
              </TabsTrigger>
            ))}
          </TabsList>

          {tools.map((tool) => (
            <TabsContent key={tool.id} value={tool.id}>
              {tool.id === 'claude-code' && <ClaudeConfigManager />}
              {tool.id === 'codex' && <CodexConfigManager />}
              {tool.id === 'gemini-cli' && <GeminiConfigManager />}
            </TabsContent>
          ))}
        </Tabs>
      )}
    </PageContainer>
  );
}
