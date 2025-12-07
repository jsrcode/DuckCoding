// 代理配置切换 Hook
// 用于透明代理开关框内的配置切换功能

import { useState, useCallback } from 'react';
import { pmListToolProfiles, updateProxyFromProfile } from '@/lib/tauri-commands';
import type { ToolId } from '../types/proxy-history';

/**
 * 代理配置切换 Hook
 *
 * 功能：
 * - 加载指定工具的配置列表（使用新的 ProfileManager API）
 * - 切换配置（直接更新代理配置，不激活 Profile）
 */
export function useProxyConfigSwitch(toolId: ToolId) {
  const [profiles, setProfiles] = useState<string[]>([]);
  const [loading, setLoading] = useState(false);

  /**
   * 加载配置列表
   */
  const loadProfiles = useCallback(async () => {
    try {
      const profileList = await pmListToolProfiles(toolId);
      setProfiles(profileList);
    } catch (error) {
      console.error('Failed to load profiles:', error);
      setProfiles([]);
    }
  }, [toolId]);

  /**
   * 切换配置（仅更新代理的 real_* 字段，不激活 Profile）
   * @param profileName - 配置名称
   * @returns 操作结果
   */
  const switchConfig = useCallback(
    async (profileName: string): Promise<{ success: boolean; error?: string }> => {
      setLoading(true);
      try {
        await updateProxyFromProfile(toolId, profileName);
        return { success: true };
      } catch (error) {
        return { success: false, error: String(error) };
      } finally {
        setLoading(false);
      }
    },
    [toolId],
  );

  return {
    /** 配置列表 */
    profiles,
    /** 加载状态 */
    loading,
    /** 加载配置列表 */
    loadProfiles,
    /** 切换配置 */
    switchConfig,
  };
}
