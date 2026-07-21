//! `Session` and `SessionManager` - the unified multi-input / multi-output core.

use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{Notify, RwLock};

use crate::file_writer::SessionFileWriter;
use crate::input::{
    InputHandle, InputHandleInfo, InputRequest, InputSource, InputStatus,
};
use crate::record::SessionRecord;
use crate::runner::{InputRunContext, SharedInputRunner};

/// One session per conversation. Owns the input queue, output subscribers, and
/// a reference to the runner that executes inputs.
pub struct Session {
    pub conversation_id: String,
    queue: Arc<RwLock<VecDeque<Arc<InputRequest>>>>,
    notify: Arc<Notify>,
    /// The input currently being executed (None if idle).
    current: Arc<RwLock<Option<Arc<InputRequest>>>>,
    /// Monotonic seq counter for output records.
    seq_counter: Arc<std::sync::atomic::AtomicU64>,
    /// Output subscribers. Each gets a copy of every emitted record.
    output_subscribers: Arc<RwLock<Vec<tokio::sync::mpsc::UnboundedSender<SessionRecord>>>>,
    /// JSONL file writer. None if file writing failed to initialize.
    file_writer: Arc<RwLock<Option<SessionFileWriter>>>,
    runner: SharedInputRunner,
}

impl Session {
    pub fn new(
        conversation_id: String,
        runner: SharedInputRunner,
        sessions_dir: PathBuf,
    ) -> Arc<Self> {
        let file_writer = match SessionFileWriter::open(&sessions_dir, &conversation_id) {
            Ok(w) => {
                tracing::info!(
                    "Opened session file for {}: {}",
                    conversation_id,
                    w.path().display()
                );
                Some(w)
            }
            Err(e) => {
                tracing::error!(
                    "Failed to open session file for {}: {}",
                    conversation_id,
                    e
                );
                None
            }
        };
        let session = Arc::new(Self {
            conversation_id,
            queue: Arc::new(RwLock::new(VecDeque::new())),
            notify: Arc::new(Notify::new()),
            current: Arc::new(RwLock::new(None)),
            seq_counter: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            output_subscribers: Arc::new(RwLock::new(Vec::new())),
            file_writer: Arc::new(RwLock::new(file_writer)),
            runner,
        });
        session.spawn_processor();
        session
    }

    /// Enqueue a new input. Returns a handle immediately; the input runs later
    /// when the processor reaches it.
    pub async fn enqueue(&self, req: InputRequest) -> InputHandle {
        let handle = InputHandle {
            handle_id: req.handle_id.clone(),
            conversation_id: req.conversation_id.clone(),
        };
        // Initial queue position = current queue length
        let pos = {
            let q = self.queue.read().await;
            q.len()
        };
        *req.status.write().await = InputStatus::Queued(pos);
        let req_arc = Arc::new(req);
        self.queue.write().await.push_back(req_arc);
        self.notify.notify_one();
        handle
    }

    /// Cancel a queued or running input by handle_id. Returns true if found.
    pub async fn cancel_input(&self, handle_id: &str) -> bool {
        // Check current running input first
        {
            let current = self.current.read().await;
            if let Some(req) = current.as_ref() {
                if req.handle_id == handle_id {
                    req.cancel_token.cancel();
                    return true;
                }
            }
        }
        // Check queue
        let mut q = self.queue.write().await;
        if let Some(idx) = q.iter().position(|r| r.handle_id == handle_id) {
            if let Some(req) = q.remove(idx) {
                req.cancel_token.cancel();
                *req.status.write().await = InputStatus::Cancelled;
                signal_done(&req, Err("cancelled".to_string()));
                return true;
            }
        }
        false
    }

    /// Cancel the currently-running input (if any). Returns true if cancelled.
    pub async fn cancel_active(&self) -> bool {
        let current = self.current.read().await;
        if let Some(req) = current.as_ref() {
            req.cancel_token.cancel();
            return true;
        }
        false
    }

