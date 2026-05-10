//! Lower call and return ops.
//!
//! Calling convention (from `sw-ibm1130-target/docs/abi.md` Sec 3-4):
//!
//! - First scalar arg in **ACC**.
//! - Subsequent args in-line as DC words after the BSI; the callee
//!   reads them through its return-address slot.
//! - Scalar return in ACC.
//! - Return via `BSC I NAME` (indirect through entry word). Step-9
//!   codegen emits a placeholder for the entry-word address; a
//!   later assembler pass binds it to the function symbol.
//!
//! Inline DC parameters are represented here as `DC_inline_word`
//! placeholder long-forms whose address field carries the slot index
//! of the argument value. A future asm pass will lower these to real
//! `DC` directives. They are *not* legal 1130 instructions on their
//! own and must not survive into a binary -- this is intermediate
//! shape only.

use sw_codegen_core::{BackendError, FixupKind};
use sw_ibm1130_isa::Opcode;
use sw_tir::ValueId;

use super::LowerCtx;

pub fn call(
    ctx: &mut LowerCtx,
    callee_name: &str,
    args: &[ValueId],
    result: Option<ValueId>,
) -> Result<(), BackendError> {
    // Load first scalar arg into ACC, if any.
    if let Some(first) = args.first() {
        ctx.emit_load(*first);
    }
    // BSI long with placeholder; fixup names the callee.
    ctx.emit_long(Opcode::BranchStore, 0, false, 0);
    ctx.add_fixup_to_last(FixupKind::Branch, callee_name.to_string());
    // Inline DC words for remaining args. We pseudo-encode each as a
    // "Wait" long-form (an arbitrary opcode we never expect to
    // execute) carrying the slot index in its address field. A later
    // asm pass will rewrite these to proper DC directives.
    for arg in args.iter().skip(1) {
        ctx.emit_long(Opcode::Wait, 0, false, LowerCtx::slot(*arg));
    }
    // Store the returned ACC into the result slot if a result is
    // expected.
    if let Some(r) = result {
        ctx.emit_store(r);
    }
    Ok(())
}

pub fn ret(ctx: &mut LowerCtx, val: Option<ValueId>) -> Result<(), BackendError> {
    if let Some(v) = val {
        ctx.emit_load(v);
    }
    // Indirect BSC through entry word: `BSC I NAME`. Address is a
    // placeholder; a fixup names the current function's entry-word
    // symbol so a later asm pass can resolve it.
    ctx.emit_long(Opcode::BranchSkipCondition, 0, true, 0);
    let entry_symbol = format!("{}.entry", ctx.func.name);
    ctx.add_fixup_to_last(FixupKind::Address, entry_symbol);
    Ok(())
}
