//! `sw-ibm1130-codegen`: IBM 1130 backend.
//!
//! Implements `sw_codegen_core::Backend` for the IBM 1130 by hand-
//! writing per-TIR-op lowering. There is no register allocator yet --
//! every TIR `ValueId` is bound to a memory word slot in the
//! function's frame, and ACC is the single computational register.
//! A future allocator pass (saga step beyond this bring-up) will
//! rewrite slot references to real frame offsets.
//!
//! The lowering follows the historical IBM 1130 calling convention
//! documented in `sw-ibm1130-target/docs/abi.md`.

pub mod lower;
pub mod select;

use sw_codegen_core::{Backend, BackendError, EmittedFunction, EmittedGlobal, Object};
use sw_ibm1130_isa::Ibm1130;
use sw_ibm1130_target::Ibm1130Target;
use sw_isa_core::Architecture;
use sw_tir::{Function, Module};

/// IBM 1130 backend.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Default)]
pub struct Ibm1130Backend;

impl Backend for Ibm1130Backend {
    type Target = Ibm1130Target;

    fn lower_module(&self, m: &Module) -> Result<Object<Self::Target>, BackendError> {
        let functions = m
            .functions
            .iter()
            .map(lower_function)
            .collect::<Result<Vec<_>, _>>()?;
        let globals = m
            .globals
            .iter()
            .map(|g| EmittedGlobal {
                name: g.name.clone(),
                bytes: Vec::new(),
            })
            .collect();
        Ok(Object { functions, globals })
    }

    fn emit_asm(&self, m: &Module, w: &mut dyn core::fmt::Write) -> Result<(), BackendError> {
        for f in &m.functions {
            let ef = lower_function(f)?;
            writeln!(w, "{}:", ef.name).map_err(|_| BackendError::EncodeFailure)?;
            for insn in &ef.instrs {
                w.write_str("  ").map_err(|_| BackendError::EncodeFailure)?;
                <Ibm1130 as Architecture>::disassemble(insn, w)
                    .map_err(|_| BackendError::EncodeFailure)?;
                writeln!(w).map_err(|_| BackendError::EncodeFailure)?;
            }
        }
        Ok(())
    }
}

/// Lower a single TIR function. Public so tests can drive a function
/// in isolation.
pub fn lower_function(f: &Function) -> Result<EmittedFunction<Ibm1130Target>, BackendError> {
    let mut ctx = lower::LowerCtx::new(f);
    for block in &f.blocks {
        ctx.enter_block(block.label);
        for instr in &block.instrs {
            select::dispatch(&mut ctx, instr)?;
        }
        select::lower_terminator(&mut ctx, &block.terminator)?;
    }
    Ok(EmittedFunction {
        name: f.name.clone(),
        instrs: ctx.instrs,
        fixups: ctx.fixups,
        frame_size: ctx.frame_size,
    })
}

/// Render a sequence of IBM 1130 instructions as one-line-per-instr
/// disassembly text. Convenience for snapshot tests.
pub fn render_instrs(instrs: &[sw_ibm1130_isa::Instruction]) -> String {
    use core::fmt::Write;
    let mut s = String::new();
    for insn in instrs {
        let mut line = String::new();
        let _ = <Ibm1130 as Architecture>::disassemble(insn, &mut line);
        let _ = writeln!(s, "{line}");
    }
    s
}
