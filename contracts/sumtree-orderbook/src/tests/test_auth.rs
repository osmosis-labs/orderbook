use cosmwasm_std::{
    from_json,
    testing::{mock_dependencies, mock_env, mock_info},
    Addr,
};

use crate::{
    auth::{
        dispatch_cancel_admin_transfer, dispatch_claim_admin, dispatch_claim_moderator,
        dispatch_offer_moderator, dispatch_reject_admin_transfer, dispatch_reject_moderator_offer,
        dispatch_renounce_adminship, dispatch_transfer_admin, ADMIN, ADMIN_OFFER, MODERATOR,
        MODERATOR_OFFER,
    },
    contract::{execute, query},
    msg::{AuthExecuteMsg, AuthQueryMsg, ExecuteMsg, QueryMsg},
    state::IS_ACTIVE,
    ContractError,
};

struct TransferAdminTestCase {
    name: &'static str,
    current_admin: &'static str,
    new_admin: &'static str,
    expected_error: Option<ContractError>,
}

#[test]
fn test_transfer_admin() {
    let sender = "sender";
    let new_admin = "new_admin";
    let test_cases = vec![
        TransferAdminTestCase {
            name: "valid transfer",
            current_admin: sender,
            new_admin,
            expected_error: None,
        },
        TransferAdminTestCase {
            name: "unauthorized",
            current_admin: "notthesender",
            new_admin,
            expected_error: Some(ContractError::Unauthorized {}),
        },
    ];

    for test in test_cases {
        // -- Test Setup --
        let mut deps = mock_dependencies();
        let info = mock_info(sender, &[]);

        // Store current admin
        ADMIN
            .save(deps.as_mut().storage, &Addr::unchecked(test.current_admin))
            .unwrap();

        // -- System under test --
        let res = dispatch_transfer_admin(deps.as_mut(), info, Addr::unchecked(test.new_admin));

        // Assert expected error
        if let Some(err) = test.expected_error {
            assert_eq!(
                res.unwrap_err(),
                err,
                "{}: did not receive expected error",
                test.name
            );

            // Ensure nothing is stored in state
            assert!(ADMIN_OFFER
                .may_load(deps.as_ref().storage)
                .unwrap()
                .is_none());
            continue;
        }

        // Assert the offer to the new admin is stored
        let new_admin = ADMIN_OFFER.load(deps.as_ref().storage).unwrap();
        assert_eq!(
            new_admin, test.new_admin,
            "{}: admin offer was not correctly set",
            test.name
        );
    }
}

struct CancelAdminTransferTestCase {
    name: &'static str,
    current_admin: &'static str,
    new_admin: &'static str,
    expected_error: Option<ContractError>,
}

#[test]
fn test_cancel_admin_transfer() {
    let sender = "sender";
    let new_admin = "new_admin";
    let test_cases = vec![
        CancelAdminTransferTestCase {
            name: "valid transfer",
            current_admin: sender,
            new_admin,
            expected_error: None,
        },
        CancelAdminTransferTestCase {
            name: "unauthorized",
            current_admin: "notthesender",
            new_admin,
            expected_error: Some(ContractError::Unauthorized {}),
        },
    ];

    for test in test_cases {
        // -- Test Setup --
        let mut deps = mock_dependencies();
        let info = mock_info(sender, &[]);

        // Store current admin
        ADMIN
            .save(deps.as_mut().storage, &Addr::unchecked(test.current_admin))
            .unwrap();
        // Store admin transfer
        ADMIN_OFFER
            .save(deps.as_mut().storage, &Addr::unchecked(test.new_admin))
            .unwrap();

        // -- System under test --
        let res = dispatch_cancel_admin_transfer(deps.as_mut(), info);

        // Assert expected error
        if let Some(err) = test.expected_error {
            assert_eq!(
                res.unwrap_err(),
                err,
                "{}: did not receive expected error",
                test.name
            );

            // Ensure nothing is stored in state
            assert!(ADMIN_OFFER.load(deps.as_ref().storage).unwrap() == test.new_admin);
            continue;
        }

        // Assert the offer has been rescinded
        assert!(
            ADMIN_OFFER
                .may_load(deps.as_ref().storage)
                .unwrap()
                .is_none(),
            "{}: admin offer was not correctly set",
            test.name
        );
    }
}

