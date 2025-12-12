// 统一状态管理 Hook
// 集中管理 AddInstanceDialog 的所有状态

import { useState, useCallback } from 'react';
import type { ToolCandidate, InstallerCandidate } from '@/lib/tauri-commands';

export interface AddInstanceState {
  // 基础状态
  step: number;
  baseId: string;
  envType: 'local' | 'wsl' | 'ssh';
  localMethod: 'auto' | 'manual';

  // 路径状态
  manualPath: string;
  installMethod: 'npm' | 'brew' | 'official' | 'other';
  installerPath: string;

  // 候选状态
  toolCandidates: ToolCandidate[];
  selectedToolCandidate: ToolCandidate | null;
  installerCandidates: InstallerCandidate[];
  showCustomInstaller: boolean;

  // UI 状态
  loading: boolean;
  scanning: boolean;
  validating: boolean;
  validationError: string | null;
  scanResult: { installed: boolean; version: string } | null;

  // WSL 状态
  wslDistros: string[];
  selectedDistro: string;
  loadingDistros: boolean;
}

export interface AddInstanceActions {
  // 基础操作
  setStep: (step: number) => void;
  setBaseId: (id: string) => void;
  setEnvType: (type: 'local' | 'wsl' | 'ssh') => void;
  setLocalMethod: (method: 'auto' | 'manual') => void;

  // 路径操作
  setManualPath: (path: string) => void;
  setInstallMethod: (method: 'npm' | 'brew' | 'official' | 'other') => void;
  setInstallerPath: (path: string) => void;

  // 候选操作
  setToolCandidates: (candidates: ToolCandidate[]) => void;
  setSelectedToolCandidate: (candidate: ToolCandidate | null) => void;
  setInstallerCandidates: (candidates: InstallerCandidate[]) => void;
  setShowCustomInstaller: (show: boolean) => void;

  // UI 操作
  setLoading: (loading: boolean) => void;
  setScanning: (scanning: boolean) => void;
  setValidating: (validating: boolean) => void;
  setValidationError: (error: string | null) => void;
  setScanResult: (result: { installed: boolean; version: string } | null) => void;

  // WSL 操作
  setWslDistros: (distros: string[]) => void;
  setSelectedDistro: (distro: string) => void;
  setLoadingDistros: (loading: boolean) => void;

  // 批量重置
  resetScanState: () => void;
  resetAllState: () => void;
}

const initialState: AddInstanceState = {
  step: 1,
  baseId: 'claude-code',
  envType: 'local',
  localMethod: 'auto',
  manualPath: '',
  installMethod: 'npm',
  installerPath: '',
  toolCandidates: [],
  selectedToolCandidate: null,
  installerCandidates: [],
  showCustomInstaller: false,
  loading: false,
  scanning: false,
  validating: false,
  validationError: null,
  scanResult: null,
  wslDistros: [],
  selectedDistro: '',
  loadingDistros: false,
};

export function useAddInstanceState() {
  const [state, setState] = useState<AddInstanceState>(initialState);

  // 基础操作
  const setStep = useCallback((step: number) => {
    setState((prev) => ({ ...prev, step }));
  }, []);

  const setBaseId = useCallback((baseId: string) => {
    setState((prev) => ({ ...prev, baseId }));
  }, []);

  const setEnvType = useCallback((envType: 'local' | 'wsl' | 'ssh') => {
    setState((prev) => ({ ...prev, envType }));
  }, []);

  const setLocalMethod = useCallback((localMethod: 'auto' | 'manual') => {
    setState((prev) => ({ ...prev, localMethod }));
  }, []);

  // 路径操作
  const setManualPath = useCallback((manualPath: string) => {
    setState((prev) => ({ ...prev, manualPath }));
  }, []);

  const setInstallMethod = useCallback((installMethod: 'npm' | 'brew' | 'official' | 'other') => {
    setState((prev) => ({ ...prev, installMethod }));
  }, []);

  const setInstallerPath = useCallback((installerPath: string) => {
    setState((prev) => ({ ...prev, installerPath }));
  }, []);

  // 候选操作
  const setToolCandidates = useCallback((toolCandidates: ToolCandidate[]) => {
    setState((prev) => ({ ...prev, toolCandidates }));
  }, []);

  const setSelectedToolCandidate = useCallback((selectedToolCandidate: ToolCandidate | null) => {
    setState((prev) => ({ ...prev, selectedToolCandidate }));
  }, []);

  const setInstallerCandidates = useCallback((installerCandidates: InstallerCandidate[]) => {
    setState((prev) => ({ ...prev, installerCandidates }));
  }, []);

  const setShowCustomInstaller = useCallback((showCustomInstaller: boolean) => {
    setState((prev) => ({ ...prev, showCustomInstaller }));
  }, []);

  // UI 操作
  const setLoading = useCallback((loading: boolean) => {
    setState((prev) => ({ ...prev, loading }));
  }, []);

  const setScanning = useCallback((scanning: boolean) => {
    setState((prev) => ({ ...prev, scanning }));
  }, []);

  const setValidating = useCallback((validating: boolean) => {
    setState((prev) => ({ ...prev, validating }));
  }, []);

  const setValidationError = useCallback((validationError: string | null) => {
    setState((prev) => ({ ...prev, validationError }));
  }, []);

  const setScanResult = useCallback(
    (scanResult: { installed: boolean; version: string } | null) => {
      setState((prev) => ({ ...prev, scanResult }));
    },
    [],
  );

  // WSL 操作
  const setWslDistros = useCallback((wslDistros: string[]) => {
    setState((prev) => ({ ...prev, wslDistros }));
  }, []);

  const setSelectedDistro = useCallback((selectedDistro: string) => {
    setState((prev) => ({ ...prev, selectedDistro }));
  }, []);

  const setLoadingDistros = useCallback((loadingDistros: boolean) => {
    setState((prev) => ({ ...prev, loadingDistros }));
  }, []);

  // 批量重置
  const resetScanState = useCallback(() => {
    setState((prev) => ({
      ...prev,
      scanResult: null,
      toolCandidates: [],
      selectedToolCandidate: null,
      installerCandidates: [],
    }));
  }, []);

  const resetAllState = useCallback(() => {
    setState(initialState);
  }, []);

  const actions: AddInstanceActions = {
    setStep,
    setBaseId,
    setEnvType,
    setLocalMethod,
    setManualPath,
    setInstallMethod,
    setInstallerPath,
    setToolCandidates,
    setSelectedToolCandidate,
    setInstallerCandidates,
    setShowCustomInstaller,
    setLoading,
    setScanning,
    setValidating,
    setValidationError,
    setScanResult,
    setWslDistros,
    setSelectedDistro,
    setLoadingDistros,
    resetScanState,
    resetAllState,
  };

  return { state, actions };
}
