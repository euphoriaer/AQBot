import { create } from 'zustand';
import { invoke } from '@/lib/invoke';
import type { SessionInfo } from '@/types/agent';

interface SessionInteropStore {
  /** All registered sessions (local + remote peers) */
  sessions: SessionInfo[];
  /** Sessions explicitly connected to the current conversation */
  connectedSessions: SessionInfo[];
  /** Loading / error state */
  loading: boolean;
  error: string | null;
  /** Set of target addresses this client currently holds the input lock on */
  heldLocks: Set<string>;

  /** Fetch the list of registered sessions from the backend */
  listSessions: () => Promise<void>;
  /** Fetch connected sessions for a conversation */
  getConnections: (conversationId: string) => Promise<void>;
  /** Connect a local session to a target session */
  connect: (conversationId: string, targetAddress: string) => Promise<void>;
  /** Disconnect from a target session */
  disconnect: (conversationId: string, targetAddress: string) => Promise<void>;
  /** Send input text to a target session (must hold the lock) */
  sendInput: (conversationId: string, targetAddress: string, content: string) => Promise<void>;
  /** Acquire the input lock on a target session */
  acquireLock: (conversationId: string, targetAddress: string) => Promise<void>;
  /** Release the input lock on a target session */
  releaseLock: (conversationId: string, targetAddress: string) => Promise<void>;
}

export const useSessionInteropStore = create<SessionInteropStore>((set) => ({
  sessions: [],
  connectedSessions: [],
  loading: false,
  error: null,
  heldLocks: new Set(),

  listSessions: async () => {
    set({ loading: true, error: null });
    try {
      const sessions: SessionInfo[] = await invoke('agent_session_list');
      set({ sessions, loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  getConnections: async (conversationId) => {
    set({ loading: true, error: null });
    try {
      const connectedSessions: SessionInfo[] = await invoke('agent_session_get_connections', { conversationId });
      set({ connectedSessions, loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  connect: async (conversationId, targetAddress) => {
    set({ loading: true, error: null });
    try {
      await invoke('agent_session_connect', { conversationId, targetAddress });
      set({ loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  disconnect: async (conversationId, targetAddress) => {
    set({ loading: true, error: null });
    try {
      await invoke('agent_session_disconnect', { conversationId, targetAddress });
      set((s) => {
        const next = new Set(s.heldLocks);
        next.delete(targetAddress);
        return { heldLocks: next, loading: false };
      });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  sendInput: async (conversationId, targetAddress, content) => {
    set({ error: null });
    try {
      await invoke('agent_session_send_input', { conversationId, targetAddress, content });
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  acquireLock: async (conversationId, targetAddress) => {
    set({ error: null });
    try {
      await invoke('agent_session_acquire_lock', { conversationId, targetAddress });
      set((s) => {
        const next = new Set(s.heldLocks);
        next.add(targetAddress);
        return { heldLocks: next };
      });
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  releaseLock: async (conversationId, targetAddress) => {
    set({ error: null });
    try {
      await invoke('agent_session_release_lock', { conversationId, targetAddress });
      set((s) => {
        const next = new Set(s.heldLocks);
        next.delete(targetAddress);
        return { heldLocks: next };
      });
    } catch (e) {
      set({ error: String(e) });
    }
  },
}));
