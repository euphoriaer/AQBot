use aqbot_core::error::Result;
use aqbot_core::types::*;
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

use crate::openai_compat::{OpenAICompatAdapter, OpenAICompatPolicy};
use crate::reasoning::ReasoningStyle;
use crate::{ProviderAdapter, ProviderRequestContext};

pub struct DeepSeekAdapter {
    inner: OpenAICompatAdapter<DeepSeekPolicy>,
}

const DEEPSEEK_V4_MAX_COMPLETION_TOKENS: u32 = 262_144;

#[derive(Clone, Copy)]
pub(crate) struct DeepSeekPolicy;

pub(crate) fn deepseek_compat_max_completion_tokens_cap(request: &ChatRequest) -> Option<u32> {
    request
        .model
        .to_ascii_lowercase()
        .contains("deepseek-v4")
        .then_some(DEEPSEEK_V4_MAX_COMPLETION_TOKENS)
}

impl OpenAICompatPolicy for DeepSeekPolicy {
    fn default_base_url(&self) -> &'static str {
        "https://api.deepseek.com/v1"
    }

    fn error_label(&self) -> &'static str {
        "DeepSeek API"
    }

    fn default_reasoning_style(&self, _request: &ChatRequest) -> ReasoningStyle {
        ReasoningStyle::OpenAIReasoningEffort
    }

    fn normalize_reasoning_effort(&self, level: &str, effort: String) -> Option<String> {
        if matches!(level, "off" | "none") {
            return None;
        }
        match effort.as_str() {
            "low" | "medium" | "high" | "xhigh" | "max" => Some(effort),
            "minimal" => Some("low".to_string()),
            _ => None,
        }
    }

    fn use_max_completion_tokens(&self, _request: &ChatRequest) -> bool {
        false
    }

    fn max_completion_tokens_cap(&self, request: &ChatRequest) -> Option<u32> {
        deepseek_compat_max_completion_tokens_cap(request)
    }

    fn include_assistant_reasoning_content(
        &self,
        messages: &[ChatMessage],
        tools: &Option<Vec<ChatTool>>,
    ) -> bool {
        tools.as_ref().is_some_and(|tools| !tools.is_empty())
            && messages
                .iter()
                .any(|msg| msg.role == "assistant" && msg.reasoning_content.is_some())
    }
}

impl DeepSeekAdapter {
    pub fn new() -> Self {
        Self {
            inner: OpenAICompatAdapter::new(DeepSeekPolicy),
        }
    }
}

#[async_trait]
impl ProviderAdapter for DeepSeekAdapter {
    async fn chat(
        &self,
        ctx: &ProviderRequestContext,
        request: ChatRequest,
    ) -> Result<ChatResponse> {
        self.inner.chat(ctx, request).await
    }

    fn chat_stream(
        &self,
        ctx: &ProviderRequestContext,
        request: ChatRequest,
    ) -> Pin<Box<dyn Stream<Item = Result<ChatStreamChunk>> + Send>> {
        self.inner.chat_stream(ctx, request)
    }

    async fn list_models(&self, ctx: &ProviderRequestContext) -> Result<Vec<Model>> {
        self.inner.list_models(ctx).await
    }

    async fn embed(
        &self,
        ctx: &ProviderRequestContext,
        request: EmbedRequest,
    ) -> Result<EmbedResponse> {
        self.inner.embed(ctx, request).await
    }

    async fn validate_key(&self, ctx: &ProviderRequestContext) -> Result<bool> {
        self.inner.validate_key(ctx).await
    }
}
