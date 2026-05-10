//! Snapshot-style tests for the IBM 1130 backend.
//!
//! Each test builds a small TIR `Module` by hand, lowers it, and
//! asserts on the disassembled instruction sequence. Equality
//! against an inlined string keeps the test self-describing without
//! pulling in an `insta` dev-dep; if richer diffing becomes
//! necessary later, swapping in `insta::assert_snapshot!` is a
//! one-line change per test.

use sw_codegen_core::Backend;
use sw_ibm1130_codegen::{Ibm1130Backend, lower_function, render_instrs};
use sw_tir::{
    Block, BlockId, FnSig, Function, ICmp, Instr, Linkage, Module, Op, Symbol, Terminator, Type,
    ValueId,
};

fn block(label: u32, params: Vec<u32>, instrs: Vec<Instr>, term: Terminator) -> Block {
    Block {
        label: BlockId(label),
        params: params.into_iter().map(ValueId).collect(),
        instrs,
        terminator: term,
    }
}

fn instr(result: u32, op: Op, ty: Type) -> Instr {
    Instr {
        result: Some(ValueId(result)),
        op,
        ty,
    }
}

fn instr_void(op: Op, ty: Type) -> Instr {
    Instr {
        result: None,
        op,
        ty,
    }
}

fn make_function(name: &str, blocks: Vec<Block>, ret: Type, params: Vec<Type>) -> Function {
    Function {
        name: name.to_string(),
        linkage: Linkage::External,
        signature: FnSig { params, ret },
        blocks,
        locals: Vec::new(),
    }
}

fn render_function(f: &Function) -> String {
    let ef = lower_function(f).expect("lower_function");
    render_instrs(&ef.instrs)
}

#[test]
fn lower_i16_add() {
    // fn add(%0: i16, %1: i16) -> i16 { %2 = add %0, %1; ret %2 }
    let f = make_function(
        "add",
        vec![block(
            0,
            vec![0, 1],
            vec![instr(2, Op::Add(ValueId(0), ValueId(1)), Type::I16)],
            Terminator::Return(Some(ValueId(2))),
        )],
        Type::I16,
        vec![Type::I16, Type::I16],
    );
    let actual = render_function(&f);
    let expected = "\
<Long op=Load tag=0 indirect=false address=0>
<Long op=Add tag=0 indirect=false address=1>
<Long op=Store tag=0 indirect=false address=2>
<Long op=Load tag=0 indirect=false address=2>
<Long op=BranchSkipCondition tag=0 indirect=true address=0>
";
    assert_eq!(actual, expected);
}

#[test]
fn lower_i16_sub_mul_div() {
    // %3 = sub %0, %1; %4 = mul %0, %1; %5 = sdiv %0, %1; ret %3
    let f = make_function(
        "ops",
        vec![block(
            0,
            vec![0, 1],
            vec![
                instr(3, Op::Sub(ValueId(0), ValueId(1)), Type::I16),
                instr(4, Op::Mul(ValueId(0), ValueId(1)), Type::I16),
                instr(5, Op::SDiv(ValueId(0), ValueId(1)), Type::I16),
            ],
            Terminator::Return(Some(ValueId(3))),
        )],
        Type::I16,
        vec![Type::I16, Type::I16],
    );
    let actual = render_function(&f);
    let expected = "\
<Long op=Load tag=0 indirect=false address=0>
<Long op=Subtract tag=0 indirect=false address=1>
<Long op=Store tag=0 indirect=false address=3>
<Long op=Load tag=0 indirect=false address=0>
<Long op=Multiply tag=0 indirect=false address=1>
<Long op=Store tag=0 indirect=false address=4>
<Long op=Load tag=0 indirect=false address=0>
<Long op=Divide tag=0 indirect=false address=1>
<Long op=Store tag=0 indirect=false address=5>
<Long op=Load tag=0 indirect=false address=3>
<Long op=BranchSkipCondition tag=0 indirect=true address=0>
";
    assert_eq!(actual, expected);
}

#[test]
fn lower_load() {
    // fn read(%0: ptr<i16>) -> i16 { %1 = load %0; ret %1 }
    let f = make_function(
        "read",
        vec![block(
            0,
            vec![0],
            vec![instr(
                1,
                Op::Load {
                    addr: ValueId(0),
                    ty: Type::I16,
                },
                Type::I16,
            )],
            Terminator::Return(Some(ValueId(1))),
        )],
        Type::I16,
        vec![Type::Ptr(Box::new(Type::I16))],
    );
    let actual = render_function(&f);
    let expected = "\
<Long op=Load tag=0 indirect=true address=0>
<Long op=Store tag=0 indirect=false address=1>
<Long op=Load tag=0 indirect=false address=1>
<Long op=BranchSkipCondition tag=0 indirect=true address=0>
";
    assert_eq!(actual, expected);
}

#[test]
fn lower_store() {
    // fn write(%0: ptr<i16>, %1: i16) -> void { store %0, %1; ret }
    let f = make_function(
        "write",
        vec![block(
            0,
            vec![0, 1],
            vec![instr_void(
                Op::Store {
                    addr: ValueId(0),
                    val: ValueId(1),
                },
                Type::Void,
            )],
            Terminator::Return(None),
        )],
        Type::Void,
        vec![Type::Ptr(Box::new(Type::I16)), Type::I16],
    );
    let actual = render_function(&f);
    let expected = "\
<Long op=Load tag=0 indirect=false address=1>
<Long op=Store tag=0 indirect=true address=0>
<Long op=BranchSkipCondition tag=0 indirect=true address=0>
";
    assert_eq!(actual, expected);
}

