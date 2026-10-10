//! Stateless JSON Streamable HTTP MCP for the existing Hermes MCP client.
//! Credentials terminate here; tools receive business receipts, never bearer tokens.
use app::AppContext;
use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
};
use domain::PmToolCall;
use serde::Deserialize;
use serde_json::{Value, json};
use shared::AppError;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    jsonrpc: String,
    #[serde(default)]
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ToolCall {
    name: String,
    arguments: PmToolCall,
}

fn object(properties: Value, required: &[&str]) -> Value {
    json!({"type":"object","properties":properties,"required":required,"additionalProperties":false})
}

fn tools() -> Value {
    let uuid = json!({"type":"string","format":"uuid"});
    let text = json!({"type":"string"});
    let integer = json!({"type":"integer","minimum":1,"maximum":9007199254740991_i64});
    let optional_integer =
        json!({"type":["integer","null"],"minimum":1,"maximum":9007199254740991_i64});
    let fence = object(
        json!({"assignment_id":uuid,"execution_id":uuid,"agent_id":uuid,"assignment_version":integer}),
        &[
            "assignment_id",
            "execution_id",
            "agent_id",
            "assignment_version",
        ],
    );
    let mut document = json!({"goal":text});
    let lists = [
        "scope",
        "exclusions",
        "scenarios",
        "acceptance_criteria",
        "constraints",
        "dependencies",
        "assumptions",
        "checklist",
        "prerequisites",
    ];
    for key in lists {
        document[key] = json!({"type":"array","items":text});
    }
    let mut required = vec!["goal"];
    required.extend(lists);
    let revision = object(
        json!({"fence":fence,"expected_requirement_revision":optional_integer,
        "document":object(document,&required),"idempotency_key":text}),
        &[
            "fence",
            "expected_requirement_revision",
            "document",
            "idempotency_key",
        ],
    );
    let option = object(
        json!({"id":uuid,"label":text,"consequences":text,"is_custom":{"type":"boolean"}}),
        &["id", "label", "consequences", "is_custom"],
    );
    let question = object(
        json!({"fence":fence,"request_id":uuid,"question_id":uuid,
        "expected_question_version":optional_integer,"requirement_revision":integer,"checkpoint_id":uuid,
        "requirement_reference":{"type":["string","null"]},"text":text,"rationale":text,"required":{"type":"boolean"},
        "mode":{"type":"string","enum":["single","multiple","text"]},"options":{"type":"array","items":option},
        "recommended_option_id":{"type":["string","null"],"format":"uuid"},"idempotency_key":text}),
        &[
            "fence",
            "request_id",
            "question_id",
            "expected_question_version",
            "requirement_revision",
            "checkpoint_id",
            "requirement_reference",
            "text",
            "rationale",
            "required",
            "mode",
            "options",
            "recommended_option_id",
            "idempotency_key",
        ],
    );
    let step = object(
        json!({"step_operation_key":text,"report":{"type":["string","null"],"maxLength":32768}}),
        &["step_operation_key", "report"],
    );
    let definitions = [
        (
            "tracker_context",
            "Read the current machine assignment and requirements revision.",
            object(json!({}), &[]),
        ),
        (
            "tracker_clarifications",
            "Read published questions and saved answers.",
            object(json!({}), &[]),
        ),
        (
            "tracker_requirements",
            "Read immutable requirements revisions.",
            object(json!({}), &[]),
        ),
        (
            "tracker_publish_revision",
            "Publish a requirements revision with the current assignment fence.",
            revision,
        ),
        (
            "tracker_publish_question",
            "Publish a clarification and checkpoint this PM run. Wait for the owner's saved answer.",
            question,
        ),
        (
            "workflow_step",
            "Read current Workflow instructions with report null; submit a report with an original operation key.",
            step,
        ),
    ];
    json!({"tools":definitions.into_iter().map(|(name,description,command)|json!({
        "name":name,"description":description,"inputSchema":object(json!({
            "operation_id":uuid,"session_run_id":uuid,"command":command}),
            &["operation_id","session_run_id","command"])})).collect::<Vec<_>>()})
}

fn reply(id: Value, result: Value) -> Response {
    (
        [(header::CACHE_CONTROL, "no-store")],
        Json(json!({"jsonrpc":"2.0","id":id,"result":result})),
    )
        .into_response()
}

