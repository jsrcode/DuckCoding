// 安装器扫描逻辑 Hook
// 封装安装器检测和管理的业务逻辑

import { useCallback } from 'react';
import { scanInstallerForToolPath, type InstallerCandidate } from '@/lib/tauri-commands';

export interface UseInstallerScannerParams {
  onInstallersFound: (candidates: InstallerCandidate[]) => void;
  onInstallerSelected: (path: string, type: string) => void;
}

export function useInstallerScanner(params: UseInstallerScannerParams) {
  /**
   * 扫描工具路径的安装器
   */
  const scanInstallersForPath = useCallback(
    async (toolPath: string) => {
      console.log('[useInstallerScanner] 扫描安装器，路径:', toolPath);

      try {
        const installers = await scanInstallerForToolPath(toolPath);
        console.log('[useInstallerScanner] 扫描到', installers.length, '个安装器候选');

        params.onInstallersFound(installers);

        // 如果找到安装器，自动选择第一个
        if (installers.length > 0) {
          const first = installers[0];
          params.onInstallerSelected(first.path, first.installer_type);
        }

        return installers;
      } catch (error) {
        console.error('[useInstallerScanner] 扫描安装器失败:', error);
        params.onInstallersFound([]);
        return [];
      }
    },
    [params],
  );

  /**
   * 选择安装器
   */
  const selectInstaller = useCallback(
    (path: string, type: string) => {
      params.onInstallerSelected(path, type);
    },
    [params],
  );

  return {
    scanInstallersForPath,
    selectInstaller,
  };
}
