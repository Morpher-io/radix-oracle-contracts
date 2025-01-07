use std::cmp::max;

use scrypto::prelude::*;

use crate::utils::*;

#[derive(NonFungibleData, ScryptoSbor)]
pub struct OracleSubscription {
    #[mutable]
    pub expiration_time: u64,
    #[mutable]
    pub authorized_pub_key: String,
}

#[derive(ScryptoEvent, ScryptoSbor)]
pub struct OracleSubscriptionUpdate {
    pub new_expiration_time: u64,
    pub nft_id: NonFungibleLocalId,
}

#[blueprint]
#[events(OracleSubscriptionUpdate)]
mod morpher_oracle_subscription {

    enable_method_auth! {
        roles{
            admin => updatable_by: [SELF];
        }, methods {
            update_subscription_pub_key => PUBLIC;
            new_subscription => PUBLIC;
            renew_subscription => PUBLIC;
            collect_subscription_fees => restrict_to: [admin];
            set_subscription_price => restrict_to: [admin];
        }
    }

    const SECONDS_IN_A_MONTH: u64 = 2_592_000;
    // const API_CALLS_PER_MONTH: u64 = 1_000_000;

    pub struct MorpherOracleSubscription {
        monthly_subscription_fee: Decimal,
        subscription_fees_vault: FungibleVault,
        subscription_manager: ResourceManager,
    }

