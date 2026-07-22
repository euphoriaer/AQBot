import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { useSettingsStore } from './settingsStore';
import { useProviderStore } from './providerStore';
import { useGatewayStore } from './gatewayStore';
import { useMcpStore } from './mcpStore';
import { useSearchStore } from './searchStore';
import { useMemoryStore } from './memoryStore';
import { useKnowledgeStore } from './knowledgeStore';
import { useRoleStore } from './roleStore';
import { useSkillStore } from './skillStore';
import { useDrawingStore } from './drawingStore';

interface SettingsChangedPayload {
  category: string;
  action: string;
}

let started = false;
let unlisten: UnlistenFn | null = null;

async function reloadCategory(category: string): Promise<void> {
  switch (category) {
    case 'app':
      await useSettingsStore.getState().ensureSettingsLoaded({ force: true });
      break;
    case 'providers':
      await useProviderStore.getState().ensureProvidersLoaded({ force: true });
      break;
    case 'gateway':
      await Promise.all([
        useGatewayStore.getState().ensureStatusLoaded({ force: true }),
        useGatewayStore.getState().ensureKeysLoaded({ force: true }),
      ]);
      break;
    case 'mcp':
      await useMcpStore.getState().ensureServersLoaded({ force: true });
      break;
    case 'search':
      await useSearchStore.getState().ensureProvidersLoaded({ force: true });
      break;
    case 'memory':
      await useMemoryStore.getState().ensureNamespacesLoaded({ force: true });
      break;
    case 'knowledge':
      await useKnowledgeStore.getState().ensureBasesLoaded({ force: true });
      break;
    case 'roles':
      await useRoleStore.getState().ensureRolesLoaded({ force: true });
      break;
    case 'skills':
      await useSkillStore.getState().ensureSkillsLoaded({ force: true });
      break;
    case 'drawing':
      await useDrawingStore.getState().ensureHistoryLoaded({ force: true });
      break;
    default:
      console.warn('[settingsSync] no reload handler for category:', category);
  }
}

export async function startSettingsSync(): Promise<void> {
  if (started) return;
  started = true;
  unlisten = await listen<SettingsChangedPayload>('settings-changed', (event) => {
    const { category } = event.payload;
    reloadCategory(category).catch((e) => {
      console.warn('[settingsSync] reload failed for', category, e);
    });
  });
}

export async function stopSettingsSync(): Promise<void> {
  if (unlisten) {
    unlisten();
    unlisten = null;
  }
  started = false;
}
