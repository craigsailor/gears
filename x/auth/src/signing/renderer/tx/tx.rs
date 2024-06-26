use proto_messages::cosmos::{
    base::v1beta1::SendCoins,
    tx::v1beta1::{
        message::Message,
        public_key::PublicKey,
        screen::{Indent, Screen},
        signer_data::SignerData,
        tip::Tip,
        tx_data::TxData,
        tx_metadata::Metadata,
    },
};
use proto_types::{AccAddress, Denom};

use proto_messages::cosmos::ibc::protobuf::Protobuf;
use tendermint::informal::chain::Id;

use crate::signing::{
    hasher::hash_get,
    renderer::value_renderer::{
        DefaultPrimitiveRenderer, Error, PrimitiveValueRenderer, TryPrimitiveValueRenderer,
        TryPrimitiveValueRendererWithMetadata, ValueRenderer,
    },
};

/// Envelope is an internal data structure used to generate the tx envelope
/// screens. Used in the same way as the Cosmos SDK Envelope type:
/// https://github.com/cosmos/cosmos-sdk/blob/main/x/tx/signing/textual/tx.go
pub struct Envelope<M> {
    chain_id: Id,
    account_number: u64,
    sequence: u64,
    address: AccAddress,
    public_key: PublicKey,
    message: Vec<M>,
    memo: String,
    fees: Option<SendCoins>,
    fee_payer: Option<AccAddress>,
    fee_granter: String, // TODO: this should be an AccAddress
    tip: Option<SendCoins>,
    tipper: Option<AccAddress>,
    gas_limit: u64,
    timeout_height: u64,
    // TODO: need to add the fields below in:
    //other_signer: Vec<SignerInfo>,
    //extension_options: Vec<>,
    //non_critical_extension_options: Vec<>,
    hash_of_raw_bytes: String,
}

impl<M: Message> Envelope<M> {
    pub fn new(signer_data: SignerData, tx_data: TxData<M>) -> Self {
        let body_bytes = tx_data.body.to_owned().encode_vec();
        let auth_info_bytes = tx_data.auth_info.to_owned().encode_vec();

        let (tip, tipper) = match tx_data.auth_info.tip {
            Some(Tip { amount, tipper }) => (amount, Some(tipper)),
            None => (None, None),
        };

        Envelope {
            chain_id: signer_data.chain_id,
            account_number: signer_data.account_number,
            sequence: signer_data.sequence,
            address: signer_data.address,
            public_key: signer_data.pub_key,
            message: tx_data.body.messages,
            memo: tx_data.body.memo,
            fees: tx_data.auth_info.fee.amount,
            fee_payer: tx_data.auth_info.fee.payer,
            fee_granter: tx_data.auth_info.fee.granter,
            tip,
            tipper,
            gas_limit: tx_data.auth_info.fee.gas_limit,
            timeout_height: tx_data.body.timeout_height,
            hash_of_raw_bytes: hash_get(&body_bytes, &auth_info_bytes),
        }
    }
}

