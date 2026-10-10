use super::*;
use domain::PmRunRecord;

pub(super) fn verify_stream_pin(
    record: &PmRunRecord,
    stream_run: &SessionAgentRun,
    native_run: &str,
) -> Result<(), AppError> {
    if record.reservation.session_run_id != stream_run.id
        || record.reservation.session_id != stream_run.session_id
        || record.reservation.identity.agent_id()? != stream_run.agent_id
        || record.hermes_run_ref.as_deref() != Some(native_run)
        || stream_run.runtime_run_id.as_deref() != Some(native_run)
        || record.hermes_session_ref.is_none()
        || record.hermes_session_ref != stream_run.runtime_session_id
    {
        return Err(AppError::conflict(
            "PM stream effective runtime identity changed",
        ));
    }
    Ok(())
}

impl LocalRuntimeSupervisor {
    pub(super) fn attach_pm_event_worker(
        &self,
        id: Uuid,
        tasks: &mut container_workers::AgentTasks,
    ) {
        tasks.reap();
        let supervisor = self.clone();
        tasks.spawn(id, async move {
            if supervisor.follow_pm_run(id).await.is_err() {
                // Keep native acceptance/capacity intact. Recovery only GETs the original ACK.
                tracing::warn!(run_id = %id, "PM event attachment requires readback reconciliation");
            }
        });
    }

    async fn follow_pm_run(&self, id: Uuid) -> Result<(), AppError> {
        let (record, committed) = self.repo.pm_stream_context(id).await?;
        if committed {
            return Ok(());
        }
        let session = self.repo.get_session(record.reservation.session_id).await?;
        let agent = self
            .repo
            .get_agent(record.reservation.identity.agent_id()?)
            .await?;
        let intent = self
            .repo
            .get_pm_dispatch(id)
            .await?
            .ok_or_else(pm_dispatch::unavailable)?;
        let base = pm_dispatch::verify_context(self, &agent, &intent).await?;
        let native_run = intent
            .hermes_run_ref
            .as_deref()
            .filter(|_| intent.submitted)
            .ok_or_else(pm_dispatch::unavailable)?;
        let token = crate::agent_runtime_token(&self.config, agent.id)?;
        let payload =
            hermes_wire::read_accepted_run(&self.client, &base, &token, native_run).await?;
        let effective = hermes_wire::effective_session(&payload, native_run)?;
        pm_dispatch::verify_context(self, &agent, &intent).await?;
        // ACK readback may have been interrupted before its effective-session pin was saved.
        let accepted = if record.hermes_run_ref.is_none() || record.hermes_session_ref.is_none() {
            self.repo
                .accept_pm_run(id, native_run.into(), effective.clone())
                .await?
        } else {
            if record.hermes_run_ref.as_deref() != Some(native_run)
                || record.hermes_session_ref.as_deref() != Some(effective.as_str())
            {
                return Err(AppError::conflict("PM effective runtime session changed"));
            }
            record
        };
        let mut run = self.repo.get_session_agent_run(id).await?;
        if run.runtime_session_id.as_deref()
            != Some(accepted.reservation.runtime_session_id().as_str())
        {
            return Err(AppError::conflict("PM requested runtime session changed"));
        }
        // The DB retains the requested alias. Only this stream view carries the immutable effective pin.
        run.runtime_session_id = Some(effective);
        verify_stream_pin(&accepted, &run, native_run)?;
        if matches!(
            payload["status"].as_str(),
            Some("completed" | "failed" | "cancelled" | "interrupted" | "stopped")
        ) {
            let event = hermes_wire::terminal_readback(&payload, native_run)?;
            self.handle_hermes_event(
                &agent,
                &session,
                None,
                &run,
                native_run,
                Some(event.into()),
                payload.to_string(),
                &mut sse_wire::Transcript::default(),
            )
            .await?;
            return Ok(());
        }
        self.emit_run(&run);
        self.follow_hermes_events(agent, session, None, run, native_run.into())
            .await
    }
}
