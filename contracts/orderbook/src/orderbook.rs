use crate::error::ContractError;
use crate::state::{new_orderbook_id, MAX_TICK, MIN_TICK, ORDERBOOKS};
use crate::types::Orderbook;
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};

pub fn create_orderbook(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    quote_denom: String,
    base_denom: String,
) -> Result<Response, ContractError> {
    // TODO: add necessary validation logic
    // https://github.com/osmosis-labs/orderbook/issues/26

    let book_id = new_orderbook_id(deps.storage)?;
    let _book = Orderbook {
        book_id,
        quote_denom,
        base_denom,
        current_tick: 0,
        next_bid_tick: MIN_TICK,
        next_ask_tick: MAX_TICK,
    };

    ORDERBOOKS.save(deps.storage, &book_id, &_book)?;

    Ok(Response::new()
        .add_attribute("method", "createOrderbook")
        .add_attribute("book_id", book_id.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

    #[test]
    fn test_create_orderbook() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("creator", &[]);

        // Attempt to create an orderbook
        let quote_denom = "quote".to_string();
        let base_denom = "base".to_string();
        let create_response = create_orderbook(
            deps.as_mut(),
            env,
            info,
            quote_denom.clone(),
            base_denom.clone(),
        )
        .unwrap();

        // Verify response
        let expected_book_id: u64 = 0;
        assert_eq!(create_response.attributes[0], ("method", "createOrderbook"));
        assert_eq!(
            create_response.attributes[1],
            ("book_id", &expected_book_id.to_string())
        );

        // Verify orderbook is saved correctly
        let orderbook = ORDERBOOKS
            .load(deps.as_ref().storage, &expected_book_id)
            .unwrap();
        assert_eq!(orderbook.quote_denom, quote_denom);
        assert_eq!(orderbook.base_denom, base_denom);
        assert_eq!(orderbook.current_tick, 0);
        assert_eq!(orderbook.next_bid_tick, MIN_TICK);
        assert_eq!(orderbook.next_ask_tick, MAX_TICK);
    }
}
