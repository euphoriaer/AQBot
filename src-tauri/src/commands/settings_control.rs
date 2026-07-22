use aqbot_core::repo;
use aqbot_core::types::*;
use open_agent_sdk::tools::settings_control::{SettingsControlFn, SettingsControlRequest};
use sea_orm::DatabaseConnection;
use serde_json::{json, Value};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};

use crate::AppState;

/// Payload emitted with the `settings-changed` event after write operations.
#[derive(Clone, serde::Serialize)]
struct SettingsChangedPayload {
    category: String,
    action: String,
}

/// Build the SettingsControlFn callback that the SettingsControl tool invokes.
/// Holds an AppHandle + DatabaseConnection so it can read/write the same
/// data the Settings panel UI operates on, and emit `settings-changed` to
/// force frontend stores to reload.
pub fn build_settings_control_fn(app: AppHandle, db: DatabaseConnection) -> SettingsControlFn {
    Arc::new(move |request: SettingsControlRequest| {
        let app = app.clone();
        let db = db.clone();
        Box::pin(async move {
            let category = request.category.clone();
            let action = request.action.clone();
            let is_write = !is_read_action(&action);

            let result = handle(&app, &db, request).await;

            if result.is_ok() && is_write {
                let _ = app.emit(
                    "settings-changed",
                    SettingsChangedPayload {
                        category: category.clone(),
                        action: action.clone(),
                    },
                );
            }

            result
        })
    })
}

fn is_read_action(action: &str) -> bool {
    matches!(
        action,
        "list" | "get" | "status" | "get_config" | "list_servers" | "get_server"
    )
}

async fn handle(
    app: &AppHandle,
    db: &DatabaseConnection,
    request: SettingsControlRequest,
) -> Result<String, String> {
    let payload = request.payload.unwrap_or(Value::Null);
    match request.category.as_str() {
        "app" => handle_app(app, db, &request.action, payload).await,
        "providers" => handle_providers(app, db, &request.action, payload).await,
        "gateway" => handle_gateway(app, &request.action, payload).await,
        "mcp" => handle_mcp(db, &request.action, payload).await,
        "search" => handle_search(db, &request.action, payload).await,
        "memory" => handle_memory(db, &request.action, payload).await,
        "knowledge" => handle_knowledge(db, &request.action, payload).await,
        "roles" => handle_roles(db, &request.action, payload).await,
        "skills" => handle_skills(db, &request.action, payload).await,
        "drawing" => handle_drawing(db, &request.action, payload).await,
        "sync" | "backup" => Err(format!(
            "category '{}' write actions not yet implemented; reads use list/get",
            request.category
        )),
        other => Err(format!("Unknown category: {}", other)),
    }
}

// ============ app (AppSettings) ============

async fn handle_app(
    app: &AppHandle,
    db: &DatabaseConnection,
    action: &str,
    payload: Value,
) -> Result<String, String> {
    match action {
        "get" => {
            let mut settings = repo::settings::get_settings(db)
                .await
                .map_err(|e| e.to_string())?;
            decode_path_fields(&mut settings);
            Ok(serde_json::to_string(&settings).map_err(|e| e.to_string())?)
        }
        "update" => {
            let patch = payload
                .get("patch")
                .ok_or_else(|| "Missing 'patch' in payload".to_string())?;
            let mut settings = repo::settings::get_settings(db)
                .await
                .map_err(|e| e.to_string())?;
            let mut value = serde_json::to_value(&settings).map_err(|e| e.to_string())?;
            if let (Value::Object(ref mut target), Value::Object(ref src)) = (&mut value, patch) {
                for (k, v) in src {
                    target.insert(k.clone(), v.clone());
                }
            } else {
                return Err("patch must be an object".to_string());
            }
            settings = serde_json::from_value(value).map_err(|e| e.to_string())?;
            encode_path_fields(&mut settings);
            repo::settings::save_settings(db, &settings)
                .await
                .map_err(|e| e.to_string())?;

            // Sync runtime atoms like the save_settings command does.
            let app_state = app.state::<AppState>();
            app_state
                .close_to_tray
                .store(settings.minimize_to_tray, std::sync::atomic::Ordering::Relaxed);
            app_state
                .release_webview_on_tray
                .store(
                    settings.release_webview_on_tray,
                    std::sync::atomic::Ordering::Relaxed,
                );
            let _ = crate::tray::sync_tray_language(app, &settings.language);

            Ok(json!({"ok": true}).to_string())
        }
        _ => Err(format!("Unknown app action: {}", action)),
    }
}

