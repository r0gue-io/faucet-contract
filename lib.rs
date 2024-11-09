#![cfg_attr(not(feature = "std"), no_std, no_main)]

use ink::{
	storage::Mapping,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[ink::scale_derive(Encode, Decode, TypeInfo)]
pub enum FaucetError {
	InCoolDown,
	NotActive,
	NotEnoughFunds,
	NotOwner,
	ValueTooLarge,
}

#[ink::contract]
mod fungibles {
	use super::*;

	/// Some tokens have been dripped.
	#[ink(event)]
	pub struct Drip {
		value: Balance,
		to: AccountId,
	}

	#[ink(storage)]
	pub struct Faucet {
		// Whether this faucet is active.
		active: bool,
		// Number of blocks an account should wait between drip requests.
		cooldown: BlockNumber,
		// Amount of tokens to drip per request.
		drip_amount: Balance,
		// Account owner of the contract. Set to the deployer at constructor.
		owner: Option<AccountId>,
		// Accounting of last request per account.
		last_request_of: Mapping<AccountId, BlockNumber>,
	}

	impl Faucet {
		/// Instantiate the faucet with the given cooldown and drip amount.
		/// Deployer becomes the contract owner.
		///
		/// # Parameters
		/// * - `cooldown` - Number of blocks an account should wait between drip requests.
		/// * - `drip_amount` - Amount of tokens to drip per `drip` call.
		#[ink(constructor, payable)]
		pub fn new(cooldown: BlockNumber, drip_amount: Balance) -> Self {
			Self {
				active: false,
				cooldown,
				drip_amount,
				owner: Some(Self::env().caller()),
				last_request_of: Mapping::default(),
			}
		}

		/// Check if the faucet is active.
		fn ensure_active(&self) -> Result<(), FaucetError> {
			if !&self.active {
				return Err(FaucetError::NotActive);
			}
			Ok(())
		}

		/// Check if the caller is the owner of the contract.
		fn ensure_owner(&self) -> Result<(), FaucetError> {
			if self.owner != Some(self.env().caller()) {
				return Err(FaucetError::NotOwner);
			}
			Ok(())
		}

		/// Check if caller can request a drip.
		fn can_request(&self) -> Result<(), FaucetError> {
			let caller = Self::env().caller();
			let last_request_result = self.last_request_of.try_get(caller);

			match last_request_result {
				Some(Ok(last_drip)) => {
					let current_block = self.env().block_number();
					if last_drip.saturating_add(self.cooldown) > current_block {
						return Err(FaucetError::InCoolDown);
					}
				}
				Some(Err(_)) => {
					// if either
					// - (a) the encoded key doesn’t fit into the static buffer
					// - (b) the value existed but its length exceeds the static buffer size.
					return Err(FaucetError::ValueTooLarge);
				}
				None => {
					return Ok(());
				}
			}

			Ok(())
		}

		/// Check if faucet holds enough balance to drip.
		fn can_withdraw(&self) -> Result<(), FaucetError> {
			// Don't let balance go under 1.
			if self.drip_amount.saturating_add(1) >= self.env().balance() {
				return Err(FaucetError::NotEnoughFunds);
			}
			Ok(())
		}

		/// Faucet's cooldown.
		#[ink(message)]
		pub fn cooldown(&self) -> BlockNumber {
			self.cooldown
		}

		/// Faucet's drip amount.
		#[ink(message)]
		pub fn drip_amount(&self) -> Balance {
			self.drip_amount
		}

		/// Whether Faucet is active or not.
		#[ink(message)]
		pub fn is_active(&self) -> bool {
			self.active
		}

		/// Caller's last drip block number.
		#[ink(message)]
		pub fn last_request_of(&self) -> Option<BlockNumber> {
			self.last_request_of.get(self.env().caller())
		}

		/// Faucet owner account, if there is one.
		#[ink(message)]
		pub fn owner(&self) -> Option<AccountId> {
			self.owner
		}

		/// Transfer drip_amount tokens to the caller.
		/// if:
		/// - faucet is active,
		/// - caller is not in cooldown,
		/// - faucet holds enough funds.
		#[ink(message)]
		pub fn drip(&mut self) -> Result<(), FaucetError> {
			self.ensure_active()?;
			self.can_withdraw()?;
			self.can_request()?;

			let caller = self.env().caller();

			// Do drip.
			self.env()
				.transfer(caller, self.drip_amount).expect("Some tokens have been transferred");
			// Register drip block# for caller.
			self.last_request_of
				.try_insert(caller, &self.env().block_number())
				.map_err(|_| FaucetError::ValueTooLarge)?;
			// Notify.
			self.env().emit_event(
				Drip {
					value: self.drip_amount,
					to: self.env().caller(),
				}
			);
			Ok(())
		}

		/// Mutate the value of cooldown.
		///
		/// # Parameters
		/// - `cooldown` - New cooldown time.
		#[ink(message)]
		pub fn set_cooldown(&mut self, cooldown: BlockNumber) -> Result<(), FaucetError> {
			self.ensure_owner()?;
			self.cooldown = cooldown;
			Ok(())
		}

		/// Activates or deactivates the faucet.
		/// The faucet will only drip tokens while active.
		#[ink(message)]
		pub fn start_stop(&mut self) -> Result<(), FaucetError> {
			self.ensure_owner()?;
			self.active = !self.active;
			Ok(())
		}

		/// Removes the owner of the contract.
		/// Effectively disable any action that requires ownership authorization.
		#[ink(message)]
		pub fn remove_ownership(&mut self) -> Result<(), FaucetError> {
			self.ensure_owner()?;
			self.owner = None;
			Ok(())
		}

		/// Mutate the value of drip_amount.
		///
		/// # Parameters
		/// - `drip_amount` - New drip amount.
		#[ink(message)]
		pub fn set_drip_amount(&mut self, drip_amount: Balance) -> Result<(), FaucetError> {
			self.ensure_owner()?;
			self.drip_amount = drip_amount;
			Ok(())
		}

		/// Transfer the ownership of the contract to another account.
		///
		/// # Parameters
		/// - `owner` - New owner account.
		#[ink(message)]
		pub fn transfer_ownership(&mut self, new_owner: AccountId) -> Result<(), FaucetError> {
			self.ensure_owner()?;
			self.owner = Some(new_owner);
			Ok(())
		}
	}
}
