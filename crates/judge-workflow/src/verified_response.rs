//! Workflow 原错误消息到实际协议错误码与严重程度的唯一判断。
#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
use vstd::string::StringSliceAdditionalSpecFns;
use vstd::{contrib::exec_spec::ExecSpecEq, prelude::*};

verus! {

pub open spec fn error_code(message: Seq<u8>) -> Seq<u8> {
    let end = message.index_of_first(58).unwrap_or(message.len() as int);
    let head = message.take(end);
    if head.len() >= 2 && head[0] == 69 && head[1] == 95 {
        head
    } else {
        "E_WORKFLOW_INPUT".spec_bytes()
    }
}

pub proof fn code_ends_at(bytes: Seq<u8>, end: int)
    requires
        0 <= end <= bytes.len(),
        end < bytes.len() ==> bytes[end] == 58,
        forall|i: int| 0 <= i < end ==> bytes[i] != 58,
    ensures bytes.index_of_first(58).unwrap_or(bytes.len() as int) == end,
{
    bytes.index_of_first_ensures(58);
    match bytes.index_of_first(58) {
        Some(first) => {
            if first < end { assert(bytes[first] != 58); }
            if end < first { assert(bytes[end] != 58); }
        },
        None => {
            if end < bytes.len() { assert(bytes.contains(58)); }
        },
    }
}

fn workflow_code(message: &str) -> (code: &str)
    ensures code.spec_bytes() == error_code(message.spec_bytes()),
{
    let bytes = message.as_bytes();
    proof { bytes@.index_of_first_ensures(58); }
    if bytes.len() < 2 || bytes[0] != 69u8 || bytes[1] != 95u8 {
        return "E_WORKFLOW_INPUT";
    }
    for i in 2..bytes.len()
        invariant
            bytes@ == message.spec_bytes(),
            bytes.len() >= 2, bytes[0] == 69, bytes[1] == 95,
            forall|j: int| 0 <= j < i ==> bytes@[j] != 58,
    {
        if bytes[i] == 58u8 {
            proof {
                code_ends_at(bytes@, i as int);
                vstd::utf8::encode_utf8_valid_utf8(message@);
                vstd::utf8::is_char_boundary_iff_not_is_continuation_byte(bytes@, i as int);
            }
            return message.split_at(i).0;
        }
    }
    proof {
        code_ends_at(bytes@, bytes.len() as int);
        assert(bytes@.take(bytes.len() as int) =~= bytes@);
    }
    message
}

pub open spec fn rejected(code: Seq<char>) -> bool {
    code == "E_BRANCH_STALE"@
        || code == "E_RETIREMENT"@
        || code == "E_REQUIRED_TEST_REMOVED"@
        || code == "E_INTEGRATION_REQUIRED"@
        || code == "E_INTEGRATION_MISMATCH"@
}

/// 只取首个 ASCII 冒号之前的完整字段；其余原消息由调用方原样保留。
/// 恰好五种领域拒绝产生 Fail，其余合法错误码及缺省码都产生 Error。
pub fn classify_error<T>(message: &str, fail: T, error: T) -> (out: (&str, T))
    ensures out.0.spec_bytes() == error_code(message.spec_bytes()),
        out.1 == if rejected(out.0@) { fail } else { error },
{
    let code = workflow_code(message);
    let is_rejection = <&str>::exec_eq(code, "E_BRANCH_STALE")
        || <&str>::exec_eq(code, "E_RETIREMENT")
        || <&str>::exec_eq(code, "E_REQUIRED_TEST_REMOVED")
        || <&str>::exec_eq(code, "E_INTEGRATION_REQUIRED")
        || <&str>::exec_eq(code, "E_INTEGRATION_MISMATCH");
    (code, if is_rejection { fail } else { error })
}

}
