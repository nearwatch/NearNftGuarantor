use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, ext_contract, PublicKey, AccountId, require, Promise, PromiseResult, Gas, near_bindgen};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::json_types::{U128};
use near_sdk::utils::{assert_self};
use near_sdk::collections::{LookupMap};

pub fn get_wallet(account: AccountId, subaccount:String) -> AccountId {
	let s1 = account.to_string();
	let s2: String = s1.chars().skip(s1.len()-7).take(7).collect();
	let s3 = match s2 == "testnet" {true => format!("{}.nftsale.testnet",subaccount), false => format!("{}.nftsale.near",subaccount)};
	AccountId::new_unchecked(s3.to_string())
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct Contract {
	accounts: LookupMap<AccountId, String>,
}
impl Default for Contract {
	fn default() -> Self {
		Self {
			accounts: LookupMap::new(b"a".to_vec()), 
		}
	}
}
#[allow(non_snake_case)]
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct Web4Response {
    contentType: String,
    body: Vec<u8>,
    preloadUrls: Vec<String>,
}
#[ext_contract(ext_self)]
pub trait ExtContract {
    fn on_create(&mut self, subaccount:String, account:AccountId) -> bool;
    fn on_payment(&mut self, account:AccountId, price:u128) -> bool;
}
#[ext_contract(ext_nftslave)]
pub trait NFTslave {
    fn withdraw(nft_contract:AccountId, token_id:String);
    fn set_price(contract_id:String, token_id:String, price:U128);
    fn buy(contract_id:String, token_id:String, pay_value:U128);
    fn unlock(public_key:PublicKey);
    fn destroy(beneficiary_id:AccountId);
}
fn is_promise_success() -> bool {
    assert_eq!(env::promise_results_count(),1,"Contract expected a result on the callback");
    match env::promise_result(0) {PromiseResult::Successful(_) => true, _ => false}
}

#[near_bindgen]
impl Contract {
    pub fn web4_get(&self) -> Web4Response {
        let (content_type, body) = ("text/html",include_bytes!("../../index.html").to_vec());
        Web4Response {contentType:content_type.to_string(), body, preloadUrls: vec![]}
    }
    pub fn on_transfer(&mut self, succeeded:bool, price:U128, owner:AccountId){
		match self.accounts.get(&owner){
			Some(sub) => {
				assert_eq!(env::predecessor_account_id(), get_wallet(env::predecessor_account_id(),sub), "Access denied");
				let value = u128::from(price);
				if succeeded {
					env::log_str(&format!("NFT transfered to @{}", env::signer_account_id()));
					Promise::new(owner).transfer(value - value/100);
				} else {
					env::log_str(&format!("Failed to buy NFT. Deposit redirected to @{}", env::signer_account_id()));
					Promise::new(env::signer_account_id()).transfer(value);
				}
			},
			None => env::log_str("Access denied")
		}
	}
    pub fn on_payment(&mut self, account:AccountId, price:u128) -> bool {
        assert_self();
        let succeeded = is_promise_success();
        if !succeeded {
			env::log_str(&format!("Failed to buy NFT. Deposit redirected to @{}", account));
			Promise::new(account).transfer(price);
        }
		succeeded
    }
    pub fn on_create(&mut self, subaccount:String, account:AccountId) -> bool {
        assert_self();
		let sub_name = get_wallet(account.clone(), subaccount.clone());
        let succeeded = is_promise_success();
        if succeeded {
			self.accounts.insert(&account,&subaccount);
            env::log_str(&format!("Account @{} creation completed successfully", sub_name));
        } else {
			env::log_str(&format!("Failed to create @{}", sub_name));
        }
		succeeded
    }
    pub fn get_subaccount(&self, account:AccountId) -> String {
		match self.accounts.get(&account){
			Some(acc) => acc,
			None => "".to_string()
		}
    }
    pub fn unlock_sub(&mut self, sub_account:AccountId, public_key:PublicKey) {
        assert_self();
		ext_nftslave::unlock(public_key, sub_account, 0, Gas(20_000_000_000_000));
    }
	#[payable]
    pub fn destroy_subaccount(&mut self) {  
		require!(env::attached_deposit() >= 100_000_000_000_000_000_000_000,"Not enough deposit to destroy a subaccount");
		let sub = self.accounts.get(&env::predecessor_account_id());
		require!(sub.is_some(),"No market account found");
		match sub {
			Some(name) => {
				let sub_name = get_wallet(env::predecessor_account_id(), name);
				self.accounts.remove(&env::predecessor_account_id());
				ext_nftslave::destroy(env::predecessor_account_id(), sub_name, 0, Gas(20_000_000_000_000));
			},
			None => {}
		};
		
	}
	#[payable]
    pub fn nft_buy(&mut self, nft_contract:String, token_id:String, subaccount:String){  
		env::log_str(&format!("@{} buy {} of @{} from @{}, value:{}", env::predecessor_account_id(), token_id, nft_contract, subaccount, env::attached_deposit()));
		ext_nftslave::buy(nft_contract, token_id, env::attached_deposit().into(), AccountId::new_unchecked(subaccount), 1, Gas(80_000_000_000_000))
		.then(ext_self::on_payment(env::predecessor_account_id(), env::attached_deposit().into(), env::current_account_id(), 0, Gas(20_000_000_000_000)));
	}
	#[payable]
    pub fn nft_price(&mut self, nft_contract:String, token_id:String, price:U128){  
		let sub = self.accounts.get(&env::predecessor_account_id());
		require!(sub.is_some(),"No market account found");
		match sub {
			Some(name) => {
				let sub_name = get_wallet(env::predecessor_account_id(), name);
				ext_nftslave::set_price(nft_contract, token_id, price, sub_name, 0, Gas(20_000_000_000_000));
			},
			None => ()
		};
	}
	#[payable]
    pub fn nft_withdraw(&mut self, nft_contract:AccountId, token_id:String){  
		let sub = self.accounts.get(&env::predecessor_account_id());
		require!(sub.is_some(),"No market account found");
		match sub {
			Some(name) => {
				let sub_name = get_wallet(env::predecessor_account_id(), name);
				ext_nftslave::withdraw(nft_contract, token_id, sub_name, 0, Gas(40_000_000_000_000));
			},
			None => ()
		};
	}
	#[payable]
    pub fn create_subaccount(&mut self, subaccount:String) {  
		require!(env::attached_deposit() >= 200_000_000_000_000_000_000_000,"Not enough deposit to create a subaccount");
		require!(self.accounts.get(&env::predecessor_account_id()).is_none(),"You cannot create more than one market account");
		let sub_name = get_wallet(env::current_account_id(), subaccount.clone());
		let subcode = include_bytes!("sub.wasm").to_vec();
		Promise::new(sub_name).create_account().transfer(1_600_000_000_000_000_000_000_000).deploy_contract(subcode)
		.then(ext_self::on_create(subaccount, env::predecessor_account_id(), env::current_account_id(), 0, Gas(20_000_000_000_000)));
	}
}
