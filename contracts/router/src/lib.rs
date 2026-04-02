//! Stellar Money Bridge — Smart Router Contract
//!
//! Selects the optimal conversion path (cheapest + fastest) between
//! USDC and a target fiat rail (mobile money or bank).

#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Address, Env, String, Vec};

/// Supported fiat rails
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Rail {
    MobileMoney, // e.g. MTN, Airtel
    BankTransfer,
}

/// A registered liquidity provider / anchor
#[contracttype]
#[derive(Clone, Debug)]
pub struct Provider {
    pub id: Address,
    pub rail: Rail,
    pub fee_bps: u32,   // fee in basis points (e.g. 50 = 0.5%)
    pub active: bool,
}

/// Routing result returned to the caller
#[contracttype]
#[derive(Clone, Debug)]
pub struct RouteResult {
    pub provider: Address,
    pub fee_bps: u32,
    pub rail: Rail,
}

const PROVIDERS_KEY: &str = "providers";

#[contract]
pub struct RouterContract;

#[contractimpl]
impl RouterContract {
    /// Register a new liquidity provider (admin only — extend with auth as needed)
    pub fn register_provider(env: Env, provider: Provider) {
        let mut providers: Vec<Provider> = env
            .storage()
            .persistent()
            .get(&symbol_short!("provs"))
            .unwrap_or(Vec::new(&env));

        providers.push_back(provider);
        env.storage()
            .persistent()
            .set(&symbol_short!("provs"), &providers);
    }

    /// Find the cheapest active provider for a given rail
    pub fn get_best_route(env: Env, rail: Rail) -> Option<RouteResult> {
        let providers: Vec<Provider> = env
            .storage()
            .persistent()
            .get(&symbol_short!("provs"))
            .unwrap_or(Vec::new(&env));

        let mut best: Option<RouteResult> = None;

        for i in 0..providers.len() {
            let p = providers.get(i).unwrap();
            if !p.active || p.rail != rail {
                continue;
            }
            match &best {
                None => {
                    best = Some(RouteResult {
                        provider: p.id.clone(),
                        fee_bps: p.fee_bps,
                        rail: p.rail.clone(),
                    });
                }
                Some(current) if p.fee_bps < current.fee_bps => {
                    best = Some(RouteResult {
                        provider: p.id.clone(),
                        fee_bps: p.fee_bps,
                        rail: p.rail.clone(),
                    });
                }
                _ => {}
            }
        }

        best
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::Env;

    #[test]
    fn test_register_and_route() {
        let env = Env::default();
        let contract_id = env.register_contract(None, RouterContract);
        let client = RouterContractClient::new(&env, &contract_id);

        let provider_addr = Address::generate(&env);
        client.register_provider(&Provider {
            id: provider_addr.clone(),
            rail: Rail::MobileMoney,
            fee_bps: 50,
            active: true,
        });

        let route = client.get_best_route(&Rail::MobileMoney);
        assert!(route.is_some());
        assert_eq!(route.unwrap().fee_bps, 50);
    }
}