// NOTE: fields with protobuf default values are not rendered to screens
impl<M: Message + ValueRenderer> ValueRenderer for Envelope<M> {
    fn format<F: Fn(&Denom) -> Option<Metadata>>(
        &self,
        get_metadata: &F,
    ) -> Result<Vec<Screen>, Error> {
        let mut screens = vec![];

        screens.push(Screen {
            title: "Chain id".to_string(),
            content: DefaultPrimitiveRenderer::try_format(self.chain_id.clone().to_string())?,
            indent: None,
            expert: false,
        });
        if self.account_number != 0 {
            screens.push(Screen {
                title: "Account number".to_string(),
                content: DefaultPrimitiveRenderer::format(self.account_number),
                indent: None,
                expert: false,
            });
        }
        if self.sequence != 0 {
            screens.push(Screen {
                title: "Sequence".to_string(),
                content: DefaultPrimitiveRenderer::format(self.sequence),
                indent: None,
                expert: false,
            })
        };
        screens.push(Screen {
            title: "Address".to_string(),
            content: DefaultPrimitiveRenderer::format(self.address.to_owned()),
            indent: None,
            expert: true,
        });

        screens.append(&mut ValueRenderer::format(&self.public_key, get_metadata)?);

        let messages_count = self.message.len();

        screens.push(Screen {
            title: String::new(),
            content: DefaultPrimitiveRenderer::try_format(match messages_count {
                1 => format!("This transaction has 1 Message"),
                _ => format!("This transaction has {} Messages", messages_count),
            })
            .expect("hard coded Strings are not empty"),
            indent: None,
            expert: false,
        });

        for (i, ms) in self.message.iter().enumerate() {
            screens.push(Screen {
                title: format!("Message ({}/{messages_count})", i + 1),
                content: DefaultPrimitiveRenderer::try_format(ms.type_url().to_string())?,
                indent: Some(Indent::one()),
                expert: false,
            });
            screens.append(&mut ValueRenderer::format(ms, get_metadata)?);
        }
        screens.push(Screen {
            title: String::new(),
            content: DefaultPrimitiveRenderer::try_format("End of Message".to_string())
                .expect("hard coded String is not empty"),
            indent: None,
            expert: false,
        });

        if let Ok(memo) = DefaultPrimitiveRenderer::try_format(self.memo.clone()) {
            screens.push(Screen {
                title: "Memo".to_string(),
                content: memo,
                indent: None,
                expert: false,
            });
        }

        if let Some(amount) = &self.fees {
            screens.push(Screen {
                title: "Fees".to_string(),
                content: DefaultPrimitiveRenderer::try_format_with_metadata(
                    amount.to_owned(),
                    get_metadata,
                )?,
                indent: None,
                expert: false,
            });
        }

        if let Some(payer) = &self.fee_payer {
            screens.push(Screen {
                title: "Fee payer".to_string(),
                content: DefaultPrimitiveRenderer::format(payer.to_owned()),
                indent: None,
                expert: true,
            });
        }
        if let Ok(granter) = DefaultPrimitiveRenderer::try_format(self.fee_granter.to_owned()) {
            screens.push(Screen {
                title: "Fee granter".to_string(),
                content: granter,
                indent: None,
                expert: true,
            });
        }

        if let Some(amount) = &self.tip {
            screens.push(Screen {
                title: "Tip".to_string(),
                content: DefaultPrimitiveRenderer::try_format_with_metadata(
                    amount.to_owned(),
                    get_metadata,
                )?,
                indent: None,
                expert: false,
            });
        }

        if let Some(tipper) = &self.tipper {
            screens.push(Screen {
                title: "Tipper".to_string(),
                content: DefaultPrimitiveRenderer::format(tipper.to_owned()),
                indent: None,
                expert: false,
            });
        }

        if self.gas_limit != 0 {
            screens.push(Screen {
                title: "Gas limit".to_string(),
                content: DefaultPrimitiveRenderer::format(self.gas_limit),
                indent: None,
                expert: true,
            });
        }

        if self.timeout_height != 0 {
            screens.push(Screen {
                title: "Timeout height".to_string(),
                content: DefaultPrimitiveRenderer::format(self.timeout_height),
                indent: None,
                expert: true,
            });
        }

        screens.push(Screen {
            title: "Hash of raw bytes".to_string(),
            content: DefaultPrimitiveRenderer::try_format(self.hash_of_raw_bytes.clone())
                .expect("hash will not be empty"),
            indent: None,
            expert: true,
        });

        Ok(screens)
    }
}

#[cfg(test)]
mod tests {
    use proto_messages::cosmos::tx::v1beta1::mode_info::{ModeInfo, SignMode};
    use proto_messages::cosmos::tx::v1beta1::signer::SignerInfo;
    use proto_messages::cosmos::tx::v1beta1::signer_data::SignerData;
    use proto_messages::cosmos::{
        bank::v1beta1::MsgSend,
        base::v1beta1::{Coin, SendCoins},
        tx::v1beta1::{
            auth_info::AuthInfo,
            fee::Fee,
            screen::{Content, Indent, Screen},
            tx_body::TxBody,
            tx_data::TxData,
        },
    };
    use proto_types::{AccAddress, Denom, Uint256};
    use tendermint::informal::chain::Id;

    use crate::signing::renderer::test_functions::get_metadata;
    use crate::signing::renderer::value_renderer::ValueRenderer;

    use super::Envelope;

    #[test]
    fn envelope_data_formatting() -> anyhow::Result<()> {
        let data = envelope_data_get()?;
        let expected_screens = expected_screens_get()?;

        let actual_screens = ValueRenderer::format(&data, &get_metadata)
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;

        if expected_screens != actual_screens {
            let expected = serde_json::to_string(&expected_screens)?;
            let actual = serde_json::to_string(&actual_screens)?;
            panic!("Expected: {expected} \n !=\n Actual: {actual}")
        }

        Ok(())
    }

    fn envelope_data_get() -> anyhow::Result<Envelope<MsgSend>> {
        let signer_info = SignerInfo {
            public_key: Some(serde_json::from_str(
                r#"{
                        "@type": "/cosmos.crypto.secp256k1.PubKey",
                        "key": "Auvdf+T963bciiBe9l15DNMOijdaXCUo6zqSOvH7TXlN"
                    }"#,
            )?),

            mode_info: ModeInfo::Single(SignMode::Textual),
            sequence: 2,
        };

