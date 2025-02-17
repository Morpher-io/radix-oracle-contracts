use std::str::FromStr;

use scrypto::prelude::*;

#[derive(ScryptoSbor, Clone)]
pub struct OracleRequestMessage {
    pub market_id: String,
    pub nonce: u64,
    pub public_key_bls: String,
    pub address: String,
}

impl OracleRequestMessage {
    pub fn to_string(&self) -> String {
        return format!(
            "{}##{}##{}##{}",
            self.market_id, 
            self.nonce, 
            self.public_key_bls, 
            self.address
        );
    }
}

impl FromStr for OracleRequestMessage {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let err_string = "Failed to parse input message".to_string();
        let parts: Vec<&str> = s.split("##").collect();

        if parts.len() != 4 {
            Err(err_string)
        } else {
            let market_id = parts
                .first()
                .unwrap()
                .parse()
                .map_err(|_| "Could not parse the market id".to_string())?;

            let nonce = parts
                .get(1)
                .unwrap()
                .parse()
                .map_err(|_| "Could not nonce the price".to_string())?;

            let public_key_bls = parts
                .get(2)
                .unwrap()
                .parse()
                .map_err(|_| "Could not parse the bls key".to_string())?;

            let address = parts
                .get(3)
                .unwrap()
                .parse()
                .map_err(|_| "Could not parse the address".to_string())?;

            Ok(OracleRequestMessage {
                market_id,
                nonce,
                public_key_bls,
                address
            })
        }
    }
}

#[cfg(test)]
mod price_message_tests {
    use scrypto::prelude::*;

    use crate::oracle_request_message::OracleRequestMessage;

    #[test]
    pub fn test_to_string() {
        let oracle_request_message = OracleRequestMessage {
            market_id: "TEST:MARKET".to_string(),
            nonce: 1,
            public_key_bls: "abcTEST".to_string(),
            address: "defTEST".to_string()
        };

        assert_eq!(oracle_request_message.to_string(), "TEST:MARKET##1##abcTEST#defTEST");
    }

    #[test]
    pub fn from_string_test() {
        let oracle_request_message = OracleRequestMessage::from_str("TEST:MARKET##1##abcTEST##defTEST").unwrap();
        assert!(
            oracle_request_message.market_id == "TEST:MARKET"
                && oracle_request_message.nonce == 1
                && oracle_request_message.public_key_bls == "abcTEST"
                && oracle_request_message.address == "defTEST"
        );

        assert!(OracleRequestMessage::from_str("0-1000.234-1-1230-5").is_err())
    }
}