fn decode_path_fields(s: &mut AppSettings) {
    s.backup_dir = aqbot_core::path_vars::decode_path_opt(&s.backup_dir);
    s.gateway_ssl_cert_path =
        aqbot_core::path_vars::decode_path_opt(&s.gateway_ssl_cert_path);
    s.gateway_ssl_key_path = aqbot_core::path_vars::decode_path_opt(&s.gateway_ssl_key_path);
    s.agent_workspace_root = aqbot_core::path_vars::decode_path_opt(&s.agent_workspace_root);
}

fn encode_path_fields(s: &mut AppSettings) {
    s.backup_dir = aqbot_core::path_vars::encode_path_opt(&s.backup_dir);
    s.gateway_ssl_cert_path =
        aqbot_core::path_vars::encode_path_opt(&s.gateway_ssl_cert_path);
    s.gateway_ssl_key_path = aqbot_core::path_vars::encode_path_opt(&s.gateway_ssl_key_path);
    s.agent_workspace_root = aqbot_core::path_vars::encode_path_opt(&s.agent_workspace_root);
}

// ============ providers ============

async fn handle_providers(
    app: &AppHandle,
    db: &DatabaseConnection,
    action: &str,
    payload: Value,
) -> Result<String, String> {
    match action {
        "list" => {
            let providers = repo::provider::list_providers(db)
                .await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_string(&providers).map_err(|e| e.to_string())?)
        }
        "get" => {
            let id = payload
                .get("id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing 'id'".to_string())?;
            let p = repo::provider::get_provider(db, id)
                .await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_string(&p).map_err(|e| e.to_string())?)
        }
        "create" => {
            let input: CreateProviderInput = serde_json::from_value(
                payload
                    .get("provider")
                    .cloned()
                    .ok_or_else(|| "Missing 'provider' in payload".to_string())?,
            )
            .map_err(|e| e.to_string())?;
            let p = repo::provider::create_provider(db, input)
                .await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_string(&p).map_err(|e| e.to_string())?)
        }
        "update" => {
            let id = payload
                .get("id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing 'id'".to_string())?;
            let patch_value = payload
                .get("patch")
                .cloned()
                .ok_or_else(|| "Missing 'patch' in payload".to_string())?;
            let input: UpdateProviderInput =
                serde_json::from_value(patch_value).map_err(|e| e.to_string())?;
            let p = repo::provider::update_provider(db, id, input)
                .await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_string(&p).map_err(|e| e.to_string())?)
        }
        "delete" => {
            let id = payload
                .get("id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing 'id'".to_string())?;
            repo::provider::delete_provider(db, id)
                .await
                .map_err(|e| e.to_string())?;
            Ok(json!({"ok": true, "id": id}).to_string())
        }
        "toggle" => {
            let id = payload
                .get("id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing 'id'".to_string())?;
            let enabled = payload
                .get("enabled")
                .and_then(|v| v.as_bool())
                .ok_or_else(|| "Missing 'enabled'".to_string())?;
            repo::provider::toggle_provider(db, id, enabled)
                .await
                .map_err(|e| e.to_string())?;
            Ok(json!({"ok": true, "id": id, "enabled": enabled}).to_string())
        }
        "add_key" => {
            let provider_id = payload
                .get("id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing 'id' (provider id)".to_string())?;
            let raw_key = payload
                .get("key")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing 'key'".to_string())?;
            let state = app.state::<AppState>();
            let real_id = repo::provider::resolve_provider_id(db, provider_id)
                .await
                .map_err(|e| e.to_string())?;
            let encrypted =
                aqbot_core::crypto::encrypt_key(raw_key, &state.master_key).map_err(|e| e.to_string())?;
            let prefix = if raw_key.len() >= 8 {
                format!("{}...", &raw_key[..8])
            } else {
                raw_key.to_string()
            };
            let k = repo::provider::add_provider_key(db, &real_id, &encrypted, &prefix)
                .await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_string(&k).map_err(|e| e.to_string())?)
        }
        "remove_key" => {
            let key_id = payload
                .get("key_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing 'key_id'".to_string())?;
            repo::provider::delete_provider_key(db, key_id)
                .await
                .map_err(|e| e.to_string())?;
            Ok(json!({"ok": true, "key_id": key_id}).to_string())
        }
        "save_models" => {
            let provider_id = payload
                .get("id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing 'id'".to_string())?;
            let models: Vec<Model> = serde_json::from_value(
                payload
                    .get("models")
                    .cloned()
                    .ok_or_else(|| "Missing 'models' in payload".to_string())?,
            )
            .map_err(|e| e.to_string())?;
            repo::provider::save_models(db, provider_id, &models)
                .await
                .map_err(|e| e.to_string())?;
            Ok(json!({"ok": true, "id": provider_id, "count": models.len()}).to_string())
        }
        _ => Err(format!("Unknown providers action: {}", action)),
    }
}

