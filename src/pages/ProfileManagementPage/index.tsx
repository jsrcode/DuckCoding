/**
 * Profile 配置管理页面
 */

import { useState, useEffect } from 'react';
import { RefreshCw, Loader2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { PageContainer } from '@/components/layout/PageContainer';
import { ProfileCard } from './components/ProfileCard';
import { ProfileEditor } from './components/ProfileEditor';
import { ActiveProfileCard } from './components/ActiveProfileCard';
import { useProfileManagement } from './hooks/useProfileManagement';
import type { ToolId, ProfileFormData, ProfileDescriptor } from '@/types/profile';
import { logoMap } from '@/utils/constants';

export default function ProfileManagementPage() {
  const {
    profileGroups,
    loading,
    error,
    allProxyStatus,
    refresh,
    loadAllProxyStatus,
    createProfile,
    updateProfile,
    deleteProfile,
    activateProfile,
  } = useProfileManagement();

  const [selectedTab, setSelectedTab] = useState<ToolId>('claude-code');
  const [editorOpen, setEditorOpen] = useState(false);
  const [editorMode, setEditorMode] = useState<'create' | 'edit'>('create');
  const [editingProfile, setEditingProfile] = useState<ProfileDescriptor | null>(null);

  // 初始化加载透明代理状态
  useEffect(() => {
    loadAllProxyStatus();
  }, [loadAllProxyStatus]);

  // 打开创建对话框
  const handleCreateProfile = () => {
    setEditorMode('create');
    setEditingProfile(null);
    setEditorOpen(true);
  };

  // 打开编辑对话框
  const handleEditProfile = (profile: ProfileDescriptor) => {
    setEditorMode('edit');
    setEditingProfile(profile);
    setEditorOpen(true);
  };

  // 保存 Profile
  const handleSaveProfile = async (data: ProfileFormData) => {
    if (editorMode === 'create') {
      await createProfile(selectedTab, data);
    } else if (editingProfile) {
      await updateProfile(selectedTab, editingProfile.name, data);
    }
    setEditorOpen(false);
    // 对话框关闭后刷新数据
    await refresh();
  };

  // 激活 Profile
  const handleActivateProfile = async (profileName: string) => {
    await activateProfile(selectedTab, profileName);
  };

  // 删除 Profile
  const handleDeleteProfile = async (profileName: string) => {
    await deleteProfile(selectedTab, profileName);
  };

  // 构建编辑器初始数据
  const getEditorInitialData = (): ProfileFormData | undefined => {
    if (!editingProfile) return undefined;

    return {
      name: editingProfile.name,
      api_key: '', // 编辑时留空表示不修改
      base_url: editingProfile.base_url,
      wire_api: editingProfile.wire_api || editingProfile.provider, // 兼容两个字段名
      model: editingProfile.model,
    };
  };

  return (
    <PageContainer>
      {/* 页面标题 */}
      <div className="mb-6">
        <div className="flex items-center justify-between">
          <div>
            <h2 className="text-2xl font-semibold mb-1">配置管理</h2>
            <p className="text-sm text-muted-foreground">
              管理所有工具的 Profile 配置，快速切换不同的 API 端点
            </p>
          </div>
          <Button onClick={refresh} variant="outline" size="sm" disabled={loading}>
            <RefreshCw className={`mr-2 h-4 w-4 ${loading ? 'animate-spin' : ''}`} />
            刷新
          </Button>
        </div>
      </div>

      {/* 错误提示 */}
      {error && (
        <div className="mb-6 rounded-lg border border-destructive bg-destructive/10 p-4">
          <p className="text-sm text-destructive">加载失败: {error}</p>
          <Button onClick={refresh} variant="outline" size="sm" className="mt-2">
            重试
          </Button>
        </div>
      )}

      {/* 加载状态 */}
      {loading && profileGroups.length === 0 ? (
        <div className="flex items-center justify-center py-20">
          <Loader2 className="h-8 w-8 animate-spin text-primary" />
          <span className="ml-3 text-muted-foreground">加载中...</span>
        </div>
      ) : (
        <>
          {/* 工具 Tab 切换 */}
          <Tabs value={selectedTab} onValueChange={(v) => setSelectedTab(v as ToolId)}>
            <TabsList className="grid w-full grid-cols-3 mb-6">
              {profileGroups.map((group) => (
                <TabsTrigger key={group.tool_id} value={group.tool_id} className="gap-2">
                  <img src={logoMap[group.tool_id]} alt={group.tool_name} className="w-4 h-4" />
                  {group.tool_name}
                </TabsTrigger>
              ))}
            </TabsList>

            {/* 每个工具的 Profile 列表 */}
            {profileGroups.map((group) => (
              <TabsContent key={group.tool_id} value={group.tool_id} className="space-y-4">
                {/* 当前生效配置卡片 */}
                <ActiveProfileCard
                  group={group}
                  proxyRunning={allProxyStatus[group.tool_id]?.running || false}
                />

                {/* 创建按钮 */}
                <div className="flex items-center justify-between">
                  <div>
                    <p className="text-sm text-muted-foreground">
                      {group.profiles.length === 0
                        ? '暂无 Profile，点击创建新配置'
                        : `共 ${group.profiles.length} 个配置`}
                      {group.active_profile && ` · 当前激活: ${group.active_profile.name}`}
                    </p>
                  </div>
                  <Button
                    onClick={handleCreateProfile}
                    size="sm"
                    disabled={selectedTab !== group.tool_id}
                  >
                    创建 Profile
                  </Button>
                </div>

                {/* Profile 卡片列表 */}
                {group.profiles.length === 0 ? (
                  <div className="rounded-lg border border-dashed p-12 text-center">
                    <p className="text-sm text-muted-foreground">暂无 Profile 配置</p>
                  </div>
                ) : (
                  <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
                    {group.profiles.map((profile) => (
                      <ProfileCard
                        key={profile.name}
                        profile={profile}
                        onActivate={() => handleActivateProfile(profile.name)}
                        onEdit={() => handleEditProfile(profile)}
                        onDelete={() => handleDeleteProfile(profile.name)}
                        proxyRunning={allProxyStatus[group.tool_id]?.running || false}
                      />
                    ))}
                  </div>
                )}
              </TabsContent>
            ))}
          </Tabs>
        </>
      )}

      {/* Profile 编辑器对话框 */}
      <ProfileEditor
        open={editorOpen}
        onOpenChange={setEditorOpen}
        toolId={selectedTab}
        mode={editorMode}
        initialData={getEditorInitialData()}
        onSave={handleSaveProfile}
      />
    </PageContainer>
  );
}