#[test]
fn lower_icmp_and_cond_branch() {
    // fn ifeq(%0, %1) -> i16 {
    //   bb0(%0, %1):
    //     %2 = icmp eq %0 %1
    //     cond_br %2, bb1, bb2
    //   bb1: ret %0
    //   bb2: ret %1
    // }
    let f = make_function(
        "ifeq",
        vec![
            block(
                0,
                vec![0, 1],
                vec![instr(
                    2,
                    Op::Icmp {
                        pred: ICmp::Eq,
                        lhs: ValueId(0),
                        rhs: ValueId(1),
                    },
                    Type::I1,
                )],
                Terminator::CondBranch {
                    cond: ValueId(2),
                    t: BlockId(1),
                    t_args: vec![],
                    f: BlockId(2),
                    f_args: vec![],
                },
            ),
            block(1, vec![], vec![], Terminator::Return(Some(ValueId(0)))),
            block(2, vec![], vec![], Terminator::Return(Some(ValueId(1)))),
        ],
        Type::I16,
        vec![Type::I16, Type::I16],
    );
    let actual = render_function(&f);
    let expected = "\
<Long op=Load tag=0 indirect=false address=0>
<Long op=Subtract tag=0 indirect=false address=1>
<Short op=BranchSkipCondition tag=0 disp=32>
<Long op=Store tag=0 indirect=false address=2>
<Long op=Load tag=0 indirect=false address=2>
<Long op=BranchSkipCondition tag=0 indirect=false address=0>
<Long op=BranchSkipCondition tag=0 indirect=false address=0>
<Long op=Load tag=0 indirect=false address=0>
<Long op=BranchSkipCondition tag=0 indirect=true address=0>
<Long op=Load tag=0 indirect=false address=1>
<Long op=BranchSkipCondition tag=0 indirect=true address=0>
";
    assert_eq!(actual, expected);
}

#[test]
fn lower_call_and_return() {
    // fn caller(%0: i16) -> i16 {
    //   %1 = addrof @callee
    //   %2 = call %1(%0)
    //   ret %2
    // }
    let f = make_function(
        "caller",
        vec![block(
            0,
            vec![0],
            vec![
                instr(
                    1,
                    Op::AddrOf(Symbol("callee".to_string())),
                    Type::Ptr(Box::new(Type::I16)),
                ),
                instr(
                    2,
                    Op::Call {
                        callee: ValueId(1),
                        args: vec![ValueId(0)],
                    },
                    Type::I16,
                ),
            ],
            Terminator::Return(Some(ValueId(2))),
        )],
        Type::I16,
        vec![Type::I16],
    );
    let actual = render_function(&f);
    let expected = "\
<Long op=Load tag=0 indirect=false address=0>
<Long op=BranchStore tag=0 indirect=false address=0>
<Long op=Store tag=0 indirect=false address=2>
<Long op=Load tag=0 indirect=false address=2>
<Long op=BranchSkipCondition tag=0 indirect=true address=0>
";
    assert_eq!(actual, expected);
}

#[test]
fn lower_function_records_fixups_for_call_and_return() {
    let f = make_function(
        "caller",
        vec![block(
            0,
            vec![0],
            vec![
                instr(
                    1,
                    Op::AddrOf(Symbol("callee".to_string())),
                    Type::Ptr(Box::new(Type::I16)),
                ),
                instr(
                    2,
                    Op::Call {
                        callee: ValueId(1),
                        args: vec![ValueId(0)],
                    },
                    Type::I16,
                ),
            ],
            Terminator::Return(Some(ValueId(2))),
        )],
        Type::I16,
        vec![Type::I16],
    );
    let ef = lower_function(&f).unwrap();
    // Two fixups expected: one for the BSI (Branch -> "callee"),
    // one for the indirect BSC return (Address -> "caller.entry").
    assert_eq!(ef.fixups.len(), 2);
    assert_eq!(ef.fixups[0].target, "callee");
    assert_eq!(ef.fixups[1].target, "caller.entry");
}

#[test]
fn backend_emit_asm_for_module() {
    // Minimal end-to-end: emit_asm round-trips a module that
    // contains a single Add-and-return function.
    let f = make_function(
        "add",
        vec![block(
            0,
            vec![0, 1],
            vec![instr(2, Op::Add(ValueId(0), ValueId(1)), Type::I16)],
            Terminator::Return(Some(ValueId(2))),
        )],
        Type::I16,
        vec![Type::I16, Type::I16],
    );
    let mut m = Module::new("m");
    m.functions.push(f);
    let mut s = String::new();
    Ibm1130Backend.emit_asm(&m, &mut s).unwrap();
    let expected = "\
add:
  <Long op=Load tag=0 indirect=false address=0>
  <Long op=Add tag=0 indirect=false address=1>
  <Long op=Store tag=0 indirect=false address=2>
  <Long op=Load tag=0 indirect=false address=2>
  <Long op=BranchSkipCondition tag=0 indirect=true address=0>
";
    assert_eq!(s, expected);
}