// ============ gateway ============

async fn handle_gateway(
    app: &AppHandle,
    action: &str,
    payload: Value,
) -> Result<String, String> {
    let state = app.state::<AppState>();
    match action {
        "status" => {
            let status = crate::commands::gateway::get_gateway_status_inner(&state).await?;
            Ok(serde_json::to_string(&status).map_err(|e| e.to_string())?)
        }
        "start" => {
            crate::commands::gateway::start_gateway_inner(&state).await?;
            Ok(json!({"ok": true, "running": true}).to_string())
        }
        "stop" => {
            crate::commands::gateway::stop_gateway_inner(&state).await?;
            Ok(json!({"ok": true, "running": false}).to_string())
        }
        "list_keys" => {
            let keys = repo::gateway::list_gateway_keys(&state.sea_db)
                .await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_string(&keys).map_err(|e| e.to_string())?)
        }
        "create_key" => {
            let name = payload
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing 'name'".to_string())?;
            let k = repo::gateway::create_gateway_key(&state.sea_db, name, Some(&state.master_key))
                .await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_string(&k).map_err(|e| e.to_string())?)
        }
        "delete_key" => {
            let id = payload
                .get("id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing 'id'".to_string())?;
            repo::gateway::delete_gateway_key(&state.sea_db, id)
                .await
                .map_err(|e| e.to_string())?;
            Ok(json!({"ok": true, "id": id}).to_string())
        }
        "toggle_key" => {
            let id = payload
                .get("id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing 'id'".to_string())?;
            let enabled = payload
                .get("enabled")
                .and_then(|v| v.as_bool())
                .ok_or_else(|| "Missing 'enabled'".to_string())?;
            repo::gateway::toggle_gateway_key(&state.sea_db, id, enabled)
                .await
                .map_err(|e| e.to_string())?;
            Ok(json!({"ok": true, "id": id, "enabled": enabled}).to_string())
        }
        _ => Err(format!("Unknown gateway action: {}", action)),
    }
}

// ============ mcp ============

async fn handle_mcp(
    db: &DatabaseConnection,
    action: &str,
    payload: Value,
) -> Result<String, String> {
    match action {
        "list_servers" | "list" => {
            let servers = repo::mcp_server::list_mcp_servers(db)
                .await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_string(&servers).map_err(|e| e.to_string())?)
        }
        "get_server" | "get" => {
            let id = payload
                .get("id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing 'id'".to_string())?;
            let s = repo::mcp_server::get_mcp_server(db, id)
                .await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_string(&s).map_err(|e| e.to_string())?)
        }
        "add_server" | "create" => {
            let input: CreateMcpServerInput = serde_json::from_value(
                payload
                    .get("server")
                    .cloned()
                    .ok_or_else(|| "Missing 'server' in payload".to_string())?,
            )
            .map_err(|e| e.to_string())?;
            let s = repo::mcp_server::create_mcp_server(db, input)
                .await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_string(&s).map_err(|e| e.to_string())?)
        }
        "update_server" | "update" => {
            let id = payload
                .get("id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing 'id'".to_string())?;
            let patch_value = payload
                .get("patch")
                .cloned()
                .ok_or_else(|| "Missing 'patch' in payload".to_string())?;
            let input: UpdateMcpServerInput =
                serde_json::from_value(patch_value).map_err(|e| e.to_string())?;
            let s = repo::mcp_server::update_mcp_server(db, id, input)
                .await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_string(&s).map_err(|e| e.to_string())?)
        }
        "remove_server" | "delete" => {
            let id = payload
                .get("id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing 'id'".to_string())?;
            repo::mcp_server::delete_mcp_server(db, id)
                .await
                .map_err(|e| e.to_string())?;
            Ok(json!({"ok": true, "id": id}).to_string())
        }
        _ => Err(format!("Unknown mcp action: {}", action)),
    }
}

