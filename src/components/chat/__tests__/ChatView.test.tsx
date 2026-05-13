import { describe, expect, it } from 'vitest';

import type { Message } from '@/types';
import {
  buildAssistantDisplayContent,
  splitAssistantErrorDisplayContent,
} from '../toolCallDisplay';

function makeMessage(overrides: Partial<Message>): Message {
  return {
    id: 'msg-1',
    conversation_id: 'conv-1',
    role: 'assistant',
    content: '',
    provider_id: null,
    model_id: null,
    token_count: null,
    attachments: [],
    thinking: null,
    tool_calls_json: null,
    tool_call_id: null,
    created_at: 1,
    parent_message_id: null,
    version_index: 0,
    is_active: true,
    status: 'complete',
    ...overrides,
  };
}

describe('ChatView assistant display policy', () => {
  it('injects a web-search display card for normal assistant replies with searched parent messages', () => {
    const user = makeMessage({
      id: 'user-1',
      role: 'user',
      content: '<!-- search:{"sources":[{"title":"A","url":"https://example.com"}]} -->\nsearch\n---\n\n用户问题',
    });
    const assistant = makeMessage({
      id: 'assistant-1',
      parent_message_id: 'user-1',
      content: 'final answer',
    });

    const content = buildAssistantDisplayContent(assistant, [user, assistant]);

    expect(content).toContain('<web-search status="done" data-aqbot="1">');
    expect(content).toContain('final answer');
  });

  it('does not inject a web-search display card into assistant error text', () => {
    const user = makeMessage({
      id: 'user-1',
      role: 'user',
      content: '<!-- search:{"sources":[{"title":"A","url":"https://example.com"}]} -->\nsearch\n---\n\n用户问题',
    });
    const assistant = makeMessage({
      id: 'assistant-1',
      parent_message_id: 'user-1',
      content: '模型首包超时',
      status: 'error',
    });

    const content = buildAssistantDisplayContent(assistant, [user, assistant]);

    expect(content).toBe('模型首包超时');
    expect(content).not.toContain('<web-search');
  });

  it('splits display-only prefixes away from assistant error messages', () => {
    const display = splitAssistantErrorDisplayContent(
      '<web-search status="done" data-aqbot="1">[{"title":"A"}]</web-search>\n\n模型首包超时',
    );

    expect(display.prefix).toContain('<web-search status="done" data-aqbot="1">');
    expect(display.message).toBe('模型首包超时');
    expect(display.message).not.toContain('[{"title":"A"}]');
  });
});
