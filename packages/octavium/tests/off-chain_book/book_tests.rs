use crate::book::{Book, Order, Fill};
use std::time::{Duration, Instant};

const USDC_DECIMALS: u64 = 1_000_000;      // 6 decimals
const SUI_DECIMALS: u64 = 1_000_000_000;   // 9 decimals
const FLOAT_SCALING: u64 = 1_000_000_000;  // 9 decimals
const MAKER_FEE: u64 = 50;                 // 0.05% = 5 bps
const TAKER_FEE: u64 = 100;                // 0.10% = 10 bps

#[test]
fn test_partial_fill_bid() {
    let mut book = Book::new();
    
    // Create a bid order: Buy 10 SUI at $5/SUI
    let taker_order = Order {
        order_id: 1,
        price: 5 * USDC_DECIMALS,
        quantity: 10 * SUI_DECIMALS,
        filled_quantity: 0,
        owner: "alice".to_string(),
        expire_timestamp: u64::MAX,
        is_bid: true,
    };

    // Create an ask order: Sell 5 SUI at $5/SUI
    let maker_order = Order {
        order_id: 2,
        price: 5 * USDC_DECIMALS,
        quantity: 5 * SUI_DECIMALS,
        filled_quantity: 0,
        owner: "bob".to_string(),
        expire_timestamp: u64::MAX,
        is_bid: false,
    };

    // Place the maker order
    book.place_order(maker_order);
    
    // Match the taker order
    let fills = book.match_order(taker_order.clone(), 0);
    
    assert_eq!(fills.len(), 1);
    assert_eq!(fills[0].base_quantity, 5 * SUI_DECIMALS);
    assert_eq!(fills[0].quote_quantity, 25 * USDC_DECIMALS);
    assert_eq!(fills[0].maker_order_id, 2);
    assert_eq!(fills[0].taker_order_id, 1);
}

#[test]
fn test_full_fill_bid() {
    let mut book = Book::new();
    
    // Create a bid order: Buy 10 SUI at $5/SUI
    let taker_order = Order {
        order_id: 1,
        price: 5 * USDC_DECIMALS,
        quantity: 10 * SUI_DECIMALS,
        filled_quantity: 0,
        owner: "alice".to_string(),
        expire_timestamp: u64::MAX,
        is_bid: true,
    };

    // Create an ask order: Sell 50 SUI at $5/SUI
    let maker_order = Order {
        order_id: 2,
        price: 5 * USDC_DECIMALS,
        quantity: 50 * SUI_DECIMALS,
        filled_quantity: 0,
        owner: "bob".to_string(),
        expire_timestamp: u64::MAX,
        is_bid: false,
    };

    book.place_order(maker_order);
    let fills = book.match_order(taker_order.clone(), 0);
    
    assert_eq!(fills.len(), 1);
    assert_eq!(fills[0].base_quantity, 10 * SUI_DECIMALS);
    assert_eq!(fills[0].quote_quantity, 50 * USDC_DECIMALS);
}

#[test]
fn test_precision_matching() {
    let mut book = Book::new();
    
    // Create a bid order: Buy 10.86 SUI at $1.234/SUI
    let taker_order = Order {
        order_id: 1,
        price: 1_234_000,  // $1.234
        quantity: 10_860_000_000, // 10.86 SUI
        filled_quantity: 0,
        owner: "alice".to_string(),
        expire_timestamp: u64::MAX,
        is_bid: true,
    };

    // Create an ask order: Sell 10.86 SUI at $1.234/SUI
    let maker_order = Order {
        order_id: 2,
        price: 1_234_000,
        quantity: 10_860_000_000,
        filled_quantity: 0,
        owner: "bob".to_string(),
        expire_timestamp: u64::MAX,
        is_bid: false,
    };

    book.place_order(maker_order);
    let fills = book.match_order(taker_order.clone(), 0);
    
    assert_eq!(fills.len(), 1);
    assert_eq!(fills[0].base_quantity, 10_860_000_000);
    assert_eq!(fills[0].quote_quantity, 13_401_240); // 10.86 * $1.234
}

