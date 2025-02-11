use std::str::FromStr;

use scrypto::prelude::*;

#[derive(ScryptoSbor, Clone)]
pub struct PriceMessage {
    pub market_id: String,
    pub price: Decimal,
    pub nonce: u64,
    pub data_timestamp: u64,
    pub oracle_timestamp: u64,
    pub market_status_timestamp: u64,
    pub market_status: String,
}

impl PriceMessage {
    pub fn to_string(&self) -> String {
        return format!(
            "v2-{}-{}-{}-{}-{}-{}-{}",
            self.market_id, self.price, self.nonce, self.data_timestamp,
            self.oracle_timestamp, self.market_status_timestamp, self.market_status
        );
    }
}

impl FromStr for PriceMessage {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split("-").collect();

        if parts.len() != 8 {
            Err("Failed to parse input message, malformed string supplied".to_string())
        } else if parts.first().unwrap().parse() != "v2" {
            Err("Invalid message version".to_string())
        } else {
            let market_id = parts
                .get(1)
                .unwrap()
                .parse()
                .map_err(|_| "Could not parse the market id".to_string())?;

            let price = Decimal::from_str(parts.get(2).unwrap())
                .map_err(|_| "Could not parse the price".to_string())?;

            let nonce = parts
                .get(3)
                .unwrap()
                .parse()
                .map_err(|_| "Could not parse the nonce".to_string())?;

            let data_timestamp = parts
                .get(4)
                .unwrap()
                .parse()
                .map_err(|_| "Could not parse the data timestamp".to_string())?;

            let oracle_timestamp = parts
                .get(5)
                .unwrap()
                .parse()
                .map_err(|_| "Could not parse the oracle_timestamp".to_string())?;

            let market_status_timestamp = parts
                .get(6)
                .unwrap()
                .parse()
                .map_err(|_| "Could not parse the market status timestamp".to_string())?;

            let market_status = parts
                .get(7)
                .unwrap()
                .parse()
                .map_err(|_| "Could not parse the market status".to_string())?;

            Ok(PriceMessage {
                market_id,
                price,
                nonce,
                data_timestamp,
                oracle_timestamp,
                market_status_timestamp,
                market_status,
            })
        }
    }
}

#[cfg(test)]
mod price_message_tests {
    use scrypto::prelude::*;

    use crate::price_message::PriceMessage;

    #[test]
    pub fn test_to_string() {
        let price_message = PriceMessage {
            market_id: "TEST:MARKET".to_string(),
            price: dec!(1000.234),
            nonce: 1,
            data_timestamp: 1230,
            oracle_timestamp: 1235,
            market_status_timestamp: 1100,
            market_status: "open".to_string(),
        };

        assert_eq!(price_message.to_string(), "v2-TEST:MARKET-1000.234-1-1230-1235-1100-open");
    }

    #[test]
    pub fn from_string_test() {
        let price_message = PriceMessage::from_str("v2-TEST:MARKET-1000.234-1-1230-1235-1100-open").unwrap();
        assert!(
            price_message.market_id == "TEST:MARKET"
                && price_message.price == dec!(1000.234)
                && price_message.nonce == 1
                && price_message.data_timestamp == 1230
                && price_message.oracle_timestamp == 1235
                && price_message.market_status_timestamp == 1100
                && price_message.market_status == "open"
        );

        assert!(PriceMessage::from_str("TEST-1000.234-1-1230-1235-1100-open").is_err())
        assert!(PriceMessage::from_str("v1-TEST-1000.234-1-1230-1235-1100-open").is_err())
        assert!(PriceMessage::from_str("v2-TEST-1000.234-1-1230-1235-1100").is_err())
        assert!(PriceMessage::from_str("v2-TEST-1000.234-1-1230-1235-1100-open-123").is_err())
    }
}
