// 会话配置管理 Hook
// 提供配置列表加载和应用配置到会话的功能

import { useState, useCallback } from 'react';
import { pmListToolProfiles, pmGetProfile, updateSessionConfig } from '@/lib/tauri-commands';

/**
 * 会话配置管理 Hook
 *
 * 功能：
 * - 加载 Claude Code 配置列表（使用新的 ProfileManager API）
 * - 应用选中的配置到指定会话
 * - 处理配置切换逻辑（global vs custom）
 */
export function useSessionConfigManagement() {
  const [profiles, setProfiles] = useState<string[]>([]);
  const [loading, setLoading] = useState(false);

  /**
   * 加载配置列表
   * - "global" 为全局默认配置
   * - 其余为用户自定义配置文件
   */
  const loadProfiles = useCallback(async () => {
    try {
      const profileList = await pmListToolProfiles('claude-code');
      setProfiles(['global', ...profileList]);
    } catch (error) {
      console.error('Failed to load profiles:', error);
      setProfiles(['global']); // 降级到只显示 global
    }
  }, []);

  /**
   * 应用配置到会话
   * @param sessionId - 会话 ID
   * @param selectedProfile - 选中的配置名称
   * @returns 操作结果
   */
  const applyConfig = useCallback(async (sessionId: string, selectedProfile: string) => {
    setLoading(true);
    try {
      if (selectedProfile === 'global') {
        // 切换到全局配置：config_name="global", custom_profile_name=null, url="", api_key=""
        await updateSessionConfig(sessionId, 'global', null, '', '');
      } else {
        // 切换到自定义配置：读取指定配置文件的详情（不激活）
        const profileData = await pmGetProfile('claude-code', selectedProfile);
        // 保存 custom_profile_name 以便显示
        await updateSessionConfig(
          sessionId,
          'custom',
          selectedProfile,
          profileData.base_url,
          profileData.api_key,
        );
      }
      return { success: true };
    } catch (error) {
      return { success: false, error: String(error) };
    } finally {
      setLoading(false);
    }
  }, []);

  return {
    /** 配置列表 */
    profiles,
    /** 加载状态 */
    loading,
    /** 加载配置列表 */
    loadProfiles,
    /** 应用配置到会话 */
    applyConfig,
  };
}
