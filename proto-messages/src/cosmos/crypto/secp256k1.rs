pub mod v1beta1 {
    use std::fmt;

    use base64::{
        engine::general_purpose::{self},
        Engine,
    };
    use ibc_proto::Protobuf;
    use proto_types::AccAddress;
    use ripemd::Ripemd160;
    use secp256k1::{ecdsa::Signature, hashes::sha256, Message, Secp256k1};
    use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
    use sha2::{Digest, Sha256};

    use crate::{cosmos::tx::v1beta1::public_key::SigningError, Error};

    pub use secp256k1::PublicKey as Secp256k1PubKey;

    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct RawPubKey {
        #[prost(bytes = "vec", tag = "1")]
        pub key: Vec<u8>,
    }

    #[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
    pub struct PubKey {
        // a custom serde is needed since the Secp256k1 serde uses hex encoding and not base64
        #[serde(serialize_with = "serialize_key", deserialize_with = "deserialize_key")]
        key: Secp256k1PubKey,
    }

    impl TryFrom<RawPubKey> for PubKey {
        type Error = Error;

        fn try_from(raw: RawPubKey) -> Result<Self, Self::Error> {
            PubKey::try_from(raw.key)
        }
    }

    impl From<PubKey> for RawPubKey {
        fn from(key: PubKey) -> RawPubKey {
            RawPubKey {
                key: Vec::from(key),
            }
        }
    }

    impl Protobuf<RawPubKey> for PubKey {}

    impl PubKey {
        /// Returns a Bitcoin style addresses: RIPEMD160(SHA256(pubkey))
        pub fn get_address(&self) -> AccAddress {
            let mut hasher = Sha256::new();
            hasher.update(&Vec::from(self.to_owned()));
            let hash = hasher.finalize();

            let mut hasher = Ripemd160::new();
            hasher.update(hash);
            let hash = hasher.finalize();

            let res: AccAddress = hash.as_slice().try_into().expect(
                "ripemd160 digest size is 160 bytes which is less than AccAddress::MAX_ADDR_LEN",
            );

            res
        }

        pub fn verify_signature(
            &self,
            message: impl AsRef<[u8]>,
            signature: impl AsRef<[u8]>,
        ) -> Result<(), SigningError> {
            //TODO: secp256k1 lib cannot be used for bitcoin sig verification
            let signature = Signature::from_compact(signature.as_ref())?;
            let message = Message::from_hashed_data::<sha256::Hash>(message.as_ref());
            Secp256k1::verification_only().verify_ecdsa(&message, &signature, &self.key)
        }
    }

    impl From<PubKey> for Vec<u8> {
        fn from(key: PubKey) -> Vec<u8> {
            key.key.serialize().to_vec()
        }
    }

    impl TryFrom<Vec<u8>> for PubKey {
        type Error = Error;

        fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
            let key = Secp256k1PubKey::from_slice(&value)
                .map_err(|e| Error::DecodeGeneral(format!("invalid key: {e}")))?;

            Ok(PubKey { key })
        }
    }

    fn serialize_key<S>(key: &Secp256k1PubKey, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        s.serialize_str(&general_purpose::STANDARD.encode(key.serialize()))
    }

    fn deserialize_key<'de, D>(deserializer: D) -> Result<Secp256k1PubKey, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(Secp256k1Visitor)
    }

    struct Secp256k1Visitor;

    impl<'de> de::Visitor<'de> for Secp256k1Visitor {
        type Value = Secp256k1PubKey;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("string-encoded secp256k1 public key")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            let key = general_purpose::STANDARD
                .decode(v)
                .map_err(|e| E::custom(format!("Error parsing public key '{}': {}", v, e)))?;

            Secp256k1PubKey::from_slice(&key)
                .map_err(|e| E::custom(format!("Error parsing public key '{}': {}", v, e)))
        }
    }
}

#[cfg(test)]
mod tests {

    use super::v1beta1::*;

    #[test]
    fn get_address_works() {
        let key = hex::decode("02950e1cdfcb133d6024109fd489f734eeb4502418e538c28481f22bce276f248c")
            .unwrap();
        let raw = RawPubKey { key };
        let key: PubKey = raw.try_into().unwrap();
        let address = key.get_address();
        let address: Vec<u8> = address.into();

        assert_eq!(
            hex::encode(address),
            "7c2bb42a8be69791ec763e51f5a49bcd41e82237"
        )
    }

