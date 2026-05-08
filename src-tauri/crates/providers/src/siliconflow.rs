use aqbot_core::error::Result;
use aqbot_core::types::*;
use async_trait::async_trait;
use futures::Stream;
use serde_json::{Map, Value};
use std::pin::Pin;

use crate::openai_compat::{OpenAICompatAdapter, OpenAICompatPolicy};
use crate::reasoning::{ReasoningStyle, ResolvedReasoning};
use crate::{ProviderAdapter, ProviderRequestContext};

pub struct SiliconFlowAdapter {
    inner: OpenAICompatAdapter<SiliconFlowPolicy>,
}

#[derive(Clone, Copy)]
pub(crate) struct SiliconFlowPolicy;

impl OpenAICompatPolicy for SiliconFlowPolicy {
    fn default_base_url(&self) -> &'static str {
        "https://api.siliconflow.cn/v1"
    }

    fn error_label(&self) -> &'static str {
        "SiliconFlow API"
    }

    fn default_reasoning_style(&self, _request: &ChatRequest) -> ReasoningStyle {
        ReasoningStyle::SiliconFlowEnableThinking
    }

    fn normalize_reasoning_effort(&self, _level: &str, _effort: String) -> Option<String> {
        None
    }

    fn use_max_completion_tokens(&self, _request: &ChatRequest) -> bool {
        false
    }

    fn extra_body_fields(&self, reasoning: Option<&ResolvedReasoning>) -> Map<String, Value> {
        let mut extra = Map::new();
        let Some(reasoning) = reasoning else {
            return extra;
        };

        if let Some(enable_thinking) = reasoning.enable_thinking {
            extra.insert("enable_thinking".to_string(), Value::Bool(enable_thinking));
        }
        if let Some(thinking_budget) = reasoning.budget_tokens.filter(|v| *v > 0) {
            extra.insert(
                "thinking_budget".to_string(),
                serde_json::json!(thinking_budget),
            );
        }
        extra
    }
}

impl SiliconFlowAdapter {
    pub fn new() -> Self {
        Self {
            inner: OpenAICompatAdapter::new(SiliconFlowPolicy),
        }
    }
}

#[async_trait]
impl ProviderAdapter for SiliconFlowAdapter {
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
