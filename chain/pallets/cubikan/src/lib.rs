#![cfg_attr(not(feature = "std"), no_std)]

//! Canonical, bounded CubiKan lifecycle state transitions.
//!
//! Callers provide already-validated SCALE values from [`types`]. The pallet
//! owns technical authorization, deterministic mutation ordering, the complete
//! canonical aggregate, and the accepted-event sequence.

extern crate alloc;

pub mod conformance;
pub mod error;
pub mod event;
pub mod provenance;
pub mod relationship;
pub mod types;
pub mod weights;

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;

pub use pallet::*;
pub use weights::WeightInfo;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests {
    mod lifecycle;
    mod model;
    mod provenance;
    mod relationships;
}

#[frame_support::pallet]
pub mod pallet {
    use alloc::vec::Vec;

    use frame_support::{
        pallet_prelude::*,
        traits::{BuildGenesisConfig, ConstU32, StorageVersion},
        BoundedVec,
    };
    use frame_system::pallet_prelude::*;

    use crate::{
        error::map_lifecycle_error,
        event::{
            DeploymentId, COMMAND_SCHEMA_VERSION, EVENT_SCHEMA_VERSION,
            PALLET_STORAGE_SCHEMA_VERSION,
        },
        types::{
            AssociationKey, CreateUnitPayload, DefinitionKey, DomainPayload, ExternalReference,
            IntentSpecies, IntentUnitId, IntentUnitState, PhaseId, RelationshipDefinition,
            RelationshipKey, RelationshipPolicy, Workflow, MAX_ACTIVE_ASSOCIATIONS,
            MAX_AUTHORIZED_SUBMITTERS, MAX_RELATIONSHIP_EDGES,
        },
        weights::WeightInfo,
    };

    /// At most one over-bound account is decodable so the administrative call
    /// can return its typed maximum-plus-one rejection.
    pub type AuthorizedSubmitterInput<T> =
        BoundedVec<<T as frame_system::Config>::AccountId, ConstU32<17>>;

    /// Exact stored/event representation of the technical submitter allowlist.
    pub type AuthorizedSubmittersOf<T> =
        BoundedVec<<T as frame_system::Config>::AccountId, ConstU32<16>>;

    /// Complete live directed-edge set for one immutable definition version.
    pub type RelationshipEdgesOf = BoundedVec<RelationshipKey, ConstU32<128>>;

    /// Complete active provenance identities for one exact Intent Unit.
    pub type ActiveAssociationsOf = BoundedVec<AssociationKey, ConstU32<128>>;

    const STORAGE_VERSION: StorageVersion = StorageVersion::new(PALLET_STORAGE_SCHEMA_VERSION);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching runtime event type.
        #[allow(deprecated)]
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Worst-case weights for every dispatchable domain operation.
        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    /// Complete canonical state keyed by caller-supplied UUID bytes.
    #[pallet::storage]
    #[pallet::getter(fn intent_units)]
    pub type IntentUnits<T: Config> =
        StorageMap<_, Blake2_128Concat, IntentUnitId, IntentUnitState, OptionQuery>;

    /// Immutable definitions keyed by their caller-owned exact version.
    #[pallet::storage]
    #[pallet::getter(fn relationship_definitions)]
    pub type RelationshipDefinitions<T: Config> =
        StorageMap<_, Blake2_128Concat, DefinitionKey, RelationshipDefinition, OptionQuery>;

    /// Bounded live directed edges scoped to one exact definition version.
    #[pallet::storage]
    #[pallet::getter(fn relationship_edges)]
    pub type RelationshipEdges<T: Config> =
        StorageMap<_, Blake2_128Concat, DefinitionKey, RelationshipEdgesOf, ValueQuery>;

    /// Bounded many-to-many provenance membership keyed by Intent Unit.
    #[pallet::storage]
    #[pallet::getter(fn active_associations)]
    pub type ActiveAssociations<T: Config> =
        StorageMap<_, Blake2_128Concat, IntentUnitId, ActiveAssociationsOf, ValueQuery>;

    /// Directly signed technical accounts permitted to submit domain calls.
    #[pallet::storage]
    #[pallet::getter(fn authorized_submitters)]
    pub type AuthorizedSubmitters<T: Config> =
        StorageValue<_, AuthorizedSubmittersOf<T>, ValueQuery>;

    /// Last accepted CubiKan domain-event sequence, absent before the first.
    #[pallet::storage]
    #[pallet::getter(fn global_sequence)]
    pub type GlobalSequence<T: Config> = StorageValue<_, u64, OptionQuery>;

