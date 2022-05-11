mod abi;

use self::abi::{MemberAddedFilter, SemaphoreAirdrop};
use crate::ethereum::{Ethereum, ProviderStack};
use ethers::types::{Address, U256, U64};
use eyre::{eyre, Result as EyreResult};
use semaphore::Field;
use structopt::StructOpt;
use tracing::{info, instrument};
use ethers::providers::Middleware;
use tracing::error;

#[derive(Clone, Debug, PartialEq, StructOpt)]
pub struct Options {
    /// Semaphore contract address.
    #[structopt(long, env, default_value = "174ee9b5fBb5Eb68B6C61032946486dD9c2Dc4b6")]
    pub semaphore_address: Address,

    /// Mock mode: do not actually submit transactions.
    #[structopt(
        short,
        parse(try_from_str),
        default_value = "false",
        env = "SIGNUP_SEQUENCER_MOCK"
    )]
    pub mock: bool,
}

pub struct Contracts {
    ethereum:  Ethereum,
    semaphore: SemaphoreAirdrop<ProviderStack>,
    mock:      bool,
}

impl Contracts {
    #[instrument(level = "debug", skip_all)]
    pub async fn new(options: Options, ethereum: Ethereum) -> EyreResult<Self> {
        let address = options.semaphore_address;

        // Sanity check the address
        // TODO: Check that the contract is actually a Semaphore by matching bytecode.
        let code = ethereum.provider().get_code(address, None).await?;
        if code.as_ref().is_empty() {
            error!(?address, "No contract code deployed at provided Semaphore address");
        }

        // Connect to Contract
        let semaphore = SemaphoreAirdrop::new(options.semaphore_address, ethereum.provider().clone());

        // Test contract by calling a view function and make sure we are manager.
        let manager = semaphore.manager().call().await?;
        if manager != ethereum.address() {
            error!(?manager, signer = ?ethereum.address(), "Signer is not the manager of the Semaphore contract");
        }

        Ok(Self {
            ethereum,
            semaphore,
            mock: options.mock,
        })
    }

    pub async fn send_tx() {
        todo!();
        // let commitment = U256::from(commitment.to_be_bytes());
        // let mut tx = self.semaphore.add_member(group_id.into(), commitment);
        // let pending_tx = if self.eip1559 {
        // self.provider.fill_transaction(&mut tx.tx, None).await?;
        // tx.tx.set_gas(10_000_000_u64); // HACK: ethers-rs estimate is wrong.
        // tx.tx.set_nonce(nonce);
        // info!(?tx, "Sending transaction");
        // self.provider.send_transaction(tx.tx, None).await?
        // } else {
        // Our tests use ganache which doesn't support EIP-1559 transactions
        // yet. tx = tx.legacy();
        // self.provider.fill_transaction(&mut tx.tx, None).await?;
        // tx.tx.set_nonce(nonce);
        //
        // quick hack to ensure tx is so overpriced that it won't get dropped
        // tx.tx.set_gas_price(
        // tx.tx
        // .gas_price()
        // .ok_or(eyre!("no gasPrice set"))?
        // .checked_mul(2_u64.into())
        // .ok_or(eyre!("overflow in gasPrice"))?,
        // );
        // info!(?tx, "Sending transaction");
        // self.provider.send_transaction(tx.tx, None).await?
        // };
        // let receipt = pending_tx
        // .await
        // .map_err(|e| eyre!(e))?
        // .ok_or_else(|| eyre!("tx dropped from mempool"))?;
        // info!(?receipt, "Receipt");
        // if receipt.status != Some(U64::from(1_u64)) {
        // return Err(eyre!("tx failed"));
        // }
    }

    // #[instrument(level = "debug", skip_all)]
    // pub async fn last_block(&self) -> EyreResult<u64> {
    // let block_number = self.provider.get_block_number().await?;
    // Ok(block_number.as_u64())
    // }
    //
    // #[instrument(level = "debug", skip_all)]
    // pub async fn get_nonce(&self) -> EyreResult<usize> {
    // let nonce = self
    // .provider
    // .get_transaction_count(self.address, None)
    // .await?;
    // Ok(nonce.as_usize())
    // }

