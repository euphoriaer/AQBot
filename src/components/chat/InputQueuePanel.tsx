import { Tag, Tooltip, Button } from 'antd';
import { CloseOutlined, ThunderboltOutlined, ClockCircleOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { useConversationStore, type InputHandleInfo } from '@/stores';

function remoteAddress(source: InputHandleInfo['source']): string | null {
  if (source.kind === 'Remote') return source.detail.from.device_id;
  return null;
}

function statusLabel(status: InputHandleInfo['status']): { text: string; color: string; cancellable: boolean } {
  switch (status.state) {
    case 'Running':
      return { text: 'Running', color: 'processing', cancellable: true };
    case 'Queued':
      return { text: `Queued #${status.detail}`, color: 'warning', cancellable: true };
    case 'Done':
      return { text: 'Done', color: 'success', cancellable: false };
    case 'Cancelled':
      return { text: 'Cancelled', color: 'default', cancellable: false };
    case 'Failed':
      return { text: 'Failed', color: 'error', cancellable: false };
  }
}

export function InputQueuePanel() {
  const { t } = useTranslation();
  const queue = useConversationStore((s) => s.inputQueue);
  const cancelInput = useConversationStore((s) => s.cancelInput);

  if (queue.length === 0) return null;

  return (
    <div
      style={{
        padding: '4px 12px',
        borderBottom: '1px solid var(--ant-color-borderSecondary, rgba(0,0,0,0.06))',
        background: 'var(--ant-color-fillQuaternary, transparent)',
        display: 'flex',
        flexDirection: 'column',
        gap: 4,
        maxHeight: 200,
        overflowY: 'auto',
      }}
      data-testid="input-queue-panel"
    >
      {queue.map((item) => {
        const st = statusLabel(item.status);
        const remoteAddr = remoteAddress(item.source);
        return (
          <div
            key={item.handle_id}
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 8,
              padding: '4px 8px',
              borderRadius: 6,
              background: 'var(--ant-color-bgContainer, #fff)',
              border: '1px solid var(--ant-color-borderSecondary, rgba(0,0,0,0.06))',
              fontSize: 12,
            }}
          >
            {st.text === 'Running' ? (
              <ThunderboltOutlined style={{ color: 'var(--ant-color-primary, #1677ff)' }} />
            ) : st.text.startsWith('Queued') ? (
              <ClockCircleOutlined style={{ color: 'var(--ant-color-warning, #faad14)' }} />
            ) : null}
            <Tag color={st.color} style={{ margin: 0, fontSize: 11, lineHeight: '18px' }}>
              {st.text}
            </Tag>
            {remoteAddr ? (
              <Tooltip title={`Remote: ${remoteAddr}`}>
                <Tag color="purple" style={{ margin: 0, fontSize: 11, lineHeight: '18px' }}>
                  Remote · {remoteAddr.slice(0, 12)}
                </Tag>
              </Tooltip>
            ) : (
              <Tag color="blue" style={{ margin: 0, fontSize: 11, lineHeight: '18px' }}>
                Local
              </Tag>
            )}
            <span
              style={{
                flex: 1,
                overflow: 'hidden',
                textOverflow: 'ellipsis',
                whiteSpace: 'nowrap',
                color: 'var(--ant-color-textSecondary, rgba(0,0,0,0.65))',
              }}
            >
              {item.preview || '(empty)'}
            </span>
            {st.cancellable && (
              <Button
                size="small"
                type="text"
                danger
                icon={<CloseOutlined />}
                onClick={() => cancelInput(item.handle_id)}
                aria-label={t('chat.cancelInput', '取消该输入')}
              />
            )}
          </div>
        );
      })}
    </div>
  );
}
