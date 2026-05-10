//! Lower comparison and branch ops: Icmp, Branch, CondBranch.
//!
//! Status: shape-only. The 1130's BSC instruction tests a condition
//! mask in its low displacement bits; the encoding of those masks
//! per ICmp predicate is captured in `cmp_mask` below as a first
//! cut. Exact mask semantics will be revisited during the emulator
//! step (saga step 11) when round-trip behaviour drives the
//! definitive mapping.

use sw_codegen_core::{BackendError, FixupKind};
use sw_ibm1130_isa::Opcode;
use sw_tir::{BlockId, ICmp, ValueId};

use super::LowerCtx;

/// Map a TIR ICmp predicate to a 1130 BSC condition-mask byte.
///
/// 1130 BSC condition bits (from the FC manual): bit positions in
/// the displacement field map to Z (zero), N (minus), P (plus), E
/// (even), C (carry), V (overflow). For our shape-level codegen we
/// emit a mask that *names* the test; the emulator step will refine.
fn cmp_mask(pred: ICmp) -> i8 {
    match pred {
        ICmp::Eq => 0x20,              // Z
        ICmp::Ne => 0x40,              // not Z (placeholder)
        ICmp::Slt | ICmp::Ult => 0x10, // N
        ICmp::Sgt | ICmp::Ugt => 0x08, // P
        ICmp::Sle | ICmp::Ule => 0x18, // P or Z (le ~ not gt)
        ICmp::Sge | ICmp::Uge => 0x30, // N or Z (ge ~ not lt)
    }
}

pub fn icmp(
    ctx: &mut LowerCtx,
    pred: ICmp,
    lhs: ValueId,
    rhs: ValueId,
    result: ValueId,
) -> Result<(), BackendError> {
    // Compute lhs - rhs in ACC; the resulting flags drive a follow-on
    // BSC. The ACC value (the difference) is *also* stored as the i1
    // result slot for now -- an "is the difference matching the
    // predicate?" boolean materialisation is left to a later peephole.
    ctx.emit_load(lhs);
    ctx.emit_long(Opcode::Subtract, 0, false, LowerCtx::slot(rhs));
    let mask = cmp_mask(pred);
    ctx.emit_short(Opcode::BranchSkipCondition, 0, mask);
    ctx.emit_store(result);
    Ok(())
}

pub fn branch(ctx: &mut LowerCtx, target: BlockId) -> Result<(), BackendError> {
    // Unconditional long-form BSC with mask 0 (no skip condition).
    // Address is a placeholder; a fixup names the destination block
    // by symbolic label so a later branch-relaxation pass can
    // rewrite to short form when the offset is small enough.
    ctx.emit_long(Opcode::BranchSkipCondition, 0, false, 0);
    ctx.add_fixup_to_last(FixupKind::Branch, format!("blk{}", target.0));
    Ok(())
}

pub fn cond_branch(
    ctx: &mut LowerCtx,
    cond: ValueId,
    t: BlockId,
    f: BlockId,
) -> Result<(), BackendError> {
    // Load cond into ACC; emit a BSC long that branches to f-target
    // when ACC is zero (mask Z = 0x20), then an unconditional BSC
    // long to t-target. The "fall-through" semantics of BSC mean a
    // taken-when-zero branch jumps if the condition was false.
    ctx.emit_load(cond);
    ctx.emit_long(Opcode::BranchSkipCondition, 0, false, 0);
    ctx.add_fixup_to_last(FixupKind::Branch, format!("blk{}", f.0));
    ctx.emit_long(Opcode::BranchSkipCondition, 0, false, 0);
    ctx.add_fixup_to_last(FixupKind::Branch, format!("blk{}", t.0));
    Ok(())
}
