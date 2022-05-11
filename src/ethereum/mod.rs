mod contract;
mod estimator;
mod rpc_logger;
mod transport;

use self::{
    contract::{MemberAddedFilter, SemaphoreAirdrop},
    estimator::Estimator,
    rpc_logger::RpcLogger,
    transport::Transport,
};
use chrono::{Duration as ChronoDuration, Utc};
use ethers::{
    core::k256::ecdsa::SigningKey,
    middleware::{
        gas_oracle::{GasOracleMiddleware, Polygon},
        NonceManagerMiddleware, SignerMiddleware, TimeLag,
    },
    prelude::{gas_oracle::Cache, H160, U64},
    providers::{Middleware, Provider},
    signers::{LocalWallet, Signer, Wallet},
    types::{Address, BlockId, BlockNumber, Chain, H256, U256},
};
use eyre::{eyre, Result as EyreResult};
use futures::try_join;
use semaphore::Field;
use std::{sync::Arc, time::Duration};
use structopt::StructOpt;
use tracing::{error, info, instrument};
use url::Url;

const PENDING: Option<BlockId> = Some(BlockId::Number(BlockNumber::Pending));

#[derive(Clone, Debug, PartialEq, StructOpt)]
pub struct Options {
    /// Ethereum API Provider
    #[structopt(long, env, default_value = "http://localhost:8545")]
    pub ethereum_provider: Url,

    /// Semaphore contract address.
    #[structopt(long, env, default_value = "174ee9b5fBb5Eb68B6C61032946486dD9c2Dc4b6")]
    pub semaphore_address: Address,

    /// Private key used for transaction signing
    #[structopt(
        long,
        env,
        default_value = "ee79b5f6e221356af78cf4c36f4f7885a11b67dfcc81c34d80249947330c0f82"
    )]
    // NOTE: We abuse `Hash` here because it has the right `FromStr` implementation.
    pub signing_key: H256,

    /// If this module is being run with EIP-1559 support, useful in some places
    /// where EIP-1559 is not yet supported
    #[structopt(
        short,
        parse(try_from_str),
        default_value = "true",
        env = "USE_EIP1559"
    )]
    pub eip1559: bool,

    #[structopt(
        short,
        parse(try_from_str),
        default_value = "false",
        env = "SIGNUP_SEQUENCER_MOCK"
    )]
    pub mock: bool,
}

// Code out the provider stack in types
// Needed because of <https://github.com/gakonst/ethers-rs/issues/592>
type Provider0 = Provider<RpcLogger<Transport>>;
type Provider1 = SignerMiddleware<Provider0, Wallet<SigningKey>>;
type Provider2 = NonceManagerMiddleware<Provider1>;
type Provider3 = Estimator<Provider2>;
type Provider4 = GasOracleMiddleware<Provider3, Cache<Polygon>>;
type ProviderStack = Provider4;

pub struct Ethereum {
    provider:  Arc<ProviderStack>,
    address:   H160,
    semaphore: SemaphoreAirdrop<ProviderStack>,
    eip1559:   bool,
    mock:      bool,
}

