use serde::{Deserialize, Serialize};

use crate::models::issues::Comment;

/// The payload in a [`super::EventPayload::CommitCommentEvent`] type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CommitCommentEventPayload {
    /// The comment this event corresponds to.
    pub comment: Comment,
}

#[cfg(test)]
mod test {
    use crate::models::events::payload::EventPayload;
    use crate::models::events::Event;

    #[test]
    fn should_deserialize_with_correct_payload() {
        let json = include_str!("../../../tests/resources/commit_comment_event.json");
        let event: Event = serde_json::from_str(json).unwrap();
        if let Some(EventPayload::CommitCommentEvent(payload)) = event.payload {
            assert_eq!(payload.comment.id.0, 46377107);
        } else {
            panic!("unexpected event payload encountered: {:#?}", event.payload);
        }
    }
}
