//! Migration admission and rejection reasons consumed by the actual workflow.
#![allow(unexpected_cfgs)]

#[cfg(not(verus_keep_ghost))]
use vstd::prelude::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

use vstd::contrib::exec_spec::ExecSpecEq;

verus! {

pub enum RetirementError {
    ReplacementNotExecuted,
    MissingObligation,
    MissingOldCost,
}

pub open spec fn passed_status(status: Option<&str>) -> bool {
    match status { Some(text) => text@ == "passed"@, None => false }
}

pub open spec fn same_receipt(request: Option<&str>, receipt: Option<Option<&str>>) -> bool {
    match (request, receipt) {
        (None, Some(None)) => true,
        (Some(wanted), Some(Some(actual))) => wanted@ == actual@,
        _ => false,
    }
}

fn test_passed(status: Option<&str>) -> (passed: bool)
    ensures passed == (passed_status(status)),
{
    match status {
        Some(status) => <&str>::exec_eq(status, "passed"),
        None => false,
    }
}

pub fn repair_is_complete(repaired: bool, status: Option<&str>, old_cost: bool) -> (ok: bool)
    ensures ok == (repaired && passed_status(status) && old_cost),
{
    repaired && test_passed(status) && old_cost
}

// A missing receipt differs from a receipt recording retirement without replacement.
fn preserves_obligation(request: Option<&str>, receipt: Option<Option<&str>>) -> (ok: bool)
    ensures ok == (same_receipt(request, receipt)),
{
    match (request, receipt) {
        (None, Some(None)) => true,
        (Some(wanted), Some(Some(actual))) => <&str>::exec_eq(wanted, actual),
        _ => false,
    }
}

pub fn retirement_status(
    request: Option<&str>, execution: Option<&str>, listed: bool,
    receipt: Option<Option<&str>>, old_cost: bool,
) -> (out: Result<&'static str, RetirementError>)
    ensures
        out == if request.is_some() && !passed_status(execution) {
            Err(RetirementError::ReplacementNotExecuted)
        } else if listed && !same_receipt(request, receipt) {
            Err(RetirementError::MissingObligation)
        } else if !old_cost {
            Err(RetirementError::MissingOldCost)
        } else if request.is_some() {
            Ok("replaced")
        } else {
            Ok("retired")
        },
{
    if request.is_some() && !test_passed(execution) {
        return Err(RetirementError::ReplacementNotExecuted);
    }
    if listed && !preserves_obligation(request, receipt) {
        return Err(RetirementError::MissingObligation);
    }
    if !old_cost {
        return Err(RetirementError::MissingOldCost);
    }
    if request.is_some() { Ok("replaced") } else { Ok("retired") }
}

pub struct ConversionFacts {
    pub process_succeeded: bool,
    pub declared_input_digest_matches: bool,
    pub actual_input_digest_matches: Option<bool>,
    pub output_digest_matches: bool,
    pub decoded_output_matches: Option<bool>,
    pub mappings_match: bool,
}

pub struct MigrationFacts {
    pub declaration_count: usize,
    pub has_script: bool,
    pub conversion_present: bool,
    pub replacement_test_passed: bool,
    pub script_matches: bool,
    pub test_matches: bool,
    pub conversion: Option<ConversionFacts>,
}

pub enum MigrationError {
    DeclarationCount,
    UnexpectedConversion,
    MissingExecution,
    PairMismatch,
    ConversionMismatch,
}

pub open spec fn conversion_admissible(facts: ConversionFacts) -> bool {
    facts.process_succeeded
        && facts.declared_input_digest_matches
        && facts.actual_input_digest_matches == Some(true)
        && facts.output_digest_matches
        && facts.decoded_output_matches == Some(true)
        && facts.mappings_match
}

pub open spec fn migration_admissible(facts: MigrationFacts) -> bool {
    facts.declaration_count == 1
        && if facts.has_script {
            facts.replacement_test_passed
                && facts.conversion_present
                && facts.script_matches
                && facts.test_matches
                && match facts.conversion {
                    Some(conversion) => conversion_admissible(conversion),
                    None => false,
                }
        } else {
            !facts.conversion_present
        }
}

pub open spec fn migration_outcome(facts: MigrationFacts) -> Result<(), MigrationError> {
    if facts.declaration_count != 1 {
        Err(MigrationError::DeclarationCount)
    } else if !facts.has_script {
        if facts.conversion_present { Err(MigrationError::UnexpectedConversion) } else { Ok(()) }
    } else if !facts.replacement_test_passed || !facts.conversion_present {
        Err(MigrationError::MissingExecution)
    } else if !facts.script_matches || !facts.test_matches {
        Err(MigrationError::PairMismatch)
    } else if match facts.conversion {
        Some(conversion) => conversion_admissible(conversion),
        None => false,
    } {
        Ok(())
    } else {
        Err(MigrationError::ConversionMismatch)
    }
}

pub open spec fn migration_error_message(error: MigrationError) -> Seq<char> {
    match error {
        MigrationError::DeclarationCount => "E_MIGRATION_EVIDENCE: missing/ambiguous schema migration"@,
        MigrationError::UnexpectedConversion => "E_MIGRATION_EVIDENCE: actual data conversion requires its declared script/test pair"@,
        MigrationError::MissingExecution => "E_MIGRATION_EVIDENCE: declared data conversion requires its retained execution and passing test"@,
        MigrationError::PairMismatch => "E_MIGRATION_EVIDENCE: declared script/test differs from actual conversion pair"@,
        MigrationError::ConversionMismatch => "E_MIGRATION_EVIDENCE: conversion differs from declaration"@,
    }
}

impl MigrationError {
    pub fn message(self) -> (message: String)
        ensures message@ == migration_error_message(self),
    {
        match self {
            MigrationError::DeclarationCount => "E_MIGRATION_EVIDENCE: missing/ambiguous schema migration".to_string(),
            MigrationError::UnexpectedConversion => "E_MIGRATION_EVIDENCE: actual data conversion requires its declared script/test pair".to_string(),
            MigrationError::MissingExecution => "E_MIGRATION_EVIDENCE: declared data conversion requires its retained execution and passing test".to_string(),
            MigrationError::PairMismatch => "E_MIGRATION_EVIDENCE: declared script/test differs from actual conversion pair".to_string(),
            MigrationError::ConversionMismatch => "E_MIGRATION_EVIDENCE: conversion differs from declaration".to_string(),
        }
    }
}

/// Check the actual filtered declaration count before the adapter indexes it.
pub fn admit_declaration(count: usize) -> (out: Result<(), MigrationError>)
    ensures out == if count == 1 { Ok(()) } else { Err(MigrationError::DeclarationCount) },
{
    if count == 1 { Ok(()) } else { Err(MigrationError::DeclarationCount) }
}

fn conversion_matches(facts: ConversionFacts) -> (admitted: bool)
    ensures admitted == conversion_admissible(facts),
{
    facts.process_succeeded
        && facts.declared_input_digest_matches
        && match facts.actual_input_digest_matches { Some(value) => value, None => false }
        && facts.output_digest_matches
        && match facts.decoded_output_matches { Some(value) => value, None => false }
        && facts.mappings_match
}

/// This function owns both the decision and its rejection classification.
pub fn admit_migration(facts: MigrationFacts) -> (out: Result<(), MigrationError>)
    ensures
        out == migration_outcome(facts),
        out.is_ok() == migration_admissible(facts),
{
    match admit_declaration(facts.declaration_count) {
        Err(error) => return Err(error),
        Ok(()) => {},
    }
    if !facts.has_script {
        return if facts.conversion_present {
            Err(MigrationError::UnexpectedConversion)
        } else {
            Ok(())
        };
    }
    if !facts.replacement_test_passed || !facts.conversion_present {
        return Err(MigrationError::MissingExecution);
    }
    if !facts.script_matches || !facts.test_matches {
        return Err(MigrationError::PairMismatch);
    }
    let matches = match facts.conversion {
        Some(conversion) => conversion_matches(conversion),
        None => false,
    };
    if matches { Ok(()) } else { Err(MigrationError::ConversionMismatch) }
}

}
