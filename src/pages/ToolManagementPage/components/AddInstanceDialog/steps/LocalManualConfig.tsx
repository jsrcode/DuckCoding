// 手动路径配置组件（Step 2 - Manual）
// 手动输入工具路径 + 验证 + 安装器配置

import { Button } from '@/components/ui/button';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Loader2, InfoIcon } from 'lucide-react';
import type { InstallerCandidate } from '@/lib/tauri-commands';
import { PathValidator } from '../components/PathValidator';
import { InstallerSelector } from '../components/InstallerSelector';

interface LocalManualConfigProps {
  toolName: string;
  manualPath: string;
  installMethod: string;
  installerPath: string;
  installerCandidates: InstallerCandidate[];
  showCustomInstaller: boolean;
  validating: boolean;
  validationError: string | null;
  scanResult: { installed: boolean; version: string } | null;
  scanning: boolean;
  onPathChange: (path: string) => void;
  onBrowse: () => void;
  onValidate: () => void;
  onScan: () => void;
  onInstallMethodChange: (method: string) => void;
  onInstallerPathChange: (path: string) => void;
  onShowCustomInstallerChange: () => void;
  onBrowseInstaller: () => void;
}

const getCommonPaths = (baseId: string, toolName: string) => {
  const isWindows = navigator.platform.toLowerCase().includes('win');
  if (isWindows) {
    return [
      `C:\\Users\\用户名\\AppData\\Roaming\\npm\\${baseId}.cmd`,
      `C:\\Users\\用户名\\.npm-global\\${baseId}.cmd`,
      `C:\\Program Files\\${toolName}\\${baseId}.exe`,
    ];
  } else {
    return [
      `~/.npm-global/bin/${baseId}`,
      `/usr/local/bin/${baseId}`,
      `/opt/homebrew/bin/${baseId}`,
      `~/.local/bin/${baseId}`,
    ];
  }
};

export function LocalManualConfig({
  toolName,
  manualPath,
  installMethod,
  installerPath,
  installerCandidates,
  showCustomInstaller,
  validating,
  validationError,
  scanResult,
  scanning,
  onPathChange,
  onBrowse,
  onValidate,
  onScan,
  onInstallMethodChange,
  onInstallerPathChange,
  onShowCustomInstallerChange,
  onBrowseInstaller,
}: LocalManualConfigProps) {
  // 从 toolName 提取 baseId（简化处理）
  const baseId = toolName.toLowerCase().replace(/\s+/g, '-');
  const commonPaths = getCommonPaths(baseId, toolName);

  return (
    <>
      <Alert>
        <InfoIcon className="h-4 w-4" />
        <AlertDescription className="space-y-2">
          <p className="font-medium">常见安装路径：</p>
          <ul className="list-disc list-inside text-xs space-y-1">
            {commonPaths.map((path, index) => (
              <li key={index} className="font-mono">
                {path}
              </li>
            ))}
          </ul>
        </AlertDescription>
      </Alert>

      {/* 工具路径输入 */}
      <PathValidator
        value={manualPath}
        validating={validating}
        error={validationError}
        placeholder="输入或浏览选择"
        disabled={scanning}
        onValueChange={onPathChange}
        onBrowse={onBrowse}
        onValidate={onValidate}
      />

      {/* 验证路径按钮 */}
      <Button
        onClick={onScan}
        disabled={scanning || !manualPath || !!validationError}
        className="w-full"
        variant="outline"
      >
        {scanning ? (
          <>
            <Loader2 className="mr-2 h-4 w-4 animate-spin" />
            验证并扫描安装器...
          </>
        ) : (
          '验证路径'
        )}
      </Button>

      {/* 验证成功提示 */}
      {scanResult && scanResult.installed && (
        <Alert>
          <InfoIcon className="h-4 w-4" />
          <AlertDescription>
            ✓ 验证成功：{toolName} v{scanResult.version}
          </AlertDescription>
        </Alert>
      )}

      {/* 安装器配置（验证成功后显示） */}
      {scanResult && scanResult.installed && (
        <InstallerSelector
          installerCandidates={installerCandidates}
          selectedPath={installerPath}
          installMethod={installMethod}
          showCustomMode={showCustomInstaller}
          disabled={scanning}
          onInstallerSelect={onInstallerPathChange}
          onInstallMethodChange={onInstallMethodChange}
          onCustomModeToggle={onShowCustomInstallerChange}
          onBrowse={onBrowseInstaller}
        />
      )}
    </>
  );
}