#[test]
fn test_multiple_fills() {
    let mut book = Book::new();
    
    // Taker: ask order with quantity 10 at price $1
    let taker_order = Order {
        order_id: 1,
        price: USDC_DECIMALS, // $1
        quantity: 10 * SUI_DECIMALS,
        filled_quantity: 0,
        owner: "alice".to_string(),
        expire_timestamp: u64::MAX,
        is_bid: false,
    };

    // Maker1: bid order with quantity 1.001001 at price $1.001
    let maker_order1 = Order {
        order_id: 2,
        price: 1_001_000,
        quantity: 1_001_001_000,
        filled_quantity: 0,
        owner: "bob".to_string(),
        expire_timestamp: u64::MAX,
        is_bid: true,
    };

    // Maker2: bid order with quantity 1 at price $1
    let maker_order2 = Order {
        order_id: 3,
        price: USDC_DECIMALS,
        quantity: SUI_DECIMALS,
        filled_quantity: 0,
        owner: "charlie".to_string(),
        expire_timestamp: u64::MAX,
        is_bid: true,
    };

    book.place_order(maker_order1);
    book.place_order(maker_order2);
    let fills = book.match_order(taker_order.clone(), 0);
    
    assert_eq!(fills.len(), 2);
    // First fill should be at better price ($1.001)
    assert_eq!(fills[0].base_quantity, 1_001_001_000);
    assert_eq!(fills[0].quote_quantity, 1_002_002_001);
    // Second fill at $1
    assert_eq!(fills[1].base_quantity, SUI_DECIMALS);
    assert_eq!(fills[1].quote_quantity, USDC_DECIMALS);
}

#[test]
#[should_panic]
fn test_invalid_price() {
    let mut book = Book::new();
    
    let order = Order {
        order_id: 1,
        price: 0, // Invalid price
        quantity: SUI_DECIMALS,
        filled_quantity: 0,
        owner: "alice".to_string(),
        expire_timestamp: u64::MAX,
        is_bid: true,
    };

    book.place_order(order);
}

#[test]
#[should_panic]
fn test_invalid_quantity() {
    let mut book = Book::new();
    
    let order = Order {
        order_id: 1,
        price: USDC_DECIMALS,
        quantity: 0, // Invalid quantity
        filled_quantity: 0,
        owner: "alice".to_string(),
        expire_timestamp: u64::MAX,
        is_bid: true,
    };

    book.place_order(order);
}

/// Measures throughput of order processing
#[test]
fn test_order_throughput() {
    let mut book = Book::new();
    let num_orders = 100_000; // Number of orders to process
    let mut total_fills = 0;
    
    // Create a mix of bid and ask orders
    let orders: Vec<Order> = (0..num_orders)
        .map(|i| Order {
            order_id: i as u128,
            price: (1_000_000 + (i % 10) * 1000) as u64, // Vary price around $1
            quantity: 1_000_000_000, // 1 SUI
            filled_quantity: 0,
            owner: format!("trader_{}", i),
            expire_timestamp: u64::MAX,
            is_bid: i % 2 == 0, // Alternate between bids and asks
        })
        .collect();
    
    let start_time = Instant::now();
    
    // Process all orders
    for order in orders {
        let fills = book.place_order(order);
        total_fills += fills.len();
    }
    
    let elapsed = start_time.elapsed();
    let orders_per_second = num_orders as f64 / elapsed.as_secs_f64();
    let fills_per_second = total_fills as f64 / elapsed.as_secs_f64();
    
    println!("Throughput Test Results:");
    println!("Total orders processed: {}", num_orders);
    println!("Total fills generated: {}", total_fills);
    println!("Time elapsed: {:.2?}", elapsed);
    println!("Orders per second: {:.2}", orders_per_second);
    println!("Fills per second: {:.2}", fills_per_second);
    
    // Basic assertions to ensure the test is meaningful
    assert!(orders_per_second > 0.0);
    assert!(total_fills > 0);
}

