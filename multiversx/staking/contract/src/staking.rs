#![no_std]

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

// Rewards are distributed based on a global speed.
// We can use block production as speed reference (recommended approach)
// each block is created every 6 seconds
pub const BLOCKS_IN_YEAR: u64 = 60 * 60 * 24 * 365 / 6;

// Users earn rewards in proportion to their stake.
pub const REWARDS_PERCENTAGE: u64 = 10_000;

#[derive(TypeAbi, TopEncode, TopDecode, PartialEq, Debug)]
pub struct StakingPosition<M: ManagedTypeApi> {
    pub stake_amount: BigUint<M>,
    pub last_action_block: u64,
}

#[multiversx_sc::contract]
pub trait StakingContract {
    #[init] // constructor annotation
    fn init(&self) {}

    #[payable("EGLD")] // allow receive payment
    #[endpoint] // callable by users
    fn stake(&self) {
        let payment = self.call_value().egld_value().clone_value();
        require!(payment > 0, "Must pay more than 0"); // if !condition { signal_error(msg) }

        let stake_mapper = self.staking_position(&caller);

        let new_user = self.staked_addresses().insert(caller.clone());
        let mut staking_pos = if !new_user {
            stake_mapper.get()
        } else {
            let current_block = self.blockchain().get_block_epoch();
            StakingPosition {
                stake_amount: BigUint::zero(),
                last_action_block: current_block,
            }
        };

        self.claim_rewards_for_user(&caller, &mut staking_pos);
        staking_pos.stake_amount += payment_amount;

        stake_mapper.set(&staking_pos);
    }

    #[endpoint]
    fn unstake(&self, opt_unstake_amount: OptionalValue<BigUint>) {
        let caller = self.blockchain().get_caller();
        let stake_mapper = self.staking_position(&caller);
        let unstake_amount = match opt_unstake_amount {
            OptionalValue::Some(amt) => amt,
            OptionalValue::None => stake_mapper.get(),
        };
        let remaining_stake = stake_mapper.update(|stake_amount| {
            require!(
                unstake_amount > 0 && unstake_amount <= *stake_amount,
                "Invalid unstake amount"
            );
            *stake_amount -= &unstake_amount;
            stake_amount.clone()
        });

        if remaining_stake == 0 {
            self.staked_addresses().swap_remove(&caller);
        }

        self.send().direct_egld(&caller, &unstake_amount);
    }

    #[view(getStakingPosition)]
    #[storage_mapper("stakingPosition")]
    fn staking_position(&self, addr: &ManagedAddress) -> SingleValueMapper<BigUint>;

    #[view(getStakedAddresses)] // view = readonly methods, endpoint synom for now
    #[storage_mapper("stakedAddresses")] // multiple storage keys at once
    fn staked_addresses(&self) -> UnorderedSetMapper<ManagedAddress>;
}
