use super::task_chats::{TrackerGateway, decode_response, load_task_context};
use crate::middleware::{CurrentUser, VerifiedCentralSubject, VerifiedHumanSession};
use app::AppContext;
use axum::{
    Extension, Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
};
use domain::{
    ClarificationAnswerCommand, ClarificationAnswerRequest, ClarificationCommandActor,
    ClarificationDeliveryOutcome, TrackerAnswer,
};
use shared::AppError;
use std::sync::Arc;
use uuid::Uuid;

fn private<T>(value: T) -> (HeaderMap, Json<T>) {
    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::CACHE_CONTROL,
        axum::http::HeaderValue::from_static("no-store"),
    );
    (headers, Json(value))
}

async fn authorize(
    ctx: &Arc<AppContext>,
    user: &CurrentUser,
    subject: &str,
    human: Option<Extension<VerifiedHumanSession>>,
    session: Uuid,
    headers: &HeaderMap,
    require_current_agent: bool,
) -> Result<ClarificationCommandActor, AppError> {
    let Extension(_) = human.ok_or(AppError::Unauthorized)?;
    let local = ctx.repo.get_session(session).await?;
    if local.user_id != user.id {
        return Err(AppError::Forbidden);
    }
    let context = load_task_context(ctx, user, session, headers, require_current_agent).await?;
    let binding = context
        .binding
        .ok_or_else(|| AppError::conflict("task chat is not bound"))?;
    if binding.owner_subject != subject {
        return Err(AppError::Forbidden);
    }
    Ok(ClarificationCommandActor {
        session_id: session,
        user_id: user.id,
        subject: subject.into(),
        binding,
    })
}

async fn deliver(
    ctx: &Arc<AppContext>,
    actor: &ClarificationCommandActor,
    id: Uuid,
    headers: &HeaderMap,
) -> Result<ClarificationAnswerCommand, AppError> {
    let gateway = TrackerGateway::configured(&ctx.config.tracker)?;
    let permit = ctx.repo.claim_clarification_delivery(actor, id).await?;
    let Some(attempt) = permit.attempt_id else {
        if ctx.config.pm.dispatch.enabled {
            ctx.runtime.resume_pm_answer(actor, &permit.command).await?;
        }
        return Ok(permit.command);
    };
    let command = permit.command;
    let response = gateway
        .request(
            actor.binding.task_id,
            &[
                "clarifications".into(),
                command.question_id.to_string(),
                "answers".into(),
            ],
            headers,
            Some(serde_json::to_value(&command.request).map_err(AppError::internal)?),
        )
        .await;
    let outcome = classify_response(&command, &actor.subject, response);
    let delivered = ctx
        .repo
        .finish_clarification_delivery(actor, id, attempt, outcome)
        .await?;
    // Delivery custody is committed first. Retrying this command never repeats the answer POST.
    if ctx.config.pm.dispatch.enabled {
        ctx.runtime.resume_pm_answer(actor, &delivered).await?;
    }
    Ok(delivered)
}

fn classify_response(
    command: &ClarificationAnswerCommand,
    subject: &str,
    response: Result<(StatusCode, serde_json::Value), AppError>,
) -> ClarificationDeliveryOutcome {
    match response {
        Ok((StatusCode::OK, value)) => match decode_response::<TrackerAnswer>(value) {
            Ok(answer) if domain::answer_matches_command(command, &answer, subject) => {
                ClarificationDeliveryOutcome::Delivered(answer)
            }
            _ => ClarificationDeliveryOutcome::Uncertain,
        },
        Ok((status, _)) if matches!(status.as_u16(), 400 | 401 | 403 | 404 | 409 | 422) => {
            ClarificationDeliveryOutcome::Rejected(status.as_u16())
        }
        _ => ClarificationDeliveryOutcome::Uncertain,
    }
}

pub(super) async fn store_and_deliver(
    ctx: &Arc<AppContext>,
    user: &CurrentUser,
    subject: &str,
    human: Option<Extension<VerifiedHumanSession>>,
    (session, question): (Uuid, Uuid),
    headers: &HeaderMap,
    request: ClarificationAnswerRequest,
) -> Result<ClarificationAnswerCommand, AppError> {
    let actor = authorize(ctx, user, subject, human, session, headers, true).await?;
    let command = ctx
        .repo
        .store_clarification_command(&actor, question, request)
        .await?;
    deliver(ctx, &actor, command.id, headers).await
}

