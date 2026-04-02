//! Soroban Subscription Engine
//!
//! A trustless recurring payment protocol on Stellar/Soroban.
//! Merchants create plans; subscribers enroll and approve token spend;
//! a permissioned relayer triggers billing each cycle.

#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, token, Address, Env,
};

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/// Billing interval in seconds (off-chain relayer uses this to schedule calls)
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BillingCycle {
    Daily,   // 86_400 s
    Weekly,  // 604_800 s
    Monthly, // 2_592_000 s (~30 days)
}

impl BillingCycle {
    pub fn seconds(&self) -> u64 {
        match self {
            BillingCycle::Daily => 86_400,
            BillingCycle::Weekly => 604_800,
            BillingCycle::Monthly => 2_592_000,
        }
    }
}

/// A subscription plan created by a merchant
#[contracttype]
#[derive(Clone, Debug)]
pub struct Plan {
    pub merchant: Address,
    pub token: Address,
    pub amount: i128,
    pub cycle: BillingCycle,
    pub active: bool,
}

/// Status of a subscriber's subscription
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SubStatus {
    Active,
    Paused,
    Cancelled,
    GracePeriod,
}

/// A subscriber's subscription record
#[contracttype]
#[derive(Clone, Debug)]
pub struct Subscription {
    pub subscriber: Address,
    pub plan_id: u32,
    pub status: SubStatus,
    pub next_billing: u64, // Unix timestamp (ledger time)
    pub grace_ends: u64,   // 0 if not in grace period
}

// ---------------------------------------------------------------------------
// Storage keys
// ---------------------------------------------------------------------------

#[contracttype]
pub enum DataKey {
    Admin,
    Relayer,
    PlanCount,
    Plan(u32),
    Sub(Address, u32), // (subscriber, plan_id)
}

const GRACE_PERIOD: u64 = 86_400; // 24 h grace on failed payment

// ---------------------------------------------------------------------------
// Contract
// ---------------------------------------------------------------------------

#[contract]
pub struct SubscriptionContract;

#[contractimpl]
impl SubscriptionContract {
    // -----------------------------------------------------------------------
    // Initialisation
    // -----------------------------------------------------------------------

