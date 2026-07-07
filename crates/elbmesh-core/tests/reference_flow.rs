use elbmesh_core::{
    ActionExecutor, ActionMetadata, ActionScenario, ArchitectureCheckStatus, EventStore,
    InMemoryEventStore, InMemoryReactionJournal, InMemoryViewStore, ProjectionDispatcher,
    ProjectionRuntime, ReactionDispatcher, ReactionJournal, ReactionJournalRecord,
    ReactionJournalStream, ReactionRuntime, RecordedEvent, ResourceStream, StreamType,
    TypedProjectionHandler, TypedReactionHandler, ViewKey, ViewStore,
};

use serde_json::Value;

use reference_flow::{
    AcceptOfferV1, CreateInvoiceV1, CreateOfferV1, CreateOrderConfirmationV1, CreateSalesOrderV1,
    FlowStatusFromOfferAccepted, FlowStatusFromOfferCreated, FlowStatusFromSalesOrderCreated,
    Invoice, InvoiceCreatedV1, InvoiceError, Offer, OfferAcceptedCreatesSalesOrder,
    OfferAcceptedV1, OfferCreatedV1, OfferError, OrderConfirmation, OrderConfirmationCreatedV1,
    OrderConfirmationError, SalesOrder, SalesOrderCreatedV1, SalesOrderError,
};

mod reference_flow {
    use std::fmt;

