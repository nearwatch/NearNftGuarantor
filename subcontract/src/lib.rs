use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, Balance, ext_contract, PublicKey, Gas, near_bindgen, AccountId, PromiseResult, Promise};
use near_sdk::json_types::{U128};
use near_sdk::collections::{LookupMap};

pub fn get_wallet(account: AccountId) -> AccountId {
	let s1 = account.to_string();
	let s2: String = s1.chars().skip(s1.len()-7).take(7).collect();
	let s3 = match s2 == "testnet" {true => "nftsale.testnet", false => "nftsale.near"};
	AccountId::new_unchecked(s3.to_string())
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct Contract {
	tokens: LookupMap<String, Balance>,
	owner: AccountId
}
impl Default for Contract {
	fn default() -> Self {
		Self {
			tokens: LookupMap::new(b"a".to_vec()), 
			owner : AccountId::new_unchecked("near".to_string())
		}
	}
}

#[ext_contract(ext_nft)]
pub trait NFT {
    fn nft_transfer(receiver_id:AccountId, token_id:String);
}
#[ext_contract(ext_master)]
pub trait Master {
    fn on_transfer(succeeded:bool, price:U128, owner:AccountId);
}
#[ext_contract(ext_self)]
pub trait ExtContract {
    fn on_nft_transfer(&mut self, price:U128, owner:AccountId, nft_key:String) -> bool;
}
fn is_promise_success() -> bool {
    assert_eq!(env::promise_results_count(),1,"Contract expected a result on the callback");
    match env::promise_result(0) {PromiseResult::Successful(_) => true, _ => false}
}

#[near_bindgen]
impl Contract {

	#[private]
    pub fn on_nft_transfer(&mut self, price:U128, owner:AccountId, nft_key:String) -> bool {
        let succeeded = is_promise_success();
        if succeeded {
			env::log_str(&"NFT transfer completed successfully");
			self.tokens.remove(&nft_key);
        } else {
			env::log_str(&"NFT transfer failed");
        }
		ext_master::on_transfer(succeeded, price, owner, get_wallet(env::predecessor_account_id()), 0, Gas(20_000_000_000_000));
		succeeded
    }
	#[payable]
    pub fn buy(&mut self, contract_id:String, token_id:String, pay_value: U128){
        assert_eq!(env::predecessor_account_id(), get_wallet(env::predecessor_account_id()), "Access denied");
		match self.tokens.get(&format!("{}|{}",contract_id,token_id)) {
			Some(price) => {
				assert_eq!(price > 0, true, "NFT is not available for sale");
				assert_eq!(u128::from(pay_value) < price, false, "Not enough deposit to buy an account");			
				ext_nft::nft_transfer(env::signer_account_id(), token_id.clone(), AccountId::new_unchecked(contract_id.clone()), 1, Gas(20_000_000_000_000))
				.then(ext_self::on_nft_transfer(pay_value, self.owner.clone(), format!("{}|{}",&contract_id, token_id), env::current_account_id(), 0, Gas(40_000_000_000_000)));
			},
			None => {
				panic!("NFT is not available for sale");
			}
		}
    }
    pub fn set_price(&mut self, contract_id:String, token_id:String, price: U128){
        assert_eq!(env::predecessor_account_id(), get_wallet(env::predecessor_account_id()), "Access denied");
		self.tokens.insert(&format!("{}|{}",contract_id,token_id),&price.into());
		self.owner = env::signer_account_id();
    }
    pub fn get_price(&self, contract_id:String, token_id:String) -> Balance {
		match self.tokens.get(&format!("{}|{}",contract_id,token_id)){
			Some(price) => price,
			None => 0
		}
    }
    pub fn destroy(beneficiary_id:AccountId) {
        assert_eq!(env::predecessor_account_id(), get_wallet(env::predecessor_account_id()), "Access denied");
        Promise::new(env::current_account_id()).delete_account(beneficiary_id);
    }
    pub fn unlock(public_key:PublicKey) {
        assert_eq!(env::predecessor_account_id(), get_wallet(env::predecessor_account_id()), "Access denied");
        Promise::new(env::current_account_id()).add_full_access_key(public_key.into());
    }
    pub fn withdraw(&mut self, nft_contract:AccountId, token_id: String) {
        assert_eq!(env::predecessor_account_id(), get_wallet(env::predecessor_account_id()), "Access denied");
		self.tokens.remove(&format!("{}|{}",&nft_contract,token_id));
		ext_nft::nft_transfer(env::signer_account_id(), token_id, nft_contract, 1, Gas(20_000_000_000_000));
    }
}