    /// Non-self-referential 32-byte deployment identity configured at genesis.
    #[pallet::storage]
    #[pallet::getter(fn deployment_anchor)]
    pub type DeploymentAnchor<T: Config> = StorageValue<_, DeploymentId, ValueQuery>;

    /// Stored pallet schema identity configured at genesis.
    #[pallet::storage]
    #[pallet::getter(fn pallet_storage_version)]
    pub type PalletStorageVersion<T: Config> = StorageValue<_, u16, ValueQuery>;

    /// Stored accepted-event schema identity configured at genesis.
    #[pallet::storage]
    #[pallet::getter(fn event_schema_version)]
    pub type EventSchemaVersion<T: Config> = StorageValue<_, u16, ValueQuery>;

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub deployment_id: DeploymentId,
        pub pallet_storage_version: u16,
        pub event_schema_version: u16,
        pub authorized_submitters: Vec<T::AccountId>,
    }

    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                deployment_id: DeploymentId::default(),
                pallet_storage_version: PALLET_STORAGE_SCHEMA_VERSION,
                event_schema_version: EVENT_SCHEMA_VERSION,
                authorized_submitters: Vec::new(),
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
            assert_eq!(
                self.pallet_storage_version, PALLET_STORAGE_SCHEMA_VERSION,
                "unsupported CubiKan pallet storage version"
            );
            assert_eq!(
                self.event_schema_version, EVENT_SCHEMA_VERSION,
                "unsupported CubiKan accepted-event schema version"
            );
            assert!(
                self.authorized_submitters.len() <= MAX_AUTHORIZED_SUBMITTERS,
                "too many CubiKan authorized submitters"
            );
            for duplicate in 0..self.authorized_submitters.len() {
                assert!(
                    !self.authorized_submitters[..duplicate]
                        .contains(&self.authorized_submitters[duplicate]),
                    "duplicate CubiKan authorized submitter"
                );
            }

            let submitters: AuthorizedSubmittersOf<T> = self
                .authorized_submitters
                .clone()
                .try_into()
                .expect("allowlist length checked above");
            DeploymentAnchor::<T>::put(self.deployment_id);
            PalletStorageVersion::<T>::put(self.pallet_storage_version);
            EventSchemaVersion::<T>::put(self.event_schema_version);
            AuthorizedSubmitters::<T>::put(submitters);
        }
    }

    /// Pallet events separate accepted domain events from administrative work.
    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    // Accepted events intentionally carry the complete bounded replay payload;
    // heap indirection would not reduce their canonical SCALE bytes.
    #[allow(clippy::large_enum_variant)]
    pub enum Event<T: Config> {
        /// One replay-complete canonical CubiKan domain event.
        Accepted {
            deployment_id: DeploymentId,
            event_schema_version: u16,
            global_sequence: u64,
            signer: T::AccountId,
            payload: DomainPayload,
        },
        /// Root replaced the technical submitter allowlist. This is not a
        /// domain event and does not consume a global sequence.
        AuthorizedSubmittersReplaced { accounts: AuthorizedSubmittersOf<T> },
    }

    /// Exact typed lifecycle and administrative failures.
    #[pallet::error]
    pub enum Error<T> {
        UnsupportedCommandSchemaVersion,
        UnauthorizedSubmitter,
        IntentUnitAlreadyExists,
        IntentUnitNotFound,
        StaleRevision,
        LifecycleHistoryCapacityExceeded,
        LifecycleRevisionExhausted,
        IntentUnitAlreadyCompleted,
        UnknownTargetPhase,
        TransitionNotAllowed,
        CompletionPhaseNotEligible,
        RelationshipDefinitionAlreadyExists,
        RelationshipDefinitionNotFound,
        RelationshipSourceNotFound,
        RelationshipTargetNotFound,
        RelationshipSourceSpeciesMismatch,
        RelationshipTargetSpeciesMismatch,
        RelationshipSelfEdgeRejected,
        RelationshipAlreadyExists,
        RelationshipCapacityExceeded,
        RelationshipCycleRejected,
        RelationshipNotFound,
        AssociationUnitNotFound,
        AssociationRevisionNotFound,
        AssociationReferenceInvalid,
        AssociationAlreadyExists,
        AssociationCapacityExceeded,
        AssociationNotFound,
        GlobalSequenceExhausted,
        DuplicateAuthorizedSubmitter,
        TooManyAuthorizedSubmitters,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Create one required-origin unit at revision zero.
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::create_unit())]
        pub fn create_unit(
            origin: OriginFor<T>,
            command_schema_version: u16,
            id: IntentUnitId,
            unit_origin: ExternalReference,
            species: IntentSpecies,
            workflow: Workflow,
        ) -> DispatchResult {
            let signer = Self::authorized_signer(command_schema_version, origin)?;
            ensure!(
                !IntentUnits::<T>::contains_key(id),
                Error::<T>::IntentUnitAlreadyExists
            );
            let global_sequence = Self::next_global_sequence()?;

            let payload = DomainPayload::UnitCreated(CreateUnitPayload {
                command_schema_version,
                id,
                origin: unit_origin.clone(),
                species: species.clone(),
                workflow: workflow.clone(),
            });
            let state = IntentUnitState::new(id, unit_origin, species, workflow);

            IntentUnits::<T>::insert(id, state);
            GlobalSequence::<T>::put(global_sequence);
            Self::deposit_accepted(global_sequence, signer, payload);
            Ok(())
        }

        /// Apply one exact-revision workflow transition.
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::transition_unit())]
        pub fn transition_unit(
            origin: OriginFor<T>,
            command_schema_version: u16,
            id: IntentUnitId,
            target: PhaseId,
            expected_revision: u64,
        ) -> DispatchResult {
            let signer = Self::authorized_signer(command_schema_version, origin)?;
            let current = IntentUnits::<T>::get(id).ok_or(Error::<T>::IntentUnitNotFound)?;
            let from = current.phase().clone();
            let mut successor = current;
            let committed_revision = successor
                .transition_to(&target, expected_revision)
                .map_err(map_lifecycle_error::<T>)?;
            let global_sequence = Self::next_global_sequence()?;
            let payload = DomainPayload::UnitTransitioned {
                unit_id: id,
                committed_revision,
                from,
                to: target,
            };

            IntentUnits::<T>::insert(id, successor);
            GlobalSequence::<T>::put(global_sequence);
            Self::deposit_accepted(global_sequence, signer, payload);
            Ok(())
        }

        /// Complete one unit at its exact observed revision.
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::complete_unit())]
        pub fn complete_unit(
            origin: OriginFor<T>,
            command_schema_version: u16,
            id: IntentUnitId,
            expected_revision: u64,
        ) -> DispatchResult {
            let signer = Self::authorized_signer(command_schema_version, origin)?;
            let current = IntentUnits::<T>::get(id).ok_or(Error::<T>::IntentUnitNotFound)?;
            let phase = current.phase().clone();
            let mut successor = current;
            let committed_revision = successor
                .complete(expected_revision)
                .map_err(map_lifecycle_error::<T>)?;
            let global_sequence = Self::next_global_sequence()?;
            let payload = DomainPayload::UnitCompleted {
                unit_id: id,
                committed_revision,
                phase,
            };

            IntentUnits::<T>::insert(id, successor);
            GlobalSequence::<T>::put(global_sequence);
            Self::deposit_accepted(global_sequence, signer, payload);
            Ok(())
        }

        /// Replace the complete technical submitter allowlist as Root.
        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::replace_authorized_submitters(accounts.len() as u32))]
        pub fn replace_authorized_submitters(
            origin: OriginFor<T>,
            accounts: AuthorizedSubmitterInput<T>,
        ) -> DispatchResult {
            ensure_root(origin)?;
            ensure!(
                accounts.len() <= MAX_AUTHORIZED_SUBMITTERS,
                Error::<T>::TooManyAuthorizedSubmitters
            );
            for duplicate in 0..accounts.len() {
                ensure!(
                    !accounts[..duplicate].contains(&accounts[duplicate]),
                    Error::<T>::DuplicateAuthorizedSubmitter
                );
            }
            let stored: AuthorizedSubmittersOf<T> = accounts
                .into_inner()
                .try_into()
                .map_err(|_| Error::<T>::TooManyAuthorizedSubmitters)?;

            AuthorizedSubmitters::<T>::put(&stored);
            Self::deposit_event(Event::AuthorizedSubmittersReplaced { accounts: stored });
            Ok(())
        }

        /// Create one immutable exact-version relationship definition.
        #[pallet::call_index(4)]
        #[pallet::weight(T::WeightInfo::create_relationship_definition())]
        pub fn create_relationship_definition(
            origin: OriginFor<T>,
            command_schema_version: u16,
            definition: RelationshipDefinition,
        ) -> DispatchResult {
            let signer = Self::authorized_signer(command_schema_version, origin)?;
            let definition_key = definition.key().clone();
            ensure!(
                !RelationshipDefinitions::<T>::contains_key(&definition_key),
                Error::<T>::RelationshipDefinitionAlreadyExists
            );
            let global_sequence = Self::next_global_sequence()?;

            RelationshipDefinitions::<T>::insert(&definition_key, &definition);
            GlobalSequence::<T>::put(global_sequence);
            Self::deposit_accepted(
                global_sequence,
                signer,
                DomainPayload::RelationshipDefinitionCreated(definition),
            );
            Ok(())
        }

        /// Create one bounded directed edge under an exact definition version.
        #[pallet::call_index(5)]
        #[pallet::weight(T::WeightInfo::create_relationship())]
        pub fn create_relationship(
            origin: OriginFor<T>,
            command_schema_version: u16,
            relationship: RelationshipKey,
        ) -> DispatchResult {
            let signer = Self::authorized_signer(command_schema_version, origin)?;
            let definition_key = relationship.definition().clone();
            let definition = RelationshipDefinitions::<T>::get(&definition_key)
                .ok_or(Error::<T>::RelationshipDefinitionNotFound)?;
            let source_id = relationship.source_id();
            let target_id = relationship.target_id();
            let source =
                IntentUnits::<T>::get(source_id).ok_or(Error::<T>::RelationshipSourceNotFound)?;
            let target =
                IntentUnits::<T>::get(target_id).ok_or(Error::<T>::RelationshipTargetNotFound)?;

            if let Some(expected) = definition.source_species() {
                ensure!(
                    source.species() == expected,
                    Error::<T>::RelationshipSourceSpeciesMismatch
                );
            }
            if let Some(expected) = definition.target_species() {
                ensure!(
                    target.species() == expected,
                    Error::<T>::RelationshipTargetSpeciesMismatch
                );
            }

            let is_self = source_id == target_id;
            ensure!(
                !is_self || definition.self_policy() == RelationshipPolicy::Allow,
                Error::<T>::RelationshipSelfEdgeRejected
            );

            let mut edges = RelationshipEdges::<T>::get(&definition_key);
            ensure!(
                !edges.contains(&relationship),
                Error::<T>::RelationshipAlreadyExists
            );
            ensure!(
                edges.len() < MAX_RELATIONSHIP_EDGES,
                Error::<T>::RelationshipCapacityExceeded
            );
            if !is_self && definition.cycle_policy() == RelationshipPolicy::Reject {
                ensure!(
                    !crate::relationship::closes_cycle(edges.as_slice(), &relationship),
                    Error::<T>::RelationshipCycleRejected
                );
            }
            let global_sequence = Self::next_global_sequence()?;

            edges
                .try_push(relationship.clone())
                .map_err(|_| Error::<T>::RelationshipCapacityExceeded)?;
            RelationshipEdges::<T>::insert(&definition_key, edges);
            GlobalSequence::<T>::put(global_sequence);
            Self::deposit_accepted(
                global_sequence,
                signer,
                DomainPayload::RelationshipCreated(relationship),
            );
            Ok(())
        }

        /// Delete only the named live edge; definitions and neighbors remain.
        #[pallet::call_index(6)]
        #[pallet::weight(T::WeightInfo::delete_relationship())]
        pub fn delete_relationship(
            origin: OriginFor<T>,
            command_schema_version: u16,
            relationship: RelationshipKey,
        ) -> DispatchResult {
            let signer = Self::authorized_signer(command_schema_version, origin)?;
            let definition_key = relationship.definition().clone();
            let definition = RelationshipDefinitions::<T>::get(&definition_key)
                .ok_or(Error::<T>::RelationshipDefinitionNotFound)?;
            let source_id = relationship.source_id();
            let target_id = relationship.target_id();
            let source =
                IntentUnits::<T>::get(source_id).ok_or(Error::<T>::RelationshipSourceNotFound)?;
            let target =
                IntentUnits::<T>::get(target_id).ok_or(Error::<T>::RelationshipTargetNotFound)?;

            if let Some(expected) = definition.source_species() {
                ensure!(
                    source.species() == expected,
                    Error::<T>::RelationshipSourceSpeciesMismatch
                );
            }
            if let Some(expected) = definition.target_species() {
                ensure!(
                    target.species() == expected,
                    Error::<T>::RelationshipTargetSpeciesMismatch
                );
            }

            let mut edges = RelationshipEdges::<T>::get(&definition_key);
            let position = edges
                .iter()
                .position(|stored| stored == &relationship)
                .ok_or(Error::<T>::RelationshipNotFound)?;
            let global_sequence = Self::next_global_sequence()?;

            edges.remove(position);
            RelationshipEdges::<T>::insert(&definition_key, edges);
            GlobalSequence::<T>::put(global_sequence);
            Self::deposit_accepted(
                global_sequence,
                signer,
                DomainPayload::RelationshipDeleted(relationship),
            );
            Ok(())
        }

        /// Record one exact active whole-unit or revision association.
        #[pallet::call_index(7)]
        #[pallet::weight(T::WeightInfo::record_association())]
        pub fn record_association(
            origin: OriginFor<T>,
            command_schema_version: u16,
            association: AssociationKey,
        ) -> DispatchResult {
            let signer = Self::authorized_signer(command_schema_version, origin)?;
            let unit_id = association.unit_id();
            let unit = IntentUnits::<T>::get(unit_id).ok_or(Error::<T>::AssociationUnitNotFound)?;
            ensure!(
                crate::provenance::subject_exists(&unit, association.subject()),
                Error::<T>::AssociationRevisionNotFound
            );
            ensure!(
                crate::provenance::reference_is_valid(association.reference()),
                Error::<T>::AssociationReferenceInvalid
            );

            let mut active = ActiveAssociations::<T>::get(unit_id);
            ensure!(
                !active.contains(&association),
                Error::<T>::AssociationAlreadyExists
            );
            ensure!(
                active.len() < MAX_ACTIVE_ASSOCIATIONS,
                Error::<T>::AssociationCapacityExceeded
            );
            let global_sequence = Self::next_global_sequence()?;

            active
                .try_push(association.clone())
                .map_err(|_| Error::<T>::AssociationCapacityExceeded)?;
            ActiveAssociations::<T>::insert(unit_id, active);
            GlobalSequence::<T>::put(global_sequence);
            Self::deposit_accepted(
                global_sequence,
                signer,
                DomainPayload::AssociationRecorded(association),
            );
            Ok(())
        }

        /// Revoke only the named active membership and retain event history.
        #[pallet::call_index(8)]
        #[pallet::weight(T::WeightInfo::revoke_association())]
        pub fn revoke_association(
            origin: OriginFor<T>,
            command_schema_version: u16,
            association: AssociationKey,
        ) -> DispatchResult {
            let signer = Self::authorized_signer(command_schema_version, origin)?;
            let unit_id = association.unit_id();
            let unit = IntentUnits::<T>::get(unit_id).ok_or(Error::<T>::AssociationUnitNotFound)?;
            ensure!(
                crate::provenance::subject_exists(&unit, association.subject()),
                Error::<T>::AssociationRevisionNotFound
            );
            ensure!(
                crate::provenance::reference_is_valid(association.reference()),
                Error::<T>::AssociationReferenceInvalid
            );

            let mut active = ActiveAssociations::<T>::get(unit_id);
            let position = active
                .iter()
                .position(|stored| stored == &association)
                .ok_or(Error::<T>::AssociationNotFound)?;
            let global_sequence = Self::next_global_sequence()?;

            active.remove(position);
            ActiveAssociations::<T>::insert(unit_id, active);
            GlobalSequence::<T>::put(global_sequence);
            Self::deposit_accepted(
                global_sequence,
                signer,
                DomainPayload::AssociationRevoked(association),
            );
            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        fn authorized_signer(
            command_schema_version: u16,
            origin: OriginFor<T>,
        ) -> Result<T::AccountId, DispatchError> {
            ensure!(
                command_schema_version == COMMAND_SCHEMA_VERSION,
                Error::<T>::UnsupportedCommandSchemaVersion
            );
            let signer = ensure_signed(origin)?;
            ensure!(
                AuthorizedSubmitters::<T>::get().contains(&signer),
                Error::<T>::UnauthorizedSubmitter
            );
            Ok(signer)
        }

        fn next_global_sequence() -> Result<u64, DispatchError> {
            match GlobalSequence::<T>::get() {
                None => Ok(1),
                Some(sequence) => sequence
                    .checked_add(1)
                    .ok_or_else(|| Error::<T>::GlobalSequenceExhausted.into()),
            }
        }

        fn deposit_accepted(global_sequence: u64, signer: T::AccountId, payload: DomainPayload) {
            Self::deposit_event(Event::Accepted {
                deployment_id: DeploymentAnchor::<T>::get(),
                event_schema_version: EventSchemaVersion::<T>::get(),
                global_sequence,
                signer,
                payload,
            });
        }
    }
}