    #[instrument(level = "debug", skip_all)]
    pub async fn fetch_events(
        &self,
        starting_block: u64,
        last_leaf: usize,
    ) -> EyreResult<Vec<(usize, Field, Field)>> {
        todo!()

        // info!(starting_block, "Reading MemberAdded events from chains");
        // TODO: Some form of pagination.
        // TODO: Register to the event stream and track it going forward.
        // if self.mock {
        // info!(starting_block, "MOCK mode enabled, skipping");
        // return Ok(vec![]);
        // }
        // let filter = self
        // .semaphore
        // .member_added_filter()
        // .from_block(starting_block);
        // let events: Vec<MemberAddedFilter> = filter.query().await?;
        // info!(count = events.len(), "Read events");
        // let mut index = last_leaf;
        // let insertions = events
        // .iter()
        // .map(|event| {
        // let mut id_bytes = [0u8; 32];
        // event.identity_commitment.to_big_endian(&mut id_bytes);
        //
        // let mut root_bytes = [0u8; 32];
        // event.root.to_big_endian(&mut root_bytes);
        //
        // TODO: Check for < Modulus.
        // let root = Field::from_be_bytes_mod_order(&root_bytes);
        // let leaf = Field::from_be_bytes_mod_order(&id_bytes);
        // let res = (index, leaf, root);
        // index += 1;
        // res
        // })
        // .collect::<Vec<_>>();
        // Ok(insertions)
    }

    #[instrument(level = "debug", skip_all)]
    pub async fn is_manager(&self) -> EyreResult<bool> {
        todo!()

        // info!(?self.address, "My address");
        // let manager = self.semaphore.manager().call().await?;
        // info!(?manager, "Fetched manager address");
        // Ok(manager == self.address)
    }

    #[instrument(level = "debug", skip_all)]
    pub async fn create_group(&self, group_id: usize, tree_depth: usize) -> EyreResult<()> {
        todo!()
        // Must subtract one as internal rust merkle tree is eth merkle tree
        // depth + 1 let mut tx =
        // self.semaphore
        // .create_group(group_id.into(), (tree_depth - 1).try_into()?,
        // 0.into()); let create_group_pending_tx = if self.eip1559 {
        // self.provider.fill_transaction(&mut tx.tx, None).await?;
        // tx.tx.set_gas(10_000_000_u64); // HACK: ethers-rs estimate is wrong.
        // info!(?tx, "Sending transaction");
        // self.provider.send_transaction(tx.tx, None).await?
        // } else {
        // Our tests use ganache which doesn't support EIP-1559 transactions
        // yet. tx = tx.legacy();
        // info!(?tx, "Sending transaction");
        // self.provider.send_transaction(tx.tx, None).await?
        // };
        //
        // let receipt = create_group_pending_tx
        // .await
        // .map_err(|e| eyre!(e))?
        // .ok_or_else(|| eyre!("tx dropped from mempool"))?;
        // if receipt.status != Some(U64::from(1_u64)) {
        // return Err(eyre!("tx failed"));
        // }
        //
        // Ok(())
    }

    #[instrument(level = "debug", skip_all)]
    pub async fn insert_identity(
        &self,
        group_id: usize,
        commitment: &Field,
        _tree_depth: usize,
        nonce: usize,
    ) -> EyreResult<()> {
        todo!()
        // info!(%group_id, %commitment, "Inserting identity in contract");
        // if self.mock {
        // info!(%commitment, "MOCK mode enabled, skipping");
        // return Ok(());
        // }
        //
        // let depth = self
        // .semaphore
        // .get_depth(group_id.into())
        // .from(self.address)
        // .call()
        // .await?;
        //
        // info!(?group_id, ?depth, "Fetched group tree depth");
        // if depth == 0 {
        // return Err(eyre!("group {} not created", group_id));
        // }
        //
        // let commitment = U256::from(commitment.to_be_bytes());
        // let mut tx = self.semaphore.add_member(group_id.into(), commitment);
        // let pending_tx = if self.eip1559 {
        // self.provider.fill_transaction(&mut tx.tx, None).await?;
        // tx.tx.set_gas(10_000_000_u64); // HACK: ethers-rs estimate is wrong.
        // tx.tx.set_nonce(nonce);
        // info!(?tx, "Sending transaction");
        // self.provider.send_transaction(tx.tx, None).await?
        // } else {
        // Our tests use ganache which doesn't support EIP-1559 transactions
        // yet. tx = tx.legacy();
        // self.provider.fill_transaction(&mut tx.tx, None).await?;
        // tx.tx.set_nonce(nonce);
        //
        // quick hack to ensure tx is so overpriced that it won't get dropped
        // tx.tx.set_gas_price(
        // tx.tx
        // .gas_price()
        // .ok_or(eyre!("no gasPrice set"))?
        // .checked_mul(2_u64.into())
        // .ok_or(eyre!("overflow in gasPrice"))?,
        // );
        // info!(?tx, "Sending transaction");
        // self.provider.send_transaction(tx.tx, None).await?
        // };
        // let receipt = pending_tx
        // .await
        // .map_err(|e| eyre!(e))?
        // .ok_or_else(|| eyre!("tx dropped from mempool"))?;
        // info!(?receipt, "Receipt");
        // if receipt.status != Some(U64::from(1_u64)) {
        // return Err(eyre!("tx failed"));
        // }
        // Ok(())
    }
}
