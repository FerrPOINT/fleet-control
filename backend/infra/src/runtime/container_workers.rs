use super::*;
use std::future::Future;
use tokio::{sync::OwnedMutexGuard, task::JoinHandle};

#[derive(Default)]
pub(super) struct AgentTasks {
    pub(super) tasks: HashMap<Uuid, JoinHandle<()>>,
}

impl AgentTasks {
    pub(super) fn reap(&mut self) {
        self.tasks.retain(|_, task| !task.is_finished());
    }

    pub(super) fn is_running(&self, agent_id: Uuid) -> bool {
        self.tasks
            .get(&agent_id)
            .is_some_and(|task| !task.is_finished())
    }

    pub(super) fn spawn(
        &mut self,
        agent_id: Uuid,
        reconcile: impl Future<Output = ()> + Send + 'static,
    ) {
        if self.is_running(agent_id) {
            return;
        }
        self.tasks.insert(agent_id, tokio::spawn(reconcile));
    }
}

impl Drop for AgentTasks {
    fn drop(&mut self) {
        for task in self.tasks.values() {
            task.abort();
        }
    }
}

#[derive(Default)]
pub(super) struct ContainerOperations {
    agents: Mutex<HashMap<Uuid, std::sync::Weak<Mutex<()>>>>,
}

impl ContainerOperations {
    async fn agent(&self, id: Uuid) -> Arc<Mutex<()>> {
        let mut agents = self.agents.lock().await;
        agents.retain(|_, lock| lock.strong_count() > 0);
        if let Some(lock) = agents.get(&id).and_then(std::sync::Weak::upgrade) {
            return lock;
        }
        let lock = Arc::new(Mutex::new(()));
        agents.insert(id, Arc::downgrade(&lock));
        lock
    }

    pub(super) async fn lock(&self, id: Uuid) -> OwnedMutexGuard<()> {
        self.agent(id).await.lock_owned().await
    }

    pub(super) async fn try_lock(&self, id: Uuid) -> Option<OwnedMutexGuard<()>> {
        self.agent(id).await.try_lock_owned().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn activation_serializes_same_agent_without_blocking_sibling_operations() {
        let locks = Arc::new(ContainerOperations::default());
        let activating = Uuid::new_v4();
        let sibling = Uuid::new_v4();
        let activation = locks.lock(activating).await;
        assert!(locks.try_lock(activating).await.is_none());
        let other = tokio::time::timeout(Duration::from_secs(1), locks.lock(sibling))
            .await
            .unwrap();
        let (started, ready) = tokio::sync::oneshot::channel();
        let waiter_locks = locks.clone();
        let waiter = tokio::spawn(async move {
            started.send(()).unwrap();
            waiter_locks.lock(activating).await
        });
        ready.await.unwrap();
        assert!(!waiter.is_finished());
        drop(activation);
        let same_agent = tokio::time::timeout(Duration::from_secs(1), waiter)
            .await
            .unwrap()
            .unwrap();
        assert!(locks.try_lock(activating).await.is_none());
        drop(same_agent);
        drop(other);
        assert!(locks.try_lock(activating).await.is_some());
    }

    #[tokio::test]
    async fn released_agent_locks_are_reclaimed_without_changing_live_custody() {
        let locks = ContainerOperations::default();
        let live = Uuid::new_v4();
        let guard = locks.lock(live).await;
        for _ in 0..100 {
            drop(locks.lock(Uuid::new_v4()).await);
        }
        assert!(locks.try_lock(live).await.is_none());
        assert!(locks.agents.lock().await.len() <= 2);
        drop(guard);
        assert!(locks.try_lock(live).await.is_some());
    }
}