// ============ search ============

async fn handle_search(
    db: &DatabaseConnection,
    action: &str,
    payload: Value,
) -> Result<String, String> {
    match action {
        "list" | "get_config" => {
            let providers = repo::search_provider::list_search_providers(db)
                .await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_string(&providers).map_err(|e| e.to_string())?)
        }
        "get" => {
            let id = payload
                .get("id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing 'id'".to_string())?;
            let p = repo::search_provider::get_search_provider(db, id)
                .await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_string(&p).map_err(|e| e.to_string())?)
        }
        "create" => {
            let input: CreateSearchProviderInput = serde_json::from_value(
                payload
                    .get("provider")
                    .cloned()
                    .ok_or_else(|| "Missing 'provider' in payload".to_string())?,
            )
            .map_err(|e| e.to_string())?;
            let p = repo::search_provider::create_search_provider(db, input)
                .await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_string(&p).map_err(|e| e.to_string())?)
        }
        "update" => {
            let id = payload
                .get("id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing 'id'".to_string())?;
            let patch_value = payload
                .get("patch")
                .cloned()
                .ok_or_else(|| "Missing 'patch' in payload".to_string())?;
            let input: CreateSearchProviderInput =
                serde_json::from_value(patch_value).map_err(|e| e.to_string())?;
            let p = repo::search_provider::update_search_provider(db, id, input)
                .await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_string(&p).map_err(|e| e.to_string())?)
        }
        "delete" => {
            let id = payload
                .get("id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing 'id'".to_string())?;
            repo::search_provider::delete_search_provider(db, id)
                .await
                .map_err(|e| e.to_string())?;
            Ok(json!({"ok": true, "id": id}).to_string())
        }
        _ => Err(format!("Unknown search action: {}", action)),
    }
}

// ============ memory ============

async fn handle_memory(
    db: &DatabaseConnection,
    action: &str,
    payload: Value,
) -> Result<String, String> {
    match action {
        "list" | "list_namespaces" => {
            let ns = repo::memory::list_namespaces(db)
                .await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_string(&ns).map_err(|e| e.to_string())?)
        }
        "get" | "get_namespace" => {
            let id = payload
                .get("id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing 'id'".to_string())?;
            let n = repo::memory::get_namespace(db, id)
                .await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_string(&n).map_err(|e| e.to_string())?)
        }
        "create" | "create_namespace" => {
            let input: CreateMemoryNamespaceInput = serde_json::from_value(
                payload
                    .get("namespace")
                    .cloned()
                    .ok_or_else(|| "Missing 'namespace' in payload".to_string())?,
            )
            .map_err(|e| e.to_string())?;
            let n = repo::memory::create_namespace(db, input)
                .await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_string(&n).map_err(|e| e.to_string())?)
        }
        "delete" | "delete_namespace" => {
            let id = payload
                .get("id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing 'id'".to_string())?;
            repo::memory::delete_namespace(db, id)
                .await
                .map_err(|e| e.to_string())?;
            Ok(json!({"ok": true, "id": id}).to_string())
        }
        _ => Err(format!("Unknown memory action: {}", action)),
    }
}

// ============ knowledge ============

