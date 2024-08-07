# Sumtree Orderbook

## Tick Calculations

Orders are placed on ticks, each tick correspends to a unique price point and can be converted from a price using the `price_to_tick` method. Conversions from tick to a price can also be achieved via the `tick_to_price` method. The price represents the amount of quote denomination per base denomination with positive ticks representing prices greater than 1, negative ticks representing fractional prices less than 1 (but greater than 0) and at tick 0 the price is 1 to 1. Currently ticks are bounded in both directions, the max tick is currently set to `182402823` and the minimum tick is currently set to `-108000000`, these bounds are to prevent loss of precision and overflow errors where possible.

### Accounting for Decimal Places

When placing an order on an orderbook that has denoms with differing decimal places this must be adjusted for using the following calculation:

$adjustedPrice = {price * 10^{quoteDecimals - baseDecimals}}$

For example when placing an order on a WBTC/USDC orderbook there is a difference of 2 decimal places so the calculation would be at a price of $64,000 would be:

$adjustPrice = {64000 * 10^{6 - 8}} = 64000 / 100 = 640$
$adjustPrice = {64000 * 10^{6 - 8}} = 64000 / 100 = 640$

So the order would be placed on the tick that represents the price of 640.

## Placing a Limit Order

To place a limit order the following message type is used:

```rust
pub enum ExecuteMsg {
    PlaceLimit {
        tick_id: i64,
        order_direction: OrderDirection,
        quantity: Uint128,
        claim_bounty: Option<Decimal256>,
    }
}
```

The fields for placing a limit are as follows:

| Field             | Description                                                                                                                                                                                                           |
| ----------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `tick_id`         | The price for which to place the order, this can be calculated following the [tick calculations](https://github.com/osmosis-labs/orderbook/edit/main/contracts/sumtree-orderbook/README.md#tick-calculations) section |
| `order_direction` | The direction for which to place the order, should be either an ask order or a bid order                                                                                                                              |
| `quantity`        | The amount (in terms of minimal denom) for the order to be placed                                                                                                                                                     |
| `claim_bounty`    | An optional percentage bounty to claim the order, capped at 1%                                                                                                                                                        |

The amount being placed for the order must be provided with the message. An example of what this would look like if the message was being constructed in JSON would be as follows:

```json
{
  "place_limit": {
    "tick_id": 1000000,
    "order_direction": "ask",
    "quantity": "1000000",
    "claim_bounty": "0.0001"
  }
}
```

In the above example we are placing an "ask" order at tick `1000000` which correspends to a price of 2, in this case we would expect the user to send `1000000` of the base denomination and receive `2000000` of the quote denomination once the order is fully filled.

## Claiming a Limit Order

As market orders are run against the orderbook orders on the crossed ticks will be filled accordingly. This can result in an order being fully or partially filled, in either case the order can be claimed. It's important to note that the order can be claimed by anyone and if the claiming address is not the address that placed the order a claim bounty may be provided depending on the placed order, the amount of which is a percentage of the amount being claimed.

To claim a limit order the following message type is used:

```rust
pub enum ExecuteMsg {
    ClaimLimit {
        tick_id: i64,
        order_id: u64
    }
}
```

The fields are as follows:

| Field      | Description                           |
| ---------- | ------------------------------------- |
| `tick_id`  | The tick that the order was placed on |
| `order_id` | The ID of the order to claim          |

An example of what this would look like if the message was being constructed in JSON would be as follows:

```json
{
  "claim_limit": {
    "tick_id": 1000000,
    "order_id": 0
  }
}
```

If an order is fully filled it will be removed from the orderbook after being claimed.

### Batch Claiming

Orders can also be batch claimed, in this case an array of tick ID and order ID tuples is provided. Importantly, **claim errors will fail silently** and the amount of orders that can be claimed is currently capped at 100. To batch claim limit orders the following message type is used:

```rust
pub enum ExecuteMsg {
    BatchClaim {
        orders: Vec<(i64, u64)>
    }
}
```

For this message the tuple is ordered as `(tick ID, order ID)`. An example of what this would look like if the message was being constructed in JSON would be as follows:

```json
{
  "batch_claim": {
    "orders": [
      [1000000, 0],
      [1000000, 1],
      [1500, 2]
    ]
  }
}
```

This will process the claims in order and will fail silently for any erroneous claims.

## Cancelling a Limit Order

To cancel a limit order the following message type is used:

```rust
pub enum ExecuteMsg {
    CancelLimit {
        tick_id: i64,
        order_id: u64
    }
}
```

This message can only be called by the address that placed the order. The `order_id` is provided as a response attribute when placing a limit or can be queried using either the `OrdersByTick` or `OrdersByOwner` queries. The fields are as follows:

| Field      | Description                           |
| ---------- | ------------------------------------- |
| `tick_id`  | The tick that the order was placed on |
| `order_id` | The ID of the order to cancel         |

An order **cannot be cancelled if it is partially filled**, in order to cancel a partially filled order the partially filled amount must be claimed first. An example of what this would look like if the message was being constructed in JSON would be as follows:

```json
{
  "cancel_limit": {
    "tick_id": 1000000,
    "order_id": 0
  }
}
```

The address will be refunded the remaining quantity in the order and the order will be removed from the orderbook.

## Querying Orders

The following queries allow access to the current orders that have been placed on the orderbook. All queries are paginated and ordered by tick ID > order ID and have a default page size of 100 with no upper bound on page size (although one will be enforced by gas limitations). Starting and ending IDs are inclusive.

### Orders by Owner

The following query can be used to get orders placed by a specific address:

```rust
// QUERY
pub enum QueryMsg {
    OrdersByOwner {
        owner: Addr,
        start_from: Option<(i64, u64)>,
        end_at: Option<(i64, u64)>,
        limit: Option<u64>,
    }
}

// RESPONSE
pub struct OrdersResponse {
    pub orders: Vec<LimitOrder>,
    pub count: u64,
}
```

An example of what this would look like if the message was being constructed in JSON would be as follows:

```json
{
  "orders_by_owner": {
    "owner": "osmo1....",
    "start_from": [1000000, 0],
    "end_at": null,
    "limit": 4
  }
}
```

### Orders by Tick

The following query can be used to get orders placed on a specific tick:

```rust
// QUERY
pub enum QueryMsg {
    OrdersByTick {
        tick_id: i64,
        start_from: Option<u64>,
        end_at: Option<u64>,
        limit: Option<u64>,
    },
}

// RESPONSE
pub struct OrdersResponse {
    pub orders: Vec<LimitOrder>,
    pub count: u64,
}
```

An example of what this would look like if the message was being constructed in JSON would be as follows:

```json
{
  "orders_by_tick": {
    "tick_id": 1000000,
    "start_from": 0,
    "end_at": null,
    "limit": 1000
  }
}
```
