mod abi;

use self::abi::BatchingContract as ContractAbi;
use crate::{
    contracts::{EventStream, IdentityManager, Options},
    ethereum::{write::TransactionId, Ethereum, EventError, ReadProvider, TxError},
};
use async_trait::async_trait;
use ethers::{providers::Middleware, types::U256};
use semaphore::Field;
use tracing::{error, info, instrument};

// TODO [Ara] Remove the allows.
/// A structure representing the interface to the batch-based identity manager
/// contract.
pub struct Contract {
    #[allow(dead_code)]
    ethereum: Ethereum,
    #[allow(dead_code)]
    abi:      ContractAbi<ReadProvider>,
}

#[async_trait]
impl IdentityManager for Contract {
    #[instrument(level = "debug", skip_all)]
    async fn new(options: Options, ethereum: Ethereum) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        // Check that there is code deployed at the target address.
        let address = options.identity_manager_address;
        let code = ethereum.provider().get_code(address, None).await?;
        if code.as_ref().is_empty() {
            error!(
                ?address,
                "No contract code is deployed at the provided address."
            );
        }

        // Connect to the running batching contract.
        let abi = ContractAbi::new(
            options.identity_manager_address,
            ethereum.provider().clone(),
        );

        let owner = abi.owner().call().await?;
        if owner != ethereum.address() {
            error!(?owner, signer = ?ethereum.address(), "Signer is not the owner of the identity manager contract.");
            panic!("Cannot currently continue in read-only mode.")
        }
        info!(
            ?address,
            ?owner,
            "Connected to the WorldID Identity Manager"
        );

        let identity_manager = Self { ethereum, abi };

        Ok(identity_manager)
    }

    fn tree_depth(&self) -> usize {
        todo!()
    }

    fn initial_leaf_value(&self) -> Field {
        todo!()
    }

    fn group_id(&self) -> U256 {
        todo!()
    }

    async fn confirmed_block_number(&self) -> Result<u64, EventError> {
        self.ethereum
            .provider()
            .confirmed_block_number()
            .await
            .map(|num| num.as_u64())
    }

    #[instrument(level = "debug", skip_all)]
    async fn is_owner(&self) -> anyhow::Result<bool> {
        info!(address = ?self.ethereum.address(), "My address");
        let owner = self.abi.owner().call().await?;
        info!(?owner, "Fetched owner address");
        Ok(owner == self.ethereum.address())
    }

    #[instrument(level = "debug", skip_all)]
    async fn register_identities(
        &self,
        _identity_commitments: Vec<Field>,
    ) -> Result<TransactionId, TxError> {
        todo!()
    }

    async fn assert_latest_root(&self, root: Field) -> anyhow::Result<()> {
        let latest_root = self.abi.latest_root().call().await?;
        let processed_root: U256 = root.into();
        if processed_root == latest_root {
            Ok(())
        } else {
            Err(anyhow::Error::msg("Not latest root."))
        }
    }

    #[instrument(level = "debug", skip_all)]
    async fn assert_valid_root(&self, _root: Field) -> anyhow::Result<()> {
        todo!()
    }

    fn fetch_events(&self, _: u64, _: Option<u64>) -> Option<EventStream<'_>> {
        None
    }
}