async fn handle_knowledge(
    db: &DatabaseConnection,
    action: &str,
    payload: Value,
) -> Result<String, String> {
    match action {
        "list" => {
            let kbs = repo::knowledge::list_knowledge_bases(db)
                .await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_string(&kbs).map_err(|e| e.to_string())?)
        }
        "get" => {
            let id = payload
                .get("id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing 'id'".to_string())?;
            let kb = repo::knowledge::get_knowledge_base(db, id)
                .await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_string(&kb).map_err(|e| e.to_string())?)
        }
        "create" => {
            let input: CreateKnowledgeBaseInput = serde_json::from_value(
                payload
                    .get("kb")
                    .cloned()
                    .ok_or_else(|| "Missing 'kb' in payload".to_string())?,
            )
            .map_err(|e| e.to_string())?;
            let kb = repo::knowledge::create_knowledge_base(db, input)
                .await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_string(&kb).map_err(|e| e.to_string())?)
        }
        "update" => {
            let id = payload
                .get("id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing 'id'".to_string())?;
            let patch_value = payload
                .get("patch")
                .cloned()
                .ok_or_else(|| "Missing 'patch' in payload".to_string())?;
            let input: UpdateKnowledgeBaseInput =
                serde_json::from_value(patch_value).map_err(|e| e.to_string())?;
            let kb = repo::knowledge::update_knowledge_base(db, id, input)
                .await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_string(&kb).map_err(|e| e.to_string())?)
        }
        "delete" => {
            let id = payload
                .get("id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing 'id'".to_string())?;
            repo::knowledge::delete_knowledge_base(db, id)
                .await
                .map_err(|e| e.to_string())?;
            Ok(json!({"ok": true, "id": id}).to_string())
        }
        _ => Err(format!("Unknown knowledge action: {}", action)),
    }
}

// ============ roles ============

async fn handle_roles(
    db: &DatabaseConnection,
    action: &str,
    payload: Value,
) -> Result<String, String> {
    match action {
        "list" => {
            let roles = repo::role::list_roles(db)
                .await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_string(&roles).map_err(|e| e.to_string())?)
        }
        "get" => {
            let id = payload
                .get("id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing 'id'".to_string())?;
            let r = repo::role::get_role(db, id)
                .await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_string(&r).map_err(|e| e.to_string())?)
        }
        "create" => {
            let input: CreateRoleInput = serde_json::from_value(
                payload
                    .get("role")
                    .cloned()
                    .ok_or_else(|| "Missing 'role' in payload".to_string())?,
            )
            .map_err(|e| e.to_string())?;
            let r = repo::role::create_role(db, input)
                .await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_string(&r).map_err(|e| e.to_string())?)
        }
        "update" => {
            let id = payload
                .get("id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing 'id'".to_string())?;
            let patch_value = payload
                .get("patch")
                .cloned()
                .ok_or_else(|| "Missing 'patch' in payload".to_string())?;
            let input: UpdateRoleInput =
                serde_json::from_value(patch_value).map_err(|e| e.to_string())?;
            let r = repo::role::update_role(db, id, input)
                .await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_string(&r).map_err(|e| e.to_string())?)
        }
        "delete" => {
            let id = payload
                .get("id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing 'id'".to_string())?;
            repo::role::delete_role(db, id)
                .await
                .map_err(|e| e.to_string())?;
            Ok(json!({"ok": true, "id": id}).to_string())
        }
        _ => Err(format!("Unknown roles action: {}", action)),
    }
}

// ============ skills ============

async fn handle_skills(
    db: &DatabaseConnection,
    action: &str,
    payload: Value,
) -> Result<String, String> {
    match action {
        "list" => {
            let disabled = repo::skill::get_disabled_skills(db)
                .await
                .map_err(|e| e.to_string())?;
            Ok(json!({"disabled": disabled}).to_string())
        }
        "toggle" => {
            let name = payload
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing 'name'".to_string())?;
            let enabled = payload
                .get("enabled")
                .and_then(|v| v.as_bool())
                .ok_or_else(|| "Missing 'enabled'".to_string())?;
            repo::skill::set_skill_enabled(db, name, enabled)
                .await
                .map_err(|e| e.to_string())?;
            Ok(json!({"ok": true, "name": name, "enabled": enabled}).to_string())
        }
        _ => Err(format!("Unknown skills action: {}", action)),
    }
}

// ============ drawing ============

async fn handle_drawing(
    db: &DatabaseConnection,
    action: &str,
    payload: Value,
) -> Result<String, String> {
    match action {
        "list_generations" | "list" => {
            let limit = payload
                .get("limit")
                .and_then(|v| v.as_u64())
                .unwrap_or(50);
            let gens = repo::drawing::list_generations(db, limit, None)
                .await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_string(&gens).map_err(|e| e.to_string())?)
        }
        "delete_generation" | "delete" => {
            let id = payload
                .get("id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing 'id'".to_string())?;
            repo::drawing::delete_generation(db, id)
                .await
                .map_err(|e| e.to_string())?;
            Ok(json!({"ok": true, "id": id}).to_string())
        }
        _ => Err(format!("Unknown drawing action: {}", action)),
    }
}
