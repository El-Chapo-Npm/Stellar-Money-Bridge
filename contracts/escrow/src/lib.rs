//! Stellar Money Bridge — Escrow Contract
//!
//! Locks USDC until off-chain settlement is confirmed, then releases
//! funds to the provider or refunds the sender on failure.

#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, token, Address, Env,
};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EscrowStatus {
    Pending,
    Settled,
    Refunded,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct Escrow {
    pub sender: Address,
    pub provide