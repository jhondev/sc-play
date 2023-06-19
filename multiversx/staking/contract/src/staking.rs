#![no_std]

multiversx_sc::imports!();

#[multiversx_sc::contract]
pub trait StakingContract {
    #[init] // constructor annotation
    fn init(&self) {}

    #[payable("EGLD")] // allow receive payment
    #[endpoint] // callable by users
    fn stake(&self) {
        let payment = self.call_value().egld_value().clone_value();
        require!(payment > 0, "Must pay more than 0"); // if !condition { signal_error(msg) }

        let caller = self.blockchain().get_caller();
        self.staking_position(&caller)
            .update(|current| *current += payment);
        self.staked_addresses().insert(caller);
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
