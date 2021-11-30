use crate::{
	mock::*,
	test_utils::{get_hash_params, prove, setup_circuit_with_raw_inputs, setup_keys, setup_random_circuit},
};
use ark_ff::{BigInteger, PrimeField};
use arkworks_gadgets::setup::common::Curve;
use codec::Encode;
use darkwebb_primitives::{
	hashing::ethereum::keccak256,
	merkle_tree::TreeInspector,
	types::vanchor::{ExtData, ProofData},
	utils::truncate_and_pad,
	AccountId, ElementTrait,
};
use frame_benchmarking::account;
use frame_support::{assert_err, assert_ok, error::BadOrigin, traits::OnInitialize};
use pallet_asset_registry::AssetType;
use std::convert::TryInto;

const SEED: u32 = 0;
const TREE_DEPTH: usize = 30;
const M: usize = 2;

pub fn get_account(id: u32) -> AccountId {
	account::<AccountId>("", id, SEED)
}

fn setup_environment(curve: Curve) -> Vec<u8> {
	let (_, params3, ..) = get_hash_params::<ark_bn254::Fr>(Curve::Bn254);
	// 1. Setup The Hasher Pallet.
	assert_ok!(HasherPallet::force_set_parameters(Origin::root(), params3.to_bytes()));
	// 2. Initialize MerkleTree pallet.
	<MerkleTree as OnInitialize<u64>>::on_initialize(1);
	// 3. Setup the VerifierPallet
	//    but to do so, we need to have a VerifyingKey

	let circuit = setup_random_circuit();
	let (proving_key_bytes, verifier_key_bytes) = setup_keys(circuit);

	assert_ok!(VerifierPallet::force_set_parameters(Origin::root(), verifier_key_bytes));
	// 4. and top-up some accounts with some balance
	for account_id in [
		account::<AccountId>("", 1, SEED),
		account::<AccountId>("", 2, SEED),
		account::<AccountId>("", 3, SEED),
		account::<AccountId>("", 4, SEED),
		account::<AccountId>("", 5, SEED),
		account::<AccountId>("", 6, SEED),
	] {
		assert_ok!(Balances::set_balance(Origin::root(), account_id, 100_000_000, 0));
	}

	// finally return the provingkey bytes
	proving_key_bytes
}

fn create_vanchor(asset_id: u32) -> u32 {
	let max_edges = M as u32;
	let depth = TREE_DEPTH as u8;
	assert_ok!(VAnchor::create(Origin::root(), max_edges, depth, asset_id));
	MerkleTree::next_tree_id() - 1
}

fn create_vanchor_with_deposits(amounts: &Vec<Balance>, leaves: &Vec<Element>) -> (u32, Element) {
	let tree_id = create_vanchor(0);
	for (leaf, amount) in leaves.iter().zip(amounts.iter()) {
		VAnchor::deposit(Origin::signed(get_account(1)), tree_id, *leaf, *amount).unwrap();
	}

	let on_chain_root = MerkleTree::get_root(tree_id).unwrap();

	(tree_id, on_chain_root)
}

#[test]
fn should_create_new_vanchor() {
	new_test_ext().execute_with(|| {
		let pub_key_bytes = setup_environment(Curve::Bn254);

		let recipient: AccountId = get_account(4);
		let relayer: AccountId = get_account(3);
		let ext_amount: Amount = 0;
		let fee: Balance = 0;

		let public_amount = 0;
		let in_chain_id = 0;
		let in_amounts = vec![5, 5];
		let out_chain_ids = vec![0, 0];
		let out_amounts = vec![5, 5];

		let (circuit, root_set, nullifiers, leaves, commitments, ext_data_hash) = setup_circuit_with_raw_inputs(
			public_amount,
			recipient.clone(),
			relayer.clone(),
			ext_amount,
			fee,
			in_chain_id,
			in_amounts.clone(),
			out_chain_ids,
			out_amounts,
		);

		let proof = prove(circuit, pub_key_bytes);

		let (tree_id, on_chain_root) = create_vanchor_with_deposits(&in_amounts, &leaves);
		assert_eq!(root_set[0], on_chain_root);

		let output1 = commitments[0].clone();
		let output2 = commitments[1].clone();
		let ext_data =
			ExtData::<AccountId, Amount, Balance, Element>::new(recipient, relayer, ext_amount, fee, output1, output2);

		let proof_data = ProofData::new(proof, root_set, nullifiers, commitments, public_amount, ext_data_hash);

		let transactor = get_account(1);
		assert_ok!(VAnchor::transact(
			Origin::signed(transactor),
			tree_id,
			proof_data,
			ext_data
		));
	});
}

// #[test]
// fn should_fail_to_create_new_vanchor_if_not_root() {
// 	new_test_ext().execute_with(|| {
// 		setup_environment(Curve::Bn254);
// 		let max_edges = M as _;
// 		let depth = TREE_DEPTH as u8;
// 		let asset_id = 0;
// 		assert_err!(
// 			VAnchor::create(
// 				Origin::signed(account::<AccountId>("", 1, SEED)),
// 				max_edges,
// 				depth,
// 				asset_id
// 			),
// 			BadOrigin,
// 		);
// 	});
// }

