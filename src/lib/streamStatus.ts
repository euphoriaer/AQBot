export type StreamActivityPhase = 'waiting_first_packet' | 'streaming' | 'waiting_next_chunk';

export interface StreamActivity {
  startedAt: number;
  firstChunkAt: number | null;
  lastChunkAt: number | null;
  providerId?: string | null;
  modelId?: string | null;
  phase: StreamActivityPhase;
  error?: string | null;
  errorKind?: string | null;
  timeoutSecs?: number | null;
}

export interface StreamingStatusPresentation {
  labelKey: string;
  phase: StreamActivityPhase;
  tone: 'active' | 'warning';
}

export const STREAM_WAITING_PROVIDER_THRESHOLD_MS = 10_000;
export const STREAM_WAITING_NEXT_CHUNK_THRESHOLD_MS = 15_000;
export const STREAM_ERROR_CONTENT_MARKER = '<!-- aqbot-stream-error -->';

export function getStreamingStatusPresentation(input: {
  isStreaming: boolean;
  activity?: StreamActivity | null;
  now: number;
  hasModelText: boolean;
}): StreamingStatusPresentation | null {
  if (!input.isStreaming) {
    return null;
  }

  const activity = input.activity;
  const startedAt = activity?.startedAt ?? input.now;
  const firstChunkAt = activity?.firstChunkAt ?? null;
  const lastChunkAt = activity?.lastChunkAt ?? firstChunkAt;

  if (!firstChunkAt && !input.hasModelText) {
    const elapsed = Math.max(0, input.now - startedAt);
    if (elapsed >= STREAM_WAITING_PROVIDER_THRESHOLD_MS) {
      return {
        labelKey: 'chat.streamingStatus.waitingProvider',
        phase: 'waiting_first_packet',
        tone: 'warning',
      };
    }

    return {
      labelKey: 'chat.streamingStatus.waitingFirstPacket',
      phase: 'waiting_first_packet',
      tone: 'active',
    };
  }

  const idleFor = lastChunkAt ? Math.max(0, input.now - lastChunkAt) : 0;
  if (idleFor >= STREAM_WAITING_NEXT_CHUNK_THRESHOLD_MS) {
    return {
      labelKey: 'chat.streamingStatus.waitingNextChunk',
      phase: 'waiting_next_chunk',
      tone: 'warning',
    };
  }

  return {
    labelKey: 'chat.streamingStatus.generating',
    phase: 'streaming',
    tone: 'active',
  };
}

export function appendStreamErrorToContent(content: string, error: string): string {
  const trimmedError = error.trim();
  if (!content.trim()) {
    return trimmedError;
  }

  if (content.includes(STREAM_ERROR_CONTENT_MARKER)) {
    const [prefix] = content.split(STREAM_ERROR_CONTENT_MARKER);
    return `${prefix.trimEnd()}\n\n${STREAM_ERROR_CONTENT_MARKER}\n${trimmedError}`;
  }

  return `${content.trimEnd()}\n\n${STREAM_ERROR_CONTENT_MARKER}\n${trimmedError}`;
}

export function splitStreamErrorContent(content: string): { prefix: string; error: string } | null {
  const markerIndex = content.indexOf(STREAM_ERROR_CONTENT_MARKER);
  if (markerIndex < 0) {
    return null;
  }

  return {
    prefix: content.slice(0, markerIndex).trimEnd(),
    error: content.slice(markerIndex + STREAM_ERROR_CONTENT_MARKER.length).trim(),
  };
}