    use async_trait::async_trait;
    use elbmesh_core::{
        apply_recorded_event, Action, ActionContext, ActionDecision, ActionDefinition,
        ActionFailure, Apply, ArchitectureManifest, Event, EventDefinition, Handle, HandlerError,
        Projection, Reaction, ReactionDefinition, Resource, ResourceDefinition, ResourceError,
        ViewDefinition, ViewDocument, ViewIndexEntry, ViewStore, ViewStoreError,
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
            reactions: vec![ReactionDefinition {
                reaction_type: OfferAcceptedCreatesSalesOrder::REACTION_TYPE.to_string(),
                trigger_event_type: OfferAcceptedV1::EVENT_TYPE.to_string(),
                target_action_type: CreateSalesOrderV1::ACTION_TYPE.to_string(),
                schema_id: OfferAcceptedCreatesSalesOrder::SCHEMA_ID.to_string(),
                schema_version: OfferAcceptedCreatesSalesOrder::SCHEMA_VERSION,
            }],
            views: vec![ViewDefinition {
                view_type: "flow_status".to_string(),
                schema_id: "view.flow_status.v1".to_string(),
                schema_version: 1,
            }],
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

    pub struct OfferAcceptedCreatesSalesOrder;

    #[async_trait]
    impl Reaction for OfferAcceptedCreatesSalesOrder {
        type Trigger = OfferAcceptedV1;
        type Resource = SalesOrder;
        type Action = CreateSalesOrderV1;

        const REACTION_TYPE: &'static str = "offer_accepted_to_create_sales_order";
        const SCHEMA_ID: &'static str = "reaction.offer_accepted_to_create_sales_order.v1";
        const SCHEMA_VERSION: u32 = 1;

        async fn react(&self, event: Self::Trigger) -> Self::Action {
            CreateSalesOrderV1 {
                sales_order_id: format!("sales-order-for-{}", event.offer_id),
                offer_id: event.offer_id,
            }
        }
    }

    pub struct FlowStatusFromOfferCreated;

    #[async_trait]
    impl Projection for FlowStatusFromOfferCreated {
        type Source = OfferCreatedV1;

        const PROJECTION_TYPE: &'static str = "flow_status_from_offer_created";

        async fn project<V>(
            &self,
            event: Self::Source,
            view_store: &V,
        ) -> Result<(), ViewStoreError>
        where
            V: ViewStore,
        {
            view_store
                .put(flow_status_document(
                    &event.offer_id,
                    "offer_created",
                    json!({
                        "offer_id": event.offer_id,
                        "status": "offer_created",
                        "title": event.title,
                    }),
                ))
                .await
        }
    }

    pub struct FlowStatusFromOfferAccepted;

    #[async_trait]
    impl Projection for FlowStatusFromOfferAccepted {
        type Source = OfferAcceptedV1;

        const PROJECTION_TYPE: &'static str = "flow_status_from_offer_accepted";

        async fn project<V>(
            &self,
            event: Self::Source,
            view_store: &V,
        ) -> Result<(), ViewStoreError>
        where
            V: ViewStore,
        {
            view_store
                .put(flow_status_document(
                    &event.offer_id,
                    "offer_accepted",
                    json!({
                        "offer_id": event.offer_id,
                        "status": "offer_accepted",
                    }),
                ))
                .await
        }
    }

    pub struct FlowStatusFromSalesOrderCreated;

    #[async_trait]
    impl Projection for FlowStatusFromSalesOrderCreated {
        type Source = SalesOrderCreatedV1;

        const PROJECTION_TYPE: &'static str = "flow_status_from_sales_order_created";

        async fn project<V>(
            &self,
            event: Self::Source,
            view_store: &V,
        ) -> Result<(), ViewStoreError>
        where
            V: ViewStore,
        {
            view_store
                .put(flow_status_document(
                    &event.offer_id,
                    "sales_order_created",
                    json!({
                        "offer_id": event.offer_id,
                        "status": "sales_order_created",
                        "sales_order_id": event.sales_order_id,
                    }),
                ))
                .await
        }
    }

    fn flow_status_document(
        offer_id: &str,
        status: &str,
        payload: serde_json::Value,
    ) -> ViewDocument {
        ViewDocument::new("flow_status", offer_id, payload).with_indexes(vec![
            ViewIndexEntry::new("all", offer_id),
            ViewIndexEntry::new("by_status", format!("{status}/{offer_id}")),
        ])
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
async fn dispatching_offer_accepted_creates_sales_order_through_reference_flow_reaction() {
    let event_store = InMemoryEventStore::new();
    let action_executor = ActionExecutor::new(event_store.clone());

    action_executor
        .execute::<Offer, CreateOfferV1>(
            CreateOfferV1 {
                offer_id: "offer-1".to_string(),
                title: "Initial offer".to_string(),
            },
            action_metadata("create-offer-action-1"),
        )
        .await
        .expect("create offer should succeed");
    action_executor
        .execute::<Offer, AcceptOfferV1>(
            AcceptOfferV1 {
                offer_id: "offer-1".to_string(),
            },
            action_metadata("accept-offer-action-1"),
        )
        .await
        .expect("accept offer should succeed");

    let offer_events = event_store
        .load(&ResourceStream::new("offer", "offer-1"))
        .await
        .expect("load offer events");
    let trigger = offer_events
        .iter()
        .find(|event| event.metadata.message_type == "offer_accepted")
        .expect("offer accepted event should exist")
        .clone();
    let expected_action_id =
        ReactionRuntime::<InMemoryEventStore, InMemoryReactionJournal>::reaction_action_id::<
            OfferAcceptedCreatesSalesOrder,
        >(&trigger);

    let reaction_journal = InMemoryReactionJournal::new();
    let dispatcher =
        ReactionDispatcher::new(ReactionRuntime::new(
            event_store.clone(),
            reaction_journal.clone(),
        ))
        .with_handler(TypedReactionHandler::new(
            OfferAcceptedCreatesSalesOrder,
            |trigger: &RecordedEvent| {
                ReactionRuntime::<InMemoryEventStore, InMemoryReactionJournal>::
                reaction_action_metadata::<OfferAcceptedCreatesSalesOrder>(trigger)
            },
        ));

    let receipts = dispatcher
        .dispatch(&trigger)
        .await
        .expect("dispatch should succeed");

    assert_eq!(receipts.len(), 1);
    assert_eq!(receipts[0].action_receipt.action_id, expected_action_id);

    let sales_order_events = dispatcher
        .event_store()
        .load(&ResourceStream::new(
            "sales_order",
            "sales-order-for-offer-1",
        ))
        .await
        .expect("load sales order events");
    assert_eq!(sales_order_events.len(), 1);
    assert_eq!(
        sales_order_events[0].metadata.message_type,
        "sales_order_created"
    );
    assert_eq!(
        sales_order_events[0].metadata.stream_type,
        StreamType::Resource
    );
    assert_eq!(sales_order_events[0].metadata.action_id, expected_action_id);
    assert_eq!(
        sales_order_events[0].payload["sales_order_id"],
        "sales-order-for-offer-1"
    );
    assert_eq!(sales_order_events[0].payload["offer_id"], "offer-1");
    assert!(dispatcher
        .event_store()
        .all_events()
        .iter()
        .all(|event| event.metadata.stream_type == StreamType::Resource));

    let reaction_records = reaction_journal
        .load(&ReactionJournalStream::for_reaction(
            receipts[0].reaction_id.clone(),
        ))
        .await
        .expect("load reaction journal records");
    assert_eq!(reaction_records.len(), 2);
    match &reaction_records[0] {
        ReactionJournalRecord::ReactionTriggered {
            metadata,
            reaction_type,
            trigger_event_type,
            ..
        } => {
            assert_eq!(metadata.stream_type, StreamType::Reaction);
            assert_eq!(reaction_type, "offer_accepted_to_create_sales_order");
            assert_eq!(trigger_event_type, "offer_accepted");
        }
        other => panic!("expected ReactionTriggered record, got {other:?}"),
    }
    match &reaction_records[1] {
        ReactionJournalRecord::ReactionCompleted {
            metadata,
            triggered_action_id,
            ..
        } => {
            assert_eq!(metadata.stream_type, StreamType::Reaction);
            assert_eq!(triggered_action_id, &expected_action_id);
        }
        other => panic!("expected ReactionCompleted record, got {other:?}"),
    }
}

#[tokio::test]
async fn reference_flow_projects_document_flow_status_view() {
    let event_store = InMemoryEventStore::new();
    let action_executor = ActionExecutor::new(event_store.clone());

    action_executor
        .execute::<Offer, CreateOfferV1>(
            CreateOfferV1 {
                offer_id: "offer-1".to_string(),
                title: "Initial offer".to_string(),
            },
            action_metadata("create-offer-action-1"),
        )
        .await
        .expect("create offer should succeed");
    action_executor
        .execute::<Offer, AcceptOfferV1>(
            AcceptOfferV1 {
                offer_id: "offer-1".to_string(),
            },
            action_metadata("accept-offer-action-1"),
        )
        .await
        .expect("accept offer should succeed");

    let offer_events = event_store
        .load(&ResourceStream::new("offer", "offer-1"))
        .await
        .expect("load offer events");
    let offer_created = offer_events
        .iter()
        .find(|event| event.metadata.message_type == "offer_created")
        .expect("offer created event should exist")
        .clone();
    let offer_accepted = offer_events
        .iter()
        .find(|event| event.metadata.message_type == "offer_accepted")
        .expect("offer accepted event should exist")
        .clone();

    let reaction_dispatcher =
        ReactionDispatcher::new(ReactionRuntime::new(
            event_store.clone(),
            InMemoryReactionJournal::new(),
        ))
        .with_handler(TypedReactionHandler::new(
            OfferAcceptedCreatesSalesOrder,
            |trigger: &RecordedEvent| {
                ReactionRuntime::<InMemoryEventStore, InMemoryReactionJournal>::
                reaction_action_metadata::<OfferAcceptedCreatesSalesOrder>(trigger)
            },
        ));
    reaction_dispatcher
        .dispatch(&offer_accepted)
        .await
        .expect("reaction dispatch should succeed");
    let sales_order_events = event_store
        .load(&ResourceStream::new(
            "sales_order",
            "sales-order-for-offer-1",
        ))
        .await
        .expect("load sales order events");
    let sales_order_created = sales_order_events
        .iter()
        .find(|event| event.metadata.message_type == "sales_order_created")
        .expect("sales order created event should exist")
        .clone();

    let projection_dispatcher =
        ProjectionDispatcher::new(ProjectionRuntime::new(InMemoryViewStore::new()))
            .with_handler(TypedProjectionHandler::new(FlowStatusFromOfferCreated))
            .with_handler(TypedProjectionHandler::new(FlowStatusFromOfferAccepted))
            .with_handler(TypedProjectionHandler::new(FlowStatusFromSalesOrderCreated));

    let offer_created_report = projection_dispatcher
        .dispatch(&offer_created)
        .await
        .expect("offer created projection dispatch should succeed");
    assert_eq!(offer_created_report.applied, 1);
    let offer_created_status = projection_dispatcher
        .view_store()
        .load(&ViewKey::new("flow_status", "offer-1"))
        .await
        .expect("load offer created flow status")
        .expect("offer created flow status should exist");
    assert_eq!(offer_created_status.payload["status"], "offer_created");
    assert_eq!(offer_created_status.payload["title"], "Initial offer");

    let offer_accepted_report = projection_dispatcher
        .dispatch(&offer_accepted)
        .await
        .expect("offer accepted projection dispatch should succeed");
    assert_eq!(offer_accepted_report.applied, 1);
    let offer_accepted_status = projection_dispatcher
        .view_store()
        .load(&ViewKey::new("flow_status", "offer-1"))
        .await
        .expect("load offer accepted flow status")
        .expect("offer accepted flow status should exist");
    assert_eq!(offer_accepted_status.payload["status"], "offer_accepted");

    let sales_order_report = projection_dispatcher
        .dispatch(&sales_order_created)
        .await
        .expect("sales order projection dispatch should succeed");
    assert_eq!(sales_order_report.applied, 1);

    let flow_status = projection_dispatcher
        .view_store()
        .load(&ViewKey::new("flow_status", "offer-1"))
        .await
        .expect("load flow status")
        .expect("flow status should exist");
    assert_eq!(flow_status.payload["offer_id"], "offer-1");
    assert_eq!(flow_status.payload["status"], "sales_order_created");
    assert_eq!(
        flow_status.payload["sales_order_id"],
        "sales-order-for-offer-1"
    );

    let listed = projection_dispatcher
        .view_store()
        .list_by_index_prefix("flow_status", "all", "")
        .await
        .expect("list flow status all index");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0], flow_status);
    assert!(event_store
        .all_events()
        .iter()
        .all(|event| event.metadata.stream_type == StreamType::Resource));
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
    assert_eq!(
        vec!["offer_accepted_to_create_sales_order"],
        json_field_values(&manifest_json, "reactions", "reaction_type")
    );
    assert_eq!(
        vec!["offer_accepted"],
        json_field_values(&manifest_json, "reactions", "trigger_event_type")
    );
    assert_eq!(
        vec!["create_sales_order"],
        json_field_values(&manifest_json, "reactions", "target_action_type")
    );
    assert_eq!(
        vec!["flow_status"],
        json_field_values(&manifest_json, "views", "view_type")
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

fn action_metadata(action_id: &str) -> ActionMetadata {
    ActionMetadata::with_ids(
        action_id,
        "reference-flow-correlation",
        "reference-flow-causation",
        "reference-flow-test",
    )
}
