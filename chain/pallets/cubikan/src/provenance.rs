//! Exact provenance-subject and external-reference validation.
//!
//! The bounded SCALE types reject malformed subjects and references before
//! dispatch. These helpers re-check the selected unit/revision and preserve an
//! explicit defense-in-depth reference-validation step in dispatch order.

use crate::types::{
    AssociationSubject, ExternalReference, IntentUnitState, Namespace, ReferenceScope,
    ReferenceValue,
};

/// Returns whether the selected whole-unit or exact revision subject exists.
#[must_use]
pub(crate) const fn subject_exists(unit: &IntentUnitState, subject: AssociationSubject) -> bool {
    match subject {
        AssociationSubject::WholeUnit => true,
        AssociationSubject::Revision(revision) => revision <= unit.revision(),
    }
}

/// Revalidates a structurally decoded reference without normalization.
///
/// Safe constructors and SCALE decoding already guarantee this predicate. It
/// remains explicit here so provenance dispatch order is locally auditable and
/// future internal construction cannot silently bypass the reference contract.
#[must_use]
pub(crate) fn reference_is_valid(reference: &ExternalReference) -> bool {
    Namespace::try_from_bytes(reference.namespace().as_bytes())
        .is_ok_and(|value| &value == reference.namespace())
        && ReferenceScope::try_from_bytes(reference.scope().as_bytes())
            .is_ok_and(|value| &value == reference.scope())
        && ReferenceValue::try_from_bytes(reference.value().as_bytes())
            .is_ok_and(|value| &value == reference.value())
}
