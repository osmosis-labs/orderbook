use cosmwasm_std::{
    testing::{mock_dependencies, mock_info},
    Addr,
};

use crate::{
    auth::{
        dispatch_cancel_admin_transfer, dispatch_claim_admin, dispatch_reject_admin_transfer,
        dispatch_renounce_adminship, dispatch_transfer_admin, ADMIN, ADMIN_OFFER,
    },
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

        // Assert the admin has been claimed
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

        // Assert the admin has been claimed
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

        // Assert the admin has been removed
        assert!(
            ADMIN.may_load(deps.as_ref().storage).unwrap().is_none(),
            "{}: admin was not correctly removed",
            test.name
        );
    }
}