    /// List all inputs: queued + currently running. Done/Failed/Cancelled inputs
    /// are not retained (they're dropped after completion).
    pub async fn list_queue(&self) -> Vec<InputHandleInfo> {
        let mut out = Vec::new();
        // Current running
        {
            let current = self.current.read().await;
            if let Some(req) = current.as_ref() {
                out.push(InputHandleInfo {
                    handle_id: req.handle_id.clone(),
                    conversation_id: req.conversation_id.clone(),
                    source: req.source.clone(),
                    status: req.status.read().await.clone(),
                    preview: preview_content(&req.content),
                    enqueued_at: req.enqueued_at,
                });
            }
        }
        // Queued
        let q = self.queue.read().await;
        for (idx, req) in q.iter().enumerate() {
            // Update queue position in status
            *req.status.write().await = InputStatus::Queued(idx);
            out.push(InputHandleInfo {
                handle_id: req.handle_id.clone(),
                conversation_id: req.conversation_id.clone(),
                source: req.source.clone(),
                status: req.status.read().await.clone(),
                preview: preview_content(&req.content),
                enqueued_at: req.enqueued_at,
            });
        }
        out
    }

    /// Subscribe to output records. Returns a receiver that will receive every
    /// record emitted by this session going forward.
    pub async fn subscribe(&self) -> tokio::sync::mpsc::UnboundedReceiver<SessionRecord> {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        self.output_subscribers.write().await.push(tx);
        rx
    }

    /// Emit a record to all three sinks: UI events, file, subscribers.
    /// For Phase 3, file + subscribers are wired; UI events are emitted by
    /// the runner (agent.rs) directly via Tauri's app.emit.
    pub async fn emit_output(&self, mut record: SessionRecord) {
        record.seq = self
            .seq_counter
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        // 1. File (JSONL append)
        let file_writer = self.file_writer.read().await;
        if let Some(ref writer) = *file_writer {
            if let Err(e) = writer.append(&record) {
                tracing::warn!("Failed to append session record: {}", e);
            }
        }
        // 2. Subscribers (broadcast)
        let subs = self.output_subscribers.read().await;
        for tx in subs.iter() {
            let _ = tx.send(record.clone());
        }
    }

    fn spawn_processor(self: &Arc<Self>) {
        let session = Arc::clone(self);
        tokio::spawn(async move {
            session.processor_loop().await;
        });
    }

    async fn processor_loop(self: Arc<Self>) {
        loop {
            // Wait for at least one item to be available
            self.notify.notified().await;
            // Drain the queue
            loop {
                let next = {
                    let mut q = self.queue.write().await;
                    q.pop_front()
                };
                match next {
                    Some(req) => {
                        // Skip if already cancelled
                        if req.cancel_token.is_cancelled() {
                            *req.status.write().await = InputStatus::Cancelled;
                            signal_done(&req, Err("cancelled".to_string()));
                            continue;
                        }
                        // Mark running
                        *req.status.write().await = InputStatus::Running;
                        *self.current.write().await = Some(req.clone());

                        let ctx = InputRunContext::from_request(&req);
                        let runner = Arc::clone(&self.runner);
                        let cancel_token = req.cancel_token.clone();

                        let result = tokio::select! {
                            r = runner.run(ctx) => r,
                            _ = cancel_token.cancelled() => Err("cancelled".to_string()),
                        };

                        // Update status
                        {
                            let mut status = req.status.write().await;
                            *status = match &result {
                                Ok(()) => InputStatus::Done,
                                Err(e) => {
                                    if req.cancel_token.is_cancelled() && e != "cancelled" {
                                        InputStatus::Cancelled
                                    } else if e == "cancelled" {
                                        InputStatus::Cancelled
                                    } else {
                                        InputStatus::Failed(e.clone())
                                    }
                                }
                            };
                        }
                        signal_done(&req, result);
                        *self.current.write().await = None;
                    }
                    None => break, // queue empty, wait for next notify
                }
            }
        }
    }
}