struct ClaimAdminTestCase {
    name: &'static str,
    sender: &'static str,
    new_admin: Option<&'static str>,
    expected_error: Option<ContractError>,
}

#[test]
fn test_claim_admin() {
    let new_admin = "new_admin";
    let test_cases = vec![
        ClaimAdminTestCase {
            name: "valid claim",
            sender: new_admin,
            new_admin: Some(new_admin),
            expected_error: None,
        },
        ClaimAdminTestCase {
            name: "no offer",
            sender: new_admin,
            new_admin: None,
            expected_error: Some(ContractError::Unauthorized {}),
        },
        ClaimAdminTestCase {
            name: "unauthorized",
            sender: "notthenewadmin",
            new_admin: Some(new_admin),
            expected_error: Some(ContractError::Unauthorized {}),
        },
    ];

    for test in test_cases {
        // -- Test Setup --
        let mut deps = mock_dependencies();
        let info = mock_info(test.sender, &[]);

        // Save admin offer if one is required
        if let Some(new_admin) = test.new_admin {
            ADMIN_OFFER
                .save(deps.as_mut().storage, &Addr::unchecked(new_admin))
                .unwrap();
        }

        // -- System under test --
        let res = dispatch_claim_admin(deps.as_mut(), info);

        // Assert expected error
        if let Some(err) = test.expected_error {
            assert_eq!(
                res.unwrap_err(),
                err,
                "{}: did not receive expected error",
                test.name
            );

            // Ensure nothing is stored in state
            assert!(
                ADMIN.may_load(deps.as_ref().storage).unwrap().is_none(),
                "{}: invalid admin stored",
                test.name
            );
            continue;
        }

        // Assert the admin role has been claimed
        assert_eq!(
            ADMIN.may_load(deps.as_ref().storage).unwrap(),
            test.new_admin.map(Addr::unchecked),
            "{}: admin was not correctly claimed",
            test.name
        );
        // Assert the offer has been removed
        assert!(
            ADMIN_OFFER
                .may_load(deps.as_ref().storage)
                .unwrap()
                .is_none(),
            "{}: admin offer not correctly removed",
            test.name
        );
    }
}

struct RejectAdminTransferTestCase {
    name: &'static str,
    sender: &'static str,
    new_admin: Option<&'static str>,
    expected_error: Option<ContractError>,
}

#[test]
fn test_reject_admin_transfer() {
    let new_admin = "new_admin";
    let current_admin = "current_admin";
    let test_cases = vec![
        RejectAdminTransferTestCase {
            name: "valid rejection",
            sender: new_admin,
            new_admin: Some(new_admin),
            expected_error: None,
        },
        RejectAdminTransferTestCase {
            name: "no offer",
            sender: new_admin,
            new_admin: None,
            expected_error: Some(ContractError::Unauthorized {}),
        },
        RejectAdminTransferTestCase {
            name: "unauthorized",
            sender: "notthenewadmin",
            new_admin: Some(new_admin),
            expected_error: Some(ContractError::Unauthorized {}),
        },
    ];

    for test in test_cases {
        // -- Test Setup --
        let mut deps = mock_dependencies();
        let info = mock_info(test.sender, &[]);

        // Store current admin for post check
        ADMIN
            .save(deps.as_mut().storage, &Addr::unchecked(current_admin))
            .unwrap();

        // Save admin offer if one is required
        if let Some(new_admin) = test.new_admin {
            ADMIN_OFFER
                .save(deps.as_mut().storage, &Addr::unchecked(new_admin))
                .unwrap();
        }

        // -- System under test --
        let res = dispatch_reject_admin_transfer(deps.as_mut(), info);

        // Assert expected error
        if let Some(err) = test.expected_error {
            assert_eq!(
                res.unwrap_err(),
                err,
                "{}: did not receive expected error",
                test.name
            );

            // Ensure state remains unchanged
            assert_eq!(
                ADMIN.load(deps.as_ref().storage).unwrap(),
                Addr::unchecked(current_admin),
                "{}: invalid admin stored",
                test.name
            );
            assert_eq!(
                ADMIN_OFFER.may_load(deps.as_ref().storage).unwrap(),
                test.new_admin.map(Addr::unchecked),
                "{}: admin offer was unexpectedly altered",
                test.name
            );
            continue;
        }

        // Assert the admin role has been claimed
        assert_eq!(
            ADMIN.load(deps.as_ref().storage).unwrap(),
            Addr::unchecked(current_admin),
            "{}: admin was not correctly claimed",
            test.name
        );
        // Assert the offer has been removed
        assert!(
            ADMIN_OFFER
                .may_load(deps.as_ref().storage)
                .unwrap()
                .is_none(),
            "{}: admin offer not correctly removed",
            test.name
        );
    }
}

