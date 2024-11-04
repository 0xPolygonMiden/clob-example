use miden_client::{assets::Asset, notes::NoteId, store::InputNoteRecord};

use crate::{errors::OrderError, utils::get_assets_from_swap_note};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Order {
    id: Option<NoteId>,
    source_asset: Asset,
    target_asset: Asset,
}

impl Order {
    pub fn new(id: Option<NoteId>, source_asset: Asset, target_asset: Asset) -> Self {
        Order {
            id,
            source_asset,
            target_asset,
        }
    }

    pub fn id(&self) -> Option<NoteId> {
        self.id
    }

    pub fn source_asset(&self) -> Asset {
        self.source_asset
    }

    pub fn target_asset(&self) -> Asset {
        self.target_asset
    }

    pub fn price(&self) -> f64 {
        let source_asset_amount = self.source_asset.unwrap_fungible().amount();
        let target_asset_amount = self.target_asset.unwrap_fungible().amount();

        target_asset_amount as f64 / source_asset_amount as f64
    }
}

// Conversions
/////////////////////////////////////////////////

impl From<InputNoteRecord> for Order {
    fn from(value: InputNoteRecord) -> Self {
        let (source_asset, target_asset) = get_assets_from_swap_note(&value);
        let id = value.id();
        Order {
            id: Some(id),
            source_asset,
            target_asset,
        }
    }
}

// Utils
/////////////////////////////////////////////////

pub fn match_orders(incoming_order: Order, existing_order: Order) -> Result<Order, OrderError> {
    // Orders match if:
    // - They have inversed source and target assets
    // - Exisiting order price is lower than incoming order price

    // assets do not match
    if !(existing_order.source_asset.faucet_id() == incoming_order.target_asset.faucet_id()
        && existing_order.target_asset.faucet_id() == incoming_order.source_asset.faucet_id())
    {
        return Err(OrderError::AssetsNotMatching);
    }

    // existing order price is too high
    if existing_order.price() > incoming_order.price() {
        return Err(OrderError::PriceTooHigh(
            incoming_order.price() as u64,
            existing_order.price() as u64,
        ));
    }

    Ok(existing_order)
}

pub fn sort_orders(mut orders: Vec<Order>) -> Vec<Order> {
    orders.sort_by(|a, b| {
        let a_price = a.price();
        let b_price = b.price();

        a_price
            .partial_cmp(&b_price)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    orders
}

// Tests
/////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use miden_client::{
        accounts::AccountId,
        assets::{Asset, FungibleAsset},
        notes::NoteId,
    };

    use crate::{errors::OrderError, order::match_orders};

    use super::Order;

    fn build_orders() -> (Order, Vec<Order>) {
        // create faucets
        let source_faucet_id_hex = "0x227bd163275aa1bf";
        let source_faucet_id = AccountId::from_hex(source_faucet_id_hex).unwrap();
        let target_faucet_id_hex = "0x2540b08edc3b087d";
        let target_faucet_id = AccountId::from_hex(target_faucet_id_hex).unwrap();

        // mock note id
        let note_id_hex = "0x17c0bee79464320cc0d5d835cb9c2971b5c23fcea665c66d4f73c54fc7860129";
        let note_id = NoteId::try_from_hex(note_id_hex).unwrap();

        // create assets
        let source_amount = 10;
        let target_amount = 20;
        let source_asset =
            Asset::Fungible(FungibleAsset::new(source_faucet_id, source_amount).unwrap());
        let target_asset =
            Asset::Fungible(FungibleAsset::new(target_faucet_id, target_amount).unwrap());

        // incoming order
        let order = Order::new(Some(note_id), source_asset, target_asset);

        // existing orders

        // Full swap
        let order1 = Order::new(Some(note_id), target_asset, source_asset);

        // Assets do not match
        let order2 = Order::new(Some(note_id), source_asset, source_asset);

        // Price is too high
        let new_source_amount = 30;
        let new_target_amount = 10;
        let new_source_asset =
            Asset::Fungible(FungibleAsset::new(source_faucet_id, new_source_amount).unwrap());
        let new_target_asset =
            Asset::Fungible(FungibleAsset::new(target_faucet_id, new_target_amount).unwrap());
        let order3 = Order::new(Some(note_id), new_target_asset, new_source_asset);

        // Partial swap
        let new_source_amount = 10;
        let new_target_amount = 5;
        let new_source_asset =
            Asset::Fungible(FungibleAsset::new(source_faucet_id, new_source_amount).unwrap());
        let new_target_asset =
            Asset::Fungible(FungibleAsset::new(target_faucet_id, new_target_amount).unwrap());
        let order4 = Order::new(Some(note_id), new_target_asset, new_source_asset);

        let orders = vec![order1, order2, order3, order4];

        (order, orders)
    }

    #[test]
    fn order_matching_succeeds() {
        let (incoming_order, existing_orders) = build_orders();
        let expected_results = [
            Ok(existing_orders[0]),
            Err(OrderError::AssetsNotMatching),
            Err(OrderError::PriceTooHigh(
                incoming_order.price() as u64,
                existing_orders[2].price() as u64,
            )),
            Ok(existing_orders[3]),
        ];

        for (existing_order, expected_result) in existing_orders.into_iter().zip(expected_results) {
            assert_eq!(
                match_orders(incoming_order, existing_order),
                expected_result,
                "Mismatch for order: {:?}",
                existing_order
            );
        }
    }
}
