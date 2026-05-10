//! Codegen lowering helpers for the IBM 1130 backend.
//!
//! See `lib.rs` for the strategy summary. This module owns
//! `LowerCtx` (the running state of a single function lowering) and
//! re-exports the per-IR-op submodules.

pub mod arith;
pub mod call;
pub mod cmp_branch;
pub mod mem;

use sw_codegen_core::{Fixup, FixupKind};
use sw_ibm1130_isa::{Instruction, Opcode};
use sw_tir::{BlockId, Function, ValueId};

/// Context for a single function being lowered.
///
/// Naive slot-based lowering: each TIR `ValueId(N)` is bound to
/// memory word slot `N` within the function frame. Address fields
/// in emitted instructions reference these slot indices directly;
/// a future linker / allocator pass will rewrite them to real
/// frame offsets.
pub struct LowerCtx<'a> {
    pub func: &'a Function,
    pub instrs: Vec<Instruction>,
    pub fixups: Vec<Fixup>,
    pub frame_size: u32,
    pub current_block: Option<BlockId>,
}

impl<'a> LowerCtx<'a> {
    pub fn new(func: &'a Function) -> Self {
        let max_value = func
            .blocks
            .iter()
            .flat_map(|b| {
                let params = b.params.iter().map(|v| v.0);
                let results = b.instrs.iter().filter_map(|i| i.result.map(|v| v.0));
                params.chain(results)
            })
            .max()
            .unwrap_or(0);
        Self {
            func,
            instrs: Vec::new(),
            fixups: Vec::new(),
            frame_size: max_value + 1,
            current_block: None,
        }
    }

    pub fn enter_block(&mut self, label: BlockId) {
        self.current_block = Some(label);
    }

    /// Word-address slot bound to `v`.
    pub fn slot(v: ValueId) -> u16 {
        v.0 as u16
    }

    pub fn emit_short(&mut self, op: Opcode, tag: u8, disp: i8) {
        self.instrs.push(Instruction::Short { op, tag, disp });
    }

    pub fn emit_long(&mut self, op: Opcode, tag: u8, indirect: bool, address: u16) {
        self.instrs.push(Instruction::Long {
            op,
            tag,
            indirect,
            address,
        });
    }

    pub fn emit_load(&mut self, v: ValueId) {
        self.emit_long(Opcode::Load, 0, false, Self::slot(v));
    }

    pub fn emit_store(&mut self, v: ValueId) {
        self.emit_long(Opcode::Store, 0, false, Self::slot(v));
    }

    pub fn emit_load_indirect(&mut self, ptr: ValueId) {
        self.emit_long(Opcode::Load, 0, true, Self::slot(ptr));
    }

    pub fn emit_store_indirect(&mut self, ptr: ValueId) {
        self.emit_long(Opcode::Store, 0, true, Self::slot(ptr));
    }

    /// Cumulative byte size of all instructions emitted so far. Each
    /// short form is 2 bytes; each long form is 4 bytes. Used to
    /// place fixups at byte offsets per the codegen-core trait.
    pub fn current_byte_offset(&self) -> usize {
        self.instrs
            .iter()
            .map(|i| match i {
                Instruction::Short { .. } => 2,
                Instruction::Long { .. } => 4,
            })
            .sum()
    }

    /// Add a fixup pointing at the address word of the *most-recently
    /// emitted* long-form instruction. The address word is the
    /// second half of a 4-byte long instruction; offset = (current
    /// byte offset) - 2.
    pub fn add_fixup_to_last(&mut self, kind: FixupKind, target: impl Into<String>) {
        debug_assert!(matches!(self.instrs.last(), Some(Instruction::Long { .. })));
        let at = self.current_byte_offset() - 2;
        self.fixups.push(Fixup {
            at,
            kind,
            target: target.into(),
        });
    }
}
