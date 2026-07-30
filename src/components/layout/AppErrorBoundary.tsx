import React from 'react';

interface AppErrorBoundaryProps {
  children: React.ReactNode;
}

interface AppErrorBoundaryState {
  hasError: boolean;
  error: Error | null;
}

export class AppErrorBoundary extends React.Component<
  AppErrorBoundaryProps,
  AppErrorBoundaryState
> {
  state: AppErrorBoundaryState = { hasError: false, error: null };

  static getDerivedStateFromError(error: Error): AppErrorBoundaryState {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, info: React.ErrorInfo) {
    console.error('[AppErrorBoundary] Render crash caught:', error, info.componentStack);
  }

  handleReload = () => {
    // Reset error state first, then reload
    this.setState({ hasError: false, error: null });
    window.location.reload();
  };

  render() {
    if (!this.state.hasError) return this.props.children;

    const { error } = this.state;

    return (
      <div
        style={{
          minHeight: '100vh',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          background: '#111827',
          color: '#f9fafb',
          fontFamily: 'system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif',
          padding: 32,
          boxSizing: 'border-box',
        }}
      >
        <div style={{ maxWidth: 520, textAlign: 'center' }}>
          <h1 style={{ fontSize: 22, fontWeight: 600, margin: '0 0 8px' }}>
            AQBot 遇到了意外错误
          </h1>
          <p style={{ color: '#9ca3af', margin: '0 0 24px', lineHeight: 1.6 }}>
            应用渲染过程中出现了异常。请尝试重新加载，如果问题持续请重启应用。
          </p>
          <button
            onClick={this.handleReload}
            style={{
              padding: '10px 28px',
              fontSize: 14,
              fontWeight: 500,
              color: '#fff',
              background: '#3b82f6',
              border: 'none',
              borderRadius: 8,
              cursor: 'pointer',
              marginBottom: 24,
            }}
          >
            重新加载
          </button>
          {error && (
            <details style={{ textAlign: 'left' }}>
              <summary style={{ cursor: 'pointer', color: '#6b7280', fontSize: 13 }}>
                错误详情
              </summary>
              <pre
                style={{
                  marginTop: 8,
                  padding: 12,
                  borderRadius: 6,
                  background: '#1f2937',
                  color: '#fca5a5',
                  fontSize: 12,
                  whiteSpace: 'pre-wrap',
                  wordBreak: 'break-word',
                  maxHeight: 300,
                  overflow: 'auto',
                }}
              >
                {error.name}: {error.message}
                {'\n\n'}
                {error.stack}
              </pre>
            </details>
          )}
        </div>
      </div>
    );
  }
}
