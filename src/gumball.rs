use scrypto::prelude::*;

use crate::oracle::morpher_oracle::MorpherOracle;
use crate::price_message::PriceMessage;
use crate::utils::get_time;

#[derive(ScryptoSbor)]
pub struct Status {
    pub amount: Decimal,
}

#[blueprint]
mod gumball_machine {
    enable_method_auth! {
        // decide which methods are public and which are restricted to the component's owner
        methods {
            buy_gumball => PUBLIC;
            get_status => PUBLIC;
            withdraw_earnings => restrict_to: [OWNER];
            refill_gumball_machine => restrict_to: [OWNER];
        }
    }
    struct GumballMachine {
        gum_resource_manager: ResourceManager,
        gumballs: Vault,
        collected_xrd: Vault,
        price_lifetime: u64,
        oracle_address: ComponentAddress,
    }

    impl GumballMachine {
        // given a price in XRD, creates a ready-to-use gumball machine
        pub fn instantiate_gumball_machine(
            price_lifetime: u64,
            oracle_address: ComponentAddress,
            dapp_definition: ComponentAddress,
        ) -> (Global<GumballMachine>, Bucket) {
            // reserve an address for the component
            let (address_reservation, component_address) =
                Runtime::allocate_component_address(GumballMachine::blueprint_id());

            // create a new Owner Badge resource, with a fixed quantity of 1
            let owner_badge: Bucket = ResourceBuilder::new_fungible(OwnerRole::None)
                .metadata(metadata!(init{
                    "name" => "Gumball Machine Owner Badge", locked;
                }))
                .divisibility(DIVISIBILITY_NONE)
                .mint_initial_supply(1)
                .into();

            // create a new Gumball resource, with an initial supply of 100
            let bucket_of_gumballs: Bucket = ResourceBuilder::new_fungible(OwnerRole::Fixed(rule!(require(
                owner_badge.resource_address()
            ))))
                .divisibility(DIVISIBILITY_NONE)
                .metadata(metadata!(
                    init {
                        "name" => "Gumball in USD", locked;
                        "symbol" => "GUM", locked;
                        "description" => "A delicious gumball", locked;
                        "icon_url" => Url::of("https://assets.radixdlt.com/icons/icon-gumball-pink.png"), locked;
                    }
                ))
                // adding minting rules allows the minting of more gumballs
                .mint_roles(mint_roles! {
                    minter => rule!(require(global_caller(component_address)));
                    minter_updater => rule!(deny_all);
                })
                .mint_initial_supply(100)
                .into();

            // populate a GumballMachine struct and instantiate a new component
            let component = Self {
                gum_resource_manager: bucket_of_gumballs.resource_manager(),
                gumballs: Vault::with_bucket(bucket_of_gumballs),
                collected_xrd: Vault::new(XRD),
                price_lifetime,
                oracle_address,
            }
            .instantiate()
            // Assign the component owner role to the possessor of the owner_badge resource
            .prepare_to_globalize(OwnerRole::Fixed(rule!(require(
                owner_badge.resource_address()
            ))))
            .with_address(address_reservation)
            .metadata(metadata!(roles {
            metadata_setter => rule!(deny_all);
            metadata_setter_updater => rule!(deny_all);
            metadata_locker => rule!(deny_all);
            metadata_locker_updater => rule!(deny_all);
            },
            init {
                    "dapp_definition" => GlobalAddress::from(dapp_definition), updatable;
                    "name" => "Gumball Component", updatable;
                }))
            .globalize();

            (component, owner_badge)
        }

        pub fn buy_gumball(
            &mut self,
            mut payment: Bucket,
            message: String,
            signature: String,
        ) -> (Bucket, Bucket) {
            // Checks the price message is right from the oracle
            let price_message = self.make_all_trading_checks(message, signature);

            // Check that the payment is made in XRD and is enough.
            assert_eq!(
                payment.resource_address(),
                XRD,
                "Cannot buy with other tokens than XRD"
            );
            // take our price in XRD out of the payment
            // if the caller has sent too few, or sent something other than XRD, they'll get a runtime error
            let price = 1 / price_message.price;
            let our_share = payment.take(price);
            self.collected_xrd.put(our_share);

            // return a tuple containing a gumball, plus whatever change is left on the input payment (if any)
            // if we're out of gumballs to give, we'll see a runtime error when we try to grab one
            (self.gumballs.take(1), payment)
        }

        pub fn get_status(&self) -> Status {
            Status {
                amount: self.gumballs.amount(),
            }
        }

        pub fn withdraw_earnings(&mut self) -> Bucket {
            // retrieve all the XRD collected by the gumball machine component.
            // requires the owner badge
            self.collected_xrd.take_all()
        }

        pub fn refill_gumball_machine(&mut self) {
            // mint enough gumball tokens to fill the gumball machine back up to 100.
            // requires the owner badge
            let gumball_amount = 100 - self.gumballs.amount();
            self.gumballs
                .put(self.gum_resource_manager.mint(gumball_amount));
        }

        fn make_all_trading_checks(&mut self, message: String, signature: String) -> PriceMessage {
            let morpher_oracle: Global<MorpherOracle> = self.oracle_address.into();

            let price_message = morpher_oracle.check_price_input(
                // self.subscription.create_proof_of_non_fungibles(&ids),
                message, signature,
            );

            // Checks that the oracle has signed the price no more than price_lifetime seconds ago (i.e. 120)
            assert!(
                price_message.oracle_timestamp + self.price_lifetime >= get_time(),
                "This price is out of date!"
            );

            // Checks that the data timestamp is not outdated in respect to the market status
            // i.e. if the market closed at 17:00 the data timestamp has to be at least 16:59
            assert!(
                price_message.data_timestamp + 60 >= price_message.market_status_timestamp,
                "This price is out of date!"
            );

            price_message
        }
    }
}