struct RenounceAdminshipTestCase {
    name: &'static str,
    sender: &'static str,
    expected_error: Option<ContractError>,
}

#[test]
fn test_renounce_adminship() {
    let current_admin = "current_admin";
    let test_cases = vec![
        RenounceAdminshipTestCase {
            name: "valid renouncement",
            sender: current_admin,
            expected_error: None,
        },
        RenounceAdminshipTestCase {
            name: "unauthorized",
            sender: "notthenewadmin",
            expected_error: Some(ContractError::Unauthorized {}),
        },
    ];

    for test in test_cases {
        // -- Test Setup --
        let mut deps = mock_dependencies();
        let info = mock_info(test.sender, &[]);

        // Store current admin for post check
        ADMIN
            .save(deps.as_mut().storage, &Addr::unchecked(current_admin))
            .unwrap();

        // Save admin offer if one is required
        ADMIN
            .save(deps.as_mut().storage, &Addr::unchecked(current_admin))
            .unwrap();

        // -- System under test --
        let res = dispatch_renounce_adminship(deps.as_mut(), info);

        // Assert expected error
        if let Some(err) = test.expected_error {
            assert_eq!(
                res.unwrap_err(),
                err,
                "{}: did not receive expected error",
                test.name
            );

            // Ensure state remains unchanged
            assert_eq!(
                ADMIN.load(deps.as_ref().storage).unwrap(),
                Addr::unchecked(current_admin),
                "{}: invalid admin stored",
                test.name
            );
            continue;
        }

        // Assert the admin role has been removed
        assert!(
            ADMIN.may_load(deps.as_ref().storage).unwrap().is_none(),
            "{}: admin was not correctly removed",
            test.name
        );
    }
}

// -- Moderator Execute Tests --

struct OfferModeratorTestCase {
    name: &'static str,
    sender: &'static str,
    new_moderator: &'static str,
    expected_error: Option<ContractError>,
}