fn error(id: Value, code: i64, message: &'static str) -> Response {
    (
        [(header::CACHE_CONTROL, "no-store")],
        Json(json!({"jsonrpc":"2.0","id":id,"error":{"code":code,"message":message}})),
    )
        .into_response()
}

pub async fn handle(
    State(ctx): State<Arc<AppContext>>,
    Path(agent): Path<Uuid>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Response, AppError> {
    if !ctx.config.pm.dispatch.enabled {
        return Err(AppError::Unavailable("PM tools are disabled".into()));
    }
    if headers.get_all(header::AUTHORIZATION).iter().count() != 1 {
        return Err(AppError::Unauthorized);
    }
    let bearer = headers
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .filter(|h| !h.is_empty() && h.len() <= 512)
        .ok_or(AppError::Unauthorized)?;
    ctx.runtime.authorize_pm_tool(agent, bearer)?;
    if headers.contains_key(header::ORIGIN) {
        return Err(AppError::Forbidden);
    }
    if headers.get_all("mcp-protocol-version").iter().count() > 1
        || headers
            .get("mcp-protocol-version")
            .is_some_and(|v| v != "2025-03-26")
    {
        return Err(AppError::validation("unsupported MCP protocol"));
    }
    let request: Request = match serde_json::from_value(body) {
        Ok(v) => v,
        Err(_) => return Ok(error(Value::Null, -32600, "Invalid request")),
    };
    if request.jsonrpc != "2.0"
        || request
            .id
            .as_ref()
            .is_some_and(|id| !id.is_string() && !id.is_i64() && !id.is_u64())
    {
        return Ok(error(Value::Null, -32600, "Invalid request"));
    }
    let Some(id) = request.id else {
        return if request.method == "notifications/initialized" {
            Ok(StatusCode::ACCEPTED.into_response())
        } else {
            Ok(error(Value::Null, -32600, "Unsupported notification"))
        };
    };
    match request.method.as_str() {
        "initialize" => Ok(reply(
            id,
            json!({"protocolVersion":"2025-03-26","capabilities":{"tools":{"listChanged":false}},"serverInfo":{"name":"fleet-pm","version":"1"}}),
        )),
        "ping" => Ok(reply(id, json!({}))),
        "tools/list" => Ok(reply(id, tools())),
        "tools/call" => {
            let call: ToolCall = match serde_json::from_value(request.params) {
                Ok(v) => v,
                Err(_) => return Ok(error(id, -32602, "Invalid tool arguments")),
            };
            match ctx
                .runtime
                .call_pm_tool(agent, &call.name, call.arguments)
                .await
            {
                Ok(result) => Ok(reply(
                    id,
                    json!({"content":[{"type":"text","text":serde_json::to_string(&result).map_err(|_|AppError::Unavailable("PM receipt unavailable".into()))?}],"isError":false}),
                )),
                // Errors may contain database/provider details. Never publish them to a model.
                Err(_) => Ok(reply(
                    id,
                    json!({"content":[{"type":"text","text":"PM command not confirmed. Read current state; do not change the original operation key or retry with a new run."}],"isError":true}),
                )),
            }
        }
        _ => Ok(error(id, -32601, "Method not found")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tool_inventory_is_bounded_without_answer_confirm_or_credentials() {
        let value = tools();
        let definitions = value["tools"].as_array().unwrap();
        assert_eq!(definitions.len(), 6);
        for tool in definitions {
            assert_eq!(tool["inputSchema"]["additionalProperties"], false);
            assert_eq!(
                tool["inputSchema"]["properties"]["command"]["additionalProperties"],
                false
            );
        }
        let encoded = value.to_string();
        for forbidden in [
            "parent_pat",
            "execution_token",
            "Authorization",
            "tracker_answer",
            "tracker_confirm",
        ] {
            assert!(!encoded.contains(forbidden));
        }
        assert_eq!(
            value["tools"][5]["inputSchema"]["properties"]["command"]["required"],
            json!(["step_operation_key", "report"])
        );
    }
    #[test]
    fn rpc_rejects_unrecognized_fields_and_tool_target_overrides() {
        assert!(
            serde_json::from_value::<Request>(
                json!({"jsonrpc":"2.0","id":1,"method":"ping","authorization":"secret"})
            )
            .is_err()
        );
        assert!(serde_json::from_value::<ToolCall>(json!({"name":"tracker_context","arguments":{"operation_id":Uuid::new_v4(),"session_run_id":Uuid::new_v4(),"command":{},"task_id":Uuid::new_v4()}})).is_err());
    }
}
