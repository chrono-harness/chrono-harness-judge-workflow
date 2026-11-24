use chrono_judge_workflow::retired_test_report;
use serde_json::json;
use std::collections::BTreeMap;

#[test]
fn report_uses_the_old_test_identity_for_cost_and_receipt() {
    let old = "test:obsolete";
    let next = "test:successor";
    let request = Ok(Some(next.to_owned()));
    let impact = json!({"retired_tests":[old]});
    let tests = BTreeMap::from([(next.to_owned(), "passed".to_owned())]);
    let receipts = BTreeMap::from([(old.to_owned(), Some(next.to_owned()))]);
    let costs = json!({old: null});
    let report = |request: &Result<Option<String>, String>,
                  tests: &BTreeMap<String, String>,
                  receipts: &BTreeMap<String, Option<String>>,
                  costs: &serde_json::Value| {
        retired_test_report(old, request, &impact, tests, receipts, costs)
    };
    assert_eq!(
        report(&request, &tests, &receipts, &costs).unwrap(),
        json!({"kind":"test","id":"obsolete","replacement":next,"status":"replaced","executed_old_test":false})
    );
    assert_eq!(
        report(&request, &BTreeMap::new(), &BTreeMap::new(), &json!({})).unwrap_err(),
        "E_REQUIRED_TEST_REMOVED: replacement not executed: test:obsolete"
    );
    assert_eq!(
        report(
            &request,
            &tests,
            &BTreeMap::from([(next.to_owned(), Some(next.to_owned()))]),
            &costs
        )
        .unwrap_err(),
        "E_RETIREMENT: execution did not preserve removed obligation"
    );
    assert_eq!(
        report(&request, &tests, &receipts, &json!({next: null})).unwrap_err(),
        "E_RETIREMENT: old cost missing: test:obsolete"
    );
    assert_eq!(
        report(
            &Ok(None),
            &tests,
            &BTreeMap::from([(old.to_owned(), None)]),
            &costs
        )
        .unwrap(),
        json!({"kind":"test","id":"obsolete","replacement":null,"status":"retired","executed_old_test":false})
    );
}