#[test]
fn test_offer_moderator() {
    let current_admin = "current_admin";
    let current_moderator = "current_moderator";
    let new_moderator = "new_admin";
    let test_cases = vec![
        OfferModeratorTestCase {
            name: "valid offer",
            sender: current_admin,
            new_moderator,
            expected_error: None,
        },
        OfferModeratorTestCase {
            name: "unauthorized",
            sender: "nottheadmin",
            new_moderator,
            expected_error: Some(ContractError::Unauthorized {}),
        },
        OfferModeratorTestCase {
            name: "unauthorized current moderator",
            sender: current_moderator,
            new_moderator,
            expected_error: Some(ContractError::Unauthorized {}),
        },
    ];

    for test in test_cases {
        // -- Test Setup --
        let mut deps = mock_dependencies();
        let info = mock_info(test.sender, &[]);

        // Store current admin
        ADMIN
            .save(deps.as_mut().storage, &Addr::unchecked(current_admin))
            .unwrap();
        // Store current moderator
        MODERATOR
            .save(deps.as_mut().storage, &Addr::unchecked(current_moderator))
            .unwrap();

        // -- System under test --
        let res =
            dispatch_offer_moderator(deps.as_mut(), info, Addr::unchecked(test.new_moderator));

        // Assert expected error
        if let Some(err) = test.expected_error {
            assert_eq!(
                res.unwrap_err(),
                err,
                "{}: did not receive expected error",
                test.name
            );

            // Ensure nothing is stored in state
            assert!(MODERATOR_OFFER
                .may_load(deps.as_ref().storage)
                .unwrap()
                .is_none());
            continue;
        }

        // Assert the offer to the new moderator is stored
        let new_moderator_offer = MODERATOR_OFFER.load(deps.as_ref().storage).unwrap();
        assert_eq!(
            new_moderator_offer, test.new_moderator,
            "{}: moderator offer was not correctly set",
            test.name
        );
    }
}

struct RejectModeratorOfferTestCase {
    name: &'static str,
    sender: &'static str,
    new_moderator: Option<&'static str>,
    expected_error: Option<ContractError>,
}

#[test]
fn test_reject_moderator_offer() {
    let new_moderator = "new_moderator";
    let current_moderator = "current_moderator";
    let test_cases = vec![
        RejectModeratorOfferTestCase {
            name: "valid rejection",
            sender: new_moderator,
            new_moderator: Some(new_moderator),
            expected_error: None,
        },
        RejectModeratorOfferTestCase {
            name: "no offer",
            sender: new_moderator,
            new_moderator: None,
            expected_error: Some(ContractError::Unauthorized {}),
        },
        RejectModeratorOfferTestCase {
            name: "unauthorized",
            sender: "notthenewmoderator",
            new_moderator: Some(new_moderator),
            expected_error: Some(ContractError::Unauthorized {}),
        },
    ];

    for test in test_cases {
        // -- Test Setup --
        let mut deps = mock_dependencies();
        let info = mock_info(test.sender, &[]);

        // Store current moderator for post check
        MODERATOR
            .save(deps.as_mut().storage, &Addr::unchecked(current_moderator))
            .unwrap();

        // Save moderator offer if one is required
        if let Some(new_admin) = test.new_moderator {
            MODERATOR_OFFER
                .save(deps.as_mut().storage, &Addr::unchecked(new_admin))
                .unwrap();
        }

        // -- System under test --
        let res = dispatch_reject_moderator_offer(deps.as_mut(), info);

        // Assert expected error
        if let Some(err) = test.expected_error {
            assert_eq!(
                res.unwrap_err(),
                err,
                "{}: did not receive expected error",
                test.name
            );

            // Ensure state remains unchanged
            assert_eq!(
                MODERATOR.load(deps.as_ref().storage).unwrap(),
                Addr::unchecked(current_moderator),
                "{}: invalid admin stored",
                test.name
            );
            assert_eq!(
                MODERATOR_OFFER.may_load(deps.as_ref().storage).unwrap(),
                test.new_moderator.map(Addr::unchecked),
                "{}: admin offer was unexpectedly altered",
                test.name
            );
            continue;
        }

        // Assert the moderator role has been claimed
        assert_eq!(
            MODERATOR.load(deps.as_ref().storage).unwrap(),
            Addr::unchecked(current_moderator),
            "{}: admin was not correctly claimed",
            test.name
        );
        // Assert the offer has been removed
        assert!(
            MODERATOR_OFFER
                .may_load(deps.as_ref().storage)
                .unwrap()
                .is_none(),
            "{}: admin offer not correctly removed",
            test.name
        );
    }
}

struct ClaimModeratorTestCase {
    name: &'static str,
    sender: &'static str,
    new_moderator: Option<&'static str>,
    expected_error: Option<ContractError>,
}

