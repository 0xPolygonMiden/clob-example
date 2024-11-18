use clap::Parser;
use miden_client::{crypto::FeltRng, Client};
use miden_order_book::{
    order::{sort_orders, Order},
    utils::{get_notes_by_tag, print_order_table},
};

#[derive(Debug, Clone, Parser)]
#[clap(about = "List avaible order book orders")]
pub struct ListCmd {
    // tags
    pub tags: Vec<u32>,
}

impl ListCmd {
    pub async fn execute(&self, client: &Client<impl FeltRng>) -> Result<(), String> {
        for (i, tag) in self.tags.clone().into_iter().enumerate() {
            let notes = get_notes_by_tag(client, tag.into()).await;
            // println!("{:?}", notes[0].details().script().hash());
            let orders: Vec<Order> = notes.into_iter().map(Order::from).collect();

            let sorted_orders = sort_orders(orders);
            let title = format!("Relevant orders for tag {}:", tag);
            if i == 0 {
                print_order_table(title.as_str(), &sorted_orders, true);
            } else {
                print_order_table(title.as_str(), &sorted_orders, false);
            }
        }

        Ok(())
    }
}
