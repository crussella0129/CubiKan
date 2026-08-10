use cubikan_core::IntentUnitId;

use crate::{IntentUnitSummary, ListCursor, ListFilters, PageLimit, RelationshipDefinitionKey};

/// Optional direct relationship membership predicate for projection query v1.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DirectRelationshipPredicate {
    Outgoing {
        definition: RelationshipDefinitionKey,
        anchor: IntentUnitId,
    },
    Incoming {
        definition: RelationshipDefinitionKey,
        anchor: IntentUnitId,
    },
}

impl DirectRelationshipPredicate {
    #[must_use]
    pub const fn definition(&self) -> &RelationshipDefinitionKey {
        match self {
            Self::Outgoing { definition, .. } | Self::Incoming { definition, .. } => definition,
        }
    }

    #[must_use]
    pub const fn anchor(&self) -> IntentUnitId {
        match self {
            Self::Outgoing { anchor, .. } | Self::Incoming { anchor, .. } => *anchor,
        }
    }
}

/// Ephemeral, versioned lifecycle-and-relationship projection query.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectionQueryV1 {
    filters: ListFilters,
    predicate: Option<DirectRelationshipPredicate>,
    limit: PageLimit,
    after: Option<ListCursor>,
}

impl ProjectionQueryV1 {
    pub const VERSION: u64 = 1;

    #[must_use]
    pub const fn new(
        filters: ListFilters,
        predicate: Option<DirectRelationshipPredicate>,
        limit: PageLimit,
        after: Option<ListCursor>,
    ) -> Self {
        Self {
            filters,
            predicate,
            limit,
            after,
        }
    }

    #[must_use]
    pub const fn version(&self) -> u64 {
        Self::VERSION
    }

    #[must_use]
    pub const fn filters(&self) -> &ListFilters {
        &self.filters
    }

    #[must_use]
    pub const fn predicate(&self) -> Option<&DirectRelationshipPredicate> {
        self.predicate.as_ref()
    }

    #[must_use]
    pub const fn limit(&self) -> PageLimit {
        self.limit
    }

    #[must_use]
    pub const fn after(&self) -> Option<ListCursor> {
        self.after
    }
}

/// One bounded page of validated ephemeral projection summaries.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectionPage {
    query: ProjectionQueryV1,
    items: Vec<IntentUnitSummary>,
    next_cursor: Option<ListCursor>,
}

impl ProjectionPage {
    #[must_use]
    pub const fn new(
        query: ProjectionQueryV1,
        items: Vec<IntentUnitSummary>,
        next_cursor: Option<ListCursor>,
    ) -> Self {
        Self {
            query,
            items,
            next_cursor,
        }
    }

    #[must_use]
    pub const fn query(&self) -> &ProjectionQueryV1 {
        &self.query
    }

    #[must_use]
    pub fn items(&self) -> &[IntentUnitSummary] {
        &self.items
    }

    #[must_use]
    pub const fn next_cursor(&self) -> Option<ListCursor> {
        self.next_cursor
    }
}
