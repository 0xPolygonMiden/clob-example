use std::io::{self, Write};

use miden_client::{
    accounts::AccountId,
    assets::{Asset, FungibleAsset},
    crypto::FeltRng,
    notes::{build_swap_tag, NoteId, NoteType},
    transactions::{NoteArgs, SwapTransactionData, TransactionRequest},
    Client, Felt, ZERO,
};

use clap::Parser;

use crate::{
    commands::sync::SyncCmd,
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
    pub async fn execute(&self, client: &mut Client<impl FeltRng>) -> Result<(), String> {
        // Parse id's
        let account_id = AccountId::from_hex(self.user.as_str()).unwrap();
        let source_faucet_id = AccountId::from_hex(self.source_faucet.as_str()).unwrap();
        let target_faucet_id = AccountId::from_hex(self.target_faucet.as_str()).unwrap();

        // Check if user has balance
        let (account, _) = client.get_account(account_id).await.unwrap();
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
        let tag = build_swap_tag(NoteType::Public, &target_asset, &source_asset).unwrap();
        let notes = get_notes_by_tag(client, tag).await;
        let existing_orders: Vec<Order> = notes.into_iter().map(Order::from).collect();

        // fill order
        match Self::fill_order(incoming_order, existing_orders) {
            Ok((orders, args)) => Self::fill_success(orders, args, account_id, client)
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
    ) -> Result<(Vec<Order>, Vec<NoteArgs>), OrderError> {
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

        let mut remaining_source = incoming_order.source_asset().unwrap_fungible().amount();

        let mut final_orders = Vec::new();
        let mut args = Vec::new();
        for order in matching_orders {
            let order_amount = order.target_asset().unwrap_fungible().amount();

            if remaining_source == 0 {
                break;
            }

            if order_amount <= remaining_source {
                remaining_source = remaining_source.saturating_sub(order_amount);
                args.push([Felt::new(order_amount), ZERO, ZERO, ZERO]);
                final_orders.push(order)
            } else {
                args.push([Felt::new(remaining_source), ZERO, ZERO, ZERO]);
                final_orders.push(order);
                break;
            }
        }

        if final_orders.len() == 0 {
            return Err(OrderError::FailedFill(incoming_order));
        }

        Ok((final_orders, args))
    }

    async fn fill_success(
        orders: Vec<Order>,
        args: Vec<NoteArgs>,
        account_id: AccountId,
        client: &mut Client<impl FeltRng>,
    ) -> Result<(), OrderError> {
        // sync
        let sync = SyncCmd {};
        sync.execute(client).await.unwrap();

        // print final orders
        print_order_table("Final orders:", &orders);

        // print user balance update
        print_balance_update(&orders, &args);

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
        let final_order_ids_and_args = orders
            .into_iter()
            .enumerate()
            .map(|(i, order)| {
                order
                    .id()
                    .ok_or(OrderError::MissingId)
                    .map(|id| (id, Some(args[i])))
            })
            .collect::<Result<Vec<(NoteId, Option<NoteArgs>)>, OrderError>>()?;
        // Create transaction
        let transaction_request =
            TransactionRequest::new().with_authenticated_input_notes(final_order_ids_and_args);
        let transaction = client
            .new_transaction(account_id, transaction_request)
            .await
            .map_err(|e| {
                OrderError::InternalError(format!("Failed to create transaction: {}", e))
            })?;

        client.submit_transaction(transaction).await.map_err(|e| {
            OrderError::InternalError(format!("Failed to submit transaction: {}", e))
        })?;

        println!("Order filled successfully.");
        Ok(())
    }

    async fn fill_failure(
        order: Order,
        account_id: AccountId,
        client: &mut Client<impl FeltRng>,
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
            .await
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