#[utoipa::path(post,path="/api/v1/sessions/{session_id}/clarifications/{question_id}/answer-commands",tag="task-chats",params(("session_id"=Uuid,Path),("question_id"=Uuid,Path)),request_body=ClarificationAnswerRequest,responses((status=200,body=ClarificationAnswerCommand)))]
pub async fn store(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<CurrentUser>,
    subject: Option<Extension<VerifiedCentralSubject>>,
    human: Option<Extension<VerifiedHumanSession>>,
    Path((session, question)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Json(request): Json<ClarificationAnswerRequest>,
) -> Result<(HeaderMap, Json<ClarificationAnswerCommand>), AppError> {
    let Extension(subject) = subject.ok_or(AppError::Unauthorized)?;
    let actor = authorize(&ctx, &user, &subject.0, human, session, &headers, true).await?;
    Ok(private(
        ctx.repo
            .store_clarification_command(&actor, question, request)
            .await?,
    ))
}

#[utoipa::path(get,path="/api/v1/sessions/{session_id}/clarification-answer-commands",tag="task-chats",params(("session_id"=Uuid,Path)),responses((status=200,body=Vec<ClarificationAnswerCommand>)))]
pub async fn pending(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<CurrentUser>,
    subject: Option<Extension<VerifiedCentralSubject>>,
    human: Option<Extension<VerifiedHumanSession>>,
    Path(session): Path<Uuid>,
    headers: HeaderMap,
) -> Result<(HeaderMap, Json<Vec<ClarificationAnswerCommand>>), AppError> {
    let Extension(subject) = subject.ok_or(AppError::Unauthorized)?;
    let actor = authorize(&ctx, &user, &subject.0, human, session, &headers, false).await?;
    Ok(private(
        ctx.repo.list_pending_clarification_commands(&actor).await?,
    ))
}

#[utoipa::path(get,path="/api/v1/sessions/{session_id}/clarification-answer-commands/{command_id}",tag="task-chats",params(("session_id"=Uuid,Path),("command_id"=Uuid,Path)),responses((status=200,body=ClarificationAnswerCommand)))]
pub async fn get(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<CurrentUser>,
    subject: Option<Extension<VerifiedCentralSubject>>,
    human: Option<Extension<VerifiedHumanSession>>,
    Path((session, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
) -> Result<(HeaderMap, Json<ClarificationAnswerCommand>), AppError> {
    let Extension(subject) = subject.ok_or(AppError::Unauthorized)?;
    let actor = authorize(&ctx, &user, &subject.0, human, session, &headers, false).await?;
    Ok(private(
        ctx.repo.get_clarification_command(&actor, id).await?,
    ))
}

#[utoipa::path(post,path="/api/v1/sessions/{session_id}/clarification-answer-commands/{command_id}/delivery",tag="task-chats",params(("session_id"=Uuid,Path),("command_id"=Uuid,Path)),responses((status=200,body=ClarificationAnswerCommand)))]
pub async fn delivery(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<CurrentUser>,
    subject: Option<Extension<VerifiedCentralSubject>>,
    human: Option<Extension<VerifiedHumanSession>>,
    Path((session, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
) -> Result<(HeaderMap, Json<ClarificationAnswerCommand>), AppError> {
    let Extension(subject) = subject.ok_or(AppError::Unauthorized)?;
    let actor = authorize(&ctx, &user, &subject.0, human, session, &headers, true).await?;
    Ok(private(deliver(&ctx, &actor, id, &headers).await?))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn command() -> ClarificationAnswerCommand {
        ClarificationAnswerCommand {
            id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            question_id: Uuid::new_v4(),
            request: ClarificationAnswerRequest {
                expected_question_version: 1,
                requirement_revision: 2,
                selected_option_ids: vec![],
                text: Some("Synthetic answer".into()),
                comment: None,
                idempotency_key: "original".into(),
            },
            payload_sha256: "a".repeat(64),
            state: domain::ClarificationDeliveryState::Delivering,
            answer: None,
            rejection_status: None,
            created_at: "2026-10-10T00:00:00Z".into(),
            updated_at: "2026-10-10T00:00:00Z".into(),
        }
    }

    #[test]
    fn exact_answer_is_required_before_delivery_confirmation() {
        let command = command();
        let answer = serde_json::json!({"id":Uuid::new_v4(),"question_id":command.question_id,
            "question_version":1,"requirement_revision":2,"selected_option_ids":[],"text":"Synthetic answer",
            "comment":null,"author_subject":"owner","created_at":"2026-10-10T00:00:00Z"});
        assert!(matches!(
            classify_response(&command, "owner", Ok((StatusCode::OK, answer.clone()))),
            ClarificationDeliveryOutcome::Delivered(_)
        ));
        for (field, value) in [
            ("id", serde_json::json!(Uuid::nil())),
            ("question_id", serde_json::json!(Uuid::new_v4())),
            ("question_version", serde_json::json!(2)),
            ("author_subject", serde_json::json!("foreign")),
            ("text", serde_json::json!("changed")),
        ] {
            let mut invalid = answer.clone();
            invalid[field] = value;
            assert!(matches!(
                classify_response(&command, "owner", Ok((StatusCode::OK, invalid))),
                ClarificationDeliveryOutcome::Uncertain
            ));
        }
    }

    #[test]
    fn transport_errors_timeout_statuses_and_unexpected_success_remain_uncertain() {
        let command = command();
        for status in [
            StatusCode::REQUEST_TIMEOUT,
            StatusCode::TOO_MANY_REQUESTS,
            StatusCode::BAD_GATEWAY,
            StatusCode::CREATED,
            StatusCode::NO_CONTENT,
        ] {
            assert!(matches!(
                classify_response(&command, "owner", Ok((status, serde_json::json!({})))),
                ClarificationDeliveryOutcome::Uncertain
            ));
        }
        assert!(matches!(
            classify_response(
                &command,
                "owner",
                Err(AppError::Unavailable("unknown".into()))
            ),
            ClarificationDeliveryOutcome::Uncertain
        ));
        assert!(matches!(
            classify_response(
                &command,
                "owner",
                Ok((StatusCode::CONFLICT, serde_json::json!({})))
            ),
            ClarificationDeliveryOutcome::Rejected(409)
        ));
    }
}
