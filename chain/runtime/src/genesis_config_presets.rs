//! Exact local genesis preset. These public development identities are never
//! production keys and carry no economic meaning.

use super::*;
use alloc::{vec, vec::Vec};
use cumulus_primitives_core::ParaId;
use frame_support::build_struct_json_patch;

const ALICE: [u8; 32] = [
    0xd4, 0x35, 0x93, 0xc7, 0x15, 0xfd, 0xd3, 0x1c, 0x61, 0x14, 0x1a, 0xbd, 0x04, 0xa9, 0x9f, 0xd6,
    0x82, 0x2c, 0x85, 0x58, 0x85, 0x4c, 0xcd, 0xe3, 0x9a, 0x56, 0x84, 0xe7, 0xa5, 0x6d, 0xa2, 0x7d,
];
const BOB: [u8; 32] = [
    0x8e, 0xaf, 0x04, 0x15, 0x16, 0x87, 0x73, 0x63, 0x26, 0xc9, 0xfe, 0xa1, 0x7e, 0x25, 0xfc, 0x52,
    0x87, 0x61, 0x36, 0x93, 0xc9, 0x12, 0x90, 0x9c, 0xb2, 0x26, 0xaa, 0x47, 0x94, 0xf2, 0x6a, 0x48,
];
const CHARLIE: [u8; 32] = [
    0x90, 0xb5, 0xab, 0x20, 0x5c, 0x69, 0x74, 0xc9, 0xea, 0x84, 0x1b, 0xe6, 0x88, 0x86, 0x46, 0x33,
    0xdc, 0x9c, 0xa8, 0xa3, 0x57, 0x84, 0x3e, 0xea, 0xcf, 0x23, 0x14, 0x64, 0x99, 0x65, 0xfe, 0x22,
];
const DAVE: [u8; 32] = [
    0x30, 0x67, 0x21, 0x21, 0x1d, 0x54, 0x04, 0xbd, 0x9d, 0xa8, 0x8e, 0x02, 0x04, 0x36, 0x0a, 0x1a,
    0x9a, 0xb8, 0xb8, 0x7c, 0x66, 0xc1, 0xbc, 0x2f, 0xcd, 0xd3, 0x7f, 0x3c, 0x22, 0x22, 0xcc, 0x20,
];

pub fn session_keys(aura: sp_consensus_aura::sr25519::AuthorityId) -> SessionKeys {
    SessionKeys { aura }
}

pub fn local_genesis_config() -> RuntimeGenesisConfig {
    let alice: AccountId = ALICE.into();
    let bob: AccountId = BOB.into();
    let charlie: AccountId = CHARLIE.into();
    let dave: AccountId = DAVE.into();
    let invulnerables = vec![
        (
            alice.clone(),
            sp_core::sr25519::Public::from_raw(ALICE).into(),
        ),
        (bob.clone(), sp_core::sr25519::Public::from_raw(BOB).into()),
    ];

    RuntimeGenesisConfig {
        system: Default::default(),
        parachain_system: Default::default(),
        parachain_info: ParachainInfoConfig {
            parachain_id: ParaId::from(LOCAL_PARA_ID),
            ..Default::default()
        },
        balances: BalancesConfig {
            balances: vec![alice.clone(), bob.clone(), charlie.clone(), dave.clone()]
                .into_iter()
                .map(|account| (account, 1u128 << 60))
                .collect::<Vec<_>>(),
            ..Default::default()
        },
        transaction_payment: Default::default(),
        collator_selection: CollatorSelectionConfig {
            invulnerables: invulnerables
                .iter()
                .map(|(account, _)| account.clone())
                .collect(),
            candidacy_bond: EXISTENTIAL_DEPOSIT * 16,
            ..Default::default()
        },
        session: SessionConfig {
            keys: invulnerables
                .into_iter()
                .map(|(account, aura)| (account.clone(), account, session_keys(aura)))
                .collect(),
            ..Default::default()
        },
        aura: Default::default(),
        aura_ext: Default::default(),
        cubikan: CubikanConfig {
            deployment_id: DEPLOYMENT_ID,
            pallet_storage_version: PALLET_STORAGE_VERSION,
            event_schema_version: EVENT_SCHEMA_VERSION,
            authorized_submitters: vec![charlie, dave],
        },
    }
}

pub fn get_preset(id: &sp_genesis_builder::PresetId) -> Option<Vec<u8>> {
    match id.as_ref() {
        sp_genesis_builder::DEV_RUNTIME_PRESET
        | sp_genesis_builder::LOCAL_TESTNET_RUNTIME_PRESET => {
            let alice: AccountId = ALICE.into();
            let bob: AccountId = BOB.into();
            let charlie: AccountId = CHARLIE.into();
            let dave: AccountId = DAVE.into();
            let invulnerables = vec![
                (
                    alice.clone(),
                    sp_core::sr25519::Public::from_raw(ALICE).into(),
                ),
                (bob.clone(), sp_core::sr25519::Public::from_raw(BOB).into()),
            ];
            let patch = build_struct_json_patch!(RuntimeGenesisConfig {
                balances: BalancesConfig {
                    balances: vec![alice.clone(), bob.clone(), charlie.clone(), dave.clone()]
                        .into_iter()
                        .map(|account| (account, 1u128 << 60))
                        .collect::<Vec<_>>(),
                },
                parachain_info: ParachainInfoConfig {
                    parachain_id: ParaId::from(LOCAL_PARA_ID),
                },
                collator_selection: CollatorSelectionConfig {
                    invulnerables: invulnerables
                        .iter()
                        .map(|(account, _)| account.clone())
                        .collect(),
                    candidacy_bond: EXISTENTIAL_DEPOSIT * 16,
                },
                session: SessionConfig {
                    keys: invulnerables
                        .into_iter()
                        .map(|(account, aura)| { (account.clone(), account, session_keys(aura)) })
                        .collect(),
                },
                cubikan: CubikanConfig {
                    deployment_id: DEPLOYMENT_ID,
                    pallet_storage_version: PALLET_STORAGE_VERSION,
                    event_schema_version: EVENT_SCHEMA_VERSION,
                    authorized_submitters: vec![charlie, dave],
                },
            });
            serde_json::to_vec(&patch).ok()
        }
        _ => None,
    }
}

pub fn preset_names() -> Vec<sp_genesis_builder::PresetId> {
    vec![
        sp_genesis_builder::PresetId::from(sp_genesis_builder::DEV_RUNTIME_PRESET),
        sp_genesis_builder::PresetId::from(sp_genesis_builder::LOCAL_TESTNET_RUNTIME_PRESET),
    ]
}
