// This file is part of Webb.

// Copyright (C) 2021 Webb Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Anchor pallet benchmarking.

#![cfg(feature = "runtime-benchmarks")]

use super::*;

use frame_benchmarking::{
	account, benchmarks_instance_pallet, impl_benchmark_test_suite, whitelist_account,
	whitelisted_caller,
};
use frame_system::RawOrigin;
use orml_traits::MultiCurrency;
use webb_primitives::{anchor::AnchorInterface, traits::merkle_tree::TreeInspector, ElementTrait};
// Run the zk-setup binary before compiling the with runtime-benchmarks to
// generate the zk_config.rs file if it doesn't exist The accounts used in
// generating the proofs have to be the same accounts used in the withdraw
// benchmark
use crate::Pallet as Anchor;
use arkworks_setups::{common::setup_params, Curve};
use frame_support::{
	storage,
	traits::{Currency, Get, OnInitialize, PalletInfo},
};
use webb_primitives::runtime::*;
use zk_config::*;

fn assert_last_event<T: Config>(generic_event: <T as Config>::Event) {
	frame_system::Pallet::<T>::assert_last_event(generic_event.into());
}

pub fn hasher_params() -> Vec<u8> {
	let curve = Curve::Bn254;
	let params = setup_params::<ark_bn254::Fr>(curve, 5, 3);
	params.to_bytes()
}

const SEED: u32 = 0;
const MAX_EDGES: u32 = 256;

benchmarks_instance_pallet! {

	where_clause { where T: pallet::Config<I> + pallet_mt::Config<I> + pallet_hasher::Config<I>, <T as pallet_mt::Config<I>>::Element : From<Element> }
	create {
	  let i in 1..MAX_EDGES;
	  let d in 1..<T as pallet_mt::Config<I>>::MaxTreeDepth::get() as u32;
	  pallet_hasher::Pallet::<T, I>::force_set_parameters(RawOrigin::Root.into(), hasher_params()).unwrap();
	  let deposit_size: u32 = 1_000_000_000;
	  let asset_id = <<T as crate::Config<I>>::NativeCurrencyId as Get<crate::CurrencyIdOf<T, I>>>::get();
	}: _(RawOrigin::Root, deposit_size.into(), i, d as u8, asset_id)

	deposit {
	  let caller: T::AccountId = whitelisted_caller();
	  let deposit_size: u32 = 50_000_000;
	  pallet_hasher::Pallet::<T, I>::force_set_parameters(RawOrigin::Root.into(), hasher_params()).unwrap();
	  let asset_id = <<T as crate::Config<I>>::NativeCurrencyId as Get<crate::CurrencyIdOf<T, I>>>::get();
	  let depth = <T as pallet_mt::Config<I>>::MaxTreeDepth::get();

	  let tree_id = <Anchor<T, I> as AnchorInterface<AnchorConfigration<T, _>>>::create(None, deposit_size.into(), depth, MAX_EDGES as u32, asset_id)?;
	  let leaf = <T as pallet_mt::Config<I>>::Element::from_bytes(&[1u8; 32]);
	  <<T as pallet_mt::Config<I>>::Currency as Currency<T::AccountId>>::make_free_balance_be(&caller.clone(), 200_000_000u32.into());

	}: _(RawOrigin::Signed(caller.clone()), tree_id, leaf)
	verify {
	  assert_eq!(<<T as crate::Config<I>>::Currency as MultiCurrency<T::AccountId>>::total_balance(asset_id, &crate::Pallet::<T, I>::account_id()), deposit_size.into())
	}

	withdraw {
		let hasher_pallet_name = <T as frame_system::Config>::PalletInfo::name::<<T as pallet_mt::Config<I>>::Hasher>().unwrap();
		let verifier_pallet_name = <T as frame_system::Config>::PalletInfo::name::<<T as Config<I>>::Verifier>().unwrap();
		// 1. Setup The Hasher Pallet.
		storage::unhashed::put(&storage::storage_prefix(hasher_pallet_name.as_bytes(), "Parameters".as_bytes()),&HASH_PARAMS[..]);

		// 2. Initialize MerkleTree pallet
		<pallet_mt::Pallet<T, I> as OnInitialize<_>>::on_initialize(Default::default());
		storage::unhashed::put(&storage::storage_prefix(verifier_pallet_name.as_bytes(), "Parameters".as_bytes()),&VK_BYTES[..]);

		// inputs
		let caller: T::AccountId = whitelisted_caller();
		<<T as pallet_mt::Config<I>>::Currency as Currency<T::AccountId>>::make_free_balance_be(&caller.clone(), 200_000_000u32.into());
		let src_chain_id: u32 = 1;
		let recipient_account_id: T::AccountId = account("recipient", 0, SEED);
		let relayer_account_id: T::AccountId = account("relayer", 1, SEED);
		whitelist_account!(recipient_account_id);
		whitelist_account!(relayer_account_id);
		<<T as pallet_mt::Config<I>>::Currency as Currency<T::AccountId>>::make_free_balance_be(&recipient_account_id.clone(), 100_000_000u32.into());
		let fee_value: u32 = 0;
		let refund_value: u32 = 0;

		let deposit_size: u32 = 50_000_000;
		let depth = <T as pallet_mt::Config<I>>::MaxTreeDepth::get();
		let asset_id = <<T as crate::Config<I>>::NativeCurrencyId as Get<crate::CurrencyIdOf<T, I>>>::get();

		let tree_id = <Anchor<T, I> as AnchorInterface<AnchorConfigration<T, I>>>::create(None, deposit_size.into(), depth, 2, asset_id)?;

		<Anchor<T, I> as AnchorInterface<AnchorConfigration<T, I>>>::deposit(
			caller.clone(),
			tree_id,
			<T as pallet_mt::Config<I>>::Element::from_bytes(&LEAF[..]),
		)?;

		let tree_root = <pallet_mt::Pallet<T, I> as TreeInspector<T::AccountId, <T as pallet_mt::Config<I>>::TreeId, <T as pallet_mt::Config<I>>::Element>>::get_root(tree_id).unwrap();
		// sanity check.
		assert_eq!(<T as pallet_mt::Config<I>>::Element::from_bytes(&ROOT_ELEMENT_BYTES[0]), tree_root);

		let roots_element = ROOT_ELEMENT_BYTES
			.iter()
			.map(|v| <T as pallet_mt::Config<I>>::Element::from_bytes(&v[..]))
			.collect();

		let nullifier_hash_element = <T as pallet_mt::Config<I>>::Element::from_bytes(&NULLIFIER_HASH_ELEMENTS_BYTES[..]);
		let commitment_bytes = vec![0u8; 32];
		let commitment_element = Element::from_bytes(&commitment_bytes);
	}: _(
		RawOrigin::Signed(caller),
		tree_id,
		PROOF_BYTES.to_vec(),
		roots_element,
		nullifier_hash_element,
		recipient_account_id.clone(),
		relayer_account_id,
		fee_value.into(),
		refund_value.into(),
		commitment_element.into()
	)
	verify {
		assert_eq!(<<T as crate::Config<I>>::Currency as MultiCurrency<T::AccountId>>::total_balance(asset_id, &recipient_account_id), (100_000_000u32 + deposit_size).into())
	}

}

impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
