use super::Trace;
use z3::{
    ast::{self as z3_ast, Ast, Bool, Datatype, Int, BV},
    DatatypeAccessor, DatatypeBuilder, DatatypeSort, Solver, Sort,
};

struct ProblemContext<'a> {
    banks: Vec<z3_ast::Array<'a>>,
    routing_fns: Vec<Datatype<'a>>,
    addr_size: u32,
    terminals: DatatypeSort<'a>,
}

impl<'a> ProblemContext<'a> {
    fn map_addr(
        &self,
        input_index: &z3_ast::Int<'a>,
        bank_idx: usize,
        ctx: &'a z3::Context,
    ) -> (Bool<'a>, z3_ast::Int<'a>) {
        let (out, cond) =
            self.apply_terminal(input_index, bank_idx, &self.routing_fns[bank_idx], ctx);

        (cond, self.banks[bank_idx].select(&out).as_int().unwrap())
    }

    fn apply_terminal(
        &self,
        input_index: &z3_ast::Int<'a>,
        bank_idx: usize,
        datatype: &Datatype<'a>,
        ctx: &'a z3::Context,
    ) -> (Int<'a>, Bool<'a>) {
        assert_eq!(datatype.get_sort(), self.terminals.sort);
        let out = Int::new_const(
            ctx,
            format!("out_{}_{}", bank_idx, input_index.as_u64().unwrap()),
        );
        let in_bv = BV::from_int(input_index, self.addr_size);
        let out_bv = BV::from_int(&out, self.addr_size);
        let bools = vec![
            // No Op
            {
                let test = self.terminals.variants[0]
                    .tester
                    .apply(&[datatype])
                    .as_bool()
                    .unwrap();
                (!test) | in_bv.to_int(false)._eq(&out)
            },
            // Shift Right
            {
                let test = self.terminals.variants[1]
                    .tester
                    .apply(&[datatype])
                    .as_bool()
                    .unwrap();

                let shifted = in_bv
                    .bvlshr(
                        &self.terminals.variants[1].accessors[0]
                            .apply(&[datatype])
                            .as_bv()
                            .unwrap(),
                    )
                    .to_int(false);

                (!test) | shifted._eq(&out)
            },
            // ADD
            {
                let test = self.terminals.variants[2]
                    .tester
                    .apply(&[datatype])
                    .as_bool()
                    .unwrap();
                let held_int = self.terminals.variants[2].accessors[0]
                    .apply(&[datatype])
                    .as_bv()
                    .unwrap();

                (!test) | ((&held_int + &in_bv)._eq(&out_bv))
            },
            // SUB PV
            {
                let test = self.terminals.variants[3]
                    .tester
                    .apply(&[datatype])
                    .as_bool()
                    .unwrap();
                let held_int = self.terminals.variants[3].accessors[0]
                    .apply(&[datatype])
                    .as_bv()
                    .unwrap();

                (!test) | ((&in_bv - &held_int)._eq(&out_bv))
            },
            // SUB VP
            {
                let test = self.terminals.variants[4]
                    .tester
                    .apply(&[datatype])
                    .as_bool()
                    .unwrap();
                let held_int = self.terminals.variants[4].accessors[0]
                    .apply(&[datatype])
                    .as_bv()
                    .unwrap();

                (!test) | (&(&held_int - &in_bv)._eq(&out_bv))
            },
            // CONST
            {
                let test = self.terminals.variants[5]
                    .tester
                    .apply(&[datatype])
                    .as_bool()
                    .unwrap();
                let held_int = self.terminals.variants[5].accessors[0]
                    .apply(&[datatype])
                    .as_bv()
                    .unwrap();

                (!test) | held_int._eq(&out_bv)
            },
        ];

        let variants_test = (0_usize..=5)
            .map(|x| {
                self.terminals.variants[x]
                    .tester
                    .apply(&[datatype])
                    .as_bool()
                    .unwrap()
            })
            .collect::<Vec<_>>();

        let b = Bool::and(ctx, &bools.iter().collect::<Vec<_>>());
        let c = Bool::or(ctx, &variants_test.iter().collect::<Vec<_>>());
        (out, b & c)
    }
}

