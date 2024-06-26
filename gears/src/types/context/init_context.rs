use crate::types::context::context::Context;
use database::{Database, PrefixDB};
use proto_messages::cosmos::tx::v1beta1::tx_metadata::{DenomUnit, Metadata};
use store_crate::{KVStore, MultiStore, StoreKey};
use tendermint::informal::{abci::Event, chain::Id};

#[derive(Debug)]
pub struct InitContext<'a, DB, SK> {
    pub multi_store: &'a mut MultiStore<DB, SK>,
    pub height: u64,
    pub events: Vec<Event>,
    pub chain_id: Id,
}

impl<'a, DB: Database, SK: StoreKey> InitContext<'a, DB, SK> {
    pub fn new(multi_store: &'a mut MultiStore<DB, SK>, height: u64, chain_id: Id) -> Self {
        InitContext {
            multi_store,
            height,
            events: vec![],
            chain_id,
        }
    }

    pub fn as_any<'b>(&'b mut self) -> Context<'b, 'a, DB, SK> {
        Context::InitContext(self)
    }

    pub fn get_height(&self) -> u64 {
        self.height
    }

    pub fn push_event(&mut self, event: Event) {
        self.events.push(event);
    }

    pub fn append_events(&mut self, mut events: Vec<Event>) {
        self.events.append(&mut events);
    }

    pub fn metadata_get(&self) -> Metadata {
        Metadata {
            description: String::new(),
            denom_units: vec![
                DenomUnit {
                    denom: "ATOM".try_into().unwrap(),
                    exponent: 6,
                    aliases: Vec::new(),
                },
                DenomUnit {
                    denom: "uatom".try_into().unwrap(),
                    exponent: 0,
                    aliases: Vec::new(),
                },
            ],
            base: "uatom".into(),
            display: "ATOM".into(),
            name: String::new(),
            symbol: String::new(),
        }
    }

    ///  Fetches an immutable ref to a KVStore from the MultiStore.
    pub fn get_kv_store(&self, store_key: &SK) -> &KVStore<PrefixDB<DB>> {
        self.multi_store.get_kv_store(store_key)
    }

    /// Fetches a mutable ref to a KVStore from the MultiStore.
    pub fn get_mutable_kv_store(&mut self, store_key: &SK) -> &mut KVStore<PrefixDB<DB>> {
        self.multi_store.get_mutable_kv_store(store_key)
    }
}
