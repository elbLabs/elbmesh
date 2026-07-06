use elbmesh_core::{ActionScenario, ArchitectureCheckStatus};

use serde_json::Value;

use reference_flow::{
    AcceptOfferV1, CreateInvoiceV1, CreateOfferV1, CreateOrderConfirmationV1, CreateSalesOrderV1,
    Invoice, InvoiceCreatedV1, InvoiceError, Offer, OfferAcceptedV1, OfferCreatedV1, OfferError,
    OrderConfirmation, OrderConfirmationCreatedV1, OrderConfirmationError, SalesOrder,
    SalesOrderCreatedV1, SalesOrderError,
};

mod reference_flow {
    use std::fmt;

    use async_trait::async_trait;
    use elbmesh_core::{
        apply_recorded_event, Action, ActionContext, ActionDecision, ActionDefinition,
        ActionFailure, Apply, ArchitectureManifest, Event, EventDefinition, Handle, HandlerError,
        Resource, ResourceDefinition, ResourceError,
    };
    use serde::{Deserialize, Serialize};
    use serde_json::json;

    pub fn architecture_manifest() -> ArchitectureManifest {
        ArchitectureManifest {
            manifest_schema_id: "elbmesh.reference_flow.architecture_manifest".to_string(),
            manifest_schema_version: 1,
            resources: vec![
                ResourceDefinition {
                    resource_type: Offer::RESOURCE_TYPE.to_string(),
                    schema_id: "resource.offer.v1".to_string(),
                    schema_version: 1,
                    components: Vec::new(),
                },
                ResourceDefinition {
                    resource_type: SalesOrder::RESOURCE_TYPE.to_string(),
                    schema_id: "resource.sales_order.v1".to_string(),
                    schema_version: 1,
                    components: Vec::new(),
                },
                ResourceDefinition {
                    resource_type: OrderConfirmation::RESOURCE_TYPE.to_string(),
                    schema_id: "resource.order_confirmation.v1".to_string(),
                    schema_version: 1,
                    components: Vec::new(),
                },
                ResourceDefinition {
                    resource_type: Invoice::RESOURCE_TYPE.to_string(),
                    schema_id: "resource.invoice.v1".to_string(),
                    schema_version: 1,
                    components: Vec::new(),
                },
            ],
            actions: vec![
                ActionDefinition {
                    action_type: CreateOfferV1::ACTION_TYPE.to_string(),
                    resource_type: Offer::RESOURCE_TYPE.to_string(),
                    schema_id: CreateOfferV1::SCHEMA_ID.to_string(),
                    schema_version: CreateOfferV1::SCHEMA_VERSION,
                    emitted_event_types: vec![OfferCreatedV1::EVENT_TYPE.to_string()],
                    external_operation_types: Vec::new(),
                },
                ActionDefinition {
                    action_type: AcceptOfferV1::ACTION_TYPE.to_string(),
                    resource_type: Offer::RESOURCE_TYPE.to_string(),
                    schema_id: AcceptOfferV1::SCHEMA_ID.to_string(),
                    schema_version: AcceptOfferV1::SCHEMA_VERSION,
                    emitted_event_types: vec![OfferAcceptedV1::EVENT_TYPE.to_string()],
                    external_operation_types: Vec::new(),
                },
                ActionDefinition {
                    action_type: CreateSalesOrderV1::ACTION_TYPE.to_string(),
                    resource_type: SalesOrder::RESOURCE_TYPE.to_string(),
                    schema_id: CreateSalesOrderV1::SCHEMA_ID.to_string(),
                    schema_version: CreateSalesOrderV1::SCHEMA_VERSION,
                    emitted_event_types: vec![SalesOrderCreatedV1::EVENT_TYPE.to_string()],
                    external_operation_types: Vec::new(),
                },
                ActionDefinition {
                    action_type: CreateOrderConfirmationV1::ACTION_TYPE.to_string(),
                    resource_type: OrderConfirmation::RESOURCE_TYPE.to_string(),
                    schema_id: CreateOrderConfirmationV1::SCHEMA_ID.to_string(),
                    schema_version: CreateOrderConfirmationV1::SCHEMA_VERSION,
                    emitted_event_types: vec![OrderConfirmationCreatedV1::EVENT_TYPE.to_string()],
                    external_operation_types: Vec::new(),
                },
                ActionDefinition {
                    action_type: CreateInvoiceV1::ACTION_TYPE.to_string(),
                    resource_type: Invoice::RESOURCE_TYPE.to_string(),
                    schema_id: CreateInvoiceV1::SCHEMA_ID.to_string(),
                    schema_version: CreateInvoiceV1::SCHEMA_VERSION,
                    emitted_event_types: vec![InvoiceCreatedV1::EVENT_TYPE.to_string()],
                    external_operation_types: Vec::new(),
                },
            ],
            events: vec![
                EventDefinition {
                    event_type: OfferCreatedV1::EVENT_TYPE.to_string(),
                    resource_type: Offer::RESOURCE_TYPE.to_string(),
                    schema_id: OfferCreatedV1::SCHEMA_ID.to_string(),
                    schema_version: OfferCreatedV1::SCHEMA_VERSION,
                },
                EventDefinition {
                    event_type: OfferAcceptedV1::EVENT_TYPE.to_string(),
                    resource_type: Offer::RESOURCE_TYPE.to_string(),
                    schema_id: OfferAcceptedV1::SCHEMA_ID.to_string(),
                    schema_version: OfferAcceptedV1::SCHEMA_VERSION,
                },
                EventDefinition {
                    event_type: SalesOrderCreatedV1::EVENT_TYPE.to_string(),
                    resource_type: SalesOrder::RESOURCE_TYPE.to_string(),
                    schema_id: SalesOrderCreatedV1::SCHEMA_ID.to_string(),
                    schema_version: SalesOrderCreatedV1::SCHEMA_VERSION,
                },
                EventDefinition {
                    event_type: OrderConfirmationCreatedV1::EVENT_TYPE.to_string(),
                    resource_type: OrderConfirmation::RESOURCE_TYPE.to_string(),
                    schema_id: OrderConfirmationCreatedV1::SCHEMA_ID.to_string(),
                    schema_version: OrderConfirmationCreatedV1::SCHEMA_VERSION,
                },
                EventDefinition {
                    event_type: InvoiceCreatedV1::EVENT_TYPE.to_string(),
                    resource_type: Invoice::RESOURCE_TYPE.to_string(),
                    schema_id: InvoiceCreatedV1::SCHEMA_ID.to_string(),
                    schema_version: InvoiceCreatedV1::SCHEMA_VERSION,
                },
            ],
            reactions: Vec::new(),
            views: Vec::new(),
            queries: Vec::new(),
            external_operations: Vec::new(),
        }
    }

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

