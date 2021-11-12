use crate::{
    mimc_tree::{Hash, MimcTree, Proof},
    solidity::{ContractSigner, SemaphoreContract},
};
use ethers::prelude::Middleware;
use eyre::{bail, Error as EyreError, Result as EyreResult};
use std::convert::TryInto;

pub type Commitment = Hash;

pub fn inclusion_proof_helper(tree: &MimcTree, commitment: &str) -> Result<Proof, EyreError> {
    let decoded_commitment = hex::decode(commitment)?;
    let decoded_commitment: Commitment = (&decoded_commitment[..]).try_into()?;
    if let Some(index) = tree.position(&decoded_commitment) {
        return Ok(tree.proof(index));
    }
    bail!("Commitment not found {}", commitment);
}

pub fn insert_identity_commitment(
    tree: &mut MimcTree,
    commitment: &str,
    index: usize,
) -> EyreResult<()> {
    let decoded_commitment = hex::decode(commitment)?;
    let commitment: Commitment = (&decoded_commitment[..]).try_into()?;
    tree.set(index, commitment);
    Ok(())
}

pub async fn insert_identity_to_contract(
    semaphore_contract: &SemaphoreContract,
    signer: &ContractSigner,
    commitment: &str,
) -> EyreResult<bool> {
    let decoded_commitment = hex::decode(commitment).unwrap();
    let tx = semaphore_contract.insert_identity(decoded_commitment[..].into());
    let pending_tx = signer.send_transaction(tx.tx, None).await.unwrap();
    pending_tx.await?.unwrap();
    Ok(true)
}