#[test]
fn test_claim_moderator() {
    let new_moderator = "new_moderator";
    let current_moderator = "current_moderator";
    let test_cases = vec![
        ClaimModeratorTestCase {
            name: "valid rejection",
            sender: new_moderator,
            new_moderator: Some(new_moderator),
            expected_error: None,
        },
        ClaimModeratorTestCase {
            name: "no offer",
            sender: new_moderator,
            new_moderator: None,
            expected_error: Some(ContractError::Unauthorized {}),
        },
        ClaimModeratorTestCase {
            name: "unauthorized",
            sender: "notthenewmoderator",
            new_moderator: Some(new_moderator),
            expected_error: Some(ContractError::Unauthorized {}),
        },
    ];

    for test in test_cases {
        // -- Test Setup --
        let mut deps = mock_dependencies();
        let info = mock_info(test.sender, &[]);

        // Store current moderator for post check
        MODERATOR
            .save(deps.as_mut().storage, &Addr::unchecked(current_moderator))
            .unwrap();

        // Save moderator offer if one is required
        if let Some(new_admin) = test.new_moderator {
            MODERATOR_OFFER
                .save(deps.as_mut().storage, &Addr::unchecked(new_admin))
                .unwrap();
        }

        // -- System under test --
        let res = dispatch_claim_moderator(deps.as_mut(), info);

        // Assert expected error
        if let Some(err) = test.expected_error {
            assert_eq!(
                res.unwrap_err(),
                err,
                "{}: did not receive expected error",
                test.name
            );

            // Ensure state remains unchanged
            assert_eq!(
                MODERATOR.load(deps.as_ref().storage).unwrap(),
                Addr::unchecked(current_moderator),
                "{}: invalid admin stored",
                test.name
            );
            assert_eq!(
                MODERATOR_OFFER.may_load(deps.as_ref().storage).unwrap(),
                test.new_moderator.map(Addr::unchecked),
                "{}: admin offer was unexpectedly altered",
                test.name
            );
            continue;
        }

        // Assert the moderator role has been claimed
        assert_eq!(
            MODERATOR.load(deps.as_ref().storage).unwrap(),
            Addr::unchecked(test.new_moderator.unwrap()),
            "{}: admin was not correctly claimed",
            test.name
        );
        // Assert the offer has been removed
        assert!(
            MODERATOR_OFFER
                .may_load(deps.as_ref().storage)
                .unwrap()
                .is_none(),
            "{}: admin offer not correctly removed",
            test.name
        );
    }
}

struct SetActiveTestCase {
    name: &'static str,
    sender: &'static str,
    current_active_state: bool,
    new_active_state: bool,
    expected_error: Option<ContractError>,
}

#[test]
fn test_set_active() {
    let current_admin = "current_admin";
    let current_moderator = "current_moderator";
    let test_cases = vec![
        SetActiveTestCase {
            name: "valid admin set active: true -> false",
            sender: current_admin,
            current_active_state: true,
            new_active_state: false,
            expected_error: None,
        },
        SetActiveTestCase {
            name: "valid admin set active: false -> true",
            sender: current_admin,
            current_active_state: false,
            new_active_state: true,
            expected_error: None,
        },
        SetActiveTestCase {
            name: "valid moderator set active: true -> false",
            sender: current_moderator,
            current_active_state: true,
            new_active_state: false,
            expected_error: None,
        },
        SetActiveTestCase {
            name: "valid moderator set active: false -> true",
            sender: current_moderator,
            current_active_state: false,
            new_active_state: true,
            expected_error: None,
        },
        SetActiveTestCase {
            name: "unauthorized",
            sender: "notadminormoderator",
            current_active_state: true,
            new_active_state: false,
            expected_error: Some(ContractError::Unauthorized {}),
        },
    ];

    for test in test_cases {
        // -- Test Setup --
        let mut deps = mock_dependencies();
        let info = mock_info(test.sender, &[]);
        let env = mock_env();

        // Setup state variables
        IS_ACTIVE
            .save(deps.as_mut().storage, &test.current_active_state)
            .unwrap();
        ADMIN
            .save(deps.as_mut().storage, &Addr::unchecked(current_admin))
            .unwrap();
        MODERATOR
            .save(deps.as_mut().storage, &Addr::unchecked(current_moderator))
            .unwrap();

        let msg = AuthExecuteMsg::SetActive {
            active: test.new_active_state,
        };

        // -- System under test --
        let res = execute(deps.as_mut(), env, info, ExecuteMsg::Auth(msg));

        // -- Test Assertions --
        if let Some(err) = test.expected_error {
            assert_eq!(
                res.unwrap_err(),
                err,
                "{}: did not receive expected error",
                test.name
            );
            continue;
        }

        assert!(
            res.is_ok(),
            "{}: message errored unexpectedly; {}",
            test.name,
            res.unwrap_err()
        );

        res.unwrap();

        let is_active = IS_ACTIVE.load(deps.as_ref().storage).unwrap();
        assert_eq!(
            is_active, test.new_active_state,
            "{}: active state did not update correctly",
            test.name
        );
    }
}

