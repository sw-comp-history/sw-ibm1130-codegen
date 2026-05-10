//! Lower arithmetic ops: Add, Sub, Mul, SDiv.
//!
//! Pattern: `LD lhs_slot ; <OP> rhs_slot ; STO result_slot`.
//! ACC is the implicit accumulator. For Mul and SDiv the result
//! pair (ACC, EXT) is produced; we keep the low half in ACC for
//! the i16 case. A future fix-up will spill EXT for i32 results.

use sw_codegen_core::BackendError;
use sw_ibm1130_isa::Opcode;
use sw_tir::ValueId;

use super::LowerCtx;

fn binop(ctx: &mut LowerCtx, op: Opcode, lhs: ValueId, rhs: ValueId, result: ValueId) {
    ctx.emit_load(lhs);
    ctx.emit_long(op, 0, false, LowerCtx::slot(rhs));
    ctx.emit_store(result);
}

pub fn add(
    ctx: &mut LowerCtx,
    lhs: ValueId,
    rhs: ValueId,
    result: ValueId,
) -> Result<(), BackendError> {
    binop(ctx, Opcode::Add, lhs, rhs, result);
    Ok(())
}

pub fn sub(
    ctx: &mut LowerCtx,
    lhs: ValueId,
    rhs: ValueId,
    result: ValueId,
) -> Result<(), BackendError> {
    binop(ctx, Opcode::Subtract, lhs, rhs, result);
    Ok(())
}

pub fn mul(
    ctx: &mut LowerCtx,
    lhs: ValueId,
    rhs: ValueId,
    result: ValueId,
) -> Result<(), BackendError> {
    // M produces a 32-bit result in (ACC, EXT). For i16 we keep ACC
    // and discard EXT; for i32 codegen would store the pair.
    binop(ctx, Opcode::Multiply, lhs, rhs, result);
    Ok(())
}

pub fn sdiv(
    ctx: &mut LowerCtx,
    lhs: ValueId,
    rhs: ValueId,
    result: ValueId,
) -> Result<(), BackendError> {
    // D divides (ACC, EXT) by M; quotient -> ACC, remainder -> EXT.
    // For i16 lhs the high half (EXT) should be sign-extended; that
    // codegen detail is left for the postmortem step.
    binop(ctx, Opcode::Divide, lhs, rhs, result);
    Ok(())
}
