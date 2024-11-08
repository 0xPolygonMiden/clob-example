use crate::{
    order::{sort_orders, Order},
    utils::{get_notes_by_tag, print_order_table},
};
use clap::Parser;
use miden_client::{crypto::FeltRng, Client};

#[derive(Debug, Clone, Parser)]
#[clap(about = "List avaible order book orders")]
pub struct ListCmd {
    // tags
    pub tags: Vec<u32>,
}

impl ListCmd {
    pub async fn execute(&self, client: &Client<impl FeltRng>) -> Result<(), String> {
        for tag in self.tags.clone() {
            let notes = get_notes_by_tag(client, tag.into()).await;
            let orders: Vec<Order> = notes.into_iter().map(Order::from).collect();

            let sorted_orders = sort_orders(orders);
            let title = format!("Relevant orders for tag {}:", tag);
            print_order_table(title.as_str(), &sorted_orders);
        }

        Ok(())
    }
}
