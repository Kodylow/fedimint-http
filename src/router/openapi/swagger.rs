use utoipa::OpenApi;
use crate::error::AppError;
use crate::router::handlers::*;
use crate::router::handlers::fedimint::admin::backup::BackupRequest;
use crate::router::handlers::fedimint::admin::info::InfoResponse;
use crate::router::handlers::fedimint::admin::list_operations::ListOperationsRequest;
use crate::router::handlers::fedimint::admin::module::ModuleSelector;

use crate::router::handlers::fedimint::ln::await_invoice::AwaitInvoiceRequest;
use crate::router::handlers::fedimint::ln::invoice::{LnInvoiceRequest, LnInvoiceResponse};
use crate::router::handlers::fedimint::ln::list_gateways::ListGatewaysRequest;
use crate::router::handlers::fedimint::ln::pay::{LnPayRequest, LnPayResponse};
use crate::router::handlers::fedimint::ln::switch_gateway::SwitchGatewayRequest;

use crate::router::handlers::fedimint::mint::combine::{CombineRequest, CombineResponse};
use crate::router::handlers::fedimint::mint::reissue::{ReissueResponse, ReissueRequest};
use crate::router::handlers::fedimint::mint::spend::{SpendRequest, SpendResponse};
use crate::router::handlers::fedimint::mint::split::{SplitRequest, SplitResponse};
use crate::router::handlers::fedimint::mint::validate::{ValidateRequest, ValidateResponse};

use crate::router::handlers::fedimint::wallet::await_deposit::{AwaitDepositRequest, AwaitDepositResponse};
use crate::router::handlers::fedimint::wallet::deposit_address::{DepositAddressRequest, DepositAddressResponse};
use crate::router::handlers::fedimint::wallet::withdraw::{WithdrawRequest, WithdrawResponse};

#[derive(OpenApi)]
#[openapi(
paths(
fedimint::admin::backup::handle_rest,
fedimint::admin::config::handle_rest,
fedimint::admin::discover_version::handle_rest,
fedimint::admin::info::handle_rest,
fedimint::admin::list_operations::handle_rest,
fedimint::admin::module::handle_rest,
fedimint::admin::restore::handle_rest,

fedimint::ln::await_invoice::handle_rest,
fedimint::ln::await_pay::handle_rest,
fedimint::ln::invoice::handle_rest,
fedimint::ln::list_gateways::handle_rest,
fedimint::ln::pay::handle_rest,
fedimint::ln::switch_gateway::handle_rest,

fedimint::mint::combine::handle_rest,
fedimint::mint::reissue::handle_rest,
fedimint::mint::spend::handle_rest,
fedimint::mint::split::handle_rest,
fedimint::mint::validate::handle_rest,

fedimint::wallet::await_deposit::handle_rest,
fedimint::wallet::deposit_address::handle_rest,
fedimint::wallet::withdraw::handle_rest,
),
components(
schemas(
AppError,
AwaitDepositRequest,
AwaitDepositResponse,
AwaitInvoiceRequest,
BackupRequest,
CombineRequest,
CombineResponse,
DepositAddressRequest,
DepositAddressResponse,
InfoResponse,
ListGatewaysRequest,
ListOperationsRequest,
LnInvoiceRequest,
LnInvoiceResponse,
LnPayRequest,
LnPayResponse,
ModuleSelector,
ReissueRequest,
ReissueResponse,
SpendRequest,
SpendResponse,
SplitRequest,
SplitResponse,
SwitchGatewayRequest,
ValidateRequest,
ValidateResponse,
WithdrawRequest,
WithdrawResponse,
)
),
tags(
(name = "Fedimint-http", description = "fedimint-http exposes a REST API to interact with the Fedimint client.")
)
)]
pub struct ApiDoc;