    #[test]
    fn deserialize_works() {
        let _: PubKey = serde_json::from_str(
            r#"{
            "key": "Auvdf+T963bciiBe9l15DNMOijdaXCUo6zqSOvH7TXlN"
        }"#,
        )
        .unwrap();
    }

    #[test]
    fn verify_signature_works() {
        let key: PubKey = serde_json::from_str(
            r#"{
            "key": "A7Jg0Wg+RHwI7CAkSbCjpfWFROGtYYkUlaBVxCT6UXJ4"
        }"#,
        )
        .expect("hard coded key is valid");

        let message = [
            161, 1, 142, 162, 1, 104, 67, 104, 97, 105, 110, 32, 105, 100, 2, 106, 116, 101, 115,
            116, 45, 99, 104, 97, 105, 110, 162, 1, 110, 65, 99, 99, 111, 117, 110, 116, 32, 110,
            117, 109, 98, 101, 114, 2, 97, 56, 162, 1, 104, 83, 101, 113, 117, 101, 110, 99, 101,
            2, 98, 49, 56, 163, 1, 103, 65, 100, 100, 114, 101, 115, 115, 2, 120, 45, 99, 111, 115,
            109, 111, 115, 49, 50, 118, 114, 103, 117, 110, 119, 118, 115, 122, 103, 122, 112, 121,
            107, 100, 114, 113, 108, 120, 51, 109, 54, 112, 117, 101, 100, 118, 99, 97, 106, 108,
            120, 99, 121, 119, 56, 122, 4, 245, 163, 1, 106, 80, 117, 98, 108, 105, 99, 32, 107,
            101, 121, 2, 120, 31, 47, 99, 111, 115, 109, 111, 115, 46, 99, 114, 121, 112, 116, 111,
            46, 115, 101, 99, 112, 50, 53, 54, 107, 49, 46, 80, 117, 98, 75, 101, 121, 4, 245, 164,
            1, 99, 75, 101, 121, 2, 120, 82, 48, 51, 66, 50, 32, 54, 48, 68, 49, 32, 54, 56, 51,
            69, 32, 52, 52, 55, 67, 32, 48, 56, 69, 67, 32, 50, 48, 50, 52, 32, 52, 57, 66, 48, 32,
            65, 51, 65, 53, 32, 70, 53, 56, 53, 32, 52, 52, 69, 49, 32, 65, 68, 54, 49, 32, 56, 57,
            49, 52, 32, 57, 53, 65, 48, 32, 53, 53, 67, 52, 32, 50, 52, 70, 65, 32, 53, 49, 55, 50,
            32, 55, 56, 3, 1, 4, 245, 161, 2, 120, 30, 84, 104, 105, 115, 32, 116, 114, 97, 110,
            115, 97, 99, 116, 105, 111, 110, 32, 104, 97, 115, 32, 49, 32, 77, 101, 115, 115, 97,
            103, 101, 163, 1, 109, 77, 101, 115, 115, 97, 103, 101, 32, 40, 49, 47, 49, 41, 2, 120,
            28, 47, 99, 111, 115, 109, 111, 115, 46, 98, 97, 110, 107, 46, 118, 49, 98, 101, 116,
            97, 49, 46, 77, 115, 103, 83, 101, 110, 100, 3, 1, 163, 1, 108, 70, 114, 111, 109, 32,
            97, 100, 100, 114, 101, 115, 115, 2, 120, 45, 99, 111, 115, 109, 111, 115, 49, 50, 118,
            114, 103, 117, 110, 119, 118, 115, 122, 103, 122, 112, 121, 107, 100, 114, 113, 108,
            120, 51, 109, 54, 112, 117, 101, 100, 118, 99, 97, 106, 108, 120, 99, 121, 119, 56,
            122, 3, 2, 163, 1, 106, 84, 111, 32, 97, 100, 100, 114, 101, 115, 115, 2, 120, 45, 99,
            111, 115, 109, 111, 115, 49, 115, 121, 97, 118, 121, 50, 110, 112, 102, 121, 116, 57,
            116, 99, 110, 99, 100, 116, 115, 100, 122, 102, 55, 107, 110, 121, 57, 108, 104, 55,
            55, 55, 112, 97, 104, 117, 117, 120, 3, 2, 163, 1, 102, 65, 109, 111, 117, 110, 116, 2,
            103, 49, 32, 117, 97, 116, 111, 109, 3, 2, 161, 2, 110, 69, 110, 100, 32, 111, 102, 32,
            77, 101, 115, 115, 97, 103, 101, 163, 1, 105, 71, 97, 115, 32, 108, 105, 109, 105, 116,
            2, 103, 50, 48, 48, 39, 48, 48, 48, 4, 245, 163, 1, 113, 72, 97, 115, 104, 32, 111,
            102, 32, 114, 97, 119, 32, 98, 121, 116, 101, 115, 2, 120, 64, 101, 54, 98, 50, 55, 50,
            49, 51, 48, 99, 49, 54, 99, 99, 57, 99, 50, 52, 49, 99, 49, 53, 97, 49, 50, 98, 97, 49,
            48, 53, 99, 98, 52, 55, 100, 53, 54, 97, 99, 56, 53, 99, 99, 97, 57, 97, 54, 98, 52,
            53, 49, 55, 51, 54, 99, 49, 55, 57, 98, 50, 54, 56, 97, 98, 4, 245,
        ];

        let signature = [
            58, 82, 12, 57, 152, 57, 250, 8, 46, 241, 147, 157, 44, 182, 184, 29, 221, 102, 157,
            254, 158, 235, 181, 225, 30, 234, 79, 249, 138, 125, 12, 146, 58, 237, 48, 9, 4, 170,
            123, 106, 170, 104, 32, 177, 86, 248, 108, 64, 181, 187, 26, 165, 167, 227, 57, 116,
            109, 86, 11, 164, 83, 9, 12, 79,
        ];

        key.verify_signature(message, signature).unwrap();
    }
}
