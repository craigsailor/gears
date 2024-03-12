use keyring::key_pair::KeyPair;
use proto_messages::cosmos::tx::v1beta1::message::Message;
use proto_types::AccAddress;
use tendermint::{informal::block::Height, rpc::HttpClient};

use crate::{
    client::{query::run_query, tx::broadcast_tx_commit},
    crypto::{create_signed_transaction, SigningInfo},
    runtime::runtime,
};
use proto_messages::cosmos::{
    auth::v1beta1::{QueryAccountRequest, QueryAccountResponse},
    base::v1beta1::SendCoins,
    ibc::{auth::RawQueryAccountResponse, protobuf::Protobuf},
    tx::v1beta1::{fee::Fee, tx_body::TxBody},
};

pub trait TxHandler {
    type Message: Message;
    type TxCommands;

    fn prepare_tx(
        &self,
        command: Self::TxCommands,
        from_address: AccAddress,
    ) -> anyhow::Result<Self::Message>;

    fn handle_tx(
        &self,
        msg: Self::Message,
        key: KeyPair,
        node: url::Url,
        chain_id: tendermint::informal::chain::Id,
        fee: Option<SendCoins>,
    ) -> anyhow::Result<()> {
        let fee = Fee {
            amount: fee,
            gas_limit: 100000000, //TODO: remove hard coded gas limit
            payer: None,          //TODO: remove hard coded payer
            granter: "".into(),   //TODO: remove hard coded granter
        };

        let address = key.get_address();

        let account = runtime().block_on(get_account_latest(address, node.as_str()))?;

        let signing_info = SigningInfo {
            key,
            sequence: account.account.get_sequence(),
            account_number: account.account.get_account_number(),
        };

        let tx_body = TxBody {
            messages: vec![msg],
            memo: String::new(),                    // TODO: remove hard coded
            timeout_height: 0,                      // TODO: remove hard coded
            extension_options: vec![],              // TODO: remove hard coded
            non_critical_extension_options: vec![], // TODO: remove hard coded
        };

        let tip = None; //TODO: remove hard coded

        let raw_tx = create_signed_transaction(vec![signing_info], tx_body, fee, tip, chain_id);

        let client = HttpClient::new(tendermint::rpc::Url::try_from(node)?)?;

        runtime().block_on(broadcast_tx_commit(client, raw_tx))
    }
}

pub trait QueryHandler {
    type QueryCommands;

    fn handle_query_command(
        &self,
        command: Self::QueryCommands,
        node: &str,
        height: Option<Height>,
    ) -> anyhow::Result<()>;
}

/// Name aux stands for `auxiliary`. In terms of implementation this is more like user extension to CLI.
/// It's reason exists to add user specific commands which doesn't supports usually.
pub trait AuxHandler {
    type AuxCommands; // TODO: use NilAuxCommand as default if/when associated type defaults land https://github.com/rust-lang/rust/issues/29661

    #[allow(unused_variables)]
    fn handle_aux_commands(&self, command: Self::AuxCommands) -> anyhow::Result<()> {
        Ok(())
    }
}

// TODO: we're assuming here that the app has an auth module which handles this query
async fn get_account_latest(
    address: AccAddress,
    node: &str,
) -> anyhow::Result<QueryAccountResponse> {
    let query = QueryAccountRequest { address };

    run_query::<QueryAccountResponse, RawQueryAccountResponse>(
        query.encode_vec(),
        "/cosmos.auth.v1beta1.Query/Account".into(),
        node,
        None,
    )
    .await
}
