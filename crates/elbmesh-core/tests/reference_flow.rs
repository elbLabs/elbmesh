use elbmesh_core::ActionScenario;

use reference_flow::{
    AcceptOfferV1, CreateOfferV1, Offer, OfferAcceptedV1, OfferCreatedV1, OfferError,
};

mod reference_flow {
    use std::fmt;

    use async_trait::async_trait;
    use elbmesh_core::{
        apply_recorded_event, Action, ActionContext, ActionDecision, ActionFailure, Apply, Event,
        Handle, HandlerError, Resource, ResourceError,
    };
    use serde::{Deserialize, Serialize};
    use serde_json::json;

    #[derive(Debug, Default, Clone)]
    pub struct Offer {
        id: Option<String>,
        title: Option<String>,
        accepted: bool,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum OfferError {
        AlreadyExists,
        MissingOffer,
        AlreadyAccepted,
    }

    impl fmt::Display for OfferError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::AlreadyExists => write!(f, "offer already exists"),
                Self::MissingOffer => write!(f, "offer does not exist"),
                Self::AlreadyAccepted => write!(f, "offer already accepted"),
            }
        }
    }

    impl ActionFailure for OfferError {
        fn code(&self) -> &'static str {
            match self {
                Self::AlreadyExists => "offer.already_exists",
                Self::MissingOffer => "offer.missing_offer",
                Self::AlreadyAccepted => "offer.already_accepted",
            }
        }

        fn details(&self) -> serde_json::Value {
            json!({
                "error_type": "OfferError",
                "error_variant": match self {
                    Self::AlreadyExists => "AlreadyExists",
                    Self::MissingOffer => "MissingOffer",
                    Self::AlreadyAccepted => "AlreadyAccepted",
                },
            })
        }
    }

    impl Resource for Offer {
        type Id = String;

        const RESOURCE_TYPE: &'static str = "offer";

        fn apply_recorded(
            &mut self,
            event: &elbmesh_core::RecordedEvent,
        ) -> Result<(), ResourceError> {
            if apply_recorded_event::<Self, OfferCreatedV1>(self, event)? {
                return Ok(());
            }

            if apply_recorded_event::<Self, OfferAcceptedV1>(self, event)? {
                return Ok(());
            }

            Err(ResourceError::UnsupportedEvent {
                resource_type: Self::RESOURCE_TYPE.to_string(),
                message_type: event.metadata.message_type.clone(),
                schema_version: event.metadata.schema_version,
            })
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct CreateOfferV1 {
        pub offer_id: String,
        pub title: String,
    }

    impl Action for CreateOfferV1 {
        type Resource = Offer;

        const ACTION_TYPE: &'static str = "create_offer";
        const SCHEMA_ID: &'static str = "action.create_offer.v1";
        const SCHEMA_VERSION: u32 = 1;

        fn resource_id(&self) -> <Self::Resource as Resource>::Id {
            self.offer_id.clone()
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct AcceptOfferV1 {
        pub offer_id: String,
    }

    impl Action for AcceptOfferV1 {
        type Resource = Offer;

        const ACTION_TYPE: &'static str = "accept_offer";
        const SCHEMA_ID: &'static str = "action.accept_offer.v1";
        const SCHEMA_VERSION: u32 = 1;

        fn resource_id(&self) -> <Self::Resource as Resource>::Id {
            self.offer_id.clone()
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct OfferCreatedV1 {
        pub offer_id: String,
        pub title: String,
    }

    impl Event for OfferCreatedV1 {
        type Resource = Offer;

        const EVENT_TYPE: &'static str = "offer_created";
        const SCHEMA_ID: &'static str = "event.offer_created.v1";
        const SCHEMA_VERSION: u32 = 1;

        fn resource_id(&self) -> <Self::Resource as Resource>::Id {
            self.offer_id.clone()
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct OfferAcceptedV1 {
        pub offer_id: String,
    }

    impl Event for OfferAcceptedV1 {
        type Resource = Offer;

        const EVENT_TYPE: &'static str = "offer_accepted";
        const SCHEMA_ID: &'static str = "event.offer_accepted.v1";
        const SCHEMA_VERSION: u32 = 1;

        fn resource_id(&self) -> <Self::Resource as Resource>::Id {
            self.offer_id.clone()
        }
    }

    impl Apply<OfferCreatedV1> for Offer {
        fn apply(&mut self, event: OfferCreatedV1) -> Result<(), ResourceError> {
            self.id = Some(event.offer_id);
            self.title = Some(event.title);
            Ok(())
        }
    }

    impl Apply<OfferAcceptedV1> for Offer {
        fn apply(&mut self, _event: OfferAcceptedV1) -> Result<(), ResourceError> {
            self.accepted = true;
            Ok(())
        }
    }

    #[async_trait]
    impl Handle<CreateOfferV1> for Offer {
        type Error = OfferError;

        async fn handle(
            &mut self,
            action: CreateOfferV1,
            ctx: &mut ActionContext<Self>,
        ) -> Result<ActionDecision, HandlerError<Self::Error>> {
            if self.id.is_some() {
                return Err(HandlerError::domain(OfferError::AlreadyExists));
            }

            ctx.record_applied(
                self,
                OfferCreatedV1 {
                    offer_id: action.offer_id,
                    title: action.title,
                },
            )?;

            Ok(ActionDecision::with_message("offer created"))
        }
    }

    #[async_trait]
    impl Handle<AcceptOfferV1> for Offer {
        type Error = OfferError;

        async fn handle(
            &mut self,
            action: AcceptOfferV1,
            ctx: &mut ActionContext<Self>,
        ) -> Result<ActionDecision, HandlerError<Self::Error>> {
            if self.id.is_none() {
                return Err(HandlerError::domain(OfferError::MissingOffer));
            }

            if self.accepted {
                return Err(HandlerError::domain(OfferError::AlreadyAccepted));
            }

            ctx.record_applied(
                self,
                OfferAcceptedV1 {
                    offer_id: action.offer_id,
                },
            )?;

            Ok(ActionDecision::with_message("offer accepted"))
        }
    }
}

#[tokio::test]
async fn create_offer_emits_offer_created() {
    ActionScenario::<Offer>::new()
        .when(CreateOfferV1 {
            offer_id: "offer-1".to_string(),
            title: "Initial offer".to_string(),
        })
        .then(vec![OfferCreatedV1 {
            offer_id: "offer-1".to_string(),
            title: "Initial offer".to_string(),
        }])
        .assert()
        .await;
}

#[tokio::test]
async fn accept_offer_after_create_emits_offer_accepted() {
    ActionScenario::<Offer>::new()
        .given(vec![OfferCreatedV1 {
            offer_id: "offer-1".to_string(),
            title: "Initial offer".to_string(),
        }])
        .when(AcceptOfferV1 {
            offer_id: "offer-1".to_string(),
        })
        .then(vec![OfferAcceptedV1 {
            offer_id: "offer-1".to_string(),
        }])
        .assert()
        .await;
}

#[tokio::test]
async fn create_offer_twice_returns_typed_already_exists_error() {
    ActionScenario::<Offer>::new()
        .given(vec![OfferCreatedV1 {
            offer_id: "offer-1".to_string(),
            title: "Initial offer".to_string(),
        }])
        .when(CreateOfferV1 {
            offer_id: "offer-1".to_string(),
            title: "Replacement offer".to_string(),
        })
        .then_error(OfferError::AlreadyExists)
        .assert()
        .await;
}

#[tokio::test]
async fn accept_offer_before_create_returns_typed_missing_offer_error() {
    ActionScenario::<Offer>::new()
        .when(AcceptOfferV1 {
            offer_id: "offer-1".to_string(),
        })
        .then_error(OfferError::MissingOffer)
        .assert()
        .await;
}

#[tokio::test]
async fn accept_offer_twice_returns_typed_already_accepted_error() {
    ActionScenario::<Offer>::new()
        .given_event(OfferCreatedV1 {
            offer_id: "offer-1".to_string(),
            title: "Initial offer".to_string(),
        })
        .given_event(OfferAcceptedV1 {
            offer_id: "offer-1".to_string(),
        })
        .when(AcceptOfferV1 {
            offer_id: "offer-1".to_string(),
        })
        .then_error(OfferError::AlreadyAccepted)
        .assert()
        .await;
}
