import { useState, useCallback, useEffect } from 'react';
import { Typography, Button, App, Spin, theme, Input, Tooltip, Switch } from 'antd';
import { Copy, Check, Link2, Radio, Users, Edit3, Save, X, Unplug, Send, RefreshCw } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { invoke, isTauri } from '@/lib/invoke';
import { useGatewayStore } from '@/stores/gatewayStore';
import { useSettingsStore } from '@/stores/settingsStore';
import { useSessionInteropStore } from '@/stores/sessionInteropStore';

interface SessionInteropPopoverContentProps {
  activeConversationId: string | null;
}

function clientHost(listenAddress: string): string {
  const trimmed = listenAddress.trim();
  if (trimmed === '0.0.0.0') return '127.0.0.1';
  if (trimmed === '::' || trimmed === '[::]') return 'localhost';
  return trimmed;
}

export function SessionInteropPopoverContent({ activeConversationId }: SessionInteropPopoverContentProps) {
  const { t } = useTranslation();
  const { token: themeToken } = theme.useToken();
  const { message } = App.useApp();
  const { status, startGateway, stopGateway, ensureStatusLoaded, fetchStatus } = useGatewayStore();
  const {
    connectedSessions,
    loading,
    getConnections,
    disconnect,
    sendInput,
    acquireLock,
    releaseLock,
    heldLocks,
  } = useSessionInteropStore();
  const [copied, setCopied] = useState(false);
  const [editingField, setEditingField] = useState<'address' | 'port' | null>(null);
  const [editValue, setEditValue] = useState('');
  const [saving, setSaving] = useState(false);
  const [togglingGateway, setTogglingGateway] = useState(false);
  const [refreshingIp, setRefreshingIp] = useState(false);
  const [deviceId, setDeviceId] = useState<string>('');
  const [connectingIds, setConnectingIds] = useState<Set<string>>(new Set());
  const [sendInputTarget, setSendInputTarget] = useState<string | null>(null);
  const [sendInputText, setSendInputText] = useState('');

  // Fetch gateway status and device ID on mount
  useEffect(() => {
    if (isTauri()) {
      ensureStatusLoaded({ maxAgeMs: 5_000 });
      invoke<string>('get_device_id').then(setDeviceId).catch(() => {});
    }
  }, [ensureStatusLoaded]);

  const isRunning = status.is_running;
  const activeColor = themeToken.colorSuccess;
  const host = clientHost(status.listen_address);
  const port = status.port;

  // Gateway reachability address (for external clients to connect to us)
  const gatewayAddress = host && port && activeConversationId
    ? `${host}:${port}/${activeConversationId}`
    : '';

  // Internal session address (used in session registry)
  const internalAddress = deviceId && activeConversationId
    ? `${deviceId}/${activeConversationId}`
    : '';

  // Load connected sessions when popover opens
  const loadConnected = useCallback(async () => {
    if (!isTauri() || !activeConversationId) return;
    await getConnections(activeConversationId);
  }, [activeConversationId, getConnections]);

  useEffect(() => {
    loadConnected();
    const interval = setInterval(loadConnected, 5000);
    return () => clearInterval(interval);
  }, [loadConnected]);

  const handleCopyAddress = useCallback(async () => {
    if (!gatewayAddress) return;
    try {
      await navigator.clipboard.writeText(gatewayAddress);
      setCopied(true);
      message.success(t('chat.sessionInterop.copied'));
      setTimeout(() => setCopied(false), 2000);
    } catch {
      message.error('Copy failed');
    }
  }, [gatewayAddress, message, t]);

  const handleDisconnect = useCallback(async (targetAddress: string) => {
    if (!activeConversationId) return;
    setConnectingIds((prev) => new Set(prev).add(targetAddress));
    try {
      await disconnect(activeConversationId, targetAddress);
      await getConnections(activeConversationId);
      message.success(t('chat.sessionInterop.disconnected'));
    } catch (e) {
      message.error(String(e));
    } finally {
      setConnectingIds((prev) => {
        const next = new Set(prev);
        next.delete(targetAddress);
        return next;
      });
    }
  }, [activeConversationId, disconnect, getConnections, message, t]);

  const handleSendInput = useCallback(async (targetAddress: string) => {
    if (!activeConversationId || !sendInputText.trim()) return;
    try {
      await sendInput(activeConversationId, targetAddress, sendInputText.trim());
      setSendInputText('');
      setSendInputTarget(null);
      message.success(t('chat.sessionInterop.inputSent'));
    } catch (e) {
      message.error(String(e));
    }
  }, [activeConversationId, sendInput, sendInputText, message, t]);

  const handleToggleLock = useCallback(async (targetAddress: string) => {
    if (!activeConversationId) return;
    try {
      if (heldLocks.has(targetAddress)) {
        await releaseLock(activeConversationId, targetAddress);
      } else {
        await acquireLock(activeConversationId, targetAddress);
      }
    } catch (e) {
      message.error(String(e));
    }
  }, [activeConversationId, heldLocks, acquireLock, releaseLock, message]);

  const handleRefreshIp = useCallback(async () => {
    if (!isTauri()) return;
    setRefreshingIp(true);
    try {
      const result = await invoke<{ ip: string; port: number }>('get_local_ip');
      if (result.ip && result.ip !== '0.0.0.0') {
        await useSettingsStore.getState().saveSettings({ gateway_listen_address: result.ip });
      }
      if (result.port > 0) {
        await useSettingsStore.getState().saveSettings({ gateway_port: result.port });
      }
      await fetchStatus();
      message.success(t('chat.sessionInterop.ipRefreshed'));
    } catch (e) {
      message.error(String(e));
    } finally {
      setRefreshingIp(false);
    }
  }, [fetchStatus, message, t]);

  const startEdit = useCallback((field: 'address' | 'port') => {
    if (field === 'address') {
      setEditValue(status.listen_address);
    } else {
      setEditValue(String(status.port));
    }
    setEditingField(field);
  }, [status.listen_address, status.port]);

  const cancelEdit = useCallback(() => {
    setEditingField(null);
    setEditValue('');
  }, []);

  const saveEdit = useCallback(async () => {
    const trimmed = editValue.trim();
    if (!trimmed) {
      cancelEdit();
      return;
    }
    setSaving(true);
    try {
      if (editingField === 'address') {
        await useSettingsStore.getState().saveSettings({ gateway_listen_address: trimmed });
      } else if (editingField === 'port') {
        const portNum = parseInt(editValue, 10);
        if (isNaN(portNum) || portNum < 1 || portNum > 65535) {
          message.error(t('chat.sessionInterop.invalidPort'));
          setSaving(false);
          return;
        }
        await useSettingsStore.getState().saveSettings({ gateway_port: portNum });
      }
      if (isTauri()) {
        await fetchStatus();
      }
      setEditingField(null);
      setEditValue('');
      message.success(t('chat.sessionInterop.saved'));
    } catch (e) {
      message.error(String(e));
    } finally {
      setSaving(false);
    }
  }, [editValue, editingField, message, t, cancelEdit, fetchStatus]);

  const handleToggleGateway = useCallback(async () => {
    setTogglingGateway(true);
    try {
      if (isRunning) {
        await stopGateway();
      } else {
        await startGateway();
      }
    } catch (e: any) {
      message.error(String(e));
    } finally {
      setTogglingGateway(false);
    }
  }, [isRunning, startGateway, stopGateway, message]);

  const labelStyle = {
    fontSize: 11,
    color: themeToken.colorTextSecondary,
    marginBottom: 6,
    display: 'flex',
    alignItems: 'center',
    gap: 4,
  } as const;

  const rowStyle = {
    display: 'flex',
    alignItems: 'center',
    gap: 6,
    background: themeToken.colorFillAlter,
    borderRadius: 6,
    padding: '6px 10px',
  } as const;

  return (
    <div style={{ minWidth: 280, maxWidth: 400 }}>
      {/* Title */}
      <div style={{
        fontSize: 13,
        fontWeight: 600,
        marginBottom: 12,
        display: 'flex',
        alignItems: 'center',
        gap: 6,
      }}>
        <Link2 size={14} color={isRunning ? activeColor : undefined} />
        {t('chat.sessionInterop.title')}
      </div>

      {/* Gateway Toggle */}
      <div style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        marginBottom: 14,
        padding: '6px 10px',
        background: themeToken.colorFillAlter,
        borderRadius: 6,
      }}>
        <span style={{ fontSize: 12, color: themeToken.colorTextSecondary }}>
          {isRunning ? t('gateway.running') : t('gateway.stopped')}
        </span>
        <Switch
          size="small"
          checked={isRunning}
          loading={togglingGateway}
          onChange={handleToggleGateway}
          checkedChildren="ON"
          unCheckedChildren="OFF"
        />
      </div>

      {/* Listen Address Section */}
      <div style={{ marginBottom: 16 }}>
        <div style={labelStyle}>
          <Radio size={11} color={isRunning ? activeColor : undefined} />
          {t('chat.sessionInterop.listenAddress')}
        </div>

        {/* Listen IP */}
        <div style={{ ...rowStyle, marginBottom: 6 }}>
          {editingField === 'address' ? (
            <>
              <Input
                size="small"
                value={editValue}
                onChange={(e) => setEditValue(e.target.value)}
                style={{ flex: 1, fontSize: 12, fontFamily: 'monospace' }}
                placeholder="0.0.0.0"
              />
              <Button type="text" size="small" icon={<Save size={12} />} onClick={saveEdit} loading={saving} />
              <Button type="text" size="small" icon={<X size={12} />} onClick={cancelEdit} />
            </>
          ) : (
            <>
              <Typography.Text
                style={{
                  flex: 1,
                  fontSize: 12,
                  fontFamily: 'ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, monospace',
                  color: themeToken.colorText,
                }}
              >
                {status.listen_address}
              </Typography.Text>
              {!isRunning && (
                <Tooltip title={t('chat.sessionInterop.edit')}>
                  <Button type="text" size="small" icon={<Edit3 size={12} />} onClick={() => startEdit('address')} />
                </Tooltip>
              )}
            </>
          )}
        </div>

        {/* Listen Port */}
        <div style={rowStyle}>
          {editingField === 'port' ? (
            <>
              <Input
                size="small"
                value={editValue}
                onChange={(e) => setEditValue(e.target.value)}
                style={{ flex: 1, fontSize: 12, fontFamily: 'monospace' }}
                placeholder="8080"
              />
              <Button type="text" size="small" icon={<Save size={12} />} onClick={saveEdit} loading={saving} />
              <Button type="text" size="small" icon={<X size={12} />} onClick={cancelEdit} />
            </>
          ) : (
            <>
              <Typography.Text
                style={{
                  flex: 1,
                  fontSize: 12,
                  fontFamily: 'ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, monospace',
                  color: themeToken.colorText,
                }}
              >
                {String(status.port)}
              </Typography.Text>
              {!isRunning && (
                <Tooltip title={t('chat.sessionInterop.edit')}>
                  <Button type="text" size="small" icon={<Edit3 size={12} />} onClick={() => startEdit('port')} />
                </Tooltip>
              )}
            </>
          )}
        </div>

        {/* Auto-detect IP + Port */}
        {!isRunning && (
          <div style={{ marginTop: 8 }}>
            <Button
              size="small"
              icon={<RefreshCw size={12} />}
              loading={refreshingIp}
              onClick={handleRefreshIp}
              style={{ fontSize: 11, width: '100%' }}
            >
              {t('chat.sessionInterop.refreshIp')}
            </Button>
          </div>
        )}
      </div>

      {/* Session Address */}
      <div style={{ marginBottom: 16 }}>
        <div style={labelStyle}>
          <Radio size={11} />
          {t('chat.sessionInterop.sessionAddress')}
        </div>
        <div style={{ ...rowStyle, marginBottom: 6 }}>
          <Typography.Text
            ellipsis
            style={{
              flex: 1,
              fontSize: 12,
              fontFamily: 'ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, monospace',
              color: themeToken.colorText,
            }}
          >
            {gatewayAddress || '...'}
          </Typography.Text>
          <Button
            type="text"
            size="small"
            icon={copied ? <Check size={12} /> : <Copy size={12} />}
            onClick={handleCopyAddress}
            disabled={!gatewayAddress}
            style={{ color: copied ? themeToken.colorSuccess : undefined }}
          />
        </div>
        {deviceId && (
          <div style={{ fontSize: 10, color: themeToken.colorTextDescription, paddingLeft: 4 }}>
            ID: {internalAddress || '...'}
          </div>
        )}
      </div>

      {/* Connected Sessions */}
      <div>
        <div style={labelStyle}>
          <Users size={11} />
          {t('chat.sessionInterop.connectedSessions')}
          {connectedSessions.length > 0 && (
            <span style={{ fontSize: 10, color: themeToken.colorTextQuaternary }}>
              ({connectedSessions.length})
            </span>
          )}
        </div>

        {loading ? (
          <div style={{ display: 'flex', justifyContent: 'center', padding: 12 }}>
            <Spin size="small" />
          </div>
        ) : connectedSessions.length === 0 ? (
          <div style={{
            fontSize: 12,
            color: themeToken.colorTextDescription,
            padding: '8px 0',
            textAlign: 'center',
          }}>
            {t('chat.sessionInterop.noConnections')}
          </div>
        ) : (
          <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
            {connectedSessions.map((s) => (
              <div
                key={s.address}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: 6,
                  padding: '4px 8px',
                  borderRadius: 4,
                  background: themeToken.colorFillAlter,
                }}
              >
                <span style={{
                  width: 6,
                  height: 6,
                  borderRadius: '50%',
                  background: s.lock_held ? themeToken.colorWarning
                    : s.is_active ? themeToken.colorSuccess
                    : themeToken.colorTextDisabled,
                  flexShrink: 0,
                }} />
                <div style={{ flex: 1, minWidth: 0 }}>
                  <Typography.Text
                    ellipsis
                    style={{ fontSize: 12, color: themeToken.colorText }}
                  >
                    {s.title || s.conversation_id}
                  </Typography.Text>
                  <Typography.Text
                    ellipsis
                    style={{
                      display: 'block',
                      fontSize: 10,
                      color: themeToken.colorTextDescription,
                      fontFamily: 'ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, monospace',
                    }}
                  >
                    {s.address}
                  </Typography.Text>
                </div>
                <div style={{ display: 'flex', gap: 2, flexShrink: 0 }}>
                  {/* Toggle input lock */}
                  <Tooltip title={heldLocks.has(s.address) ? t('chat.sessionInterop.releaseLock') : t('chat.sessionInterop.acquireLock')}>
                    <Button
                      type="text"
                      size="small"
                      onClick={() => handleToggleLock(s.address)}
                      style={{
                        color: heldLocks.has(s.address) ? themeToken.colorWarning : themeToken.colorTextQuaternary,
                        fontSize: 10,
                      }}
                    >
                      {heldLocks.has(s.address) ? '🔒' : '🔓'}
                    </Button>
                  </Tooltip>
                  {/* Send input */}
                  {sendInputTarget === s.address ? (
                    <div style={{ display: 'flex', gap: 2, alignItems: 'center' }}>
                      <Input
                        size="small"
                        value={sendInputText}
                        onChange={(e) => setSendInputText(e.target.value)}
                        onPressEnter={() => handleSendInput(s.address)}
                        style={{ width: 80, fontSize: 11 }}
                        placeholder={t('chat.sessionInterop.typeMessage')}
                      />
                      <Button
                        type="text"
                        size="small"
                        icon={<Send size={10} />}
                        onClick={() => handleSendInput(s.address)}
                      />
                    </div>
                  ) : (
                    <Tooltip title={t('chat.sessionInterop.sendInput')}>
                      <Button
                        type="text"
                        size="small"
                        icon={<Send size={10} />}
                        onClick={() => setSendInputTarget(s.address)}
                      />
                    </Tooltip>
                  )}
                  {/* Disconnect */}
                  <Tooltip title={t('chat.sessionInterop.disconnect')}>
                    <Button
                      type="text"
                      size="small"
                      icon={<Unplug size={10} />}
                      loading={connectingIds.has(s.address)}
                      onClick={() => handleDisconnect(s.address)}
                      style={{ color: themeToken.colorError }}
                    />
                  </Tooltip>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
