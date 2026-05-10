//! Top-level instruction-selection dispatch.
//!
//! Walks each TIR `Instr` and `Terminator` and routes to the
//! appropriate `lower::*` helper. Unknown / unsupported ops produce
//! `BackendError::Unimplemented` so the bring-up scope is explicit
//! rather than silently dropping work.

use sw_codegen_core::BackendError;
use sw_tir::{Instr, Op, Terminator, ValueId};

use crate::lower::{LowerCtx, arith, call, cmp_branch, mem};

pub fn dispatch(ctx: &mut LowerCtx, instr: &Instr) -> Result<(), BackendError> {
    let result = instr.result;
    match &instr.op {
        Op::Add(a, b) => arith::add(ctx, *a, *b, require_result(result)?),
        Op::Sub(a, b) => arith::sub(ctx, *a, *b, require_result(result)?),
        Op::Mul(a, b) => arith::mul(ctx, *a, *b, require_result(result)?),
        Op::SDiv(a, b) => arith::sdiv(ctx, *a, *b, require_result(result)?),
        Op::Load { addr, ty: _ } => mem::load(ctx, *addr, require_result(result)?),
        Op::Store { addr, val } => mem::store(ctx, *addr, *val),
        Op::Icmp { pred, lhs, rhs } => {
            cmp_branch::icmp(ctx, *pred, *lhs, *rhs, require_result(result)?)
        }
        Op::Call { callee, args } => {
            // For now: callee must be an `AddrOf(symbol)` that we can
            // name. Direct ValueId callees would require a more
            // elaborate indirect-call lowering, deferred.
            let callee_name = ctx
                .func
                .blocks
                .iter()
                .flat_map(|b| b.instrs.iter())
                .find_map(|i| match (&i.op, i.result) {
                    (Op::AddrOf(sym), Some(v)) if v == *callee => Some(sym.0.clone()),
                    _ => None,
                })
                .ok_or(BackendError::BadIR("Call callee must be a named AddrOf"))?;
            call::call(ctx, &callee_name, args, result)
        }
        Op::AddrOf(_) => {
            // No-op at the codegen level: AddrOf names a symbol that
            // a later pass binds. Snapshot tests that exercise calls
            // emit AddrOf instructions explicitly to feed the Call
            // lowering above.
            Ok(())
        }
        // Ops outside the bring-up scope.
        Op::UDiv(_, _)
        | Op::SRem(_, _)
        | Op::URem(_, _)
        | Op::And(_, _)
        | Op::Or(_, _)
        | Op::Xor(_, _)
        | Op::Shl(_, _)
        | Op::Shr(_, _)
        | Op::AShr(_, _)
        | Op::Alloca { .. }
        | Op::SExt(_, _)
        | Op::ZExt(_, _)
        | Op::Trunc(_, _)
        | Op::StructIndex { .. }
        | Op::ArrayIndex { .. }
        | Op::Intrinsic { .. } => Err(BackendError::Unimplemented),
    }
}

pub fn lower_terminator(ctx: &mut LowerCtx, term: &Terminator) -> Result<(), BackendError> {
    match term {
        Terminator::Return(val) => call::ret(ctx, *val),
        Terminator::Branch(target, _args) => cmp_branch::branch(ctx, *target),
        Terminator::CondBranch {
            cond,
            t,
            f,
            t_args: _,
            f_args: _,
        } => cmp_branch::cond_branch(ctx, *cond, *t, *f),
        Terminator::Unreachable => Err(BackendError::Unimplemented),
    }
}

fn require_result(r: Option<ValueId>) -> Result<ValueId, BackendError> {
    r.ok_or(BackendError::BadIR("op requires a result value"))
}