fn signal_done(req: &InputRequest, result: Result<(), String>) {
    if let Ok(mut guard) = req.done_tx.lock() {
        if let Some(tx) = guard.take() {
            let _ = tx.send(result);
        }
    }
}

fn preview_content(s: &str) -> String {
    let s = s.trim();
    if s.len() <= 80 {
        s.to_string()
    } else {
        // UTF-8 safe truncate
        let mut end = 80;
        while end > 0 && !s.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}...", &s[..end])
    }
}

/// Manages all live sessions. Sessions are created on first input and live
/// for the application lifetime (per user decision).
pub struct SessionManager {
    sessions: RwLock<HashMap<String, Arc<Session>>>,
    runner: SharedInputRunner,
    sessions_dir: PathBuf,
}

impl SessionManager {
    pub fn new(runner: SharedInputRunner, sessions_dir: PathBuf) -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
            runner,
            sessions_dir,
        }
    }

    /// Get an existing session or create a new one for the conversation.
    pub async fn get_or_create(&self, conversation_id: &str) -> Arc<Session> {
        {
            let sessions = self.sessions.read().await;
            if let Some(s) = sessions.get(conversation_id) {
                return Arc::clone(s);
            }
        }
        let mut sessions = self.sessions.write().await;
        // Double-check after acquiring write lock
        if let Some(s) = sessions.get(conversation_id) {
            return Arc::clone(s);
        }
        let session = Session::new(
            conversation_id.to_string(),
            Arc::clone(&self.runner),
            self.sessions_dir.clone(),
        );
        sessions.insert(conversation_id.to_string(), Arc::clone(&session));
        session
    }

    /// Get an existing session (None if not yet created).
    pub async fn get(&self, conversation_id: &str) -> Option<Arc<Session>> {
        self.sessions.read().await.get(conversation_id).cloned()
    }

    /// Sessions directory (where `.session` files live).
    pub fn sessions_dir(&self) -> &PathBuf {
        &self.sessions_dir
    }

    /// Enqueue an input. Creates the session if needed.
    pub async fn enqueue(&self, req: InputRequest) -> InputHandle {
        let session = self.get_or_create(&req.conversation_id).await;
        session.enqueue(req).await
    }

    /// Cancel a specific input by handle_id.
    pub async fn cancel_input(&self, conversation_id: &str, handle_id: &str) -> bool {
        if let Some(session) = self.get(conversation_id).await {
            session.cancel_input(handle_id).await
        } else {
            false
        }
    }

    /// Cancel the currently-running input for a conversation.
    pub async fn cancel_active(&self, conversation_id: &str) -> bool {
        if let Some(session) = self.get(conversation_id).await {
            session.cancel_active().await
        } else {
            false
        }
    }

    /// List the queue (queued + running) for a conversation.
    pub async fn list_queue(&self, conversation_id: &str) -> Vec<InputHandleInfo> {
        if let Some(session) = self.get(conversation_id).await {
            session.list_queue().await
        } else {
            Vec::new()
        }
    }

    /// Read session history from the `.session` file.
    pub fn get_history(
        &self,
        conversation_id: &str,
        limit: Option<usize>,
    ) -> std::io::Result<Vec<SessionRecord>> {
        match limit {
            Some(n) => crate::file_writer::SessionFileReader::read_last(
                &self.sessions_dir,
                conversation_id,
                n,
            ),
            None => crate::file_writer::SessionFileReader::read_all(
                &self.sessions_dir,
                conversation_id,
            ),
        }
    }
}

/// Helper to build an InputRequest with a done_tx wired up for callers that
/// want to block on completion.
pub fn make_blocking_request(
    conversation_id: String,
    content: String,
    provider_id: String,
    model_id: String,
    source: InputSource,
) -> (InputRequest, tokio::sync::oneshot::Receiver<Result<(), String>>) {
    let (tx, rx) = tokio::sync::oneshot::channel();
    let req = InputRequest::new(conversation_id, content, provider_id, model_id, source);
    *req.done_tx.lock().unwrap() = Some(tx);
    (req, rx)
}