fn terminal_routing_program(ctx: &z3::Context, size: u32) -> z3::DatatypeSort {
    let terminal = DatatypeBuilder::new(ctx, "TerminalProgram")
        .variant("NOOP", vec![])
        .variant(
            "RShift",
            vec![(
                "rshift_v",
                DatatypeAccessor::Sort(Sort::bitvector(ctx, size)),
            )],
        )
        .variant(
            "Add",
            vec![("add_v", DatatypeAccessor::Sort(Sort::bitvector(ctx, size)))],
        )
        .variant(
            "SubPV",
            vec![(
                "subpv_v",
                DatatypeAccessor::Sort(Sort::bitvector(ctx, size)),
            )],
        )
        .variant(
            "SubVP",
            vec![(
                "subvp_v",
                DatatypeAccessor::Sort(Sort::bitvector(ctx, size)),
            )],
        )
        .variant(
            "Constant",
            vec![(
                "constant_v",
                DatatypeAccessor::Sort(Sort::bitvector(ctx, size)),
            )],
        )
        .finish();
    terminal
}

pub fn solve_trace(input: &Trace) {
    let addr_size = input.bits_required();
    let mut ctx = z3::Context::new(&z3::Config::default());
    let mut solver = z3::Solver::new(&ctx);

    let terminal_rprogs = terminal_routing_program(&ctx, addr_size);

    let banks = (0..input.num_ports())
        .map(|i| {
            z3_ast::Array::new_const(
                &ctx,
                format!("bank_{}", i),
                &z3::Sort::int(&ctx),
                &z3::Sort::int(&ctx),
            )
        })
        .collect::<Vec<_>>();

    let routing_fns = (0..input.num_ports())
        .map(|x| Datatype::new_const(&ctx, format!("map_addr_{}", x), &terminal_rprogs.sort))
        .collect::<Vec<_>>();

    let prob_ctx = ProblemContext {
        banks,
        routing_fns,
        addr_size,
        terminals: terminal_rprogs,
    };

    for line in input.iter() {
        for (bank_idx, request) in line.iter().enumerate() {
            if let Some(request_index) = request {
                let req_int = z3_ast::Int::from_u64(&ctx, *request_index as u64);
                let (cond1, index_maps_to) = prob_ctx.map_addr(&req_int, bank_idx, &ctx);

                let index_correctness_bool = index_maps_to._eq(&req_int);
                solver.assert(&cond1);
                solver.assert(&index_correctness_bool);
                solver
                    .assert(&(index_maps_to.lt(&z3_ast::Int::from_u64(&ctx, input.size() as u64))));
                solver.assert(&index_maps_to.ge(&Int::from_u64(&ctx, 0)));
            }
        }
    }
    solver.check();

    for i in 0..input.size() {
        let req_int = z3_ast::Int::from_u64(&ctx, i as u64);
        println!("{:?}", i);

        let bools = (0..input.num_ports())
            .into_iter()
            .map(|bank_idx| {
                let (cond, index_maps_to) = prob_ctx.map_addr(&req_int, bank_idx, &ctx);
                cond & index_maps_to._eq(&req_int)
                    & index_maps_to.ge(&Int::from_u64(&ctx, 0))
                    & index_maps_to.lt(&z3_ast::Int::from_u64(&ctx, input.size() as u64))
            })
            .collect::<Vec<_>>();
        let borrow_bools = bools.iter().collect::<Vec<_>>();

        solver.assert(&z3_ast::Bool::or(&ctx, &borrow_bools));
    }

    solver.check();

    println!("{:?}", solver);

    println!("{:?}", solver.get_model().unwrap())
}