        let auth_info = AuthInfo {
            signer_infos: vec![signer_info],
            fee: Fee {
                amount: Some(
                    SendCoins::new(vec![Coin {
                        denom: Denom::try_from("uatom".to_owned())?,
                        amount: Uint256::from(2000u32),
                    }])
                    .unwrap(),
                ),
                gas_limit: 100000,
                payer: None,
                granter: String::new(),
            },
            tip: None,
        };

        let signer_data = SignerData {
            address: AccAddress::from_bech32("cosmos1ulav3hsenupswqfkw2y3sup5kgtqwnvqa8eyhs")?,
            chain_id: Id::try_from("my-chain".to_string()).expect("this is a valid chain id"),
            account_number: 1,
            sequence: 2,
            pub_key: serde_json::from_str(
                r#"{
				"@type": "/cosmos.crypto.secp256k1.PubKey",
				"key": "Auvdf+T963bciiBe9l15DNMOijdaXCUo6zqSOvH7TXlN"
			}"#,
            )?,
        };

        let tx_body = TxBody::<MsgSend> {
            messages: vec![MsgSend {
                from_address: AccAddress::from_bech32(
                    "cosmos1ulav3hsenupswqfkw2y3sup5kgtqwnvqa8eyhs",
                )?,
                to_address: AccAddress::from_bech32(
                    "cosmos1ejrf4cur2wy6kfurg9f2jppp2h3afe5h6pkh5t",
                )?,
                amount: SendCoins::new(vec![Coin {
                    denom: Denom::try_from("uatom".to_string())?,
                    amount: Uint256::from(10000000u32),
                }])
                .unwrap(),
            }],
            memo: String::new(),
            timeout_height: 0,
            extension_options: Vec::new(),
            non_critical_extension_options: Vec::new(),
        };

        let tx_data = TxData::<MsgSend> {
            body: tx_body,
            auth_info: auth_info,
        };

        let data = Envelope::new(signer_data, tx_data);

        Ok(data)
    }

    fn expected_screens_get() -> anyhow::Result<Vec<Screen>> {
        let screens = vec![
            Screen {
                title: "Chain id".to_string(),
                content: Content::new("my-chain".to_string())?,
                indent: None,
                expert: false,
            },
            Screen {
                title: "Account number".to_string(),
                content: Content::new(1.to_string())?,
                indent: None,
                expert: false,
            },
            Screen {
                title: "Sequence".to_string(),
                content: Content::new(2.to_string())?,
                indent: None,
                expert: false,
            },
            Screen {
                title: "Address".to_string(),
                content: Content::new("cosmos1ulav3hsenupswqfkw2y3sup5kgtqwnvqa8eyhs")?,
                indent: None,
                expert: true,
            },
            Screen {
                title: "Public key".to_string(),
                content: Content::new("/cosmos.crypto.secp256k1.PubKey")?,
                indent: None,
                expert: true,
            },
            Screen {
                title: "Key".to_string(),
                content: Content::new( "02EB DD7F E4FD EB76 DC8A 205E F65D 790C D30E 8A37 5A5C 2528 EB3A 923A F1FB 4D79 4D" )?,
                indent: Some(Indent::one()),
                expert: true,
            },
            Screen {
                title: String::new(),
                content: Content::new("This transaction has 1 Message")?,
                indent: None,
                expert: false,
            },
            Screen {
                title: "Message (1/1)".to_string(),
                content: Content::new("/cosmos.bank.v1beta1.MsgSend")?,
                indent: Some(Indent::one()),
                expert: false,
            },
            Screen {
                title: "From address".to_string(),
                content: Content::new("cosmos1ulav3hsenupswqfkw2y3sup5kgtqwnvqa8eyhs")?,
                indent: Some(Indent::two()),
                expert: false,
            },
            Screen {
                title: "To address".to_string(),
                content: Content::new("cosmos1ejrf4cur2wy6kfurg9f2jppp2h3afe5h6pkh5t")?,
                indent: Some(Indent::two()),
                expert: false,
            },
            Screen {
                title: "Amount".to_string(),
                content: Content::new("10 ATOM")?,
                indent: Some(Indent::two()),
                expert: false,
            },
            Screen {
                title: String::new(),
                content: Content::new("End of Message")?,
                indent: None,
                expert: false,
            },
            Screen {
                title: "Fees".to_string(),
                content: Content::new("0.002 ATOM".to_string())?,
                indent: None,
                expert: false,
            },
            Screen {
                title: "Gas limit".to_string(),
                content: Content::new("100'000".to_string())?,
                indent: None,
                expert: true,
            },
            Screen {
                title: "Hash of raw bytes".to_string(),
                content: Content::new(
                    "785bd306ea8962cdb9600089bdd65f3dc029e1aea112dee69e19546c9adad86e",
                )?,
                indent: None,
                expert: true,
            },
        ];

        Ok(screens)
    }
}
