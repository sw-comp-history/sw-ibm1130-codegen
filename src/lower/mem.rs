//! Lower memory ops: Load, Store.
//!
//! `Load { addr, ty }`: dereference `addr` (a 16-bit pointer in a
//! slot) and store the loaded word into the result slot.
//! `Store { addr, val }`: load `val` into ACC, then store ACC into
//! the word pointed to by `addr`.
//!
//! Both use the 1130 long-form indirect bit (`LD I addr` and
//! `STO I addr`) for the indirection; this is the natural shape on
//! a word-addressed machine with no register-indirect addressing
//! mode.

use sw_codegen_core::BackendError;
use sw_tir::ValueId;

use super::LowerCtx;

pub fn load(ctx: &mut LowerCtx, addr: ValueId, result: ValueId) -> Result<(), BackendError> {
    ctx.emit_load_indirect(addr);
    ctx.emit_store(result);
    Ok(())
}

pub fn store(ctx: &mut LowerCtx, addr: ValueId, val: ValueId) -> Result<(), BackendError> {
    ctx.emit_load(val);
    ctx.emit_store_indirect(addr);
    Ok(())
}