// #[test]
// fn should_be_able_to_transact() {
// 	new_test_ext().execute_with(|| {
// 		setup_environment(Curve::Bn254);
// 	});
// }

// #[test]
// fn should_fail_to_deposit_if_vanchor_not_found() {
// 	new_test_ext().execute_with(|| {
// 		setup_environment(Curve::Bn254);
// 	});
// }

// #[test]
// fn vanchor_works() {
// 	new_test_ext().execute_with(|| {
// 		let curve = Curve::Bn254;
// 		let pk_bytes = setup_environment(curve);

// 		// inputs
// 		let tree_id = create_vanchor(0);
// 		let src_chain_id = 1;
// 		let sender_account_id = account::<AccountId>("", 1, SEED);
// 		let recipient_account_id = account::<AccountId>("", 2, SEED);
// 		let relayer_account_id = account::<AccountId>("", 0, SEED);
// 		let fee_value = 0;
// 		let refund_value = 0;

// 		let recipient_bytes = truncate_and_pad(&recipient_account_id.encode()[..]);
// 		let relayer_bytes = truncate_and_pad(&relayer_account_id.encode()[..]);
// 	});
// }

// #[test]
// fn double_spending_should_fail() {
// 	new_test_ext().execute_with(|| {
// 		let curve = Curve::Bn254;
// 		let pk_bytes = setup_environment(curve);

// 		// inputs
// 		let tree_id = create_vanchor(0);
// 		let src_chain_id = 1;
// 		let sender_account_id = account::<AccountId>("", 1, SEED);
// 		let recipient_account_id = account::<AccountId>("", 2, SEED);
// 		let relayer_account_id = account::<AccountId>("", 0, SEED);
// 		let fee_value = 0;
// 		let refund_value = 0;

// 		let recipient_bytes = truncate_and_pad(&recipient_account_id.encode()[..]);
// 		let relayer_bytes = truncate_and_pad(&relayer_account_id.encode()[..]);
// 	});
// }

// #[test]
// fn should_fail_when_invalid_merkle_roots() {
// 	new_test_ext().execute_with(|| {
// 		let curve = Curve::Bn254;
// 		let pk_bytes = setup_environment(curve);

// 		// inputs
// 		let tree_id = create_vanchor(0);
// 		let src_chain_id = 1;
// 		let sender_account_id = account::<AccountId>("", 1, SEED);
// 		let recipient_account_id = account::<AccountId>("", 2, SEED);
// 		let relayer_account_id = account::<AccountId>("", 0, SEED);
// 		let fee_value = 0;
// 		let refund_value = 0;

// 		let recipient_bytes = truncate_and_pad(&recipient_account_id.encode()[..]);
// 		let relayer_bytes = truncate_and_pad(&relayer_account_id.encode()[..]);
// 	});
// }

// #[test]
// fn should_fail_with_when_any_byte_is_changed_in_proof() {
// 	new_test_ext().execute_with(|| {
// 		let curve = Curve::Bn254;
// 		let pk_bytes = setup_environment(curve);

// 		// inputs
// 		let tree_id = create_vanchor(0);
// 		let src_chain_id = 1;
// 		let sender_account_id = account::<AccountId>("", 1, SEED);
// 		let recipient_account_id = account::<AccountId>("", 2, SEED);
// 		let relayer_account_id = account::<AccountId>("", 0, SEED);
// 		let fee_value = 0;
// 		let refund_value = 0;

// 		let recipient_bytes = truncate_and_pad(&recipient_account_id.encode()[..]);
// 		let relayer_bytes = truncate_and_pad(&relayer_account_id.encode()[..]);
// 	});
// }

// #[test]
// fn should_fail_when_relayer_id_is_different_from_that_in_proof_generation() {
// 	new_test_ext().execute_with(|| {
// 		let curve = Curve::Bn254;
// 		let pk_bytes = setup_environment(curve);

// 		// inputs
// 		let tree_id = create_vanchor(0);
// 		let src_chain_id = 1;
// 		let sender_account_id = account::<AccountId>("", 1, SEED);
// 		let recipient_account_id = account::<AccountId>("", 2, SEED);
// 		let relayer_account_id = account::<AccountId>("", 0, SEED);
// 		let fee_value = 0;
// 		let refund_value = 0;

// 		let recipient_bytes = truncate_and_pad(&recipient_account_id.encode()[..]);
// 		let relayer_bytes = truncate_and_pad(&relayer_account_id.encode()[..]);
// 	});
// }

// #[test]
// fn should_fail_with_when_fee_submitted_is_changed() {
// 	new_test_ext().execute_with(|| {
// 		let curve = Curve::Bn254;
// 		let pk_bytes = setup_environment(curve);

