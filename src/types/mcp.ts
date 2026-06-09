export type McpTransport = 'stdio' | 'http' | 'sse';
export type McpPermissionPolicy = 'ask' | 'allow_safe' | 'allow_all';
export type ToolExecutionStatus = 'pending' | 'running' | 'success' | 'failed' | 'cancelled';

export type McpServerSource = 'builtin' | 'custom';

export type McpServer = {
  id: string;
  name: string;
  transport: McpTransport;
  command?: string | null;
  argsJson?: string | null;
  endpoint?: string | null;
  envJson?: string | null;
  enabled: boolean;
  permissionPolicy: McpPermissionPolicy;
  source: McpServerSource;
  discoverTimeoutSecs?: number | null;
  executeTimeoutSecs?: number | null;
  headersJson?: string | null;
  iconType?: string | null;
  iconValue?: string | null;
};

export type ToolDescriptor = {
  id: string;
  serverId: string;
  name: string;
  description?: string;
  inputSchemaJson?: string;
};

export type ToolExecution = {
  id: string;
  conversationId: string;
  messageId?: string;
  serverId: string;
  toolName: string;
  status: ToolExecutionStatus;
  inputPreview?: string;
  outputPreview?: string;
  errorMessage?: string;
  durationMs?: number;
  createdAt: string;
  approvalStatus?: string;
};

export type CreateMcpServerInput = {
  name: string;
  transport: McpTransport;
  command?: string;
  args?: string[];
  endpoint?: string;
  env?: Record<string, string>;
  enabled?: boolean;
  permissionPolicy?: McpPermissionPolicy;
  discoverTimeoutSecs?: number;
  executeTimeoutSecs?: number;
  headersJson?: string;
  iconType?: string;
  iconValue?: string;
};

export type UpdateMcpServerInput = {
  name?: string;
  transport?: McpTransport;
  command?: string | null;
  args?: string[] | null;
  endpoint?: string | null;
  env?: Record<string, string> | null;
  enabled?: boolean;
  permissionPolicy?: McpPermissionPolicy;
  discoverTimeoutSecs?: number | null;
  executeTimeoutSecs?: number | null;
  headersJson?: string | null;
  iconType?: string | null;
  iconValue?: string | null;
};
