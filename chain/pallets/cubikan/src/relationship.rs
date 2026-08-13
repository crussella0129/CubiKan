//! Bounded, exact-version relationship graph helpers.
//!
//! The pallet stores at most 128 edges per immutable definition key. Cycle
//! rejection therefore traverses a closed finite graph and never consults
//! edges belonging to another definition version.

use frame_support::{traits::ConstU32, BoundedVec};

use crate::types::{IntentUnitId, RelationshipKey, MAX_RELATIONSHIP_EDGES};

/// Returns whether adding `candidate` would close a non-self directed cycle.
///
/// The caller applies self-edge policy separately. Starting from the proposed
/// target, this bounded fixed-point traversal follows only the supplied exact-
/// definition edges and asks whether the proposed source is already reachable.
pub(crate) fn closes_cycle(edges: &[RelationshipKey], candidate: &RelationshipKey) -> bool {
    debug_assert!(edges.len() <= MAX_RELATIONSHIP_EDGES);

    // A graph with at most 128 edges can expose at most 129 distinct vertices
    // to one traversal after seeding the candidate target.
    let mut reachable: BoundedVec<IntentUnitId, ConstU32<129>> = BoundedVec::default();
    reachable
        .try_push(candidate.target_id())
        .expect("one traversal seed fits the 129-vertex bound");

    for _ in 0..=edges.len() {
        if reachable.contains(&candidate.source_id()) {
            return true;
        }

        let previous_len = reachable.len();
        for edge in edges {
            if reachable.contains(&edge.source_id()) && !reachable.contains(&edge.target_id()) {
                reachable
                    .try_push(edge.target_id())
                    .expect("128 edges plus one seed fit the traversal bound");
            }
        }
        if reachable.len() == previous_len {
            return false;
        }
    }

    reachable.contains(&candidate.source_id())
}
