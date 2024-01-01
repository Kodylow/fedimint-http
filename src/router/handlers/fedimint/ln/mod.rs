use std::str::FromStr;

use anyhow::{anyhow, bail, Context};
use fedimint_client::ClientArc;
use fedimint_core::{core::OperationId, Amount};
use fedimint_ln_client::{InternalPayState, LightningClientModule, LnPayState, PayType};
use futures::StreamExt;
use futures_util::Stream;
use lightning_invoice::Bolt11Invoice;
use tracing::{debug, info};

use self::pay::{LnPayRequest, LnPayResponse};

pub mod await_invoice;
pub mod await_pay;
pub mod invoice;
pub mod list_gateways;
pub mod pay;
pub mod switch_gateway;

pub async fn parse_invoice(
    info: &str,
    amount_msat: Option<Amount>,
) -> anyhow::Result<Bolt11Invoice> {
    match Bolt11Invoice::from_str(info) {
        Ok(invoice) => {
            debug!("Parsed parameter as bolt11 invoice: {invoice}");
            match (invoice.amount_milli_satoshis(), amount_msat) {
                (Some(_), Some(_)) => {
                    bail!("Amount specified in both invoice and command line")
                }
                (None, _) => {
                    bail!("We don't support invoices without an amount")
                }
                _ => {}
            };
            Ok(invoice)
        }
        Err(e) => Err(anyhow!(e)),
    }
}

pub async fn parse_lnurl(info: &str) -> anyhow::Result<lnurl::lnurl::LnUrl> {
    if info.to_lowercase().starts_with("lnurl") {
        lnurl::lnurl::LnUrl::from_str(info).context("Invalid lnurl")
    } else if info.contains('@') {
        Ok(lnurl::lightning_address::LightningAddress::from_str(info)?.lnurl())
    } else {
        bail!("Invalid invoice or lnurl");
    }
}

pub async fn get_invoice_from_lnurl(
    lnurl: &lnurl::lnurl::LnUrl,
    amount: Option<Amount>,
    comment: Option<String>,
) -> anyhow::Result<Bolt11Invoice> {
    let async_client = lnurl::AsyncClient::from_client(reqwest::Client::new());
    let response = async_client.make_request(&lnurl.url).await?;
    match response {
        lnurl::LnUrlResponse::LnUrlPayResponse(response) => {
            let invoice = async_client
                .get_invoice(&response, amount.unwrap().msats, None, comment.as_deref())
                .await?;
            let invoice = Bolt11Invoice::from_str(invoice.invoice())?;
            assert_eq!(invoice.amount_milli_satoshis(), Some(amount.unwrap().msats));
            Ok(invoice)
        }
        other => {
            bail!("Unexpected response from lnurl: {other:?}");
        }
    }
}

pub async fn get_invoice(req: &LnPayRequest) -> anyhow::Result<Bolt11Invoice> {
    let info = req.payment_info.trim();
    match parse_invoice(info, req.amount_msat).await {
        Ok(invoice) => Ok(invoice),
        Err(_) => {
            let lnurl = parse_lnurl(info).await?;
            debug!("Parsed parameter as lnurl: {lnurl:?}");
            let amount = req
                .amount_msat
                .context("When using a lnurl, an amount must be specified")?;
            get_invoice_from_lnurl(&lnurl, Some(amount), req.lnurl_comment.clone()).await
        }
    }
}

pub async fn ln_payment_updates_internal(
    lightning_module: &LightningClientModule,
    operation_id: OperationId,
) -> anyhow::Result<impl Stream<Item = InternalPayState>> {
    let updates = lightning_module
        .subscribe_internal_pay(operation_id)
        .await?
        .into_stream();
    Ok(updates)
}

pub async fn handle_internal_payment(
    lightning_module: &LightningClientModule,
    operation_id: OperationId,
    payment_type: PayType,
    contract_id: String,
    return_on_funding: bool,
) -> anyhow::Result<Option<LnPayResponse>> {
    let mut updates = ln_payment_updates_internal(lightning_module, operation_id).await?;

    while let Some(update) = updates.next().await {
        match update {
            InternalPayState::Preimage(_preimage) => {
                return Ok(Some(LnPayResponse {
                    operation_id,
                    payment_type,
                    contract_id,
                    fee: Amount::ZERO,
                }));
            }
            InternalPayState::RefundSuccess { out_points, error } => {
                let e = format!(
                    "Internal payment failed. A refund was issued to {:?} Error: {error}",
                    out_points
                );
                bail!("{e}");
            }
            InternalPayState::UnexpectedError(e) => {
                bail!("{e}");
            }
            InternalPayState::Funding if return_on_funding => return Ok(None),
            InternalPayState::Funding => {}
            InternalPayState::RefundError {
                error_message,
                error,
            } => bail!("RefundError: {error_message} {error}"),
            InternalPayState::FundingFailed { error } => {
                bail!("FundingFailed: {error}")
            }
        }
        info!("Update: {update:?}");
    }
    bail!("Internal Payment failed")
}

pub async fn ln_payment_updates_lightning(
    lightning_module: &LightningClientModule,
    operation_id: OperationId,
) -> anyhow::Result<impl Stream<Item = LnPayState>> {
    let updates = lightning_module
        .subscribe_ln_pay(operation_id)
        .await?
        .into_stream();
    Ok(updates)
}

pub async fn handle_lightning_payment(
    lightning_module: &LightningClientModule,
    operation_id: OperationId,
    payment_type: PayType,
    contract_id: String,
    return_on_funding: bool,
) -> anyhow::Result<Option<LnPayResponse>> {
    let mut updates = ln_payment_updates_lightning(lightning_module, operation_id).await?;

    while let Some(update) = updates.next().await {
        let update_clone = update.clone();
        match update_clone {
            LnPayState::Success { preimage: _ } => {
                return Ok(Some(LnPayResponse {
                    operation_id,
                    payment_type,
                    contract_id,
                    fee: Amount::ZERO,
                }));
            }
            LnPayState::Refunded { gateway_error } => {
                info!("{gateway_error}");
                Err(anyhow::anyhow!("Payment was refunded"))?;
            }
            LnPayState::Canceled => {
                Err(anyhow::anyhow!("Payment was canceled"))?;
            }
            LnPayState::Created
            | LnPayState::AwaitingChange
            | LnPayState::WaitingForRefund { .. } => {}
            LnPayState::Funded if return_on_funding => return Ok(None),
            LnPayState::Funded => {}
            LnPayState::UnexpectedError { error_message } => {
                bail!("UnexpectedError: {error_message}")
            }
        }
        info!("Update: {update:?}");
    }
    bail!("Lightning Payment failed")
}

pub async fn wait_for_ln_payment(
    client: &ClientArc,
    payment_type: PayType,
    contract_id: String,
    return_on_funding: bool,
) -> anyhow::Result<Option<LnPayResponse>> {
    let lightning_module = client.get_first_module::<LightningClientModule>();
    lightning_module.select_active_gateway().await?;

    match payment_type {
        PayType::Internal(operation_id) => {
            handle_internal_payment(
                &lightning_module,
                operation_id,
                payment_type,
                contract_id,
                return_on_funding,
            )
            .await
        }
        PayType::Lightning(operation_id) => {
            handle_lightning_payment(
                &lightning_module,
                operation_id,
                payment_type,
                contract_id,
                return_on_funding,
            )
            .await
        }
    }
}