/// Measures throughput with varying order book depths
#[test]
fn test_throughput_with_depth() {
    let depths = vec![10, 100, 1000, 10000];
    
    for depth in depths {
        let mut book = Book::new();
        let num_orders = depth * 2; // Process 2x the depth in orders
        let mut total_fills = 0;
        
        // Pre-fill order book to desired depth
        for i in 0..depth {
            let base_price = 1_000_000; // $1 base price
            
            // Add asks above base price
            let ask = Order {
                order_id: i as u128,
                price: base_price + (i * 100) as u64,
                quantity: 1_000_000_000,
                filled_quantity: 0,
                owner: format!("seller_{}", i),
                expire_timestamp: u64::MAX,
                is_bid: false,
            };
            book.place_order(ask);
            
            // Add bids below base price
            let bid = Order {
                order_id: (i + depth) as u128,
                price: base_price - (i * 100) as u64,
                quantity: 1_000_000_000,
                filled_quantity: 0,
                owner: format!("buyer_{}", i),
                expire_timestamp: u64::MAX,
                is_bid: true,
            };
            book.place_order(bid);
        }
        
        // Create test orders that will match against the book
        let orders: Vec<Order> = (0..num_orders)
            .map(|i| Order {
                order_id: (i + 2 * depth) as u128,
                price: 1_000_000 + (if i % 2 == 0 { 1000 } else { -1000 }),
                quantity: 1_000_000_000,
                filled_quantity: 0,
                owner: format!("trader_{}", i),
                expire_timestamp: u64::MAX,
                is_bid: i % 2 == 0,
            })
            .collect();
        
        let start_time = Instant::now();
        
        // Process all orders
        for order in orders {
            let fills = book.place_order(order);
            total_fills += fills.len();
        }
        
        let elapsed = start_time.elapsed();
        let orders_per_second = num_orders as f64 / elapsed.as_secs_f64();
        let fills_per_second = total_fills as f64 / elapsed.as_secs_f64();
        
        println!("\nThroughput Test Results for depth {}:", depth);
        println!("Total orders processed: {}", num_orders);
        println!("Total fills generated: {}", total_fills);
        println!("Time elapsed: {:.2?}", elapsed);
        println!("Orders per second: {:.2}", orders_per_second);
        println!("Fills per second: {:.2}", fills_per_second);
        
        // Ensure test is meaningful
        assert!(orders_per_second > 0.0);
        assert!(total_fills > 0);
    }
}

/// Measures latency distribution of order processing
#[test]
fn test_order_latency_distribution() {
    let mut book = Book::new();
    let num_orders = 10_000;
    let mut latencies = Vec::with_capacity(num_orders);
    
    // Create and process orders while measuring individual latencies
    for i in 0..num_orders {
        let order = Order {
            order_id: i as u128,
            price: 1_000_000 + (i % 10) * 1000,
            quantity: 1_000_000_000,
            filled_quantity: 0,
            owner: format!("trader_{}", i),
            expire_timestamp: u64::MAX,
            is_bid: i % 2 == 0,
        };
        
        let start_time = Instant::now();
        book.place_order(order);
        latencies.push(start_time.elapsed());
    }
    
    // Calculate latency statistics
    latencies.sort();
    let total_time: Duration = latencies.iter().sum();
    let avg_latency = total_time / num_orders as u32;
    let p50 = latencies[num_orders / 2];
    let p95 = latencies[(num_orders * 95) / 100];
    let p99 = latencies[(num_orders * 99) / 100];
    
    println!("\nLatency Distribution:");
    println!("Average latency: {:?}", avg_latency);
    println!("Median (P50) latency: {:?}", p50);
    println!("P95 latency: {:?}", p95);
    println!("P99 latency: {:?}", p99);
    
    // Basic assertions
    assert!(p99 >= p95);
    assert!(p95 >= p50);
}
