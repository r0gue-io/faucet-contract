# 🚰 ink! Faucet

This repository showcases an ink! smart contract implementing a faucet. 

## Use the Faucet.

The smart contract is deployed in [Pop Network Testnet](https://polkadot.js.org/apps/?rpc=wss%3A%2F%2Frpc1.paseo.popnetwork.xyz#/explorer) on the address:
`5CwTqjdN28XfcTGsaC5VbyxP8wHAfbaKMwhhGA9bNJjaSDUJ`.

To interact with it through https://contracts.onpop.io/. 

- Find the metadata of the deployed version of the contract in `./metadata/faucet.contract`.
- Register it via `Add new contract` > `Use On-Chain contract address` using the above address + the metadata provided.

Call `drip()` and get some tokens!

If you want to call this faucet contract from another contract, please refer to: https://use.ink/basics/cross-contract-calling

---

## About the contract

It's main entry point for users to call is `drip()`:
```rust
/// Transfer drip_amount tokens to caller.
/// if:
/// - faucet is active,
/// - caller is not in cooldown,
/// - faucet holds enough funds.
#[ink(message)]
pub fn drip(&mut self) -> Result<(), FaucetError> {}
```

The contract is built around the following parameters:

```rust
#[ink(storage)]
pub struct Faucet {
    // Whether this faucet is active.
    active: bool,
    // Number of blocks an account should wait between drip requests.
    cooldown: BlockNumber,
    // Amount of tokens to drip per request.
    drip_amount: Balance,
    // Account owner of the contract. Set to the deployer at constructor.
    owner: Some(AccountId),
    // Accounting of last request per account.
    last_request_of: Mapping<AccountId, BlockNumber>,
}
```

Allowing the deployer to configure `drip_amount` and `cooldown`:
```rust
/// Instantiate the faucet with the given cooldown and drip amount.
/// Deployer becomes the contract owner.
///
/// # Parameters
/// * - `cooldown` - Number of blocks an account should wait between drip requests.
/// * - `drip_amount` - Amount of tokens to drip per `drip` call.
#[ink(constructor, payable)]
pub fn new(cooldown: BlockNumber, drip_amount: Balance) -> Self {}
```
Such that every time a `drip()` call is made, the contract will verify that the caller is not in cooldown, and if there are sufficient funds, a transfer of `drip_amount` tokens will be made from the contract account, to the caller's account.
The contract checks this condition holds like so:

```rust
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
```
```rust
/// Check if faucet holds enough balance to drip.
fn can_withdraw(&self) -> Result<(), FaucetError> {
    // Don't let balance go lower than 1.
    if self.drip_amount.saturating_add(1) >= self.env().balance() {
        return Err(FaucetError::NotEnoughFunds);
    }
    Ok(())
}
```
