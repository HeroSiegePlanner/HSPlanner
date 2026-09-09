//! Identifies the latest requested subtree node independently of worker completion order.
#[derive(Default)]
pub(super) struct PreviewRequest {
    generation: u64,
    target: Option<String>,
}

impl PreviewRequest {
    /// Deduplicate motion inside one node, but replace a pending different target.
    pub(super) fn begin(&mut self, target: &str) -> Option<u64> {
        if self.target.as_deref() == Some(target) {
            return None;
        }
        self.generation = self.generation.wrapping_add(1);
        self.target = Some(target.into());
        Some(self.generation)
    }

    /// A stale completion must not clear the newer target or its task handle.
    pub(super) fn finish(&mut self, target: &str, generation: u64) -> bool {
        if self.generation != generation || self.target.as_deref() != Some(target) {
            return false;
        }
        self.target = None;
        true
    }

    pub(super) fn cancel(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.target = None;
    }
}

#[cfg(test)]
mod tests {
    use super::PreviewRequest;

    #[::core::prelude::v1::test]
    fn moving_from_a_to_b_replaces_pending_work_and_a_cannot_clear_b() {
        let mut request = PreviewRequest::default();
        let a = request.begin("a").unwrap();
        assert_eq!(request.begin("a"), None);
        let b = request.begin("b").unwrap();
        assert_ne!(a, b);
        assert!(!request.finish("a", a));
        assert_eq!(request.begin("b"), None);
        assert!(request.finish("b", b));
        assert_eq!(request.target, None);
    }

    #[::core::prelude::v1::test]
    fn returning_to_a_does_not_accept_a_result_from_before_b() {
        let mut request = PreviewRequest::default();
        let first_a = request.begin("a").unwrap();
        let b = request.begin("b").unwrap();
        let second_a = request.begin("a").unwrap();
        assert!(!request.finish("a", first_a));
        assert!(!request.finish("b", b));
        assert_eq!(request.begin("a"), None);
        assert!(request.finish("a", second_a));
    }

    #[::core::prelude::v1::test]
    fn cancelling_document_or_progression_work_allows_retry_and_rejects_old_results() {
        let mut request = PreviewRequest::default();
        let old = request.begin("a").unwrap();
        request.cancel();
        assert_eq!(request.target, None);
        assert!(!request.finish("a", old));
        let retry = request.begin("a").unwrap();
        assert!(!request.finish("a", old));
        assert!(request.finish("a", retry));
    }
}
