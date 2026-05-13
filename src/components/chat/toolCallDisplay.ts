import type { Message } from '@/types';
import { stripAqbotTags } from '@/lib/chatMarkdown';
import { parseSearchContent } from '@/lib/searchUtils';
import { splitLeadingAqbotDisplayContent } from './chatStreaming';

function buildWebSearchDisplayTag(sources: Array<{ title: string; url: string }>): string {
  const resultsJson = JSON.stringify(sources.map((source) => ({
    title: source.title,
    url: source.url,
  })));
  return `<web-search status="done" data-aqbot="1">\n${resultsJson}\n</web-search>\n\n`;
}

export function buildAssistantDisplayContent(message: Message, messages: Message[]): string {
  if (message.role !== 'assistant') {
    return message.content;
  }

  let content = message.content;
  if (message.status === 'error' || content.includes('data-aqbot="1"')) {
    return content;
  }

  const parent = message.parent_message_id
    ? messages.find((item) => item.id === message.parent_message_id && item.role === 'user')
    : undefined;
  if (!parent) {
    return content;
  }

  const parentSearch = parseSearchContent(parent.content);
  if (!parentSearch.hasSearch || parentSearch.sources.length === 0) {
    return content;
  }

  content = `${buildWebSearchDisplayTag(parentSearch.sources)}${content}`;
  return content;
}

export function splitAssistantErrorDisplayContent(content: string): { prefix: string; message: string } {
  const split = splitLeadingAqbotDisplayContent(content);
  const cleanBody = stripAqbotTags(split.body).trim();
  const cleanFullContent = stripAqbotTags(content).trim();

  return {
    prefix: split.prefix,
    message: cleanBody || cleanFullContent || split.body.trim() || content.trim(),
  };
}

export function shouldHideAssistantBubble(message: Message, displayContent: string): boolean {
  if (message.role !== 'assistant') {
    return false;
  }

  if (displayContent.trim()) {
    return false;
  }

  return !message.content.trim() && Boolean(message.tool_calls_json);
}
