use crate::signing::renderer::value_renderer::{
    DefaultPrimitiveRenderer, Error, PrimitiveValueRenderer, TryPrimitiveValueRendererWithMetadata,
};
use proto_messages::cosmos::{
    base::v1beta1::Coin,
    tx::v1beta1::{screen::Content, tx_metadata::Metadata},
};
use proto_types::Uint256;
use proto_types::{Decimal256, Denom};

impl TryPrimitiveValueRendererWithMetadata<Coin> for DefaultPrimitiveRenderer {
    fn try_format_with_metadata<F: Fn(&Denom) -> Option<Metadata>>(
        coin: Coin,
        get_metadata: &F,
    ) -> Result<Content, Error> {
        let Some(metadata) = get_metadata(&coin.denom) else {
            let display = coin.denom.to_string();
            return Ok(Content::new(format!(
                "{} {display}",
                DefaultPrimitiveRenderer::format(coin.amount).into_inner()
            ))
            .expect("this String is not empty so it will never fail to parse"));
        };

        let Metadata {
            display,
            denom_units,
            ..
        } = metadata;

        if display.is_empty() || coin.denom.to_string() == display {
            let display = coin.denom.to_string();
            return Ok(Content::new(format!(
                "{} {display}",
                DefaultPrimitiveRenderer::format(coin.amount).into_inner()
            ))
            .expect("this String is not empty so it will never fail to parse"));
        }

        let coin_exp = denom_units.iter().find(|this| this.denom == coin.denom);
        let denom_exp = denom_units
            .iter()
            .find(|this| this.denom.as_ref() == display);

        match (coin_exp, denom_exp) {
            (Some(coin_exp), Some(denom_exp)) => {
                let formatted_amount = match coin_exp.exponent.cmp(&denom_exp.exponent) {
                    std::cmp::Ordering::Less => {
                        let power = denom_exp.exponent - coin_exp.exponent;

                        let amount = Decimal256::from_atomics(coin.amount, 0).map_err(|_| {
                            Error::Rendering(format!(
                                "coin amounts greater than {} are not supported for this signing mode",
                                Decimal256::MAX
                            ))
                        })?; //TODO: this is a deficiency of the Decimal256 type, it should be able to hold any Uint256 value
                        let scaling = Uint256::from(10u32).checked_pow(power).map_err(|_| {
                            Error::Rendering(format!(
                                "{display} denom is not supported for this signing mode"
                            ))
                        })?;

                        let disp_amount = amount / scaling; // TODO: what happens if scaling  > 10**18 causing amount to go to zero?

                        DefaultPrimitiveRenderer::format(disp_amount).into_inner()
                    }

                    std::cmp::Ordering::Equal => {
                        DefaultPrimitiveRenderer::format(coin.amount).into_inner()
                    }
                    std::cmp::Ordering::Greater => {
                        // TODO: write test for this case

                        let power = coin_exp.exponent - denom_exp.exponent;

                        let scaling = Uint256::from(10u32).checked_pow(power).map_err(|_| {
                            Error::Rendering(format!(
                                "{display} denom is not supported for this signing mode"
                            ))
                        })?;

                        let disp_amount = coin.amount.checked_mul(scaling).map_err(|_| {
                            Error::Rendering(format!(
                                "coin amounts greater than {} are not supported for this signing mode and denom {}",
                                Uint256::MAX / scaling,
                                display
                            ))
                        })?;

                        DefaultPrimitiveRenderer::format(disp_amount).into_inner()
                    }
                };

                Ok(Content::new(format!("{formatted_amount} {display}"))
                    .expect("this String is not empty so it will never fail to parse"))
            }
            _ => {
                let display = coin.denom.to_string();
                Ok(Content::new(format!(
                    "{} {display}",
                    DefaultPrimitiveRenderer::format(coin.amount).into_inner()
                ))
                .expect("this String is not empty so it will never fail to parse"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::signing::renderer::{
        test_functions::get_metadata,
        value_renderer::{DefaultPrimitiveRenderer, TryPrimitiveValueRendererWithMetadata},
    };
    use anyhow::Ok;
    use proto_messages::cosmos::{base::v1beta1::Coin, tx::v1beta1::screen::Content};
    use proto_types::Uint256;

    #[test]
    fn coin_formatting() -> anyhow::Result<()> {
        let coin = Coin {
            denom: "uatom".try_into()?,
            amount: Uint256::from(10000000_u64).into(),
        };

        let expected_content = Content::new("10 ATOM".to_string()).unwrap();

        let actual_content =
            DefaultPrimitiveRenderer::try_format_with_metadata(coin, &get_metadata);

        assert_eq!(expected_content, actual_content.unwrap());

        Ok(())
    }

    #[test]
    fn coin_formatting_small_amounts_works() -> anyhow::Result<()> {
        let coin = Coin {
            denom: "uatom".try_into()?,
            amount: Uint256::from(1u8).into(),
        };

        let expected_content = Content::new("0.000001 ATOM".to_string()).unwrap();

        let actual_content =
            DefaultPrimitiveRenderer::try_format_with_metadata(coin, &get_metadata);

        assert_eq!(expected_content, actual_content.unwrap());

        Ok(())
    }

    #[test]
    fn coin_formatting_zero_amount_works() -> anyhow::Result<()> {
        let coin = Coin {
            denom: "uatom".try_into()?,
            amount: Uint256::from(0u8).into(),
        };

        let expected_content = Content::new("0 ATOM".to_string()).unwrap();

        let actual_content =
            DefaultPrimitiveRenderer::try_format_with_metadata(coin, &get_metadata);

        assert_eq!(expected_content, actual_content.unwrap());

        Ok(())
    }

    #[test]
    fn coin_formatting_large_amount_works() -> anyhow::Result<()> {
        let coin = Coin {
            denom: "ATOM".try_into()?,
            amount: Uint256::from(10_000u16).into(),
        };

        let expected_content = Content::new("10'000 ATOM".to_string()).unwrap();

        let actual_content =
            DefaultPrimitiveRenderer::try_format_with_metadata(coin, &get_metadata);

        assert_eq!(expected_content, actual_content.unwrap());

        Ok(())
    }
}