#[test]
fn test_get_admin() {
    let admin = "admin";
    let mut deps = mock_dependencies();
    let env = mock_env();

    ADMIN
        .save(deps.as_mut().storage, &Addr::unchecked(admin))
        .unwrap();

    let msg = QueryMsg::Auth(AuthQueryMsg::Admin {});
    let res = query(deps.as_ref(), env, msg).unwrap();

    let admin_res: Option<Addr> = from_json(res).unwrap();

    assert_eq!(Some(Addr::unchecked(admin)), admin_res);
}

#[test]
fn test_get_admin_offer() {
    let admin = "admin";
    let mut deps = mock_dependencies();
    let env = mock_env();

    let msg = QueryMsg::Auth(AuthQueryMsg::AdminOffer {});
    let res = query(deps.as_ref(), env.clone(), msg).unwrap();

    let admin_res: Option<Addr> = from_json(res).unwrap();

    assert!(admin_res.is_none());

    ADMIN_OFFER
        .save(deps.as_mut().storage, &Addr::unchecked(admin))
        .unwrap();

    let msg = QueryMsg::Auth(AuthQueryMsg::AdminOffer {});
    let res = query(deps.as_ref(), env, msg).unwrap();

    let admin_res: Option<Addr> = from_json(res).unwrap();

    assert_eq!(admin_res, Some(Addr::unchecked(admin)));
}

#[test]
fn test_get_moderator() {
    let moderator: &str = "mod";
    let mut deps = mock_dependencies();
    let env = mock_env();

    MODERATOR
        .save(deps.as_mut().storage, &Addr::unchecked(moderator))
        .unwrap();

    let msg = QueryMsg::Auth(AuthQueryMsg::Moderator {});
    let res = query(deps.as_ref(), env, msg).unwrap();

    let admin_res: Option<Addr> = from_json(res).unwrap();

    assert_eq!(Some(Addr::unchecked(moderator)), admin_res);
}

#[test]
fn test_get_moderator_offer() {
    let moderator = "moderator";
    let mut deps = mock_dependencies();
    let env = mock_env();

    let msg = QueryMsg::Auth(AuthQueryMsg::AdminOffer {});
    let res = query(deps.as_ref(), env.clone(), msg).unwrap();

    let admin_res: Option<Addr> = from_json(res).unwrap();

    assert!(admin_res.is_none());

    MODERATOR_OFFER
        .save(deps.as_mut().storage, &Addr::unchecked(moderator))
        .unwrap();

    let msg = QueryMsg::Auth(AuthQueryMsg::ModeratorOffer {});
    let res = query(deps.as_ref(), env, msg).unwrap();

    let admin_res: Option<Addr> = from_json(res).unwrap();

    assert_eq!(admin_res, Some(Addr::unchecked(moderator)));
}