// 		// inputs
// 		let tree_id = create_vanchor(0);
// 		let src_chain_id = 1;
// 		let sender_account_id = account::<AccountId>("", 1, SEED);
// 		let recipient_account_id = account::<AccountId>("", 2, SEED);
// 		let relayer_account_id = account::<AccountId>("", 0, SEED);
// 		let fee_value = 0;
// 		let refund_value = 0;

// 		let recipient_bytes = truncate_and_pad(&recipient_account_id.encode()[..]);
// 		let relayer_bytes = truncate_and_pad(&relayer_account_id.encode()[..]);
// 	});
// }

// #[test]
// fn should_fail_with_invalid_proof_when_account_ids_are_truncated_in_reverse()
// { 	new_test_ext().execute_with(|| {
// 		let curve = Curve::Bn254;
// 		let pk_bytes = setup_environment(curve);

// 		// inputs
// 		let tree_id = create_vanchor(0);
// 		let src_chain_id = 1;
// 		let sender_account_id = account::<AccountId>("", 1, SEED);
// 		let recipient_account_id = account::<AccountId>("", 2, SEED);
// 		let relayer_account_id = account::<AccountId>("", 0, SEED);
// 		let fee_value = 0;
// 		let refund_value = 0;

// 		let recipient_bytes =
// truncate_and_pad_reverse(&recipient_account_id.encode()[..]);
// 		let relayer_bytes =
// truncate_and_pad_reverse(&relayer_account_id.encode()[..]); 	});
// }

// #[test]
// fn vanchor_works_for_pool_tokens() {
// 	new_test_ext().execute_with(|| {
// 		let existential_balance: u32 = 1000;
// 		let first_token_id = AssetRegistry::register_asset(
// 			b"shib".to_vec().try_into().unwrap(),
// 			AssetType::Token,
// 			existential_balance.into(),
// 		)
// 		.unwrap();
// 		let second_token_id = AssetRegistry::register_asset(
// 			b"doge".to_vec().try_into().unwrap(),
// 			AssetType::Token,
// 			existential_balance.into(),
// 		)
// 		.unwrap();

// 		let pool_share_id = AssetRegistry::register_asset(
// 			b"meme".to_vec().try_into().unwrap(),
// 			AssetType::PoolShare(vec![second_token_id, first_token_id]),
// 			existential_balance.into(),
// 		)
// 		.unwrap();

// 		let curve = Curve::Bn254;
// 		let pk_bytes = setup_environment(curve);

// 		// inputs
// 		let tree_id = create_vanchor(pool_share_id);
// 		let src_chain_id = 1;
// 		let sender_account_id = account::<AccountId>("", 1, SEED);
// 		let recipient_account_id = account::<AccountId>("", 2, SEED);
// 		let relayer_account_id = account::<AccountId>("", 0, SEED);
// 		let fee_value = 0;
// 		let refund_value = 0;
// 		let balance = 30_000u32;

// 		assert_ok!(Currencies::update_balance(
// 			Origin::root(),
// 			sender_account_id.clone(),
// 			first_token_id,
// 			balance.into()
// 		));

// 		assert_ok!(Currencies::update_balance(
// 			Origin::root(),
// 			sender_account_id.clone(),
// 			second_token_id,
// 			balance.into()
// 		));

// 		assert_ok!(TokenWrapper::set_wrapping_fee(Origin::root(), 0));

// 		assert_ok!(TokenWrapper::wrap(
// 			Origin::signed(sender_account_id.clone()),
// 			first_token_id,
// 			pool_share_id,
// 			10000 as u128,
// 			sender_account_id.clone()
// 		));

// 		assert_ok!(TokenWrapper::wrap(
// 			Origin::signed(sender_account_id.clone()),
// 			second_token_id,
// 			pool_share_id,
// 			10000 as u128,
// 			sender_account_id.clone()
// 		));

// 		assert_eq!(Tokens::total_issuance(pool_share_id), 20_000u32.into());

// 		let recipient_bytes = truncate_and_pad(&recipient_account_id.encode()[..]);
// 		let relayer_bytes = truncate_and_pad(&relayer_account_id.encode()[..]);
// 	});
// }

// #[test]
// fn should_run_post_deposit_hook_sucessfully() {
// 	new_test_ext().execute_with(|| {
// 		setup_environment(Curve::Bn254);

// 		let max_edges = M as _;
// 		let depth = TREE_DEPTH as _;
// 		let asset_id = 0;
// 		assert_ok!(VAnchor::create(Origin::root(), max_edges, depth, asset_id));

// 		let tree_id = MerkleTree::next_tree_id() - 1;
// 		let account_id = account::<AccountId>("", 1, SEED);
// 		let leaf = Element::from_bytes(&[1u8; 32]);
// 		// check the balance before the deposit.
// 		let balance_before = Balances::free_balance(account_id.clone());
// 		println!("Balance before: {}", balance_before);
// 	});
// }