impl Ethereum {
    #[instrument(skip_all)]
    pub async fn new(options: Options) -> EyreResult<Self> {
        // Connect to the Ethereum provider
        // TODO: Allow multiple providers with failover / broadcast.
        // TODO: Requests don't seem to process in parallel. Check if this is
        // a limitation client side or server side.
        let (provider, chain_id) = {
            info!(
                provider = %&options.ethereum_provider,
                "Connecting to Ethereum"
            );
            let transport = Transport::new(options.ethereum_provider).await?;
            let logger = RpcLogger::new(transport);
            let provider = Provider::new(logger);

            // Fetch state of the chain.
            let (chain_id, latest_block) = try_join!(
                provider.get_chainid(),
                provider.get_block(BlockId::Number(BlockNumber::Latest))
            )?;
            // Identify chain
            let chain = Chain::try_from(chain_id)
                .map_or_else(|_| "Unknown".to_string(), |chain| chain.to_string());

            // Identify latest block by number and hash
            let latest_block = latest_block
                .ok_or_else(|| eyre!("Failed to get latest block from Ethereum provider"))?;
            let block_hash = latest_block
                .hash
                .ok_or_else(|| eyre!("Could not read latest block hash"))?;
            let block_number = latest_block
                .number
                .ok_or_else(|| eyre!("Could not read latest block number"))?;

            let block_time = latest_block.time()?;
            info!(%chain_id, %chain, %block_number, ?block_hash, %block_time, "Connected to Ethereum provider");

            // Sanity check the block timestamp
            let now = Utc::now();
            let block_age = now - block_time;
            let block_age_abs = if block_age < ChronoDuration::zero() {
                -block_age
            } else {
                block_age
            };
            if block_age_abs > ChronoDuration::minutes(30) {
                error!(%now, %block_time, %block_age, "Block time is more than 30 minutes from now.");
            }
            (provider, chain_id)
        };

        // Construct a local key signer
        let (provider, address) = {
            let signing_key = SigningKey::from_bytes(options.signing_key.as_bytes())?;
            let signer = LocalWallet::from(signing_key);
            let address = signer.address();
            let chain_id: u64 = chain_id.try_into().map_err(|e| eyre!("{}", e))?;
            let signer = signer.with_chain_id(chain_id);
            let provider = SignerMiddleware::new(provider, signer);
            let provider = { NonceManagerMiddleware::new(provider, address) };

            let (next_nonce, balance) = try_join!(
                provider.initialize_nonce(PENDING),
                provider.get_balance(address, PENDING)
            )?;
            info!(?address, %next_nonce, %balance, "Constructed wallet");
            (provider, address)
        };

        // TODO: Check signer balance regularly and keep the metric as a gauge.
        // TODO: Keep gas_price, base_fee, priority_fee as a gauge.

        // Add a gas estimator with 10% and 10k gas bonus over provider.
        let provider = Estimator::new(provider, 1.10, 10e3);

        // Add a gas oracle
        let provider = {
            let client = reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()?;
            let chain = Chain::try_from(chain_id)?;
            let gas_oracle = Polygon::with_client(client, chain)?;
            let gas_oracle = Cache::new(Duration::from_secs(5), gas_oracle);
            GasOracleMiddleware::new(provider, gas_oracle)
        };

        todo!();

        // Add a gas price escalator
        // TODO: Commit state to storage and load it on startup.
        // let provider = {
        //     let escalator = GeometricGasPrice::new(5.0, 10u64, None::<u64>);
        //     GasEscalatorMiddleware::new(provider, escalator,
        // GasEscalatorFreq::PerBlock) };

        // Connect to Contract
        let provider = Arc::new(provider);
        let semaphore = SemaphoreAirdrop::new(options.semaphore_address, provider.clone());
        // TODO: Test contract connection by calling a view function.

        Ok(Self {
            provider,
            address,
            semaphore,
            eip1559: options.eip1559,
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

    #[instrument(skip_all)]
    pub async fn last_block(&self) -> EyreResult<u64> {
        let block_number = self.provider.get_block_number().await?;
        Ok(block_number.as_u64())
    }

    #[instrument(skip_all)]
    pub async fn get_nonce(&self) -> EyreResult<usize> {
        let nonce = self
            .provider
            .get_transaction_count(self.address, None)
            .await?;
        Ok(nonce.as_usize())
    }

    #[instrument(skip_all)]
    pub async fn fetch_events(
        &self,
        starting_block: u64,
        last_leaf: usize,
    ) -> EyreResult<Vec<(usize, Field, Field)>> {
        info!(starting_block, "Reading MemberAdded events from chains");
        // TODO: Some form of pagination.
        // TODO: Register to the event stream and track it going forward.
        if self.mock {
            info!(starting_block, "MOCK mode enabled, skipping");
            return Ok(vec![]);
        }
        let filter = self
            .semaphore
            .member_added_filter()
            .from_block(starting_block);
        let events: Vec<MemberAddedFilter> = filter.query().await?;
        info!(count = events.len(), "Read events");
        let mut index = last_leaf;
        let insertions = events
            .iter()
            .map(|event| {
                let mut id_bytes = [0u8; 32];
                event.identity_commitment.to_big_endian(&mut id_bytes);

                let mut root_bytes = [0u8; 32];
                event.root.to_big_endian(&mut root_bytes);

                // TODO: Check for < Modulus.
                let root = Field::from_be_bytes_mod_order(&root_bytes);
                let leaf = Field::from_be_bytes_mod_order(&id_bytes);
                let res = (index, leaf, root);
                index += 1;
                res
            })
            .collect::<Vec<_>>();
        Ok(insertions)
    }

    #[instrument(skip_all)]
    pub async fn is_manager(&self) -> EyreResult<bool> {
        info!(?self.address, "My address");
        let manager = self.semaphore.manager().call().await?;
        info!(?manager, "Fetched manager address");
        Ok(manager == self.address)
    }

    #[instrument(skip_all)]
    pub async fn create_group(&self, group_id: usize, tree_depth: usize) -> EyreResult<()> {
        // Must subtract one as internal rust merkle tree is eth merkle tree depth + 1
        let mut tx =
            self.semaphore
                .create_group(group_id.into(), (tree_depth - 1).try_into()?, 0.into());
        let create_group_pending_tx = if self.eip1559 {
            self.provider.fill_transaction(&mut tx.tx, None).await?;
            tx.tx.set_gas(10_000_000_u64); // HACK: ethers-rs estimate is wrong.
            info!(?tx, "Sending transaction");
            self.provider.send_transaction(tx.tx, None).await?
        } else {
            // Our tests use ganache which doesn't support EIP-1559 transactions yet.
            tx = tx.legacy();
            info!(?tx, "Sending transaction");
            self.provider.send_transaction(tx.tx, None).await?
        };

        let receipt = create_group_pending_tx
            .await
            .map_err(|e| eyre!(e))?
            .ok_or_else(|| eyre!("tx dropped from mempool"))?;
        if receipt.status != Some(U64::from(1_u64)) {
            return Err(eyre!("tx failed"));
        }

        Ok(())
    }

    #[instrument(skip_all)]
    pub async fn insert_identity(
        &self,
        group_id: usize,
        commitment: &Field,
        _tree_depth: usize,
        nonce: usize,
    ) -> EyreResult<()> {
        info!(%group_id, %commitment, "Inserting identity in contract");
        if self.mock {
            info!(%commitment, "MOCK mode enabled, skipping");
            return Ok(());
        }

        let depth = self
            .semaphore
            .get_depth(group_id.into())
            .from(self.address)
            .call()
            .await?;

        info!(?group_id, ?depth, "Fetched group tree depth");
        if depth == 0 {
            return Err(eyre!("group {} not created", group_id));
        }

        let commitment = U256::from(commitment.to_be_bytes());
        let mut tx = self.semaphore.add_member(group_id.into(), commitment);
        let pending_tx = if self.eip1559 {
            self.provider.fill_transaction(&mut tx.tx, None).await?;
            tx.tx.set_gas(10_000_000_u64); // HACK: ethers-rs estimate is wrong.
            tx.tx.set_nonce(nonce);
            info!(?tx, "Sending transaction");
            self.provider.send_transaction(tx.tx, None).await?
        } else {
            // Our tests use ganache which doesn't support EIP-1559 transactions yet.
            tx = tx.legacy();
            self.provider.fill_transaction(&mut tx.tx, None).await?;
            tx.tx.set_nonce(nonce);

            // quick hack to ensure tx is so overpriced that it won't get dropped
            tx.tx.set_gas_price(
                tx.tx
                    .gas_price()
                    .ok_or(eyre!("no gasPrice set"))?
                    .checked_mul(2_u64.into())
                    .ok_or(eyre!("overflow in gasPrice"))?,
            );
            info!(?tx, "Sending transaction");
            self.provider.send_transaction(tx.tx, None).await?
        };
        let receipt = pending_tx
            .await
            .map_err(|e| eyre!(e))?
            .ok_or_else(|| eyre!("tx dropped from mempool"))?;
        info!(?receipt, "Receipt");
        if receipt.status != Some(U64::from(1_u64)) {
            return Err(eyre!("tx failed"));
        }
        Ok(())
    }
}
