use scrypto::prelude::*;

use crate::price_message::PriceMessage;
use crate::utils::*;

#[derive(ScryptoEvent, ScryptoSbor)]
pub struct OraclePublicKeyUpdate {
    pub new_public_key: Bls12381G1PublicKey,
}

type Unit = ();

#[blueprint]
#[events(OraclePublicKeyUpdate)]
#[types(u64, Unit)]
mod morpher_oracle {

    enable_method_auth! {
        roles{
            admin => updatable_by: [SELF];
        }, methods {
            check_price_input => PUBLIC;
            check_prices_input => PUBLIC;
            set_oracle_public_key => restrict_to: [admin];
        }
    }

    pub struct MorpherOracle {
        authorized_pub_key: Bls12381G1PublicKey,
        used_nonce: KeyValueStore<u64, ()>,
        // transient_resource_manager: ResourceManager, //coming in V2
    }

    impl MorpherOracle {
        pub fn instantiate(
            authorized_public_key: String,
            dapp_definition: ComponentAddress,
        ) -> (Global<MorpherOracle>, FungibleBucket) {
            // Creates the admin badge resource that can change the Oracles pub key.
            // This admin badge cannot be minted or burnt and all of its parameters are fixed.
            let admin_badge = ResourceBuilder::new_fungible(OwnerRole::None)
                .divisibility(DIVISIBILITY_NONE)
                .burn_roles(
                    burn_roles! { burner => rule!(deny_all); burner_updater => rule!(deny_all); },
                )
                .mint_roles(
                    mint_roles! { minter => rule!(deny_all); minter_updater => rule!(deny_all);},
                )
                .freeze_roles(freeze_roles! { freezer => rule!(deny_all); freezer_updater => rule!(deny_all); })
                .recall_roles(recall_roles! { recaller => rule!(deny_all); recaller_updater => rule!(deny_all); })
                .withdraw_roles(withdraw_roles! { withdrawer => rule!(allow_all); withdrawer_updater => rule!(deny_all); })
                .deposit_roles(deposit_roles! { depositor => rule!(allow_all); depositor_updater => rule!(deny_all); })
                .metadata(metadata!(roles {
                    metadata_setter => rule!(deny_all);
                    metadata_setter_updater => rule!(deny_all);
                    metadata_locker => rule!(deny_all);
                    metadata_locker_updater => rule!(deny_all);
                    },
                    init {
                            "name" => "Oracle admin badge".to_string(), locked;
                            "description" => "Controls the morpher oracle.", locked;
                        }))
                .mint_initial_supply(1);

            // // Define a "transient" resource which can never be deposited once created, only burned
            // let transient_price_message_manager = ResourceBuilder::new_ruid_non_fungible::<PriceMessage>(OwnerRole::None)
            //     .metadata(metadata!(
            //         init {
            //             "name" =>
            //             "A transient Price Message, must be returned at the end".to_owned(), locked;
            //         }
            //     ))
            //     .mint_roles(mint_roles!(
            //         minter => rule!(require(global_caller(component_address)));
            //         minter_updater => rule!(deny_all);
            //     ))
            //     .burn_roles(burn_roles!(
            //         burner => rule!(require(global_caller(component_address)));
            //         burner_updater => rule!(deny_all);
            //     ))
            //     .deposit_roles(deposit_roles!(
            //         depositor => rule!(deny_all);
            //         depositor_updater => rule!(deny_all);
            //     ))

            //     .create_with_no_initial_supply();

            let component = Self {
                authorized_pub_key: Bls12381G1PublicKey::from_str(authorized_public_key.as_str())
                    .expect("The given public key is not valid"),
                used_nonce: KeyValueStore::new_with_registered_type(),
            }
            .instantiate()
            .prepare_to_globalize(OwnerRole::None)
            .roles(roles! {
                admin => rule!(require(admin_badge.resource_address()));
            })
            .metadata(metadata!(roles {
            metadata_setter => rule!(deny_all);
            metadata_setter_updater => rule!(deny_all);
            metadata_locker => rule!(deny_all);
            metadata_locker_updater => rule!(deny_all);
            },
            init {
                    "dapp_definition" => GlobalAddress::from(dapp_definition), updatable;
                    "name" => "Morpher oracle Component", updatable;
                }))
            .globalize();

            (component, admin_badge)
        }

        pub fn set_oracle_public_key(&mut self, authorized_public_key: String) {
            self.authorized_pub_key = Bls12381G1PublicKey::from_str(authorized_public_key.as_str())
                .expect("The given public key is not valid");
            Runtime::emit_event(OraclePublicKeyUpdate {
                new_public_key: Bls12381G1PublicKey::from_str(authorized_public_key.as_str())
                    .unwrap(),
            })
        }

        pub fn check_price_input(&mut self, message: String, signature: String) -> PriceMessage {
            // Then check the message is correct
            check_signature(&message, &signature, self.authorized_pub_key);

            // If everything is fine, parse the message
            let price_message = PriceMessage::from_str(&message).unwrap();

            // Check that the nonce has not been used
            assert!(
                self.used_nonce.get(&price_message.nonce).is_none(),
                "This nonce has already been used"
            );
            self.used_nonce.insert(price_message.nonce, ());

            price_message
        }

        pub fn check_prices_input(
            &mut self,
            message: String,
            signature: String,
        ) -> Vec<PriceMessage> {
            check_signature(&message, &signature, self.authorized_pub_key);

            // Split the message into individual PriceMessages assuming they are separated by commas
            let messages: Vec<&str> = message.split(',').collect();
            let mut price_messages = Vec::new();

            for msg in messages {
                let price_message = PriceMessage::from_str(msg).unwrap();
                // Check that the nonce has not been used
                assert!(
                    self.used_nonce.get(&price_message.nonce).is_none(),
                    "This nonce has already been used"
                );
                self.used_nonce.insert(price_message.nonce, ());
                price_messages.push(price_message);
            }

            price_messages
        }
    }
}
