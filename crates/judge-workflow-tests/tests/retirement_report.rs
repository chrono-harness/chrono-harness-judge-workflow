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

#[test]
fn transition_reports_bind_first_error_and_repair_identity() {
    use chrono_judge_workflow::retirement_reports;
    use serde_json::Value;
    let old = "test:old";
    let next = "test:next";
    let impact = json!({"required_tests":[{"node":old}]});
    let costs = json!({old:null});
    let declaration = json!({"requests":{"Ok":{}},"ambiguities":{old:{"repaired":true,"definitions":["before","after"]}},
        "removed_tests":{},"removed_owners":[{"kind":"judge","id":"obsolete","replacement":{"Ok":"cost"}}]});
    let review = |raw: Option<&Value>, impact: &Value, passed: &str, old_costs: &Value| {
        let declared = raw.map(|value| serde_json::from_value(value.clone()).unwrap());
        let tests = BTreeMap::from([(passed.to_string(), "passed".to_string())]);
        retirement_reports(
            declared.as_ref(),
            impact,
            &tests,
            &BTreeMap::new(),
            old_costs,
        )
    };
    assert_eq!(
        review(None, &json!({}), "", &json!({})).unwrap_err(),
        "missing workflow transition declarations"
    );
    let mut bad = declaration.clone();
    bad["requests"] = json!({"Err":"original registration error"});
    assert_eq!(
        review(Some(&bad), &json!({}), "", &json!({})).unwrap_err(),
        "original registration error"
    );
    assert_eq!(
        review(Some(&declaration), &json!({}), "", &json!({})).unwrap_err(),
        "E_RETIREMENT: missing obligations"
    );
    let malformed = json!({"required_tests":[{"node":"unaffected"},{}]});
    assert_eq!(
        review(Some(&declaration), &malformed, "", &json!({})).unwrap_err(),
        "E_RETIREMENT: test node"
    );
    for (request, target) in [
        (json!({}), old),
        (json!({old:null}), old),
        (json!({old:next}), next),
    ] {
        let mut chosen = declaration.clone();
        chosen["requests"] = json!({"Ok":request});
        let (retired, repairs) = review(Some(&chosen), &impact, target, &costs).unwrap();
        assert_eq!(retired,json!([{"kind":"judge","id":"obsolete","replacement":"cost","status":"declared-removal"}]).as_array().unwrap().clone());
        assert_eq!(
            repairs,
            vec![
                json!({"node":old,"definitions":["before","after"],"replacement":target,"status":"repaired"})
            ]
        );
        for (passed, old_costs) in [("unexecuted", &costs), (target, &json!({}))] {
            assert_eq!(
                review(Some(&chosen), &impact, passed, old_costs).unwrap_err(),
                "E_RETIREMENT: ambiguity needs explicit repair, old costs and executed replacement: test:old"
            );
        }
    }
    let mut incomplete = declaration.clone();
    incomplete["ambiguities"][old]["repaired"] = json!(false);
    incomplete["removed_owners"][0]["replacement"] = json!({"Err":"owner registration failed"});
    assert_eq!(
        review(Some(&incomplete), &impact, old, &costs).unwrap_err(),
        "E_RETIREMENT: ambiguity needs explicit repair, old costs and executed replacement: test:old"
    );
    assert_eq!(
        review(
            Some(&incomplete),
            &json!({"required_tests":[]}),
            old,
            &costs
        )
        .unwrap_err(),
        "owner registration failed"
    );
}