    #[derive(Debug, Default, Clone)]
    pub struct SalesOrder {
        id: Option<String>,
        offer_id: Option<String>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum SalesOrderError {
        AlreadyExists,
    }

    impl fmt::Display for SalesOrderError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::AlreadyExists => write!(f, "sales order already exists"),
            }
        }
    }

    impl ActionFailure for SalesOrderError {
        fn code(&self) -> &'static str {
            match self {
                Self::AlreadyExists => "sales_order.already_exists",
            }
        }

        fn details(&self) -> serde_json::Value {
            json!({
                "error_type": "SalesOrderError",
                "error_variant": match self {
                    Self::AlreadyExists => "AlreadyExists",
                },
            })
        }
    }

    impl Resource for SalesOrder {
        type Id = String;

        const RESOURCE_TYPE: &'static str = "sales_order";

        fn apply_recorded(
            &mut self,
            event: &elbmesh_core::RecordedEvent,
        ) -> Result<(), ResourceError> {
            if apply_recorded_event::<Self, SalesOrderCreatedV1>(self, event)? {
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
    pub struct CreateSalesOrderV1 {
        pub sales_order_id: String,
        pub offer_id: String,
    }

    impl Action for CreateSalesOrderV1 {
        type Resource = SalesOrder;

        const ACTION_TYPE: &'static str = "create_sales_order";
        const SCHEMA_ID: &'static str = "action.create_sales_order.v1";
        const SCHEMA_VERSION: u32 = 1;

        fn resource_id(&self) -> <Self::Resource as Resource>::Id {
            self.sales_order_id.clone()
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct SalesOrderCreatedV1 {
        pub sales_order_id: String,
        pub offer_id: String,
    }

    impl Event for SalesOrderCreatedV1 {
        type Resource = SalesOrder;

        const EVENT_TYPE: &'static str = "sales_order_created";
        const SCHEMA_ID: &'static str = "event.sales_order_created.v1";
        const SCHEMA_VERSION: u32 = 1;

        fn resource_id(&self) -> <Self::Resource as Resource>::Id {
            self.sales_order_id.clone()
        }
    }

    impl Apply<SalesOrderCreatedV1> for SalesOrder {
        fn apply(&mut self, event: SalesOrderCreatedV1) -> Result<(), ResourceError> {
            self.id = Some(event.sales_order_id);
            self.offer_id = Some(event.offer_id);
            Ok(())
        }
    }

    #[async_trait]
    impl Handle<CreateSalesOrderV1> for SalesOrder {
        type Error = SalesOrderError;

        async fn handle(
            &mut self,
            action: CreateSalesOrderV1,
            ctx: &mut ActionContext<Self>,
        ) -> Result<ActionDecision, HandlerError<Self::Error>> {
            if self.id.is_some() {
                return Err(HandlerError::domain(SalesOrderError::AlreadyExists));
            }

            ctx.record_applied(
                self,
                SalesOrderCreatedV1 {
                    sales_order_id: action.sales_order_id,
                    offer_id: action.offer_id,
                },
            )?;

            Ok(ActionDecision::with_message("sales order created"))
        }
    }

    #[derive(Debug, Default, Clone)]
    pub struct OrderConfirmation {
        id: Option<String>,
        sales_order_id: Option<String>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum OrderConfirmationError {
        AlreadyExists,
    }

    impl fmt::Display for OrderConfirmationError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::AlreadyExists => write!(f, "order confirmation already exists"),
            }
        }
    }

    impl ActionFailure for OrderConfirmationError {
        fn code(&self) -> &'static str {
            match self {
                Self::AlreadyExists => "order_confirmation.already_exists",
            }
        }

        fn details(&self) -> serde_json::Value {
            json!({
                "error_type": "OrderConfirmationError",
                "error_variant": match self {
                    Self::AlreadyExists => "AlreadyExists",
                },
            })
        }
    }

    impl Resource for OrderConfirmation {
        type Id = String;

        const RESOURCE_TYPE: &'static str = "order_confirmation";

        fn apply_recorded(
            &mut self,
            event: &elbmesh_core::RecordedEvent,
        ) -> Result<(), ResourceError> {
            if apply_recorded_event::<Self, OrderConfirmationCreatedV1>(self, event)? {
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
    pub struct CreateOrderConfirmationV1 {
        pub order_confirmation_id: String,
        pub sales_order_id: String,
    }

    impl Action for CreateOrderConfirmationV1 {
        type Resource = OrderConfirmation;

        const ACTION_TYPE: &'static str = "create_order_confirmation";
        const SCHEMA_ID: &'static str = "action.create_order_confirmation.v1";
        const SCHEMA_VERSION: u32 = 1;

        fn resource_id(&self) -> <Self::Resource as Resource>::Id {
            self.order_confirmation_id.clone()
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct OrderConfirmationCreatedV1 {
        pub order_confirmation_id: String,
        pub sales_order_id: String,
    }

    impl Event for OrderConfirmationCreatedV1 {
        type Resource = OrderConfirmation;

        const EVENT_TYPE: &'static str = "order_confirmation_created";
        const SCHEMA_ID: &'static str = "event.order_confirmation_created.v1";
        const SCHEMA_VERSION: u32 = 1;

        fn resource_id(&self) -> <Self::Resource as Resource>::Id {
            self.order_confirmation_id.clone()
        }
    }

    impl Apply<OrderConfirmationCreatedV1> for OrderConfirmation {
        fn apply(&mut self, event: OrderConfirmationCreatedV1) -> Result<(), ResourceError> {
            self.id = Some(event.order_confirmation_id);
            self.sales_order_id = Some(event.sales_order_id);
            Ok(())
        }
    }

    #[async_trait]
    impl Handle<CreateOrderConfirmationV1> for OrderConfirmation {
        type Error = OrderConfirmationError;

        async fn handle(
            &mut self,
            action: CreateOrderConfirmationV1,
            ctx: &mut ActionContext<Self>,
        ) -> Result<ActionDecision, HandlerError<Self::Error>> {
            if self.id.is_some() {
                return Err(HandlerError::domain(OrderConfirmationError::AlreadyExists));
            }

            ctx.record_applied(
                self,
                OrderConfirmationCreatedV1 {
                    order_confirmation_id: action.order_confirmation_id,
                    sales_order_id: action.sales_order_id,
                },
            )?;

            Ok(ActionDecision::with_message("order confirmation created"))
        }
    }

    #[derive(Debug, Default, Clone)]
    pub struct Invoice {
        id: Option<String>,
        order_confirmation_id: Option<String>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum InvoiceError {
        AlreadyExists,
    }

    impl fmt::Display for InvoiceError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::AlreadyExists => write!(f, "invoice already exists"),
            }
        }
    }

    impl ActionFailure for InvoiceError {
        fn code(&self) -> &'static str {
            match self {
                Self::AlreadyExists => "invoice.already_exists",
            }
        }

        fn details(&self) -> serde_json::Value {
            json!({
                "error_type": "InvoiceError",
                "error_variant": match self {
                    Self::AlreadyExists => "AlreadyExists",
                },
            })
        }
    }

    impl Resource for Invoice {
        type Id = String;

        const RESOURCE_TYPE: &'static str = "invoice";

        fn apply_recorded(
            &mut self,
            event: &elbmesh_core::RecordedEvent,
        ) -> Result<(), ResourceError> {
            if apply_recorded_event::<Self, InvoiceCreatedV1>(self, event)? {
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
    pub struct CreateInvoiceV1 {
        pub invoice_id: String,
        pub order_confirmation_id: String,
    }

    impl Action for CreateInvoiceV1 {
        type Resource = Invoice;

        const ACTION_TYPE: &'static str = "create_invoice";
        const SCHEMA_ID: &'static str = "action.create_invoice.v1";
        const SCHEMA_VERSION: u32 = 1;

        fn resource_id(&self) -> <Self::Resource as Resource>::Id {
            self.invoice_id.clone()
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct InvoiceCreatedV1 {
        pub invoice_id: String,
        pub order_confirmation_id: String,
    }

    impl Event for InvoiceCreatedV1 {
        type Resource = Invoice;

        const EVENT_TYPE: &'static str = "invoice_created";
        const SCHEMA_ID: &'static str = "event.invoice_created.v1";
        const SCHEMA_VERSION: u32 = 1;

        fn resource_id(&self) -> <Self::Resource as Resource>::Id {
            self.invoice_id.clone()
        }
    }

    impl Apply<InvoiceCreatedV1> for Invoice {
        fn apply(&mut self, event: InvoiceCreatedV1) -> Result<(), ResourceError> {
            self.id = Some(event.invoice_id);
            self.order_confirmation_id = Some(event.order_confirmation_id);
            Ok(())
        }
    }

    #[async_trait]
    impl Handle<CreateInvoiceV1> for Invoice {
        type Error = InvoiceError;

        async fn handle(
            &mut self,
            action: CreateInvoiceV1,
            ctx: &mut ActionContext<Self>,
        ) -> Result<ActionDecision, HandlerError<Self::Error>> {
            if self.id.is_some() {
                return Err(HandlerError::domain(InvoiceError::AlreadyExists));
            }

            ctx.record_applied(
                self,
                InvoiceCreatedV1 {
                    invoice_id: action.invoice_id,
                    order_confirmation_id: action.order_confirmation_id,
                },
            )?;

            Ok(ActionDecision::with_message("invoice created"))
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

#[tokio::test]
async fn create_sales_order_emits_sales_order_created() {
    ActionScenario::<SalesOrder>::new()
        .when(CreateSalesOrderV1 {
            sales_order_id: "sales-order-1".to_string(),
            offer_id: "offer-1".to_string(),
        })
        .then(vec![SalesOrderCreatedV1 {
            sales_order_id: "sales-order-1".to_string(),
            offer_id: "offer-1".to_string(),
        }])
        .assert()
        .await;
}

#[tokio::test]
async fn create_sales_order_twice_returns_typed_already_exists_error() {
    ActionScenario::<SalesOrder>::new()
        .given(vec![SalesOrderCreatedV1 {
            sales_order_id: "sales-order-1".to_string(),
            offer_id: "offer-1".to_string(),
        }])
        .when(CreateSalesOrderV1 {
            sales_order_id: "sales-order-1".to_string(),
            offer_id: "offer-1".to_string(),
        })
        .then_error(SalesOrderError::AlreadyExists)
        .assert()
        .await;
}

#[tokio::test]
async fn create_order_confirmation_emits_order_confirmation_created() {
    ActionScenario::<OrderConfirmation>::new()
        .when(CreateOrderConfirmationV1 {
            order_confirmation_id: "order-confirmation-1".to_string(),
            sales_order_id: "sales-order-1".to_string(),
        })
        .then(vec![OrderConfirmationCreatedV1 {
            order_confirmation_id: "order-confirmation-1".to_string(),
            sales_order_id: "sales-order-1".to_string(),
        }])
        .assert()
        .await;
}

#[tokio::test]
async fn create_order_confirmation_twice_returns_typed_already_exists_error() {
    ActionScenario::<OrderConfirmation>::new()
        .given(vec![OrderConfirmationCreatedV1 {
            order_confirmation_id: "order-confirmation-1".to_string(),
            sales_order_id: "sales-order-1".to_string(),
        }])
        .when(CreateOrderConfirmationV1 {
            order_confirmation_id: "order-confirmation-1".to_string(),
            sales_order_id: "sales-order-1".to_string(),
        })
        .then_error(OrderConfirmationError::AlreadyExists)
        .assert()
        .await;
}

#[tokio::test]
async fn create_invoice_emits_invoice_created() {
    ActionScenario::<Invoice>::new()
        .when(CreateInvoiceV1 {
            invoice_id: "invoice-1".to_string(),
            order_confirmation_id: "order-confirmation-1".to_string(),
        })
        .then(vec![InvoiceCreatedV1 {
            invoice_id: "invoice-1".to_string(),
            order_confirmation_id: "order-confirmation-1".to_string(),
        }])
        .assert()
        .await;
}

#[tokio::test]
async fn create_invoice_twice_returns_typed_already_exists_error() {
    ActionScenario::<Invoice>::new()
        .given(vec![InvoiceCreatedV1 {
            invoice_id: "invoice-1".to_string(),
            order_confirmation_id: "order-confirmation-1".to_string(),
        }])
        .when(CreateInvoiceV1 {
            invoice_id: "invoice-1".to_string(),
            order_confirmation_id: "order-confirmation-1".to_string(),
        })
        .then_error(InvoiceError::AlreadyExists)
        .assert()
        .await;
}

#[test]
fn reference_flow_manifest_validates_successfully() {
    reference_flow::architecture_manifest()
        .validate()
        .expect("reference flow manifest should validate");
}

#[test]
fn reference_flow_architecture_check_report_passes() {
    let report = reference_flow::architecture_manifest().check_architecture();

    assert_eq!(ArchitectureCheckStatus::Passed, report.status);
    assert!(report.findings.is_empty());
}

#[test]
fn reference_flow_manifest_json_names_resources_actions_and_events() {
    let manifest_json = serde_json::to_value(reference_flow::architecture_manifest())
        .expect("reference flow manifest should serialize");

    assert_eq!(
        "elbmesh.reference_flow.architecture_manifest",
        manifest_json["manifest_schema_id"]
            .as_str()
            .expect("manifest schema id should be a string")
    );
    assert_eq!(1, manifest_json["manifest_schema_version"]);
    assert_eq!(
        vec!["offer", "sales_order", "order_confirmation", "invoice"],
        json_field_values(&manifest_json, "resources", "resource_type")
    );
    assert_eq!(
        vec![
            "create_offer",
            "accept_offer",
            "create_sales_order",
            "create_order_confirmation",
            "create_invoice",
        ],
        json_field_values(&manifest_json, "actions", "action_type")
    );
    assert_eq!(
        vec![
            "offer_created",
            "offer_accepted",
            "sales_order_created",
            "order_confirmation_created",
            "invoice_created",
        ],
        json_field_values(&manifest_json, "events", "event_type")
    );
}

fn json_field_values(manifest_json: &Value, array_key: &str, field_key: &str) -> Vec<String> {
    manifest_json[array_key]
        .as_array()
        .expect("manifest field should be an array")
        .iter()
        .map(|entry| {
            entry[field_key]
                .as_str()
                .expect("manifest entry field should be a string")
                .to_string()
        })
        .collect()
}
