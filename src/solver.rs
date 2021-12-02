use super::Trace;
use z3::{
    ast::{self as z3_ast, Ast},
    Solver,
};

struct ProblemContext<'a> {
    banks: Vec<z3_ast::Array<'a>>,
    routing_fns: Vec<z3::FuncDecl<'a>>,
    addr_size: u32,
}

impl<'a> ProblemContext<'a> {
    fn map_addr(&self, input_index: &z3_ast::Int<'a>, bank_idx: usize) -> z3_ast::Int<'a> {
        let req_bv = z3_ast::BV::from_int(input_index, self.addr_size);
        let addr_map_of_bv = self.routing_fns[bank_idx]
            .apply(&[&req_bv])
            .as_bv()
            .unwrap();
        let integer_output = z3_ast::Int::from_bv(&addr_map_of_bv, false);
        self.banks[bank_idx]
            .select(&integer_output)
            .as_int()
            .unwrap()
    }
}

pub fn solve_trace(input: &Trace) {
    let addr_size = input.bits_required();
    let mut ctx = z3::Context::new(&z3::Config::default());
    let mut solver = z3::Optimize::new(&ctx);

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
        .map(|x| {
            z3::FuncDecl::new(
                &ctx,
                format!("map_addr_{}", x),
                &[&z3::Sort::bitvector(&ctx, addr_size)],
                &z3::Sort::bitvector(&ctx, addr_size),
            )
        })
        .collect::<Vec<_>>();

    let prob_ctx = ProblemContext {
        banks,
        routing_fns,
        addr_size,
    };

    for line in input.iter() {
        for (bank_idx, request) in line.iter().enumerate() {
            if let Some(request_index) = request {
                let req_int = z3_ast::Int::from_u64(&ctx, *request_index as u64);
                let index_maps_to = prob_ctx.map_addr(&req_int, bank_idx);

                let index_correctness_bool = index_maps_to._eq(&req_int);
                solver.minimize(&index_maps_to);
                solver.assert(&index_correctness_bool);
                solver.assert(&index_maps_to.le(&z3_ast::Int::from_u64(&ctx, input.size() as u64)));
            }
        }
    }

    for i in 0..input.size() {
        let req_int = z3_ast::Int::from_u64(&ctx, i as u64);

        let bools = (0..input.num_ports())
            .into_iter()
            .map(|bank_idx| {
                let index_maps_to = prob_ctx.map_addr(&req_int, bank_idx);
                index_maps_to._eq(&req_int)
            })
            .collect::<Vec<_>>();
        let borrow_bools = bools.iter().collect::<Vec<_>>();

        solver.assert(&z3_ast::Bool::or(&ctx, &borrow_bools));
    }

    solver.check(&[]);
    println!("{:?}", solver.get_model().unwrap())
}
