use dreamland_core::{PostQueryRequest, QuerySessionId, SiteId};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct QuerySession {
    pub id: QuerySessionId,
    pub site: SiteId,
    pub request: PostQueryRequest,
}

#[derive(Debug, Clone)]
pub struct SessionOperation {
    pub session: QuerySessionId,
    generation: u64,
}

#[derive(Debug, Clone, Default)]
pub struct QuerySessionStore {
    sessions: Arc<Mutex<HashMap<String, SessionEntry>>>,
}

#[derive(Debug, Clone)]
struct SessionEntry {
    session: QuerySession,
    generation: u64,
}

impl QuerySessionStore {
    pub fn start(&self, site: SiteId, request: PostQueryRequest) -> QuerySession {
        let id = QuerySessionId::new(uuid::Uuid::new_v4().to_string());
        let session = QuerySession {
            id: id.clone(),
            site,
            request,
        };
        self.sessions
            .lock()
            .expect("query session lock poisoned")
            .insert(
                id.as_str().to_owned(),
                SessionEntry {
                    session: session.clone(),
                    generation: 0,
                },
            );
        session
    }

    pub fn get(&self, id: &QuerySessionId) -> Option<QuerySession> {
        self.sessions
            .lock()
            .expect("query session lock poisoned")
            .get(id.as_str())
            .map(|entry| entry.session.clone())
    }

    pub fn begin_operation(&self, id: &QuerySessionId) -> Option<SessionOperation> {
        let mut sessions = self.sessions.lock().expect("query session lock poisoned");
        let entry = sessions.get_mut(id.as_str())?;
        entry.generation = entry.generation.saturating_add(1);
        Some(SessionOperation {
            session: id.clone(),
            generation: entry.generation,
        })
    }

    pub fn begin_next_page(&self, id: &QuerySessionId) -> Option<(QuerySession, SessionOperation)> {
        let mut sessions = self.sessions.lock().expect("query session lock poisoned");
        let entry = sessions.get_mut(id.as_str())?;
        let next_page = match entry.session.request.pagination {
            dreamland_core::PaginationRequest::First { page_size } => {
                dreamland_core::PaginationRequest::Page {
                    number: 2,
                    page_size,
                }
            }
            dreamland_core::PaginationRequest::Page { number, page_size } => {
                dreamland_core::PaginationRequest::Page {
                    number: number.checked_add(1)?,
                    page_size,
                }
            }
            dreamland_core::PaginationRequest::FixedWindow => return None,
        };
        entry.session.request.pagination = next_page;
        entry.generation = entry.generation.saturating_add(1);
        Some((
            entry.session.clone(),
            SessionOperation {
                session: id.clone(),
                generation: entry.generation,
            },
        ))
    }

    pub fn is_current(&self, operation: &SessionOperation) -> bool {
        self.sessions
            .lock()
            .expect("query session lock poisoned")
            .get(operation.session.as_str())
            .is_some_and(|entry| entry.generation == operation.generation)
    }

    pub fn cancel(&self, id: &QuerySessionId) -> bool {
        self.sessions
            .lock()
            .expect("query session lock poisoned")
            .remove(id.as_str())
            .is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dreamland_core::{ContentPolicy, DiscoverySource, PaginationRequest, ReplayableQuery};

    fn request() -> PostQueryRequest {
        PostQueryRequest {
            query: ReplayableQuery {
                source: DiscoverySource::Browse,
                content_policy: ContentPolicy::SafeOnly,
            },
            pagination: PaginationRequest::First { page_size: 20 },
        }
    }

    #[test]
    fn sessions_are_owned_by_the_runtime_and_cancelable() {
        let store = QuerySessionStore::default();
        let session = store.start(SiteId::new("yandere"), request());

        assert_eq!(store.get(&session.id).unwrap().site.as_str(), "yandere");
        assert!(store.cancel(&session.id));
        assert!(store.get(&session.id).is_none());
        assert!(!store.cancel(&session.id));
    }

    #[test]
    fn only_the_latest_operation_can_publish_results() {
        let store = QuerySessionStore::default();
        let session = store.start(SiteId::new("yandere"), request());
        let first = store.begin_operation(&session.id).unwrap();
        let second = store.begin_operation(&session.id).unwrap();

        assert!(!store.is_current(&first));
        assert!(store.is_current(&second));
        assert!(store.cancel(&session.id));
        assert!(!store.is_current(&second));
    }

    #[test]
    fn next_page_advances_the_runtime_owned_request() {
        let store = QuerySessionStore::default();
        let session = store.start(SiteId::new("yandere"), request());
        let (next, operation) = store.begin_next_page(&session.id).unwrap();

        assert_eq!(
            next.request.pagination,
            PaginationRequest::Page {
                number: 2,
                page_size: 20
            }
        );
        assert!(store.is_current(&operation));
    }
}