    impl MorpherOracleSubscription {
        pub fn instantiate(
            monthly_subscription_fee: Decimal,
            dapp_definition: ComponentAddress,
        ) -> (Global<MorpherOracleSubscription>, FungibleBucket) {
            // Creates a reservation for this component so that we can set up the correct roles
            let (address_reservation, component_address) =
                Runtime::allocate_component_address(<MorpherOracleSubscription>::blueprint_id());

            // Creates the admin badge resource that can change the Oracle information and collect
            // fees.
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
                            "name" => "Oracle Subscription NFT Admin Badge".to_string(), locked;
                            "description" => "Controls the morpher oracle subscription.", locked;
                        }))
                .mint_initial_supply(1);

            // Creates the subscription NFT that can be minted, updated and burnt only by this blueprint.
            // It can only be traded by this component.
            // Its metadata can be changed with the admin badge.
            let subscription_manager = ResourceBuilder::new_ruid_non_fungible::<OracleSubscription>(OwnerRole::None)
                .burn_roles(
                    burn_roles!(burner => rule!(require(global_caller(component_address))); burner_updater => rule!(deny_all);)
                )
                .mint_roles(mint_roles! { minter => rule!(require(global_caller(component_address))); minter_updater => rule!(deny_all);})
                .freeze_roles(freeze_roles! { freezer => rule!(deny_all); freezer_updater => rule!(deny_all); })
                .recall_roles(recall_roles! { recaller => rule!(deny_all); recaller_updater => rule!(deny_all); })
                .withdraw_roles(withdraw_roles! { withdrawer => rule!(allow_all); withdrawer_updater => rule!(deny_all); })
                .deposit_roles(deposit_roles! { depositor => rule!(allow_all); depositor_updater => rule!(deny_all); })
                .non_fungible_data_update_roles(non_fungible_data_update_roles!( non_fungible_data_updater => rule!(require(global_caller(component_address)));
                    non_fungible_data_updater_updater => rule!(deny_all);))
                .metadata(metadata!(roles {
                    metadata_setter => rule!(require(admin_badge.resource_address()));
                    metadata_setter_updater => rule!(require(admin_badge.resource_address()));
                    metadata_locker => rule!(require(admin_badge.resource_address()));
                    metadata_locker_updater => rule!(require(admin_badge.resource_address()));
                },
                init {
                        "name" => "Morpher subscription".to_string(), updatable;
                        "description" => "Subscription to the Morpher protocol.", updatable;
                        "dapp_definition" => GlobalAddress::from(dapp_definition), updatable;
                    }))
                .create_with_no_initial_supply();

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
                monthly_subscription_fee,
                subscription_fees_vault: FungibleVault::new(XRD),
                subscription_manager,
            }
            .instantiate()
            .prepare_to_globalize(OwnerRole::None)
            .roles(roles! {
                admin => rule!(require(admin_badge.resource_address()));
            })
            .with_address(address_reservation)
            .metadata(metadata!(roles {
            metadata_setter => rule!(deny_all);
            metadata_setter_updater => rule!(deny_all);
            metadata_locker => rule!(deny_all);
            metadata_locker_updater => rule!(deny_all);
            },
            init {
                    "dapp_definition" => GlobalAddress::from(dapp_definition), updatable;
                    "name" => "Morpher oracle Subscription", updatable;
                }))
            .globalize();

            (component, admin_badge)
        }

        /**
         * Enrolls a public key from a DAPP into the subscription
         *
         * Subscriptions are managed by the dapp owner and then the dapp operates with its own public key.
         */
        pub fn update_subscription_pub_key(
            &mut self,
            new_public_key: String,
            token_proof: NonFungibleProof,
        ) {
            let checked_proof = token_proof.check(self.subscription_manager.address());
            info!(
                "Local ID: {}",
                &checked_proof.as_non_fungible().non_fungible_local_id()
            );
            self.subscription_manager.update_non_fungible_data(
                &checked_proof.as_non_fungible().non_fungible_local_id(),
                "authorized_pub_key",
                new_public_key.to_string(),
            );
        }

        /**
         * Sets the subscription price
         */
        pub fn set_subscription_price(&mut self, new_price: Decimal) {
            self.monthly_subscription_fee = new_price;
        }

        /// Creates a new subscription.
        ///
        /// # Arguments
        ///
        /// * `months`: number of months to be subscribed.
        /// * `payment`: An XRD bucket containing at least the payment for the desired month subscription.
        ///
        /// # Returns
        ///
        /// A new subscription soulbound NFT and the remaining tokens.
        pub fn new_subscription(
            &mut self,
            months: u64,
            payment: FungibleBucket,
        ) -> (FungibleBucket, NonFungibleBucket) {
            // First check that the payment is ok
            let remaining_tokens = self.check_payment(months, payment);

            // If it's ok, then mint a new subscription with the correct data
            let expiration_time = get_time() + SECONDS_IN_A_MONTH * (months);
            let subscription_data = OracleSubscription {
                expiration_time,
                authorized_pub_key: "".to_string(),
            };

            let subscription = self
                .subscription_manager
                .mint_ruid_non_fungible(subscription_data)
                .as_non_fungible();

            let ids = subscription.non_fungibles::<OracleSubscription>();
            let id = ids.first().unwrap().local_id();

            Runtime::emit_event(OracleSubscriptionUpdate {
                new_expiration_time: expiration_time,
                nft_id: id.clone(),
            });

            (remaining_tokens, subscription)
        }

        /// Renews a given subscription by adding a set amount of months to it.
        ///
        /// # Arguments
        ///
        /// - `subscription_proof`: Proof of the current subscription.
        /// - `months`: amount of months of subscription to add.
        /// - `payment`: An XRD bucket containing at least the payment for the desired month subscription.
        ///
        /// # Returns
        /// The remaining tokens.
        pub fn renew_subscription(
            &mut self,
            subscription_id: NonFungibleLocalId,
            months: u64,
            payment: FungibleBucket,
        ) -> FungibleBucket {
            let subscription_data: OracleSubscription = self
                .subscription_manager
                .get_non_fungible_data(&subscription_id);

            let remaining_tokens = self.check_payment(months, payment);

            let new_subscription_end =
                max(get_time(), subscription_data.expiration_time) + SECONDS_IN_A_MONTH * (months);

            self.subscription_manager.update_non_fungible_data(
                &subscription_id,
                "expiration_time",
                new_subscription_end,
            );

            Runtime::emit_event(OracleSubscriptionUpdate {
                new_expiration_time: new_subscription_end,
                nft_id: subscription_id,
            });

            remaining_tokens
        }

        // pub fn set_subscription_cur_nonce(
        //     &mut self,
        //     subscription_id: NonFungibleLocalId,
        //     oracle_request_msg: String,
        //     signature: String,
        // ) {
        //     let subscription_data: OracleSubscription = self
        //         .subscription_manager
        //         .get_non_fungible_data(&subscription_id);

        //     //todo: check signature matching pub_key so that its a non-custodial update (subscription_id, etc etc)
        //     let oracle_request_message =
        //         OracleRequestMessage::from_str(&oracle_request_msg).unwrap();
        //     check_signature(&oracle_request_msg, &signature, self.authorized_pub_key);

        //     assert!(
        //         subscription_data.cur_nonce <= oracle_request_message.nonce,
        //         "The new nonce must be bigger than the previous nonce"
        //     );

        //     self.subscription_manager.update_non_fungible_data(
        //         &subscription_id,
        //         "cur_nonce",
        //         oracle_request_message.nonce,
        //     );
        // }

        // pub fn check_price_input(
        //     &mut self,
        //     message: String,
        //     signature: String,
        // ) -> PriceMessage {

        //     // Then check the message is correct
        //     check_signature(&message, &signature, self.authorized_pub_key);

        //     // If everything is fine, parse the message
        //     let price_message = PriceMessage::from_str(&message).unwrap();

        //     // Check that the nonce has not been used
        //     assert!(
        //         self.used_nonce.insert(price_message.nonce),
        //         "This nonce has already been used"
        //     );

        //     price_message
        // }

        // pub fn check_prices_input(
        //     &mut self,
        //     message: String,
        //     signature: String,
        // ) -> Vec<PriceMessage> {

        //     check_signature(&message, &signature, self.authorized_pub_key);

        //     // Split the message into individual PriceMessages assuming they are separated by commas
        //     let messages: Vec<&str> = message.split(',').collect();
        //     let mut price_messages = Vec::new();

        //     for msg in messages {
        //         let price_message = PriceMessage::from_str(msg).unwrap();
        //         // Check that the nonce has not been used
        //         assert!(
        //             self.used_nonce.insert(price_message.nonce),
        //             "This nonce has already been used"
        //         );
        //         price_messages.push(price_message);
        //     }

        //     price_messages
        // }

        /// Collects subscription fees.
        ///
        /// # Returns
        /// All paid subscription fees.
        pub fn collect_subscription_fees(&mut self) -> FungibleBucket {
            self.subscription_fees_vault.take_all()
        }

        fn check_payment(&mut self, months: u64, mut payment: FungibleBucket) -> FungibleBucket {
            assert!(months > 0, "Cannot add 0 months to the subscription!");
            assert_eq!(
                payment.resource_address(),
                XRD,
                "The payment should be made in XRD tokens!"
            );

            let expected_payment = months * self.monthly_subscription_fee;
            assert!(
                expected_payment <= payment.amount(),
                "For {} months, the expected payment is {} XRD (only supplied {})",
                months,
                expected_payment,
                payment.amount()
            );

            self.subscription_fees_vault
                .put(payment.take(expected_payment));

            payment
        }
    }
}
