#![no_std]

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

// Rewards are distributed based on a global speed.
// We can use block production as speed reference (recommended approach)
// each block is created every 6 seconds
pub const BLOCKS_IN_YEAR: u64 = 60 * 60 * 24 * 365 / 6;

// Users earn rewards in proportion to their stake (10%).
// APY = Annual Percentage Yield
pub const APY: u64 = 1_000; // 10%
                            // Max percentage to fix precision
pub const MAX_PCT: u64 = 10_000;

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
        let payment_amount = self.call_value().egld_value().clone_value();
        require!(payment_amount > 0, "Must pay more than 0"); // if !condition { signal_error(msg) }

        let caller = self.blockchain().get_caller();
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
        self.require_user_staked(&caller);

        let stake_mapper = self.staking_position(&caller);
        let mut staking_pos = stake_mapper.get();

        let unstake_amount = match opt_unstake_amount {
            OptionalValue::Some(amt) => amt,
            OptionalValue::None => staking_pos.stake_amount.clone(),
        };
        require!(
            unstake_amount > 0 && unstake_amount <= staking_pos.stake_amount,
            "Invalid unstake amount"
        );

        self.claim_rewards_for_user(&caller, &mut staking_pos);
        staking_pos.stake_amount -= &unstake_amount;

        if staking_pos.stake_amount > 0 {
            stake_mapper.set(&staking_pos);
        } else {
            stake_mapper.clear();
            self.staked_addresses().swap_remove(&caller);
        }

        self.send().direct_egld(&caller, &unstake_amount);
    }

    #[endpoint(claimRewards)]
    fn claim_rewards(&self) {
        let caller = self.blockchain().get_caller();
        self.require_user_staked(&caller);

        let stake_mapper = self.staking_position(&caller);
        let mut staking_pos = stake_mapper.get();
        self.claim_rewards_for_user(&caller, &mut staking_pos);

        stake_mapper.set(&staking_pos);
    }

    fn require_user_staked(&self, user: &ManagedAddress) {
        require!(self.staked_addresses().contains(user), "Must stake first");
    }

    fn claim_rewards_for_user(
        &self,
        user: &ManagedAddress,
        staking_pos: &mut StakingPosition<Self::Api>,
    ) {
        let reward_amount = self.calculate_rewards(staking_pos);
        let current_block = self.blockchain().get_block_nonce();
        staking_pos.last_action_block = current_block;

        if reward_amount > 0 {
            self.send().direct_egld(user, &reward_amount);
        }
    }

    fn calculate_rewards(&self, staking_position: &StakingPosition<Self::Api>) -> BigUint {
        let current_block = self.blockchain().get_block_nonce();
        if current_block <= staking_position.last_action_block {
            return BigUint::zero();
        }

        let block_diff = current_block - staking_position.last_action_block;

        &staking_position.stake_amount * APY / MAX_PCT * block_diff / BLOCKS_IN_YEAR
    }

    #[view(calculateRewardsForUser)]
    fn calculate_rewards_for_user(&self, addr: ManagedAddress) -> BigUint {
        let staking_pos = self.staking_position(&addr).get();
        self.calculate_rewards(&staking_pos)
    }

    #[view(getStakingPosition)]
    #[storage_mapper("stakingPosition")]
    fn staking_position(
        &self,
        addr: &ManagedAddress,
    ) -> SingleValueMapper<StakingPosition<Self::Api>>;

    #[view(getStakedAddresses)] // view = readonly methods, endpoint synom for now
    #[storage_mapper("stakedAddresses")] // multiple storage keys at once
    fn staked_addresses(&self) -> UnorderedSetMapper<ManagedAddress>;
}
