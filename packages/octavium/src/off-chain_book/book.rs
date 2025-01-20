//! Off-chain order book implementation for Octavium DSE
//! This module handles order matching and management outside the blockchain
//! for improved performance and reduced on-chain load.

use std::collections::{BTreeMap, VecDeque};

/// Maximum number of fills that can be processed in a single matching operation
const MAX_FILLS: usize = 100;
/// Minimum price increment for orders
const TICK_SIZE: u64 = 1;
/// Minimum quantity increment for orders
const LOT_SIZE: u64 = 1;
/// Minimum order size allowed
const MIN_SIZE: u64 = 1;

/// Represents a single order in the order book
#[derive(Debug, Clone)]
pub struct Order {
    /// Unique identifier for the order
    order_id: u128,
    /// Price per unit of base asset
    price: u64,
    /// Total quantity of base asset to trade
    quantity: u64,
    /// Amount of base asset that has been filled
    filled_quantity: u64,
    /// Address of the order owner
    owner: String,
    /// Timestamp after which the order is considered expired
    expire_timestamp: u64,
    /// True for buy orders, false for sell orders
    is_bid: bool,
}

/// Central order book maintaining separate bid and ask sides
#[derive(Debug)]
pub struct Book {
    /// Bid orders sorted by price-time priority (highest price first)
    bids: BTreeMap<u128, Order>,
    /// Ask orders sorted by price-time priority (lowest price first)
    asks: BTreeMap<u128, Order>,
    /// Counter for generating unique bid order IDs (counting down)
    next_bid_order_id: u64,
    /// Counter for generating unique ask order IDs (counting up)
    next_ask_order_id: u64,
}

/// Represents a match between two orders
#[derive(Debug)]
pub struct Fill {
    /// Order ID of the maker (passive order)
    maker_order_id: u128,
    /// Order ID of the taker (aggressive order)
    taker_order_id: u128,
    /// Amount of base asset traded
    base_quantity: u64,
    /// Amount of quote asset traded (base_quantity * price)
    quote_quantity: u64,
    /// Timestamp when the fill occurred
    timestamp: u64,
}

impl Book {
    /// Creates a new empty order book
    pub fn new() -> Self {
        Book {
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            next_bid_order_id: u64::MAX, // Start from max for bids (counting down)
            next_ask_order_id: 1,        // Start from 1 for asks (counting up)
        }
    }

    /// Attempts to match an incoming order against existing orders
    /// Returns a vector of fills created during matching
    ///
    /// # Arguments
    /// * `taker_order` - The incoming order to match
    /// * `timestamp` - Current timestamp for order expiration checks
    pub fn match_order(&mut self, mut taker_order: Order, timestamp: u64) -> Vec<Fill> {
        let mut fills = Vec::new();
        
        // Get the appropriate order book side
        let book_side = if taker_order.is_bid {
            &mut self.asks // Match bids against asks
        } else {
            &mut self.bids // Match asks against bids
        };

        // Keep matching until order is filled or no more matches possible
        while taker_order.remaining_quantity() > 0 && !book_side.is_empty() && fills.len() < MAX_FILLS {
            let best_order_id = if taker_order.is_bid {
                book_side.first_key_value() // Lowest ask for bids
            } else {
                book_side.last_key_value() // Highest bid for asks
            };

            if let Some((order_id, maker_order)) = best_order_id {
                // Check if maker order is expired
                if maker_order.expire_timestamp < timestamp {
                    book_side.remove(order_id);
                    continue;
                }

                // Check if price matches
                if !self.prices_match(&taker_order, maker_order) {
                    break;
                }

                // Calculate fill quantity
                let fill_qty = std::cmp::min(
                    taker_order.remaining_quantity(),
                    maker_order.remaining_quantity()
                );

                if fill_qty == 0 {
                    break;
                }

                // Create fill
                let fill = Fill {
                    maker_order_id: *order_id,
                    taker_order_id: taker_order.order_id,
                    base_quantity: fill_qty,
                    quote_quantity: fill_qty * maker_order.price,
                    timestamp,
                };

                // Update orders
                taker_order.filled_quantity += fill_qty;
                maker_order.filled_quantity += fill_qty;

                // Remove fully filled maker orders
                if maker_order.is_filled() {
                    book_side.remove(order_id);
                }

                fills.push(fill);
            } else {
                break;
            }
        }

        fills
    }

    /// Checks if two orders' prices match for trading
    ///
    /// # Arguments
    /// * `taker` - The incoming aggressive order
    /// * `maker` - The resting passive order
    fn prices_match(&self, taker: &Order, maker: &Order) -> bool {
        if taker.is_bid {
            taker.price >= maker.price // Bid must be greater than or equal to ask
        } else {
            taker.price <= maker.price // Ask must be less than or equal to bid
        }
    }

    /// Places a new order in the book, attempting to match it first
    ///
    /// # Arguments
    /// * `order` - The new order to place
    /// Returns a vector of fills if any matches occurred
    pub fn place_order(&mut self, mut order: Order) -> Vec<Fill> {
        // First try to match the order
        let fills = self.match_order(order.clone(), order.expire_timestamp);
        
        // If order is not fully filled and not IOC, place it in the book
        if !order.is_filled() {
            let book_side = if order.is_bid {
                &mut self.bids
            } else {
                &mut self.asks
            };
            
            book_side.insert(order.order_id, order);
        }
        
        fills
    }

    /// Cancels an existing order
    ///
    /// # Arguments
    /// * `order_id` - ID of the order to cancel
    /// * `is_bid` - Whether the order is a bid or ask
    /// Returns the cancelled order if found
    pub fn cancel_order(&mut self, order_id: u128, is_bid: bool) -> Option<Order> {
        let book_side = if is_bid {
            &mut self.bids
        } else {
            &mut self.asks
        };
        
        book_side.remove(&order_id)
    }
}

impl Order {
    /// Returns the unfilled quantity of the order
    pub fn remaining_quantity(&self) -> u64 {
        self.quantity - self.filled_quantity
    }

    /// Checks if the order has been completely filled
    pub fn is_filled(&self) -> bool {
        self.filled_quantity >= self.quantity
    }
}
