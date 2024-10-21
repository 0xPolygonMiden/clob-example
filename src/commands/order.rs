use std::io::{self, Write};

use miden_client::{
    accounts::AccountId,
    assets::{Asset, FungibleAsset},
    auth::TransactionAuthenticator,
    crypto::FeltRng,
    notes::{NoteId, NoteType},
    rpc::NodeRpcClient,
    store::Store,
    transactions::{
        build_swap_tag,
        request::{SwapTransactionData, TransactionRequest},
    },
    Client,
};

use clap::Parser;

use crate::{
    errors::OrderError,
    order::{match_orders, sort_orders, Order},
    utils::{get_notes_by_tag, print_balance_update, print_order_table},
};

#[derive(Debug, Clone, Parser)]
#[command(about = "Execute an order")]
pub struct OrderCmd {
    /// Account executing the order
    pub user: String,

    /// Target faucet id
    pub target_faucet: String,

    /// Target asset amount
    pub target_amount: u64,

    /// Source faucet id
    pub source_faucet: String,

    /// Source asset amount
    pub source_amount: u64,
}

impl OrderCmd {
    pub async fn execute<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator>(
        &self,
        client: &mut Client<N, R, S, A>,
    ) -> Result<(), String> {
        // Parse id's
        let account_id = AccountId::from_hex(self.user.as_str()).unwrap();
        let source_faucet_id = AccountId::from_hex(self.source_faucet.as_str()).unwrap();
        let target_faucet_id = AccountId::from_hex(self.target_faucet.as_str()).unwrap();

        // Check if user has balance
        let (account, _) = client.get_account(account_id).unwrap();
        if account.vault().get_balance(source_faucet_id).unwrap() < self.source_amount {
            panic!("User does not have enough assets to execute this order.");
        }

        // Build order
        let source_asset =
            Asset::Fungible(FungibleAsset::new(source_faucet_id, self.source_amount).unwrap());
        let target_asset =
            Asset::Fungible(FungibleAsset::new(target_faucet_id, self.target_amount).unwrap());
        let incoming_order = Order::new(None, source_asset, target_asset);

        // Get relevant notes
        let tag = build_swap_tag(NoteType::Public, target_faucet_id, source_faucet_id).unwrap();
        let notes = get_notes_by_tag(&client, tag);
        let existing_orders: Vec<Order> = notes.into_iter().map(Order::from).collect();

        assert!(
            !existing_orders.is_empty(),
            "There are no relevant orders available."
        );

        // fill order
        match Self::fill_order(incoming_order, existing_orders) {
            Ok(orders) => Self::fill_success(orders, account_id, client)
                .await
                .map_err(|e| format!("Failed in fill success: {}", e))?,
            Err(err) => match err {
                OrderError::FailedFill(order) => Self::fill_failure(order, account_id, client)
                    .await
                    .map_err(|e| format!("Failed in fill failure: {}", e))?,
                _ => panic!("Unknown error."),
            },
        }

        Ok(())
    }

    pub fn fill_order(
        incoming_order: Order,
        existing_orders: Vec<Order>,
    ) -> Result<Vec<Order>, OrderError> {
        // Sort existing orders
        let sorted_orders = sort_orders(existing_orders);

        // Keep only orders that match incoming order
        let mut matching_orders = Vec::new();
        for order in sorted_orders {
            match match_orders(incoming_order, order) {
                Ok(order) => matching_orders.push(order),
                Err(_) => continue,
            }
        }

        // The goal is to find the best combination of orders that could fill the incoming order
        // - Maximize the amount of target asset that the incoming order can get
        // - Make sure that all swaps can be successfully filled
        let mut remaining_source = incoming_order.source_asset().unwrap_fungible().amount();
        let target = incoming_order.target_asset().unwrap_fungible().amount();

        let mut final_orders = Vec::new();
        for order in matching_orders {
            let order_amount = order.target_asset().unwrap_fungible().amount();

            if remaining_source == 0 {
                break;
            }

            if order_amount <= remaining_source {
                remaining_source = remaining_source.saturating_sub(order_amount);
                final_orders.push(order);
            }
        }

        let final_target_amount: u64 = final_orders
            .iter()
            .map(|order| order.source_asset().unwrap_fungible().amount())
            .sum();

        // We have not hit the required target amount
        if final_target_amount < target {
            return Err(OrderError::FailedFill(incoming_order));
        }

        Ok(final_orders)
    }

    async fn fill_success<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator>(
        orders: Vec<Order>,
        account_id: AccountId,
        client: &mut Client<N, R, S, A>,
    ) -> Result<(), OrderError> {
        // print final orders
        print_order_table("Final orders:", &orders);

        // print user balance update
        print_balance_update(&orders);

        // Prompt user for confirmation
        print!("Do you want to proceed with the execution? [Y/n]: ");
        io::stdout()
            .flush()
            .map_err(|e| OrderError::InternalError(format!("Failed to flush stdout: {}", e)))?;

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .map_err(|e| OrderError::InternalError(format!("Failed to read user input: {}", e)))?;

        let proceed = input.trim().to_lowercase();
        if proceed != "y" && proceed != "yes" && !proceed.is_empty() {
            println!("Execution cancelled by user.");
            return Ok(());
        }

        // Proceed with execution
        let final_order_ids = orders
            .into_iter()
            .map(|order| order.id().ok_or(OrderError::MissingId))
            .collect::<Result<Vec<NoteId>, OrderError>>()?;

        // Create transaction
        let transaction_request = TransactionRequest::consume_notes(final_order_ids);
        let transaction = client
            .new_transaction(account_id, transaction_request)
            .map_err(|e| {
                OrderError::InternalError(format!("Failed to create transaction: {}", e))
            })?;

        client.submit_transaction(transaction).await.map_err(|e| {
            OrderError::InternalError(format!("Failed to submit transaction: {}", e))
        })?;

        println!("Order filled successfully.");
        Ok(())
    }

    async fn fill_failure<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator>(
        order: Order,
        account_id: AccountId,
        client: &mut Client<N, R, S, A>,
    ) -> Result<(), OrderError> {
        println!("Unable to fill the requested order.");

        // Prompt user for confirmation
        print!("Do you want to add order to the order book? [Y/n]: ");
        io::stdout()
            .flush()
            .map_err(|e| OrderError::InternalError(format!("Failed to flush stdout: {}", e)))?;

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .map_err(|e| OrderError::InternalError(format!("Failed to read user input: {}", e)))?;

        let proceed = input.trim().to_lowercase();
        if proceed != "y" && proceed != "yes" && !proceed.is_empty() {
            println!("Execution cancelled by user.");
            return Ok(());
        }

        let swap_data =
            SwapTransactionData::new(account_id, order.source_asset(), order.target_asset());
        let transaction_request =
            TransactionRequest::swap(swap_data, NoteType::Public, client.rng()).unwrap();

        let transaction = client
            .new_transaction(account_id, transaction_request)
            .map_err(|e| {
                OrderError::InternalError(format!("Failed to create transaction: {}", e))
            })?;

        client.submit_transaction(transaction).await.map_err(|e| {
            OrderError::InternalError(format!("Failed to submit transaction: {}", e))
        })?;

        println!("Failed to fill order: {:?}", order);

        Ok(())
    }
}
