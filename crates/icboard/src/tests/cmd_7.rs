use crate::tests::test_output;

#[test]
fn sysop_user_maintenance_can_renew_a_subscription() {
    let output = test_output("7\nC\n12/31/2027\nQ\n".to_string(), |_| {});
    assert!(output.contains("12/31/27"), "the renewed date was not shown:\n{output}");
}

#[test]
fn sysop_user_maintenance_can_make_a_subscription_non_expiring() {
    let output = test_output("7\nC\n00/00/00\nQ\n".to_string(), |board| {
        board.users[0].expiration_date = chrono::Utc::now() + chrono::Duration::days(30);
    });
    assert!(output.contains("Expires    : \nLast on"), "the expiration date was not cleared:\n{output}");
}