    /// Deploy and set admin + relayer addresses.
    pub fn initialize(env: Env, admin: Address, relayer: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Relayer, &relayer);
        env.storage().instance().set(&DataKey::PlanCount, &0u32);
    }

    // -----------------------------------------------------------------------
    // Plan management (merchant)
    // -----------------------------------------------------------------------

    /// Create a new subscription plan. Returns the plan_id.
    pub fn create_plan(
        env: Env,
        merchant: Address,
        token: Address,
        amount: i128,
        cycle: BillingCycle,
    ) -> u32 {
        merchant.require_auth();

        let plan_id: u32 = env
            .storage()
            .instance()
            .get(&DataKey::PlanCount)
            .unwrap_or(0);

        let plan = Plan {
            merchant: merchant.clone(),
            token,
            amount,
            cycle,
            active: true,
        };

        env.storage()
            .persistent()
            .set(&DataKey::Plan(plan_id), &plan);

        env.storage()
            .instance()
            .set(&DataKey::PlanCount, &(plan_id + 1));

        env.events().publish(
            (symbol_short!("plan_new"), merchant),
            plan_id,
        );

        plan_id
    }

    /// Deactivate a plan (no new subscriptions; existing ones stop billing).
    pub fn deactivate_plan(env: Env, merchant: Address, plan_id: u32) {
        merchant.require_auth();
        let mut plan: Plan = env
            .storage()
            .persistent()
            .get(&DataKey::Plan(plan_id))
            .expect("plan not found");
        assert_eq!(plan.merchant, merchant, "not plan owner");
        plan.active = false;
        env.storage()
            .persistent()
            .set(&DataKey::Plan(plan_id), &plan);
    }

    // -----------------------------------------------------------------------
    // Subscription lifecycle (subscriber)
    // -----------------------------------------------------------------------

    /// Subscribe to a plan. Subscriber must have pre-approved token spend.
    pub fn subscribe(env: Env, subscriber: Address, plan_id: u32) {
        subscriber.require_auth();

        let plan: Plan = env
            .storage()
            .persistent()
            .get(&DataKey::Plan(plan_id))
            .expect("plan not found");
        assert!(plan.active, "plan is not active");

        // Charge the first payment immediately
        let token_client = token::Client::new(&env, &plan.token);
        token_client.transfer(&subscriber, &plan.merchant, &plan.amount);

        let now = env.ledger().timestamp();
        let sub = Subscription {
            subscriber: subscriber.clone(),
            plan_id,
            status: SubStatus::Active,
            next_billing: now + plan.cycle.seconds(),
            grace_ends: 0,
        };

        env.storage()
            .persistent()
            .set(&DataKey::Sub(subscriber.clone(), plan_id), &sub);

        env.events().publish(
            (symbol_short!("subscribed"), subscriber),
            plan_id,
        );
    }

    /// Pause an active subscription (subscriber only).
    pub fn pause(env: Env, subscriber: Address, plan_id: u32) {
        subscriber.require_auth();
        let mut sub: Subscription = Self::get_sub(&env, &subscriber, plan_id);
        assert_eq!(sub.status, SubStatus::Active, "not active");
        sub.status = SubStatus::Paused;
        env.storage()
            .persistent()
            .set(&DataKey::Sub(subscriber.clone(), plan_id), &sub);
    }

    /// Resume a paused subscription (subscriber only).
    pub fn resume(env: Env, subscriber: Address, plan_id: u32) {
        subscriber.require_auth();
        let mut sub: Subscription = Self::get_sub(&env, &subscriber, plan_id);
        assert_eq!(sub.status, SubStatus::Paused, "not paused");
        // Reset next billing from now
        let plan: Plan = env
            .storage()
            .persistent()
            .get(&DataKey::Plan(plan_id))
            .expect("plan not found");
        sub.status = SubStatus::Active;
        sub.next_billing = env.ledger().timestamp() + plan.cycle.seconds();
        env.storage()
            .persistent()
            .set(&DataKey::Sub(subscriber.clone(), plan_id), &sub);
    }

    /// Cancel a subscription (subscriber only).
    pub fn cancel(env: Env, subscriber: Address, plan_id: u32) {
        subscriber.require_auth();
        let mut sub: Subscription = Self::get_sub(&env, &subscriber, plan_id);
        sub.status = SubStatus::Cancelled;
        env.storage()
            .persistent()
            .set(&DataKey::Sub(subscriber.clone(), plan_id), &sub);

        env.events().publish(
            (symbol_short!("cancelled"), subscriber),
            plan_id,
        );
    }

    // -----------------------------------------------------------------------
    // Billing execution (relayer only)
    // -----------------------------------------------------------------------

    /// Attempt to charge a subscriber for the next billing cycle.
    /// Called by the off-chain relayer when `next_billing <= now`.
    pub fn execute_billing(env: Env, subscriber: Address, plan_id: u32) {
        // Only the registered relayer may call this
        let relayer: Address = env
            .storage()
            .instance()
            .get(&DataKey::Relayer)
            .expect("not initialized");
        relayer.require_auth();

        let mut sub: Subscription = Self::get_sub(&env, &subscriber, plan_id);

        // Only bill active or grace-period subscriptions
        assert!(
            sub.status == SubStatus::Active || sub.status == SubStatus::GracePeriod,
            "subscription not billable"
        );

        let now = env.ledger().timestamp();
        assert!(now >= sub.next_billing, "not due yet");

        let plan: Plan = env
            .storage()
            .persistent()
            .get(&DataKey::Plan(plan_id))
            .expect("plan not found");

        if !plan.active {
            sub.status = SubStatus::Cancelled;
            env.storage()
                .persistent()
                .set(&DataKey::Sub(subscriber.clone(), plan_id), &sub);
            return;
        }

        let token_client = token::Client::new(&env, &plan.token);
        let balance = token_client.balance(&subscriber);

        if balance >= plan.amount {
            // Successful charge
            token_client.transfer(&subscriber, &plan.merchant, &plan.amount);
            sub.status = SubStatus::Active;
            sub.next_billing = now + plan.cycle.seconds();
            sub.grace_ends = 0;

            env.events().publish(
                (symbol_short!("billed"), subscriber.clone()),
                plan_id,
            );
        } else {
            // Insufficient balance — enter / extend grace period
            if sub.status == SubStatus::GracePeriod && now >= sub.grace_ends {
                // Grace expired: cancel
                sub.status = SubStatus::Cancelled;
                env.events().publish(
                    (symbol_short!("lapsed"), subscriber.clone()),
                    plan_id,
                );
            } else if sub.status == SubStatus::Active {
                sub.status = SubStatus::GracePeriod;
                sub.grace_ends = now + GRACE_PERIOD;
                env.events().publish(
                    (symbol_short!("grace"), subscriber.clone()),
                    plan_id,
                );
            }
        }

        env.storage()
            .persistent()
            .set(&DataKey::Sub(subscriber, plan_id), &sub);
    }

    // -----------------------------------------------------------------------
    // Read-only helpers
    // -----------------------------------------------------------------------

    pub fn get_plan(env: Env, plan_id: u32) -> Plan {
        env.storage()
            .persistent()
            .get(&DataKey::Plan(plan_id))
            .expect("plan not found")
    }

    pub fn get_subscription(env: Env, subscriber: Address, plan_id: u32) -> Subscription {
        Self::get_sub(&env, &subscriber, plan_id)
    }

    pub fn plan_count(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&DataKey::PlanCount)
            .unwrap_or(0)
    }

    // -----------------------------------------------------------------------
    // Internal
    // -----------------------------------------------------------------------

    fn get_sub(env: &Env, subscriber: &Address, plan_id: u32) -> Subscription {
        env.storage()
            .persistent()
            .get(&DataKey::Sub(subscriber.clone(), plan_id))
            .expect("subscription not found")
    }
}
